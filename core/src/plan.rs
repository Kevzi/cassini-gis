//! Plan : suite de commandes que l'agent propose, vérifiée avant tout envoi.
//!
//! Une étape peut citer la sortie d'une étape précédente par une chaîne, valeur d'un
//! paramètre de premier niveau : `step:<k>` (k à partir de 1) pour la sortie entière, ou
//! `step:<k>.<champ>` pour un champ de premier niveau de cette sortie (le type du paramètre
//! est alors comparé au schéma de ce champ). Seule exception au premier niveau : un élément
//! d'un tableau de `Point` peut être `step:<k>.<champ>` vers un `Point` (amendement d'AD-23,
//! 2026-11-08, ticket 11.22). Cette convention est isolée ici pour pouvoir être remplacée.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::raster_path::{is_raster_path, writes_one_raster};
use crate::registry::{CommandSpec, Registry};

/// Une étape : une commande et ses paramètres (éventuellement des références).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedCall {
    pub command: String,
    pub params: Value,
}

/// Ce que l'agent rend : des appels à faire, un refus motivé (aucune commande ne fait la
/// chose), ou une question (la commande existe, mais un mot de la demande ne désigne rien
/// de l'inventaire, ou un point du plan ne vient pas de la demande). Une question n'est ni
/// gardée ni exécutée : la réponse de l'utilisateur est une nouvelle demande.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Plan {
    Calls {
        calls: Vec<PlannedCall>,
    },
    Refusal {
        reason: String,
    },
    Question {
        question: String,
        /// Provenance de la question (AD-23), hors de la grammaire du modèle : jamais lue
        /// d'un JSON, donc toujours `model` pour une sortie du modèle ; omise à l'écriture
        /// quand elle vaut `model`, pour que les complétions d'entraînement restent
        /// identiques à l'octet.
        #[serde(
            default,
            skip_deserializing,
            skip_serializing_if = "QuestionSource::is_model"
        )]
        source: QuestionSource,
    },
}

/// D'où vient une question : du modèle, ou d'un garde-fou du noyau, sur les coordonnées
/// (`point::guard_coordinates`) ou sur les attributs (`attribute_guard::guard_attributes`,
/// ticket 5.57). Jamais écrite au journal (AD-11).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionSource {
    #[default]
    Model,
    Guard,
}

impl QuestionSource {
    /// Vrai pour une question du modèle.
    pub fn is_model(&self) -> bool {
        *self == QuestionSource::Model
    }

    /// Nom stable de la provenance : `model` ou `guard`.
    pub fn as_str(&self) -> &'static str {
        match self {
            QuestionSource::Model => "model",
            QuestionSource::Guard => "guard",
        }
    }
}

/// Un plan refusé : code stable, message français, étape fautive (à partir de 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanError {
    pub code: &'static str,
    pub message: String,
    pub step: usize,
}

fn error(code: &'static str, step: usize, message: String) -> PlanError {
    PlanError {
        code,
        message,
        step,
    }
}

/// Référence lue : numéro d'étape et champ de premier niveau éventuel.
pub(crate) type Reference = (usize, Option<String>);

/// Lit une valeur de paramètre : `None` si ce n'est pas une référence,
/// `Some(Err)` si elle commence par `step:` mais est mal formée.
pub(crate) fn parse_reference(value: &Value) -> Option<Result<Reference, String>> {
    let text = value.as_str()?.strip_prefix("step:")?;
    let (number, field) = match text.split_once('.') {
        Some((n, f)) => (n, Some(f)),
        None => (text, None),
    };
    let bad = || {
        format!(
            "la référence « step:{text} » est mal formée : seules les formes `step:<numéro>` et `step:<numéro>.<champ>` (numéro à partir de 1, un seul champ) sont permises."
        )
    };
    let k = number
        .parse::<usize>()
        .ok()
        .filter(|k| {
            *k >= 1 && !number.starts_with('0') && number.bytes().all(|b| b.is_ascii_digit())
        })
        .ok_or_else(bad);
    let field = match field {
        None => Ok(None),
        Some(f) if f.is_empty() || f.contains('.') => Err(bad()),
        Some(f) => Ok(Some(f.to_string())),
    };
    Some(k.and_then(|k| field.map(|f| (k, f))))
}

/// Premier texte imbriqué dans une liste ou un objet `value` qui est une référence `step:`,
/// bien ou mal formée ; `None` pour une valeur de premier niveau ou sans référence imbriquée.
fn nested_reference(value: &Value) -> Option<&str> {
    let children: Box<dyn Iterator<Item = &Value>> = match value {
        Value::Array(items) => Box::new(items.iter()),
        Value::Object(map) => Box::new(map.values()),
        _ => return None,
    };
    for child in children {
        if parse_reference(child).is_some() {
            return child.as_str();
        }
        if let Some(text) = nested_reference(child) {
            return Some(text);
        }
    }
    None
}

