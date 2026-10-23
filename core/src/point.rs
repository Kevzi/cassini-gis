//! Point en coordonnées, et garde-fou sur sa provenance (AD-19, AD-23 étendu, 2026-10-23).
//!
//! Toute coordonnée du registre est un [`Point`] : x, y et code EPSG, marqué dans le schéma
//! engendré par l'extension `x-cassini: point`. Le contrôle de contrat refuse un champ `x` ou
//! `y` numérique hors de ce type (`contract.bare_coordinate`).
//!
//! [`guard_coordinates`] est une fonction pure, appliquée une fois à la fin de la composition
//! d'un plan par le modèle local : tout `Point` du plan doit venir de la demande (x, y et
//! système y figurent, comme nombres) ou d'une étape précédente (référence `step:` vers un
//! champ déclaré `Point`). Sinon le plan n'est pas montré : le garde-fou rend une question à
//! provenance `guard`. En cas de doute, c'est la question.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::plan::{parse_reference, Plan, PlannedCall, QuestionSource};
use crate::registry::Registry;

/// Clé d'extension qui marque un type de Cassini dans un schéma engendré.
pub const MARK_KEY: &str = "x-cassini";
/// Valeur de la marque d'un [`Point`].
pub const POINT_MARK: &str = "point";

/// Question rendue par le garde-fou quand un point du plan ne vient ni de la demande, ni
/// d'une étape.
pub const GUARD_QUESTION: &str = "Il manque à la demande un point en coordonnées : quels sont x, y et le système (code EPSG, par exemple 2154 pour le Lambert-93) ?";

/// Un point en coordonnées, avec son système (jamais de coordonnée nue, AD-19).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(extend("x-cassini" = "point"))]
pub struct Point {
    /// Abscisse (X, ou longitude).
    pub x: f64,
    /// Ordonnée (Y, ou latitude).
    pub y: f64,
    /// Code EPSG du système des coordonnées (ex. 2154 pour le Lambert-93).
    pub epsg: u32,
}

/// Profondeur maximale de parcours d'un schéma (références circulaires comprises).
const MAX_DEPTH: usize = 16;
/// Alternatives d'un schéma.
const ALTERNATIVES: [&str; 3] = ["anyOf", "oneOf", "allOf"];

/// Suit les `$ref` internes d'un sous-schéma, jusqu'à la cible.
fn resolve<'a>(schema: &'a Value, root: &'a Value) -> &'a Value {
    let mut node = schema;
    for _ in 0..MAX_DEPTH {
        let Some(target) = node.get("$ref").and_then(Value::as_str) else {
            break;
        };
        match target.strip_prefix('#').and_then(|p| root.pointer(p)) {
            Some(next) => node = next,
            None => break,
        }
    }
    node
}

/// Le sous-schéma est-il, après ses `$ref`, celui d'un [`Point`] ?
pub fn is_point(schema: &Value, root: &Value) -> bool {
    resolve(schema, root).get(MARK_KEY).and_then(Value::as_str) == Some(POINT_MARK)
}

/// Le sous-schéma admet-il un [`Point`] : lui-même, ou l'une de ses alternatives
/// (`Option<Point>` s'engendre en `anyOf`).
fn admits_point(schema: &Value, root: &Value, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    if is_point(schema, root) {
        return true;
    }
    let node = resolve(schema, root);
    ALTERNATIVES.iter().any(|key| {
        node.get(*key)
            .and_then(Value::as_array)
            .is_some_and(|alts| alts.iter().any(|a| admits_point(a, root, depth + 1)))
    })
}

