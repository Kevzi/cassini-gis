//! Contrat d'une commande, contrôlé par le registre lui-même (AD-6, AD-8, AD-21).
//!
//! `Registry::check_contract` rend **toutes** les violations d'un registre, chacune
//! avec un code stable `contract.<règle>`, l'id fautif et un message français.
//! C'est un contrôle, pas un échec de `register` : les inscriptions de 2.2 gardent
//! leur signature, et un test lance ce contrôle sur le registre réel.

use std::fmt;

use serde_json::Value;

use crate::registry::{CommandSpec, Registry, RunsIn};

/// Dialecte JSON Schema que doivent déclarer les schémas engendrés.
pub(crate) const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// Types GeoJSON qui désignent une géométrie ou un objet qui en porte une.
const GEOMETRY_TYPES: [&str; 9] = [
    "Point",
    "MultiPoint",
    "LineString",
    "MultiLineString",
    "Polygon",
    "MultiPolygon",
    "GeometryCollection",
    "Feature",
    "FeatureCollection",
];

/// Une règle du contrat non respectée par une commande.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractViolation {
    /// Code stable, destiné aux machines : `contract.<règle>`.
    pub code: &'static str,
    /// Id de la commande fautive.
    pub id: String,
    /// Explication en français.
    pub message: String,
}

impl fmt::Display for ContractViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} — {} : {}", self.code, self.id, self.message)
    }
}

impl Registry {
    /// Contrôle le contrat de toutes les commandes inscrites ; rend toutes les
    /// violations (liste vide = contrat respecté), dans l'ordre des id.
    pub fn check_contract(&self) -> Vec<ContractViolation> {
        let mut violations = Vec::new();
        for (spec, implemented_in_rust) in self.implementations() {
            check_spec(spec, &mut violations);
            let mismatch = match spec.runs_in {
                Some(RunsIn::Qgis) if implemented_in_rust => {
                    Some("la commande a une implémentation Rust mais se déclare exécutée par QGIS.")
                }
                Some(RunsIn::Core) if !implemented_in_rust => {
                    Some("la commande est déléguée à QGIS mais se déclare exécutée par le noyau.")
                }
                _ => None,
            };
            if let Some(message) = mismatch {
                violations.push(violation("contract.runs_in_mismatch", &spec.id, message));
            }
        }
        for id in self.duplicate_ids() {
            violations.push(violation(
                "contract.duplicate_id",
                id,
                "cet id est inscrit plus d'une fois ; la première entrée est gardée.",
            ));
        }
        violations
    }
}

fn violation(code: &'static str, id: &str, message: &str) -> ContractViolation {
    ContractViolation {
        code,
        id: id.to_string(),
        message: message.to_string(),
    }
}

fn check_spec(spec: &CommandSpec, out: &mut Vec<ContractViolation>) {
    let id = spec.id.as_str();
    if !is_valid_id(id) {
        out.push(violation(
            "contract.id",
            id,
            "l'id doit être de la forme `domaine.verbe`, en anglais et en snake_case.",
        ));
    }
    if spec.description_fr.trim().is_empty() || spec.description_en.trim().is_empty() {
        out.push(violation(
            "contract.description",
            id,
            "la description française et la description anglaise sont toutes deux requises.",
        ));
    }
    if spec.runs_in.is_none() {
        out.push(violation(
            "contract.runs_in",
            id,
            "l'exécutant (`core` ou `qgis`) doit être déclaré, il n'a pas de valeur par défaut.",
        ));
    }
    if spec.cancellable.is_none() {
        out.push(violation(
            "contract.cancellable",
            id,
            "la commande doit déclarer si elle est annulable ou non.",
        ));
    }
    if spec.params_schema["$schema"] != DIALECT || spec.output_schema["$schema"] != DIALECT {
        out.push(violation(
            "contract.schema",
            id,
            "les schémas des paramètres et du résultat doivent déclarer le dialecte JSON Schema 2020-12.",
        ));
    }
    if spec.tests.is_empty() {
        out.push(violation(
            "contract.test",
            id,
            "au moins un test doit être déclaré pour la commande.",
        ));
    }
    check_path_params(spec, out);
    for (which, schema) in [
        ("des paramètres", &spec.params_schema),
        ("du résultat", &spec.output_schema),
    ] {
        if has_unbounded_geometries(schema) {
            out.push(violation(
                "contract.geometry_unbounded",
                id,
                &format!(
                    "le schéma {which} contient une collection de géométries sans `maxItems` (AD-8)."
                ),
            ));
        }
    }
}