/// Traduit, dans les paramètres d'une étape de `spec`, chaque référence `step:k` d'un paramètre
/// [`RasterPath`](crate::raster_path::RasterPath) en repère `out:<j>.1`, où `j` est la ligne du
/// journal de l'étape `k` du plan (`journal_steps[k - 1]`) : les ouvertures dictées et les appels
/// d'avant le plan décalent les numéros. Appliquée avant [`resolve_references`], qui laisse alors
/// le repère intact ; le journal le garde, le rejeu le résout vers son dossier (AD-11). Toute
/// autre valeur reste intacte ; une référence que le plan vérifié ne permet pas reste aussi,
/// pour que `resolve_references` rende son erreur typée.
pub fn raster_markers(spec: &CommandSpec, params: &Value, journal_steps: &[usize]) -> Value {
    let Value::Object(map) = params else {
        return params.clone();
    };
    let mut out = map.clone();
    for (name, value) in map {
        let Some(Ok((k, None))) = parse_reference(value) else {
            continue;
        };
        let Some(schema) = param_schema(&spec.params_schema, name) else {
            continue;
        };
        if !is_raster_path(&schema, &spec.params_schema) {
            continue;
        }
        if let Some(line) = journal_steps.get(k - 1) {
            out.insert(name.clone(), Value::String(format!("out:{line}.1")));
        }
    }
    Value::Object(out)
}

/// Remplace, dans les paramètres de l'étape `step`, chaque référence de premier niveau, ou
/// élément d'un tableau (le plan vérifié ne l'admet que dans un tableau de `Point`), par la
/// valeur obtenue : `step:<k>` par la sortie entière de l'étape `k`, `step:<k>.<champ>` par ce
/// champ de sa sortie. `outputs` porte les sorties déjà obtenues, l'étape 1 en premier. Une
/// valeur qui n'est pas une référence reste intacte. Une référence mal formée, vers une étape
/// non encore exécutée, ou vers un champ absent de la sortie rend une erreur typée : rien n'est
/// deviné.
pub fn resolve_references(
    params: &Value,
    outputs: &[Value],
    step: usize,
) -> Result<Value, PlanError> {
    let Value::Object(map) = params else {
        return Ok(params.clone());
    };
    let resolve = |name: &str, value: &Value| -> Result<Value, PlanError> {
        let Some(parsed) = parse_reference(value) else {
            return Ok(value.clone());
        };
        let (k, field) = parsed.map_err(|m| error("plan.bad_reference", step, m))?;
        let output = outputs.get(k - 1).filter(|_| k < step).ok_or_else(|| {
            error(
                "plan.bad_reference",
                step,
                format!("« step:{k} » ne désigne pas une étape déjà exécutée."),
            )
        })?;
        match field {
            None => Ok(output.clone()),
            Some(f) => output.get(&f).cloned().ok_or_else(|| {
                error(
                    "plan.missing_output",
                    step,
                    format!(
                        "La sortie de l'étape {k} n'a pas de champ « {f} » : le paramètre « {name} » ne peut pas être résolu."
                    ),
                )
            }),
        }
    };
    let mut resolved = Map::new();
    for (name, value) in map {
        let new_value = match value {
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .map(|item| resolve(name, item))
                    .collect::<Result<_, _>>()?,
            ),
            other => resolve(name, other)?,
        };
        resolved.insert(name.clone(), new_value);
    }
    Ok(Value::Object(resolved))
}

impl Registry {
    /// Vérifie un plan sans rien exécuter : commandes connues, références vers une
    /// étape précédente existante, types compatibles ; paramètres sans référence
    /// validés par leur type. Une étape qui porte une référence n'est validée
    /// qu'à l'exécution, quand la valeur est résolue.
    pub fn check_plan(&self, plan: &Plan) -> Result<(), PlanError> {
        let Plan::Calls { calls } = plan else {
            return Ok(());
        };
        for (index, call) in calls.iter().enumerate() {
            let step = index + 1;
            let spec = self.spec(&call.command).ok_or_else(|| {
                error(
                    "plan.unknown_command",
                    step,
                    format!("La commande « {} » n'existe pas.", call.command),
                )
            })?;
            // Les paramètres tenus par une référence de premier niveau : leur valeur n'est connue
            // qu'à l'exécution. Un élément `step:k.point` d'un tableau de `Point` est remplacé,
            // pour la validation seule, par un point quelconque : le tableau garde sa longueur
            // et ses points littéraux restent validés.
            let mut referenced: Vec<String> = Vec::new();
            let mut checked = call.params.clone();
            if let Value::Object(map) = &call.params {
                for (name, value) in map {
                    // Une référence `step:k.<champ>` est admise comme élément d'un tableau de
                    // `Point`, et seulement là (amendement d'AD-23, 2026-11-08) : elle y désigne
                    // un `Point` rendu par une étape précédente, contrôlé comme au premier niveau.
                    let point_items = param_schema(&spec.params_schema, name)
                        .and_then(|s| point_items_schema(&s, &spec.params_schema));
                    if let (Some(items), Value::Array(elements)) = (&point_items, value) {
                        let mut stand_ins = Vec::with_capacity(elements.len());
                        for element in elements {
                            if let Some(parsed) = parse_reference(element) {
                                let (k, field) =
                                    parsed.map_err(|m| error("plan.bad_reference", step, m))?;
                                self.check_reference(calls, step, spec, name, items, k, &field)?;
                                stand_ins.push(STAND_IN_POINT.clone());
                            } else if let Some(text) = nested_reference(element) {
                                return Err(nested_error(step, text, name));
                            } else {
                                stand_ins.push(element.clone());
                            }
                        }
                        checked[name.as_str()] = Value::Array(stand_ins);
                        continue;
                    }
                    // Ailleurs, une référence ne vaut qu'au premier niveau : imbriquée dans une
                    // liste ou un objet (`criteria[i].raster`, `exclusions[i]`), elle n'est ni
                    // traduite ni résolue, et finirait en « fichier introuvable » à l'exécution.
                    if let Some(text) = nested_reference(value) {
                        return Err(nested_error(step, text, name));
                    }
                    let Some(parsed) = parse_reference(value) else {
                        continue;
                    };
                    referenced.push(name.clone());
                    let (k, field) = parsed.map_err(|m| error("plan.bad_reference", step, m))?;
                    if k >= step {
                        return Err(previous_error(step, k));
                    }
                    let source = self.cited_spec(calls, step, k)?;
                    let expected = param_schema(&spec.params_schema, name).ok_or_else(|| {
                        error(
                            "registry.invalid_params",
                            step,
                            format!(
                                "Paramètres invalides (détail technique : le champ « {name} » est inconnu de {}).",
                                spec.id
                            ),
                        )
                    })?;
                    // Un raster se désigne par `step:k`, l'étape k écrivant exactement un raster
                    // (amendement du 2026-11-05 d'AD-24) ; jamais par un champ de sa sortie, qui
                    // ferait journaliser un chemin résolu au lieu du repère `out:k.n`.
                    if is_raster_path(&expected, &spec.params_schema) {
                        if field.is_some() {
                            return Err(error(
                                "plan.bad_reference",
                                step,
                                format!(
                                    "le paramètre « {name} » de {} est un raster : désignez le raster par step:{k}, sans champ.",
                                    spec.id
                                ),
                            ));
                        }
                        if !writes_one_raster(source) {
                            return Err(error(
                                "plan.type_mismatch",
                                step,
                                format!(
                                    "Le paramètre « {name} » de {} attend un raster, mais l'étape {k} ({}) n'en écrit pas.",
                                    spec.id, source.id
                                ),
                            ));
                        }
                        continue;
                    }
                    self.check_reference(calls, step, spec, name, &expected, k, &field)?;
                }
            }
            self.validate_unreferenced(&call.command, &checked, spec, &referenced)
                .map_err(|e| {
                    // `validate_params` ne rend que ces deux causes.
                    let code = match e {
                        crate::registry::RegistryError::UnknownCommand(_) => {
                            "registry.unknown_command"
                        }
                        _ => "registry.invalid_params",
                    };
                    error(code, step, e.to_string())
                })?;
        }
        Ok(())
    }
}