/// Un [`Point`] apparaît-il quelque part sous ce sous-schéma (objets imbriqués, tableaux,
/// alternatives) ?
pub(crate) fn contains_point(schema: &Value, root: &Value, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    if is_point(schema, root) {
        return true;
    }
    let node = resolve(schema, root);
    let Value::Object(map) = node else {
        return false;
    };
    map.iter().any(|(key, value)| {
        if key == "properties" || key == "patternProperties" {
            value
                .as_object()
                .is_some_and(|m| m.values().any(|s| contains_point(s, root, depth + 1)))
        } else if ALTERNATIVES.contains(&key.as_str()) || key == "prefixItems" {
            value
                .as_array()
                .is_some_and(|a| a.iter().any(|s| contains_point(s, root, depth + 1)))
        } else if key == "items" || key == "additionalProperties" {
            contains_point(value, root, depth + 1)
        } else {
            false
        }
    })
}

/// Sous-schéma de la propriété `name` d'un objet, cherché aussi dans ses alternatives.
fn property_schema<'a>(
    schema: &'a Value,
    root: &'a Value,
    name: &str,
    depth: usize,
) -> Option<&'a Value> {
    if depth > MAX_DEPTH {
        return None;
    }
    let node = resolve(schema, root);
    if let Some(found) = node.get("properties").and_then(|p| p.get(name)) {
        return Some(found);
    }
    ALTERNATIVES.iter().find_map(|key| {
        node.get(*key)
            .and_then(Value::as_array)?
            .iter()
            .find_map(|a| property_schema(a, root, name, depth + 1))
    })
}

/// Sous-schéma des éléments d'un tableau, cherché aussi dans ses alternatives.
fn items_schema<'a>(schema: &'a Value, root: &'a Value, depth: usize) -> Option<&'a Value> {
    if depth > MAX_DEPTH {
        return None;
    }
    let node = resolve(schema, root);
    if let Some(items) = node.get("items").filter(|i| i.is_object()) {
        return Some(items);
    }
    ALTERNATIVES.iter().find_map(|key| {
        node.get(*key)
            .and_then(Value::as_array)?
            .iter()
            .find_map(|a| items_schema(a, root, depth + 1))
    })
}

// -- lecture des nombres de la demande ---------------------------------------

/// Séparateurs de milliers admis : espace, espace insécable, espace fine, espace fine
/// insécable.
fn is_group_separator(c: char) -> bool {
    matches!(c, ' ' | '\u{a0}' | '\u{2009}' | '\u{202f}')
}

/// Séparateurs admis entre le nom d'un système et son numéro (« Lambert-93 », « WGS 84 »).
fn is_name_separator(c: char) -> bool {
    matches!(c, '-' | '\u{2011}' | ' ' | '\u{a0}')
}

/// Le nombre qui commence en `start` suit-il directement le nom d'un système (« Lambert 93 ») ?
/// Il n'ouvre alors pas de groupe de milliers : « Lambert 93 652 000 » n'est pas 93 652 000.
fn follows_system_name(chars: &[char], start: usize) -> bool {
    let mut end = start;
    if end > 0 && is_name_separator(chars[end - 1]) {
        end -= 1;
    }
    NAMED_SYSTEMS.iter().any(|(name, _, _)| {
        let n = name.chars().count();
        end >= n
            && chars[end - n..end]
                .iter()
                .flat_map(|c| c.to_lowercase())
                .eq(name.chars())
            && (end == n || !chars[end - n - 1].is_alphanumeric())
    })
}

/// Noms de systèmes que le noyau associe à un code fermé : (préfixe, numéro, code EPSG).
/// Écrits avec ou sans séparateur entre le nom et le numéro (« Lambert-93 », « WGS 84 »).
const NAMED_SYSTEMS: [(&str, &str, f64); 2] = [("lambert", "93", 2154.0), ("wgs", "84", 4326.0)];