/// Les paramètres de chemin et de sortie doivent exister dans le schéma des
/// paramètres et accepter une chaîne : sinon le noyau ne les réécrit pas et un
/// chemin absolu reste dans le journal (AD-9).
fn check_path_params(spec: &CommandSpec, out: &mut Vec<ContractViolation>) {
    let named = [
        ("contract.path_param", "path_params", &spec.path_params),
        (
            "contract.output_param",
            "output_params",
            &spec.output_params,
        ),
    ];
    for (code, kind, names) in named {
        for name in names {
            let property = spec
                .params_schema
                .get("properties")
                .and_then(|p| p.get(name.as_str()));
            let message = match property {
                None => format!(
                    "`{kind}` nomme « {name} », absent des propriétés du schéma des paramètres."
                ),
                Some(p) if !accepts_string(p, &spec.params_schema, 0) => {
                    format!("`{kind}` nomme « {name} », dont le schéma n'accepte pas une chaîne.")
                }
                Some(_) => continue,
            };
            out.push(violation(code, &spec.id, &message));
        }
    }
}

/// Le sous-schéma accepte-t-il une chaîne (directement, en tableau de chaînes,
/// ou par `anyOf`/`oneOf`/`allOf`) ? Suit les `$ref` internes.
fn accepts_string(schema: &Value, root: &Value, depth: usize) -> bool {
    if depth > 8 {
        return false;
    }
    let Value::Object(map) = schema else {
        return false;
    };
    let is_string = match map.get("type") {
        Some(Value::String(t)) => t == "string",
        Some(Value::Array(ts)) => ts.iter().any(|t| t == "string"),
        _ => false,
    };
    if is_string {
        return true;
    }
    if let Some(Value::String(target)) = map.get("$ref") {
        if let Some(resolved) = target.strip_prefix('#').and_then(|p| root.pointer(p)) {
            if accepts_string(resolved, root, depth + 1) {
                return true;
            }
        }
    }
    if map
        .get("items")
        .is_some_and(|i| accepts_string(i, root, depth + 1))
    {
        return true;
    }
    ["anyOf", "oneOf", "allOf"].iter().any(|key| {
        map.get(*key)
            .and_then(Value::as_array)
            .is_some_and(|alts| alts.iter().any(|a| accepts_string(a, root, depth + 1)))
    })
}

/// `domaine.verbe` : deux segments snake_case, commençant par une lettre minuscule.
fn is_valid_id(id: &str) -> bool {
    let segment_ok = |s: &str| {
        let mut chars = s.chars();
        chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    let mut parts = id.split('.');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(domain), Some(verb), None) => segment_ok(domain) && segment_ok(verb),
        _ => false,
    }
}

/// Vrai si un tableau de géométries, sans `maxItems`, apparaît quelque part dans le schéma.
fn has_unbounded_geometries(root: &Value) -> bool {
    fn walk(node: &Value, root: &Value) -> bool {
        match node {
            Value::Object(map) => {
                let is_array = match map.get("type") {
                    Some(Value::String(t)) => t == "array",
                    Some(Value::Array(ts)) => ts.iter().any(|t| t == "array"),
                    _ => false,
                };
                if is_array
                    && !map.contains_key("maxItems")
                    && map.get("items").is_some_and(|i| is_geometry(i, root, 0))
                {
                    return true;
                }
                map.values().any(|v| walk(v, root))
            }
            Value::Array(items) => items.iter().any(|v| walk(v, root)),
            _ => false,
        }
    }
    walk(root, root)
}