impl Registry {
    /// La commande de l'étape `k`, citée par l'étape `step`.
    fn cited_spec(
        &self,
        calls: &[PlannedCall],
        step: usize,
        k: usize,
    ) -> Result<&CommandSpec, PlanError> {
        self.spec(&calls[k - 1].command).ok_or_else(|| {
            error(
                "plan.bad_reference",
                step,
                format!("l'étape {k} cite une commande inconnue."),
            )
        })
    }

    /// Contrôle une référence `step:k` ou `step:k.<champ>` (hors raster), à la place d'une valeur
    /// du schéma `expected` du paramètre `name` de l'étape `step` : étape précédente, champ
    /// présent dans sa sortie, même type.
    #[allow(clippy::too_many_arguments)]
    fn check_reference(
        &self,
        calls: &[PlannedCall],
        step: usize,
        spec: &CommandSpec,
        name: &str,
        expected: &Value,
        k: usize,
        field: &Option<String>,
    ) -> Result<(), PlanError> {
        if k >= step {
            return Err(previous_error(step, k));
        }
        let source = self.cited_spec(calls, step, k)?;
        let given = match field {
            None => normalize(&source.output_schema, &source.output_schema, 0),
            Some(f) => {
                // Champ de premier niveau, éventuellement derrière un `$ref`.
                let root = &source.output_schema;
                let top = normalize(root, root, 0);
                let Some(prop) = top.get("properties").and_then(|p| p.get(f)) else {
                    return Err(error(
                        "plan.bad_reference",
                        step,
                        format!(
                            "la sortie de l'étape {k} ({}) n'a pas de champ « {f} ».",
                            source.id
                        ),
                    ));
                };
                prop.clone()
            }
        };
        if normalize(expected, &spec.params_schema, 0) != given {
            return Err(error(
                "plan.type_mismatch",
                step,
                format!(
                    "Le paramètre « {name} » de {} n'a pas le type de la sortie de l'étape {k} ({}).",
                    spec.id, source.id
                ),
            ));
        }
        Ok(())
    }
}

/// Point qui tient lieu, pour la seule validation d'un plan, d'un élément `step:k.point` d'un
/// tableau de `Point`.
static STAND_IN_POINT: std::sync::LazyLock<Value> =
    std::sync::LazyLock::new(|| serde_json::json!({"x": 0.0, "y": 0.0, "epsg": 4326}));

/// Une référence imbriquée hors d'un tableau de `Point` : refusée avant l'exécution.
fn nested_error(step: usize, text: &str, name: &str) -> PlanError {
    error(
        "plan.bad_reference",
        step,
        format!(
            "« {text} » est écrit à l'intérieur du paramètre « {name} » : les références step:k ne valent qu'au premier niveau d'un paramètre ; nommez le fichier."
        ),
    )
}

/// Une référence vers l'étape `k`, qui n'est pas antérieure à l'étape `step`.
fn previous_error(step: usize, k: usize) -> PlanError {
    error(
        "plan.bad_reference",
        step,
        format!("« step:{k} » ne désigne pas une étape précédente de l'étape {step}."),
    )
}