/// Nombres écrits dans la demande, dans l'ordre (décision du propriétaire, 2026-10-23) :
/// `2.458` et `2,458` (virgule suivie d'un chiffre = décimale) ; groupes de trois chiffres
/// séparés par une espace, une espace insécable ou une espace fine (`6 850 000`) lus comme un
/// seul nombre ; signe moins accepté devant un nombre détaché d'un mot. S'y ajoutent les codes
/// des systèmes nommés (Lambert-93 : 2154, WGS84 : 4326). Toute autre forme n'est pas lue.
pub fn request_numbers(request: &str) -> Vec<f64> {
    let chars: Vec<char> = request.chars().collect();
    let digit = |i: usize| chars.get(i).is_some_and(char::is_ascii_digit);
    let mut numbers = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !digit(i) {
            i += 1;
            continue;
        }
        let start = i;
        while digit(i) {
            i += 1;
        }
        let mut text: String = chars[start..i].iter().collect();
        // Milliers : une tête de un à trois chiffres, puis des groupes d'exactement trois.
        if i - start <= 3 && !follows_system_name(&chars, start) {
            while chars.get(i).is_some_and(|c| is_group_separator(*c))
                && digit(i + 1)
                && digit(i + 2)
                && digit(i + 3)
                && !digit(i + 4)
            {
                text.extend(&chars[i + 1..i + 4]);
                i += 4;
            }
        }
        // Décimale : un point ou une virgule suivi d'un chiffre.
        if chars.get(i).is_some_and(|c| *c == '.' || *c == ',') && digit(i + 1) {
            text.push('.');
            i += 1;
            while digit(i) {
                text.push(chars[i]);
                i += 1;
            }
        }
        // Signe : un moins collé au nombre, qui ne suit pas un mot (« Lambert-93 » n'est pas -93).
        let negative = start >= 1
            && matches!(chars[start - 1], '-' | '\u{2212}')
            && (start == 1 || !chars[start - 2].is_alphanumeric());
        if let Ok(value) = text.parse::<f64>() {
            numbers.push(if negative { -value } else { value });
        }
    }
    let lower: Vec<char> = request.to_lowercase().chars().collect();
    for (name, number, code) in NAMED_SYSTEMS {
        let name: Vec<char> = name.chars().collect();
        let number: Vec<char> = number.chars().collect();
        for start in 0..lower.len() {
            if !lower[start..].starts_with(&name)
                || (start > 0 && lower[start - 1].is_alphanumeric())
            {
                continue;
            }
            let mut j = start + name.len();
            if lower.get(j).is_some_and(|c| is_name_separator(*c)) {
                j += 1;
            }
            if lower[j.min(lower.len())..].starts_with(&number)
                && !lower
                    .get(j + number.len())
                    .is_some_and(char::is_ascii_digit)
            {
                numbers.push(code);
            }
        }
    }
    numbers
}

/// Deux nombres égaux, à l'arrondi du flottant près.
fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

/// Retire de `pool` une occurrence égale à `value` ; vrai si elle y était.
fn take(pool: &mut Vec<f64>, value: f64) -> bool {
    match pool.iter().position(|n| same(*n, value)) {
        Some(index) => {
            pool.swap_remove(index);
            true
        }
        None => false,
    }
}

// -- garde-fou ---------------------------------------------------------------

/// Ce que porte un plan : ses nombres littéraux hors `Point`, et ses points littéraux.
#[derive(Default)]
struct Found<'a> {
    numbers: Vec<f64>,
    points: Vec<&'a Value>,
    /// Une coordonnée dont la provenance ne peut pas être établie.
    doubtful: bool,
}

/// Parcourt une valeur littérale avec son schéma (s'il est connu).
fn walk<'a>(value: &'a Value, schema: Option<&Value>, root: &Value, found: &mut Found<'a>) {
    let point_here = schema.is_some_and(|s| admits_point(s, root, 0));
    match value {
        Value::Object(map) => {
            // Sans schéma connu, un objet qui porte `x` ou `y` est lu comme un point : le
            // contrat interdit ces champs numériques ailleurs, et le doute mène à la question.
            let bare = schema.is_none() && (map.contains_key("x") || map.contains_key("y"));
            if point_here || bare {
                found.points.push(value);
                return;
            }
            for (name, item) in map {
                let sub = schema.and_then(|s| property_schema(s, root, name, 0));
                walk(item, sub, root, found);
            }
        }
        Value::Array(items) => {
            let sub = schema.and_then(|s| items_schema(s, root, 0));
            for item in items {
                walk(item, sub, root, found);
            }
        }
        Value::Number(n) => {
            if let Some(v) = n.as_f64() {
                found.numbers.push(v);
            }
        }
        Value::String(text) => {
            if point_here {
                found.doubtful = true;
            } else if let Ok(v) = text.trim().parse::<f64>() {
                if v.is_finite() {
                    found.numbers.push(v);
                }
            }
        }
        Value::Bool(_) | Value::Null => {}
    }
}