/// Le sous-schéma décrit-il une géométrie GeoJSON ? Suit les `$ref` internes.
fn is_geometry(schema: &Value, root: &Value, depth: usize) -> bool {
    if depth > 8 {
        return false;
    }
    let Value::Object(map) = schema else {
        return false;
    };
    let names_geometry = |s: &str| {
        let s = s.to_lowercase();
        s.contains("geometry") || s.contains("geojson")
    };
    if let Some(Value::String(target)) = map.get("$ref") {
        if names_geometry(target) {
            return true;
        }
        if let Some(resolved) = target.strip_prefix('#').and_then(|p| root.pointer(p)) {
            return is_geometry(resolved, root, depth + 1);
        }
    }
    if matches!(map.get("title"), Some(Value::String(t)) if names_geometry(t)) {
        return true;
    }
    if let Some(Value::Object(props)) = map.get("properties") {
        if props.contains_key("coordinates") {
            return true;
        }
        let type_prop = props.get("type");
        let declared: Vec<&Value> = match type_prop {
            Some(t) => t
                .get("const")
                .into_iter()
                .chain(
                    t.get("enum")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten(),
                )
                .collect(),
            None => Vec::new(),
        };
        if declared
            .iter()
            .any(|v| v.as_str().is_some_and(|s| GEOMETRY_TYPES.contains(&s)))
        {
            return true;
        }
    }
    ["anyOf", "oneOf", "allOf"].iter().any(|key| {
        map.get(*key)
            .and_then(Value::as_array)
            .is_some_and(|alts| alts.iter().any(|a| is_geometry(a, root, depth + 1)))
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::*;
    use crate::registry::{Command, Delegated, RunsIn};

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct Empty {}

    #[derive(Debug, Serialize, JsonSchema)]
    struct Output {}

    /// Une commande de test complète : le point de départ de chaque cas de refus.
    fn good_spec() -> CommandSpec {
        CommandSpec::new::<Empty, Output>(
            "analysis.buffer",
            "Crée une zone tampon.",
            "Creates a buffer.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(true)
        .tests(&["some_test"])
    }

    /// Déclare une commande de test dont la description part de `good_spec()`,
    /// modifiée par la fonction donnée.
    macro_rules! command {
        ($name:ident, $tweak:expr) => {
            struct $name;

            impl Command for $name {
                type Params = Empty;
                type Output = Output;

                fn spec() -> CommandSpec {
                    let tweak: fn(CommandSpec) -> CommandSpec = $tweak;
                    tweak(good_spec())
                }

                fn run(&self, _params: Empty) -> Result<Output, crate::registry::CommandFailure> {
                    Ok(Output {})
                }
            }
        };
    }

    command!(Healthy, |s| s);
    command!(BadIdCapital, |mut s| {
        s.id = "Buffer".into();
        s
    });
    command!(BadIdNoVerb, |mut s| {
        s.id = "analysis".into();
        s
    });
    command!(BadIdCamelVerb, |mut s| {
        s.id = "analysis.Buffer".into();
        s
    });
    command!(NoFrench, |mut s| {
        s.description_fr = "  ".into();
        s
    });
    command!(NoEnglish, |mut s| {
        s.description_en = String::new();
        s
    });
    command!(NoRunsIn, |mut s| {
        s.runs_in = None;
        s
    });
    command!(NoCancellable, |mut s| {
        s.cancellable = None;
        s
    });
    command!(NoDialect, |mut s| {
        s.params_schema = json!({"type": "object"});
        s
    });
    command!(NoTest, |mut s| {
        s.tests = Vec::new();
        s
    });
    command!(UnboundedGeometries, |mut s| {
        s.params_schema = json!({
            "$schema": DIALECT,
            "type": "object",
            "properties": {
                "geometries": {"type": "array", "items": {"$ref": "#/$defs/Shape"}}
            },
            "$defs": {
                "Shape": {
                    "type": "object",
                    "properties": {"type": {"enum": ["Point", "Polygon"]}}
                }
            }
        });
        s
    });
    command!(BoundedGeometries, |mut s| {
        s.params_schema = json!({
            "$schema": DIALECT,
            "type": "object",
            "properties": {
                "geometries": {
                    "type": "array",
                    "maxItems": 100,
                    "items": {"type": "object", "properties": {"coordinates": {}}}
                }
            }
        });
        s
    });
    command!(PathParamMissing, |s| s.path_params(&["pth"]));
    command!(OutputParamMissing, |s| s.output_params(&["out"]));
    command!(PathParamNotString, |mut s| {
        s.params_schema = json!({
            "$schema": DIALECT,
            "type": "object",
            "properties": {"path": {"type": "integer"}}
        });
        s.path_params(&["path"])
    });
    command!(PathParamsWellFormed, |mut s| {
        s.params_schema = json!({
            "$schema": DIALECT,
            "type": "object",
            "properties": {
                "path": {"type": ["string", "null"]},
                "inputs": {"type": "array", "items": {"$ref": "#/$defs/Name"}},
                "out": {"type": "string"}
            },
            "$defs": {"Name": {"type": "string"}}
        });
        s.path_params(&["path", "inputs"]).output_params(&["out"])
    });
    command!(FirstOfTwin, |s| s);
    command!(SecondOfTwin, |mut s| {
        s.description_fr = "Une autre description.".into();
        s
    });

    struct QgisBuffer;

    impl Delegated for QgisBuffer {
        type Params = Empty;
        type Output = Output;

        fn spec() -> CommandSpec {
            good_spec().runs_in(RunsIn::Qgis)
        }
    }

    struct WronglyCore;

    impl Delegated for WronglyCore {
        type Params = Empty;
        type Output = Output;

        fn spec() -> CommandSpec {
            good_spec()
        }
    }

    command!(WronglyQgis, |s| s.runs_in(RunsIn::Qgis));

    fn codes(registry: &Registry) -> Vec<&'static str> {
        registry.check_contract().iter().map(|v| v.code).collect()
    }

    fn only<C: Command + Send + Sync + 'static>(command: C) -> Registry {
        let mut registry = Registry::new();
        registry.register(command);
        registry
    }

    #[test]
    fn a_complete_command_has_no_violation() {
        assert!(only(Healthy).check_contract().is_empty());
        assert!(only(BoundedGeometries).check_contract().is_empty());
    }

    #[test]
    fn a_malformed_id_is_refused() {
        assert_eq!(codes(&only(BadIdCapital)), ["contract.id"]);
        assert_eq!(codes(&only(BadIdNoVerb)), ["contract.id"]);
        assert_eq!(codes(&only(BadIdCamelVerb)), ["contract.id"]);
    }

    #[test]
    fn a_declared_executor_must_match_the_implementation() {
        let mut ok = Registry::new();
        ok.register_delegated::<QgisBuffer>();
        assert!(ok.check_contract().is_empty());
        let mut bad = Registry::new();
        bad.register_delegated::<WronglyCore>();
        assert_eq!(codes(&bad), ["contract.runs_in_mismatch"]);
        assert_eq!(codes(&only(WronglyQgis)), ["contract.runs_in_mismatch"]);
    }

    #[test]
    fn a_missing_description_is_refused() {
        assert_eq!(codes(&only(NoFrench)), ["contract.description"]);
        assert_eq!(codes(&only(NoEnglish)), ["contract.description"]);
    }

    #[test]
    fn an_undeclared_executor_is_refused() {
        assert_eq!(codes(&only(NoRunsIn)), ["contract.runs_in"]);
    }

    #[test]
    fn an_undeclared_cancellation_is_refused() {
        assert_eq!(codes(&only(NoCancellable)), ["contract.cancellable"]);
    }

    #[test]
    fn a_schema_without_the_dialect_is_refused() {
        assert_eq!(codes(&only(NoDialect)), ["contract.schema"]);
    }

    #[test]
    fn a_command_without_test_is_refused() {
        assert_eq!(codes(&only(NoTest)), ["contract.test"]);
    }

    #[test]
    fn an_unbounded_geometry_array_is_refused() {
        let violations = only(UnboundedGeometries).check_contract();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].code, "contract.geometry_unbounded");
        assert_eq!(violations[0].id, "analysis.buffer");
        assert!(violations[0].message.contains("AD-8"));
    }

    #[test]
    fn path_and_output_params_must_exist_in_the_schema() {
        let violations = only(PathParamMissing).check_contract();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].code, "contract.path_param");
        assert!(violations[0].message.contains("pth"));
        assert_eq!(codes(&only(OutputParamMissing)), ["contract.output_param"]);
    }

    #[test]
    fn a_path_param_must_accept_a_string() {
        assert_eq!(codes(&only(PathParamNotString)), ["contract.path_param"]);
        assert!(only(PathParamsWellFormed).check_contract().is_empty());
    }

    #[test]
    fn a_duplicate_id_is_refused_and_the_first_entry_is_kept() {
        let mut registry = Registry::new();
        registry.register(FirstOfTwin);
        registry.register(SecondOfTwin);
        assert_eq!(codes(&registry), ["contract.duplicate_id"]);
        assert_eq!(
            registry.spec("analysis.buffer").unwrap().description_fr,
            "Crée une zone tampon."
        );
    }

    #[test]
    fn every_violation_is_reported_not_only_the_first() {
        command!(Hopeless, |mut s| {
            s.id = "Nope".into();
            s.runs_in = None;
            s.tests = Vec::new();
            s
        });
        assert_eq!(
            codes(&only(Hopeless)),
            ["contract.id", "contract.runs_in", "contract.test"]
        );
    }

    #[test]
    fn the_real_registry_respects_the_contract() {
        let registry = Registry::standard();
        assert!(registry.spec("system.about").is_some());
        let violations = registry.check_contract();
        assert!(violations.is_empty(), "{violations:#?}");
    }

    /// Une commande déclare des tests : chacun doit exister, comme fonction
    /// `#[test]`, dans un fichier de `core/src/commands/`.
    #[test]
    fn every_declared_test_exists_in_the_commands_folder() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
        let sources: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "rs"))
            .map(|p| fs::read_to_string(p).unwrap())
            .collect();
        let registry = Registry::standard();
        for (spec, _) in registry.implementations() {
            for name in &spec.tests {
                assert!(
                    sources.iter().any(|src| declares_test(src, name)),
                    "{} déclare le test « {name} », introuvable dans core/src/commands/",
                    spec.id
                );
            }
        }
    }

    /// `fn name(` précédé, parmi ses attributs, de `#[test]`.
    fn declares_test(source: &str, name: &str) -> bool {
        let lines: Vec<&str> = source.lines().map(str::trim).collect();
        lines.iter().enumerate().any(|(i, line)| {
            let is_fn = line
                .strip_prefix("pub fn ")
                .or_else(|| line.strip_prefix("fn "))
                .is_some_and(|rest| rest.starts_with(&format!("{name}(")));
            is_fn
                && lines[..i]
                    .iter()
                    .rev()
                    .take_while(|l| l.starts_with("#["))
                    .any(|l| *l == "#[test]")
        })
    }

    #[test]
    fn the_test_finder_rejects_a_plain_function() {
        let src = "#[test]\nfn real() {}\n\nfn plain() {}\n";
        assert!(declares_test(src, "real"));
        assert!(!declares_test(src, "plain"));
        assert!(!declares_test(src, "missing"));
    }
}