/// Le schéma des éléments d'un paramètre tableau dont les éléments sont des [`Point`]
/// (`Vec<Point>`, `observers`, `sources`) ; `None` pour tout autre paramètre. Public pour que la
/// grammaire du modèle (`cassini-ai`, ticket 5.56) reconnaisse les mêmes tableaux.
///
/// [`Point`]: crate::point::Point
pub fn point_items_schema(schema: &Value, root: &Value) -> Option<Value> {
    let items = crate::point::items_schema(schema, root, 0)?;
    crate::point::is_point(items, root).then(|| items.clone())
}

impl Registry {
    /// Valide les paramètres d'une étape. Sans référence : tous. Avec références : les
    /// paramètres littéraux, la valeur d'une référence n'étant connue qu'à l'exécution. Les clés
    /// de référence sont remplacées par la valeur d'un exemple déclaré de la commande ; faute
    /// d'exemple, seule l'erreur « champ requis manquant » portant sur une clé de référence est
    /// tolérée. Champ inconnu, mauvaise valeur ou énumération invalide restent refusés.
    fn validate_unreferenced(
        &self,
        id: &str,
        params: &Value,
        spec: &crate::registry::CommandSpec,
        refs: &[String],
    ) -> Result<(), crate::registry::RegistryError> {
        if refs.is_empty() {
            return self.validate_params(id, params);
        }
        let Value::Object(map) = params else {
            return self.validate_params(id, params);
        };
        let mut literal = map.clone();
        for key in refs {
            literal.remove(key);
        }
        // Une clé de référence manquante à son tour : on la comble par un exemple, au plus une fois.
        for _ in 0..=refs.len() {
            let Err(err) = self.validate_params(id, &Value::Object(literal.clone())) else {
                return Ok(());
            };
            let message = err.to_string();
            let Some(key) = refs
                .iter()
                .find(|k| message.contains(&format!("missing field `{k}`")))
            else {
                return Err(err);
            };
            match spec.examples.iter().find_map(|e| e.get(key)) {
                Some(placeholder) => {
                    literal.insert(key.clone(), placeholder.clone());
                }
                None => return Ok(()),
            }
        }
        Ok(())
    }
}

/// Sous-schéma du paramètre `name` dans le schéma des paramètres.
fn param_schema(params_schema: &Value, name: &str) -> Option<Value> {
    params_schema.get("properties")?.get(name).cloned()
}