/// La référence `value`, paramètre `name` d'une étape `step` de `calls`, désigne-t-elle un
/// champ que le schéma de sortie de l'étape citée déclare `Point`, à la place d'un paramètre
/// lui-même `Point` ?
fn reference_is_a_point(
    registry: &Registry,
    calls: &[PlannedCall],
    step: usize,
    param: &Value,
    params_root: &Value,
    value: &Value,
) -> bool {
    let Some(Ok((k, field))) = parse_reference(value) else {
        return false;
    };
    if k >= step || !admits_point(param, params_root, 0) {
        return false;
    }
    let Some(source) = registry.spec(&calls[k - 1].command) else {
        return false;
    };
    let root = &source.output_schema;
    match field {
        None => admits_point(root, root, 0),
        Some(f) => {
            property_schema(root, root, &f, 0).is_some_and(|schema| admits_point(schema, root, 0))
        }
    }
}

/// Garde-fou de AD-23 sur les coordonnées : rend `plan` inchangé si chaque `Point` de ses
/// étapes vient de `request` ou d'une étape précédente, sinon une question à provenance
/// `guard`. Une question ou un refus passe inchangé.
///
/// Les nombres de la demande forment un multiensemble : chaque nombre littéral du plan hors
/// `Point` (durée, distance) y retire d'abord une occurrence égale s'il en trouve une ; puis,
/// pour chaque `Point`, x, y et epsg retirent chacun une occurrence distincte. S'il en manque
/// une, c'est la question. Le système n'est jamais pris au projet.
pub fn guard_coordinates(registry: &Registry, plan: Plan, request: &str) -> Plan {
    let Plan::Calls { calls } = &plan else {
        return plan;
    };
    let mut found = Found::default();
    for (index, call) in calls.iter().enumerate() {
        let step = index + 1;
        let Some(spec) = registry.spec(&call.command) else {
            return guard_question();
        };
        let root = &spec.params_schema;
        let Value::Object(params) = &call.params else {
            walk(&call.params, Some(root), root, &mut found);
            continue;
        };
        for (name, value) in params {
            let schema = property_schema(root, root, name, 0);
            if parse_reference(value).is_some() {
                // Une référence ne tient lieu de point que si elle désigne un `Point`.
                let carries_point = schema.is_some_and(|s| contains_point(s, root, 0));
                if carries_point
                    && !schema.is_some_and(|s| {
                        reference_is_a_point(registry, calls, step, s, root, value)
                    })
                {
                    return guard_question();
                }
                continue;
            }
            walk(value, schema, root, &mut found);
        }
    }
    if found.doubtful {
        return guard_question();
    }
    if found.points.is_empty() {
        return plan;
    }
    let mut pool = request_numbers(request);
    for number in &found.numbers {
        take(&mut pool, *number);
    }
    for point in &found.points {
        for key in ["x", "y", "epsg"] {
            let given = point.get(key).and_then(Value::as_f64);
            if !given.is_some_and(|v| take(&mut pool, v)) {
                return guard_question();
            }
        }
    }
    plan
}

