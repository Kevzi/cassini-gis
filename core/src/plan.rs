//! Plan : suite de commandes que l'agent propose, vérifiée avant tout envoi.
//!
//! Une étape peut citer la sortie entière d'une étape précédente par la chaîne
//! exacte `step:<k>` (k à partir de 1), valeur d'un paramètre de premier niveau.
//! Cette convention est isolée ici pour pouvoir être remplacée.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::registry::Registry;

/// Une étape : une commande et ses paramètres (éventuellement des références).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedCall {
    pub command: String,
    pub params: Value,
}

/// Ce que l'agent rend : des appels à faire, ou un refus motivé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Plan {
    Calls { calls: Vec<PlannedCall> },
    Refusal { reason: String },
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

/// Lit une valeur de paramètre : `None` si ce n'est pas une référence,
/// `Some(Err)` si elle commence par `step:` mais est mal formée.
fn parse_reference(value: &Value) -> Option<Result<usize, String>> {
    let text = value.as_str()?.strip_prefix("step:")?;
    let parsed = text
        .parse::<usize>()
        .ok()
        .filter(|k| {
            *k >= 1 && !text.starts_with('0') && text.bytes().all(|b| b.is_ascii_digit())
        })
        .ok_or_else(|| {
            format!(
                "la référence « step:{text} » est mal formée : seule la forme `step:<numéro>` (à partir de 1) est permise."
            )
        });
    Some(parsed)
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
            let mut has_reference = false;
            if let Value::Object(map) = &call.params {
                for (name, value) in map {
                    let Some(parsed) = parse_reference(value) else {
                        continue;
                    };
                    has_reference = true;
                    let k = parsed.map_err(|m| error("plan.bad_reference", step, m))?;
                    if k >= step {
                        return Err(error(
                            "plan.bad_reference",
                            step,
                            format!(
                                "« step:{k} » ne désigne pas une étape précédente de l'étape {step}."
                            ),
                        ));
                    }
                    let source = self.spec(&calls[k - 1].command).ok_or_else(|| {
                        error(
                            "plan.bad_reference",
                            step,
                            format!("l'étape {k} cite une commande inconnue."),
                        )
                    })?;
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
                    let given = normalize(&source.output_schema, &source.output_schema, 0);
                    if normalize(&expected, &spec.params_schema, 0) != given {
                        return Err(error(
                            "plan.type_mismatch",
                            step,
                            format!(
                                "Le paramètre « {name} » de {} n'a pas le type de la sortie de l'étape {k} ({}).",
                                spec.id, source.id
                            ),
                        ));
                    }
                }
            }
            if !has_reference {
                self.validate_params(&call.command, &call.params)
                    .map_err(|e| error(e.code(), step, e.to_string()))?;
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
        fn run(&self, _p: SlopeParams) -> Done {
            Done { ok: true }
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
        fn run(&self, _p: NameParams) -> Done {
            Done { ok: true }
        }
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
        for bad in ["step:2", "step:0", "step:01", "step:1.id", "step:"] {
            let p = plan(json!([
                {"command": "layer.add", "params": {"path": "a"}},
                {"command": "terrain.slope", "params": {"raster": bad}}
            ]));
            let err = registry().check_plan(&p).unwrap_err();
            assert_eq!((err.code, err.step), ("plan.bad_reference", 2), "{bad}");
        }
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
    fn normalization_ignores_titles_and_descriptions_and_expands_refs() {
        let root = json!({"$defs": {"A": {"type": "string", "title": "x"}}});
        let a = json!({"$ref": "#/$defs/A", "description": "d"});
        let b = json!({"type": "string", "$schema": "s"});
        assert_eq!(normalize(&a, &root, 0), normalize(&b, &b, 0));
    }
}