/// Forme comparable d'un schéma : `$ref` développés, `$schema`, `title` et
/// `description` ôtés.
fn normalize(node: &Value, root: &Value, depth: usize) -> Value {
    const SKIPPED: [&str; 5] = ["$schema", "title", "description", "$defs", "definitions"];
    const NAME_MAPS: [&str; 2] = ["properties", "patternProperties"];
    const DATA: [&str; 4] = ["const", "enum", "default", "examples"];
    match node {
        Value::Object(map) => {
            if depth < 16 {
                if let Some(target) = map.get("$ref").and_then(Value::as_str) {
                    if let Some(resolved) = target.strip_prefix('#').and_then(|p| root.pointer(p)) {
                        return normalize(resolved, root, depth + 1);
                    }
                }
            }
            let mut out = Map::new();
            for (k, v) in map {
                if SKIPPED.contains(&k.as_str()) {
                    continue;
                }
                let value = if DATA.contains(&k.as_str()) {
                    v.clone()
                } else if NAME_MAPS.contains(&k.as_str()) {
                    match v {
                        Value::Object(named) => Value::Object(
                            named
                                .iter()
                                .map(|(n, s)| (n.clone(), normalize(s, root, depth + 1)))
                                .collect(),
                        ),
                        other => other.clone(),
                    }
                } else {
                    normalize(v, root, depth + 1)
                };
                out.insert(k.clone(), value);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|v| normalize(v, root, depth + 1))
                .collect(),
        ),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use schemars::JsonSchema;
    use serde_json::json;

    use super::*;
    use crate::registry::{Command, CommandSpec, Delegated, RunsIn};

    /// Une couche : sortie de `layer.add`, entrée de `terrain.slope`.
    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct Raster {
        id: String,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct AddParams {
        path: String,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct SlopeParams {
        raster: Raster,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct NameParams {
        raster: String,
    }

    #[derive(Debug, Serialize, JsonSchema)]
    struct Done {
        ok: bool,
    }

    struct LayerAdd;
    impl Delegated for LayerAdd {
        type Params = AddParams;
        type Output = Raster;
        fn spec() -> CommandSpec {
            CommandSpec::new::<AddParams, Raster>("layer.add", "Ajoute.", "Adds.")
                .runs_in(RunsIn::Qgis)
                .cancellable(false)
                .tests(&["t"])
        }
    }

    struct Slope;
    impl Command for Slope {
        type Params = SlopeParams;
        type Output = Done;
        fn spec() -> CommandSpec {
            CommandSpec::new::<SlopeParams, Done>("terrain.slope", "Pente.", "Slope.")
                .runs_in(RunsIn::Core)
                .cancellable(false)
                .tests(&["t"])
        }
        fn run(&self, _p: SlopeParams) -> Result<Done, crate::registry::CommandFailure> {
            Ok(Done { ok: true })
        }
    }

    struct NameOnly;
    impl Command for NameOnly {
        type Params = NameParams;
        type Output = Done;
        fn spec() -> CommandSpec {
            CommandSpec::new::<NameParams, Done>("terrain.name", "Nom.", "Name.")
                .runs_in(RunsIn::Core)
                .cancellable(false)
                .tests(&["t"])
        }
        fn run(&self, _p: NameParams) -> Result<Done, crate::registry::CommandFailure> {
            Ok(Done { ok: true })
        }
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct TwoParams {
        name: String,
        #[serde(deserialize_with = "non_negative")]
        n: i64,
    }

    fn non_negative<'de, D: serde::Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
        let v = i64::deserialize(d)?;
        if v >= 0 {
            Ok(v)
        } else {
            Err(serde::de::Error::custom("n doit être positif"))
        }
    }

    struct Two;
    impl Command for Two {
        type Params = TwoParams;
        type Output = Done;
        fn spec() -> CommandSpec {
            CommandSpec::new::<TwoParams, Done>("terrain.two", "Deux.", "Two.")
                .runs_in(RunsIn::Core)
                .cancellable(false)
                .example(json!({"name": "x", "n": 1}))
                .tests(&["t"])
        }
        fn run(&self, _p: TwoParams) -> Result<Done, crate::registry::CommandFailure> {
            Ok(Done { ok: true })
        }
    }

    fn registry_with_two() -> Registry {
        let mut r = registry();
        r.register(Two);
        r
    }

    fn registry() -> Registry {
        let mut r = Registry::new();
        r.register_delegated::<LayerAdd>();
        r.register(Slope);
        r.register(NameOnly);
        r
    }

    fn plan(calls: Value) -> Plan {
        serde_json::from_value(json!({"kind": "calls", "calls": calls})).unwrap()
    }

    #[test]
    fn a_valid_plan_is_accepted() {
        let p = plan(json!([
            {"command": "layer.add", "params": {"path": "mnt.tif"}},
            {"command": "terrain.slope", "params": {"raster": "step:1"}}
        ]));
        assert_eq!(registry().check_plan(&p), Ok(()));
    }

    #[test]
    fn a_phantom_step_is_refused_with_the_faulty_step() {
        let p = plan(json!([
            {"command": "layer.add", "params": {"path": "mnt.tif"}},
            {"command": "terrain.slope", "params": {"raster": "step:3"}}
        ]));
        let err = registry().check_plan(&p).unwrap_err();
        assert_eq!((err.code, err.step), ("plan.bad_reference", 2));
        // Pas d'auto-référence, ni step:0, ni champ, ni numéro absent.
        for bad in [
            "step:2",
            "step:0",
            "step:01",
            "step:1.",
            "step:1.a.b",
            "step:.id",
            "step:",
        ] {
            let p = plan(json!([
                {"command": "layer.add", "params": {"path": "a"}},
                {"command": "terrain.slope", "params": {"raster": bad}}
            ]));
            let err = registry().check_plan(&p).unwrap_err();
            assert_eq!((err.code, err.step), ("plan.bad_reference", 2), "{bad}");
        }
    }

    #[test]
    fn a_field_reference_is_checked_against_the_schema_of_that_field() {
        // `Raster { id: String }` : `step:1.id` est un texte, `step:1` un objet.
        let ok = plan(json!([
            {"command": "layer.add", "params": {"path": "a"}},
            {"command": "terrain.name", "params": {"raster": "step:1.id"}}
        ]));
        assert_eq!(registry().check_plan(&ok), Ok(()));
        // Le paramètre objet n'accepte pas un champ texte.
        let wrong = plan(json!([
            {"command": "layer.add", "params": {"path": "a"}},
            {"command": "terrain.slope", "params": {"raster": "step:1.id"}}
        ]));
        let err = registry().check_plan(&wrong).unwrap_err();
        assert_eq!((err.code, err.step), ("plan.type_mismatch", 2));
        // Un champ absent est refusé.
        let missing = plan(json!([
            {"command": "layer.add", "params": {"path": "a"}},
            {"command": "terrain.name", "params": {"raster": "step:1.nope"}}
        ]));
        let err = registry().check_plan(&missing).unwrap_err();
        assert_eq!((err.code, err.step), ("plan.bad_reference", 2));
        assert!(err.message.contains("nope"));
    }

    #[test]
    fn a_chained_step_still_validates_its_literal_parameters() {
        let step = |params: Value| {
            plan(json!([
                {"command": "layer.add", "params": {"path": "a"}},
                {"command": "terrain.two", "params": params}
            ]))
        };
        let reg = registry_with_two();
        // Valide : la référence tient lieu de `name`, `n` est correct.
        assert_eq!(
            reg.check_plan(&step(json!({"name": "step:1.id", "n": 3}))),
            Ok(())
        );
        // Mauvaise valeur d'un paramètre littéral.
        let err = reg
            .check_plan(&step(json!({"name": "step:1.id", "n": -3})))
            .unwrap_err();
        assert_eq!((err.code, err.step), ("registry.invalid_params", 2));
        // Champ inconnu.
        let err = reg
            .check_plan(&step(json!({"name": "step:1.id", "n": 3, "zzz": 1})))
            .unwrap_err();
        assert_eq!(err.code, "registry.invalid_params");
        // Requis manquant qui n'est pas une référence.
        let err = reg
            .check_plan(&step(json!({"name": "step:1.id"})))
            .unwrap_err();
        assert_eq!(err.code, "registry.invalid_params");
        assert!(err.message.contains("`n`"), "{}", err.message);
    }

    #[test]
    fn an_incompatible_type_is_refused() {
        let p = plan(json!([
            {"command": "layer.add", "params": {"path": "mnt.tif"}},
            {"command": "terrain.name", "params": {"raster": "step:1"}}
        ]));
        let err = registry().check_plan(&p).unwrap_err();
        assert_eq!((err.code, err.step), ("plan.type_mismatch", 2));
    }

    #[test]
    fn an_unknown_command_is_refused() {
        let p = plan(json!([{"command": "no.such", "params": {}}]));
        let err = registry().check_plan(&p).unwrap_err();
        assert_eq!((err.code, err.step), ("plan.unknown_command", 1));
        assert!(err.message.contains("no.such"));
    }

    #[test]
    fn a_step_without_reference_is_validated_by_its_type() {
        let p = plan(json!([{"command": "layer.add", "params": {"path": 3}}]));
        let err = registry().check_plan(&p).unwrap_err();
        assert_eq!((err.code, err.step), ("registry.invalid_params", 1));
    }

    #[test]
    fn a_refusal_is_accepted_and_the_plan_json_round_trips() {
        let refusal: Plan = serde_json::from_value(
            json!({"kind": "refusal", "reason": "Demande hors du catalogue."}),
        )
        .unwrap();
        assert_eq!(registry().check_plan(&refusal), Ok(()));
        let again: Plan = serde_json::from_value(serde_json::to_value(&refusal).unwrap()).unwrap();
        assert_eq!(again, refusal);
    }

    #[test]
    fn a_question_is_accepted_and_the_plan_json_round_trips() {
        let question: Plan = serde_json::from_value(json!({
            "kind": "question",
            "question": "Aucune couche de pharmacies : laquelle faut-il utiliser ?"
        }))
        .unwrap();
        assert!(
            matches!(&question, Plan::Question { question, source: QuestionSource::Model }
                if question.contains("pharmacies"))
        );
        assert_eq!(registry().check_plan(&question), Ok(()));
        let value = serde_json::to_value(&question).unwrap();
        assert_eq!(value["kind"], "question");
        let again: Plan = serde_json::from_value(value).unwrap();
        assert_eq!(again, question);
    }

    #[test]
    fn the_model_cannot_write_the_guard_source() {
        // Le modèle écrit `source` : le champ est ignoré, la question reste `model`.
        let written: Plan = serde_json::from_value(json!({
            "kind": "question", "question": "Laquelle ?", "source": "guard"
        }))
        .unwrap();
        assert!(matches!(
            &written,
            Plan::Question {
                source: QuestionSource::Model,
                ..
            }
        ));
        // Une question du modèle s'écrit sans `source`, comme avant (octet pour octet).
        assert_eq!(
            serde_json::to_string(&written).unwrap(),
            r#"{"kind":"question","question":"Laquelle ?"}"#
        );
        // Celle du garde-fou porte sa provenance, mais ne se relit pas comme telle.
        let guard = Plan::Question {
            question: "Quel point ?".into(),
            source: QuestionSource::Guard,
        };
        let value = serde_json::to_value(&guard).unwrap();
        assert_eq!(value["source"], "guard");
        let again: Plan = serde_json::from_value(value).unwrap();
        assert!(matches!(
            again,
            Plan::Question {
                source: QuestionSource::Model,
                ..
            }
        ));
        assert_eq!(QuestionSource::Guard.as_str(), "guard");
        assert_eq!(QuestionSource::Model.as_str(), "model");
    }

    #[test]
    fn a_whole_output_reference_is_replaced_by_the_output() {
        let outputs = [json!({"layer_id": "l_1", "name": "Gares"})];
        let params = json!({"raster": "step:1", "n": 3});
        let resolved = resolve_references(&params, &outputs, 2).unwrap();
        assert_eq!(
            resolved,
            json!({"raster": {"layer_id": "l_1", "name": "Gares"}, "n": 3})
        );
    }

    #[test]
    fn a_field_reference_is_replaced_by_the_field() {
        let outputs = [json!({"layer_id": "a_1"}), json!({"layer_id": "b_2"})];
        let params = json!({"layer_id": "step:2.layer_id", "other": "step:1.layer_id"});
        let resolved = resolve_references(&params, &outputs, 3).unwrap();
        assert_eq!(resolved, json!({"layer_id": "b_2", "other": "a_1"}));
    }

    #[test]
    fn a_missing_field_at_execution_is_a_typed_error() {
        let outputs = [json!({"layer_id": "a_1"})];
        let err = resolve_references(&json!({"x": "step:1.nope"}), &outputs, 2).unwrap_err();
        assert_eq!((err.code, err.step), ("plan.missing_output", 2));
        assert!(err.message.contains("nope"));
        // Une sortie qui n'est pas un objet n'a pas de champ.
        let err = resolve_references(&json!({"x": "step:1.a"}), &[json!(3)], 2).unwrap_err();
        assert_eq!(err.code, "plan.missing_output");
        // Une étape pas encore exécutée, ou mal formée.
        for bad in ["step:2", "step:0", "step:1.a.b"] {
            let err = resolve_references(&json!({"x": bad}), &outputs, 2).unwrap_err();
            assert_eq!((err.code, err.step), ("plan.bad_reference", 2), "{bad}");
        }
    }

    #[test]
    fn values_that_are_not_references_are_left_intact() {
        let params = json!({"path": "gares.gpkg", "nested": {"x": "step:1"}, "list": [["step:1"]],
            "names": ["a", "step"], "n": 1, "flag": true, "stepless": "step"});
        assert_eq!(resolve_references(&params, &[], 1).unwrap(), params);
        assert_eq!(
            resolve_references(&json!(null), &[], 1).unwrap(),
            json!(null)
        );
    }

    #[test]
    fn normalization_ignores_titles_and_descriptions_and_expands_refs() {
        let root = json!({"$defs": {"A": {"type": "string", "title": "x"}}});
        let a = json!({"$ref": "#/$defs/A", "description": "d"});
        let b = json!({"type": "string", "$schema": "s"});
        assert_eq!(normalize(&a, &root, 0), normalize(&b, &b, 0));
    }

    // ---- `RasterPath` cité par `step:k` (ticket 11.5) ----

    fn slope_then(second: Value) -> Plan {
        plan(json!([
            {"command": "terrain.slope", "params": {"dem": "mnt.tif", "unit": "degrees"}},
            second
        ]))
    }

    fn observer() -> Value {
        json!({"x": 652030, "y": 6850250, "epsg": 2154})
    }

    #[test]
    fn a_raster_is_cited_by_step_of_a_step_that_writes_one() {
        let r = Registry::standard();
        let p = slope_then(json!({"command": "terrain.viewshed",
            "params": {"dem": "step:1", "observer": observer()}}));
        assert_eq!(r.check_plan(&p), Ok(()));
        let p = slope_then(json!({"command": "terrain.slope",
            "params": {"dem": "step:1", "unit": "percent"}}));
        assert_eq!(r.check_plan(&p), Ok(()));
    }

    #[test]
    fn a_raster_cited_by_a_field_of_the_output_is_refused() {
        let r = Registry::standard();
        let p = slope_then(json!({"command": "terrain.viewshed",
            "params": {"dem": "step:1.file", "observer": observer()}}));
        let err = r.check_plan(&p).unwrap_err();
        assert_eq!((err.code, err.step), ("plan.bad_reference", 2));
        assert!(
            err.message.contains("step:1, sans champ"),
            "{}",
            err.message
        );
    }

    #[test]
    fn a_reference_inside_a_list_is_refused_before_execution() {
        let r = Registry::standard();
        let overlay = |criteria: Value, exclusions: Value| {
            slope_then(json!({"command": "raster.weighted_overlay", "params": {
                "criteria": criteria, "exclusions": exclusions, "out": "score.tif"}}))
        };
        let plain = json!([{"raster": "a.tif", "weight": 1}, {"raster": "b.tif", "weight": 1}]);
        // Un critère, un masque, une référence mal formée : refusés à l'étape 2.
        for p in [
            overlay(
                json!([{"raster": "step:1", "weight": 1}, {"raster": "b.tif", "weight": 1}]),
                json!([]),
            ),
            overlay(plain.clone(), json!(["step:1"])),
            overlay(plain.clone(), json!(["step:01"])),
            // Un paramètre objet (un point), et une référence plus profonde.
            slope_then(
                json!({"command": "terrain.viewshed", "params": {"dem": "mnt.tif",
                "observer": {"x": "step:1.x", "y": 6850250, "epsg": 2154}}}),
            ),
            overlay(
                json!([{"raster": "a.tif", "weight": 1, "extra": [["step:1"]]}]),
                json!([]),
            ),
        ] {
            let err = r.check_plan(&p).unwrap_err();
            assert_eq!((err.code, err.step), ("plan.bad_reference", 2));
            assert!(err.message.contains("premier niveau"), "{}", err.message);
        }
        // Des fichiers nommés passent, comme une référence de premier niveau.
        assert_eq!(r.check_plan(&overlay(plain, json!(["masque.tif"]))), Ok(()));
        let p = slope_then(json!({"command": "terrain.slope",
            "params": {"dem": "step:1", "unit": "percent"}}));
        assert_eq!(r.check_plan(&p), Ok(()));
    }

    // ---- `step:k.point` dans un tableau de `Point` (ticket 11.22, amendement d'AD-23) ----

    fn wgs84() -> Value {
        json!({"x": 2.4069, "y": 48.7353, "epsg": 4326})
    }

    fn converted_then(second: Value) -> Plan {
        plan(json!([
            {"command": "data.transform_point", "params": {"point": wgs84(), "target_epsg": 2154}},
            second
        ]))
    }

    #[test]
    fn a_point_reference_is_admitted_inside_a_list_of_points() {
        let r = Registry::standard();
        let p = converted_then(json!({"command": "terrain.viewshed_cumulative", "params": {
            "dem": "mnt.tif", "observers": ["step:1.point", observer()]}}));
        assert_eq!(r.check_plan(&p), Ok(()));
        let p = converted_then(json!({"command": "terrain.cost_distance", "params": {
            "cost": "cout.tif", "sources": ["step:1.point"]}}));
        assert_eq!(r.check_plan(&p), Ok(()));
        // Le littéral voisin reste validé : un point sans epsg est refusé.
        let p = converted_then(json!({"command": "terrain.viewshed_cumulative", "params": {
            "dem": "mnt.tif", "observers": ["step:1.point", {"x": 1, "y": 2}]}}));
        assert_eq!(
            r.check_plan(&p).unwrap_err().code,
            "registry.invalid_params"
        );
    }

    #[test]
    fn a_point_reference_in_a_list_must_cite_an_earlier_point() {
        let r = Registry::standard();
        let cumulative = |observers: Value| {
            converted_then(json!({"command": "terrain.viewshed_cumulative", "params": {
                "dem": "mnt.tif", "observers": observers}}))
        };
        // Vers une étape ultérieure ou elle-même, vers un champ absent, vers un champ qui n'est
        // pas un `Point`, vers la sortie entière, ou mal formée.
        for (observers, code) in [
            (json!(["step:2.point"]), "plan.bad_reference"),
            (json!(["step:3.point"]), "plan.bad_reference"),
            (json!(["step:1.nope"]), "plan.bad_reference"),
            (json!(["step:1"]), "plan.type_mismatch"),
            (json!(["step:01.point"]), "plan.bad_reference"),
        ] {
            let err = r.check_plan(&cumulative(observers.clone())).unwrap_err();
            assert_eq!((err.code, err.step), (code, 2), "{observers}");
        }
        let p = plan(json!([
            {"command": "terrain.slope", "params": {"dem": "mnt.tif", "unit": "degrees"}},
            {"command": "terrain.viewshed_cumulative", "params": {
                "dem": "mnt.tif", "observers": ["step:1.file"]}}
        ]));
        assert_eq!(r.check_plan(&p).unwrap_err().code, "plan.type_mismatch");
        // Plus profond qu'un élément : refusé comme ailleurs.
        let err = r
            .check_plan(&cumulative(
                json!([{"x": "step:1.point", "y": 1, "epsg": 2154}]),
            ))
            .unwrap_err();
        assert_eq!(err.code, "plan.bad_reference");
        assert!(err.message.contains("premier niveau"), "{}", err.message);
    }

    #[test]
    fn a_point_reference_is_refused_in_a_list_that_is_not_of_points() {
        let r = Registry::standard();
        let p = converted_then(json!({"command": "raster.weighted_overlay", "params": {
            "criteria": [{"raster": "a.tif", "weight": 1}, {"raster": "b.tif", "weight": 1}],
            "exclusions": ["step:1.point"], "out": "score.tif"}}));
        let err = r.check_plan(&p).unwrap_err();
        assert_eq!((err.code, err.step), ("plan.bad_reference", 2));
        assert!(err.message.contains("premier niveau"), "{}", err.message);
    }

    #[test]
    fn a_point_reference_in_a_list_is_resolved_in_place() {
        let outputs = [json!({"point": observer()}), json!({"point": wgs84()})];
        let params =
            json!({"dem": "mnt.tif", "observers": ["step:1.point", wgs84(), "step:2.point"]});
        let resolved = resolve_references(&params, &outputs, 3).unwrap();
        assert_eq!(
            resolved,
            json!({"dem": "mnt.tif", "observers": [observer(), wgs84(), wgs84()]})
        );
        let err = resolve_references(&params, &outputs, 2).unwrap_err();
        assert_eq!((err.code, err.step), ("plan.bad_reference", 2));
    }

    #[test]
    fn a_raster_cited_from_a_step_without_raster_is_a_type_mismatch() {
        let r = Registry::standard();
        let p = plan(json!([
            {"command": "analysis.buffer", "params": {"layer_id": "a",
                "distance": {"value": 10, "unit": "m"}}},
            {"command": "terrain.slope", "params": {"dem": "step:1", "unit": "degrees"}}
        ]));
        let err = r.check_plan(&p).unwrap_err();
        assert_eq!((err.code, err.step), ("plan.type_mismatch", 2));
        assert!(err.message.contains("analysis.buffer"), "{}", err.message);
        // Un point, ou une ligne de vue (qui n'écrit pas de raster), non plus.
        let p = plan(json!([
            {"command": "terrain.line_of_sight", "params": {"dem": "mnt.tif",
                "observer": observer(), "target": observer()}},
            {"command": "terrain.slope", "params": {"dem": "step:1", "unit": "degrees"}}
        ]));
        assert_eq!(r.check_plan(&p).unwrap_err().code, "plan.type_mismatch");
    }

    #[test]
    fn raster_markers_use_the_journal_line_of_the_cited_step() {
        let r = Registry::standard();
        let spec = r.spec("terrain.viewshed").unwrap();
        let params = json!({"dem": "step:1", "observer": observer()});
        // L'étape 1 du plan est la ligne 3 du journal (deux lignes avant le plan).
        let marked = raster_markers(spec, &params, &[3, 5]);
        assert_eq!(marked["dem"], json!("out:3.1"));
        assert_eq!(marked["observer"], observer());
        // Un chemin, une étape pas encore exécutée, ou un autre paramètre restent intacts.
        let path = json!({"dem": "mnt.tif", "observer": observer()});
        assert_eq!(raster_markers(spec, &path, &[3]), path);
        assert_eq!(raster_markers(spec, &params, &[]), params);
        let point = r.spec("terrain.line_of_sight").unwrap();
        let other = json!({"dem": "mnt.tif", "observer": "step:1", "target": observer()});
        assert_eq!(raster_markers(point, &other, &[1]), other);
        // Le repère passe ensuite intact par `resolve_references`.
        assert_eq!(
            resolve_references(&marked, &[json!({})], 2).unwrap(),
            marked
        );
    }
}