/// La question du garde-fou.
fn guard_question() -> Plan {
    Plan::Question {
        question: GUARD_QUESTION.to_string(),
        source: QuestionSource::Guard,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::registry::{Command, CommandFailure, CommandSpec, RunsIn};

    const ROUTES: &str = "routes_1";

    fn isochrone(origin: Value, duration: Value) -> Plan {
        calls(json!([{"command": "analysis.isochrone", "params": {
            "layer_id": ROUTES, "origin": origin, "mode": "walk",
            "duration": {"value": duration, "unit": "min"}}}]))
    }

    fn calls(calls: Value) -> Plan {
        serde_json::from_value(json!({"kind": "calls", "calls": calls})).unwrap()
    }

    fn is_guard(plan: &Plan) -> bool {
        matches!(plan, Plan::Question { question, source: QuestionSource::Guard }
            if question == GUARD_QUESTION)
    }

    /// Une commande dont la sortie est un `Point`, champ `point` : le cas d'une étape.
    #[derive(Debug, Serialize, JsonSchema)]
    struct Located {
        point: Point,
        layer_id: String,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct NoParams {}

    struct Locate;
    impl Command for Locate {
        type Params = NoParams;
        type Output = Located;
        fn spec() -> CommandSpec {
            CommandSpec::new::<NoParams, Located>("test.locate", "Situe.", "Locates.")
                .runs_in(RunsIn::Core)
                .cancellable(false)
                .tests(&["t"])
        }
        fn run(&self, _p: NoParams) -> Result<Located, CommandFailure> {
            Ok(Located {
                point: Point {
                    x: 1.0,
                    y: 2.0,
                    epsg: 2154,
                },
                layer_id: "l".into(),
            })
        }
    }

    fn registry_with_locate() -> Registry {
        let mut registry = Registry::standard();
        registry.register(Locate);
        registry.register(MaybeAt);
        registry
    }

    /// Une commande à point facultatif, et un objet à `x` texte que le contrat permet.
    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct MaybeAtParams {
        at: Option<Point>,
        label: Option<Label>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct Label {
        x: String,
    }

    #[derive(Debug, Serialize, JsonSchema)]
    struct Done {}

    struct MaybeAt;
    impl Command for MaybeAt {
        type Params = MaybeAtParams;
        type Output = Done;
        fn spec() -> CommandSpec {
            CommandSpec::new::<MaybeAtParams, Done>("test.maybe_at", "Peut-être.", "Maybe.")
                .runs_in(RunsIn::Core)
                .cancellable(false)
                .tests(&["t"])
        }
        fn run(&self, _p: MaybeAtParams) -> Result<Done, CommandFailure> {
            Ok(Done {})
        }
    }

    #[test]
    fn an_optional_point_takes_a_step_reference_and_a_known_object_is_not_a_point() {
        let registry = registry_with_locate();
        let plan = calls(json!([
            {"command": "test.locate", "params": {}},
            {"command": "test.maybe_at", "params": {"at": "step:1.point"}}
        ]));
        assert_eq!(guard_coordinates(&registry, plan.clone(), ""), plan);
        let plan = calls(json!([
            {"command": "test.maybe_at", "params": {"label": {"x": "nord"}}}
        ]));
        assert_eq!(guard_coordinates(&registry, plan.clone(), ""), plan);
        // Un point facultatif littéral reste gardé.
        let plan = calls(json!([
            {"command": "test.maybe_at", "params": {"at": {"x": 1, "y": 2, "epsg": 2154}}}
        ]));
        assert!(is_guard(&guard_coordinates(&registry, plan, "")));
    }

    #[test]
    fn the_point_schema_is_marked() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.isochrone").unwrap();
        let root = &spec.params_schema;
        let origin = &root["properties"]["origin"];
        assert!(is_point(origin, root), "{origin}");
        assert!(!is_point(&root["properties"]["duration"], root));
        assert!(!is_point(&root["properties"]["layer_id"], root));
        // Même désérialisation qu'avant : pas de champ inconnu, pas de système manquant.
        assert!(serde_json::from_value::<Point>(json!({"x": 1, "y": 2})).is_err());
        assert!(
            serde_json::from_value::<Point>(json!({"x": 1, "y": 2, "epsg": 2154, "z": 0})).is_err()
        );
    }

    #[test]
    fn phrase_e_without_coordinates_becomes_a_guard_question() {
        let plan = isochrone(json!({"x": 2.458, "y": 48.658, "epsg": 4326}), json!(10));
        let request =
            "Quelle zone atteint-on à pied en 10 minutes depuis la gare de Villeneuve-le-Roi ?";
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            plan,
            request
        )));
    }

    #[test]
    fn a_point_given_in_the_request_leaves_the_plan_unchanged() {
        let registry = Registry::standard();
        let plan = isochrone(json!({"x": 652000, "y": 6850000, "epsg": 2154}), json!(10));
        for request in [
            "Zone à 10 min à pied depuis x 652 000, y 6 850 000 en EPSG:2154.",
            "Zone à 10 min à pied depuis x 652\u{a0}000, y 6\u{202f}850\u{202f}000 en EPSG:2154.",
            "Zone à 10 min à pied depuis le point 652000 / 6850000 (Lambert-93).",
            "Zone à 10 min à pied depuis le point (652 000 ; 6 850 000) en Lambert 93.",
            "Zone à 10 min à pied, en Lambert 93 652 000 / 6 850 000.",
            "Zone à 10 min à pied, en Lambert-93 : 652\u{2009}000 / 6\u{2009}850\u{2009}000.",
        ] {
            assert_eq!(
                guard_coordinates(&registry, plan.clone(), request),
                plan,
                "{request}"
            );
        }
        // Décimales au point ou à la virgule, système nommé ou en code.
        let wgs = isochrone(json!({"x": 2.458, "y": 48.658, "epsg": 4326}), json!(10));
        for request in [
            "10 min depuis 2,458 / 48,658 (WGS84)",
            "10 min depuis 2.458 / 48.658 en EPSG:4326",
            "10 min depuis la longitude 2.4580 et la latitude 48.6580 (WGS 84)",
        ] {
            assert_eq!(
                guard_coordinates(&registry, wgs.clone(), request),
                wgs,
                "{request}"
            );
        }
        // Signe moins.
        let west = isochrone(json!({"x": -1.5, "y": 47.2, "epsg": 4326}), json!(10));
        assert_eq!(
            guard_coordinates(
                &registry,
                west.clone(),
                "10 min depuis -1,5 / 47,2 (EPSG:4326)"
            ),
            west
        );
    }

    #[test]
    fn a_missing_system_is_a_guard_question() {
        let plan = isochrone(json!({"x": 652000, "y": 6850000, "epsg": 2154}), json!(10));
        let request = "Zone à 10 min à pied depuis x 652 000, y 6 850 000.";
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            plan,
            request
        )));
    }

    #[test]
    fn a_number_is_not_taken_twice() {
        // La durée prend le premier 10, x le second : y n'a plus de nombre libre.
        let plan = isochrone(json!({"x": 10, "y": 10, "epsg": 2154}), json!(10));
        let request = "10 min depuis 10, 500 (EPSG:2154)";
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            plan,
            request
        )));
        // Le point du modèle tiré d'autres nombres de la demande (distance, durée).
        let plan = isochrone(json!({"x": 500, "y": 10, "epsg": 2154}), json!(10));
        let request = "à 500 m de la gare, en 10 minutes, en EPSG:2154";
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            plan,
            request
        )));
    }

    #[test]
    fn other_number_forms_are_not_read() {
        let plan = isochrone(json!({"x": 652000, "y": 6850000, "epsg": 2154}), json!(10));
        for request in [
            "10 min depuis 652.000 / 6.850.000 en EPSG:2154",
            "10 min depuis 652'000 / 6'850'000 en EPSG:2154",
        ] {
            assert!(
                is_guard(&guard_coordinates(
                    &Registry::standard(),
                    plan.clone(),
                    request
                )),
                "{request}"
            );
        }
        let dms = isochrone(json!({"x": 2.458, "y": 48.658, "epsg": 4326}), json!(10));
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            dms,
            "10 min depuis 2°27'29\" E, 48°39'29\" N (EPSG:4326)"
        )));
    }

    #[test]
    fn the_request_numbers_are_read_as_decided() {
        assert_eq!(
            request_numbers("652 000 et 6 850 000"),
            [652000.0, 6850000.0]
        );
        assert_eq!(request_numbers("2,458 / 48.658"), [2.458, 48.658]);
        assert_eq!(request_numbers("10, 500"), [10.0, 500.0]);
        assert_eq!(request_numbers("652000 6850000"), [652000.0, 6850000.0]);
        assert_eq!(request_numbers("de -3 à - 4"), [-3.0, 4.0]);
        assert_eq!(request_numbers("Lambert-93"), [93.0, 2154.0]);
        assert_eq!(request_numbers("EPSG:2154"), [2154.0]);
        assert_eq!(request_numbers("1 234,5 m"), [1234.5]);
        assert_eq!(
            request_numbers("WGS 84 ; wgs84"),
            [84.0, 84.0, 4326.0, 4326.0]
        );
        assert!(request_numbers("Lambert-930").iter().all(|n| *n != 2154.0));
        // Espace fine (U+2009) entre les milliers.
        assert_eq!(request_numbers("6\u{2009}850\u{2009}000"), [6850000.0]);
        // Le numéro d'un système nommé n'ouvre pas de groupe de milliers.
        assert_eq!(
            request_numbers("en Lambert 93 652 000 / 6 850 000"),
            [93.0, 652000.0, 6850000.0, 2154.0]
        );
        assert_eq!(request_numbers("WGS 84 652 000"), [84.0, 652000.0, 4326.0]);
        assert_eq!(request_numbers("le 93 652 000"), [93652000.0]);
    }

    #[test]
    fn a_point_from_a_step_leaves_the_plan_unchanged() {
        let registry = registry_with_locate();
        let plan = calls(json!([
            {"command": "test.locate", "params": {}},
            {"command": "analysis.isochrone", "params": {
                "layer_id": ROUTES, "origin": "step:1.point", "mode": "walk",
                "duration": {"value": 10, "unit": "min"}}}
        ]));
        assert_eq!(guard_coordinates(&registry, plan.clone(), "10 min"), plan);
    }

    #[test]
    fn a_reference_to_a_field_that_is_not_a_point_is_a_guard_question() {
        let registry = registry_with_locate();
        for reference in ["step:1.layer_id", "step:1", "step:2.point", "step:1.nope"] {
            let plan = calls(json!([
                {"command": "test.locate", "params": {}},
                {"command": "analysis.isochrone", "params": {
                    "layer_id": ROUTES, "origin": reference, "mode": "walk",
                    "duration": {"value": 10, "unit": "min"}}}
            ]));
            assert!(
                is_guard(&guard_coordinates(&registry, plan, "10 min")),
                "{reference}"
            );
        }
    }

    #[test]
    fn a_question_or_a_refusal_from_the_model_is_left_unchanged() {
        let registry = Registry::standard();
        let question = Plan::Question {
            question: "Laquelle ?".into(),
            source: QuestionSource::Model,
        };
        assert_eq!(guard_coordinates(&registry, question.clone(), ""), question);
        let refusal = Plan::Refusal {
            reason: "Hors catalogue.".into(),
        };
        assert_eq!(guard_coordinates(&registry, refusal.clone(), ""), refusal);
    }

    #[test]
    fn a_plan_without_point_is_left_unchanged() {
        let plan = calls(json!([{"command": "analysis.buffer", "params": {
            "layer_id": "gares_1", "distance": {"value": 300, "unit": "m"}}}]));
        assert_eq!(
            guard_coordinates(&Registry::standard(), plan.clone(), "Tampon."),
            plan
        );
    }

    #[test]
    fn a_coordinate_outside_the_schema_is_still_guarded() {
        // Commande inconnue : rien n'est deviné.
        let unknown = calls(json!([{"command": "no.such", "params": {}}]));
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            unknown,
            ""
        )));
        // Un objet à `x`/`y` hors schéma est lu comme un point.
        let stray = calls(json!([{"command": "project.inventory", "params": {
            "where": {"x": 1, "y": 2, "epsg": 2154}}}]));
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            stray,
            "rien"
        )));
    }
}
