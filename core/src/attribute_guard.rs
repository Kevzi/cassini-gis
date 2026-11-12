//! Garde-fou d'AD-23 sur les filtres par attribut (ticket 5.57).
//!
//! Le modèle compose parfois une sélection sur une valeur que l'inventaire qu'il a lu montre
//! absente de la liste complète d'un champ (clé `values`, au plus `MAX_VALUES`), ou sur un champ
//! que la couche n'a pas. La carte rendue serait vide, ou l'exécution échouerait sans dire
//! pourquoi. Une fonction pure, appliquée au plan validé juste après le garde-fou des
//! coordonnées, change alors le plan en une question à provenance `guard`, qui nomme la couche,
//! le champ et, pour une valeur, les valeurs connues.
//!
//! Le contrôle porte sur les commandes qui déclarent un filtre par attribut
//! (`CommandSpec::attribute_filter`) et sur l'inventaire **montré au modèle**, identifiants
//! courts compris :
//!
//! - une couche citée par une référence `step:` n'est pas contrôlée (ses champs sont propres à
//!   chaque commande), ni une couche absente de `layers` (présente seulement dans `index`, ou
//!   introuvable), ni une couche sans clé `fields` ;
//! - un champ sans `values` n'est jamais contrôlé sur sa valeur ;
//! - un champ absent n'est contrôlé que hors de la vue triée d'un gros projet : avec la clé
//!   `index`, les champs d'une couche détaillée ont pu être coupés sans marque ;
//! - une liste de plus de `MAX_VALUES` valeurs n'est pas tenue pour complète, et une valeur
//!   vide n'est pas contrôlée (l'inventaire ôte la chaîne vide) ;
//! - `eq` et `ne` comparent exactement, casse comprise (la valeur est dans la liste) ;
//!   `begins_with` (une valeur commence par elle) et `contains` (une valeur la contient)
//!   comparent sans la casse, et ne sont pas contrôlés si la valeur porte un joker du LIKE de
//!   QGIS (`%`, `_`) ; `lt`, `lte`, `gt`, `gte` ne sont jamais contrôlés ;
//! - un champ se cherche d'abord par son nom exact, puis à la casse près, comme QGIS ;
//! - plusieurs fautes : la question porte sur la première, dans l'ordre du plan.
//!
//! La question n'est jamais écrite au journal (AD-11) : sa trace est sa provenance `guard`.

use serde_json::Value;

use crate::agent_inventory::MAX_VALUES;
use crate::plan::{parse_reference, Plan, QuestionSource};
use crate::registry::{AttributeFilter, Registry};

/// Rend `plan` inchangé si chaque filtre par attribut de ses étapes porte sur un champ et une
/// valeur que `inventory` (la vue montrée au modèle) ne contredit pas ; sinon la question du
/// garde-fou sur la première faute. Une question ou un refus passe inchangé.
pub fn guard_attributes(registry: &Registry, plan: Plan, inventory: &Value) -> Plan {
    let Plan::Calls { calls } = &plan else {
        return plan;
    };
    let fault = calls.iter().find_map(|call| {
        let filter = registry.spec(&call.command)?.attribute_filter.as_ref()?;
        fault(filter, &call.params, inventory)
    });
    match fault {
        Some(question) => Plan::Question {
            question,
            source: QuestionSource::Guard,
        },
        None => plan,
    }
}

/// Opérateurs que le garde-fou connaît : le vocabulaire fermé du filtre neutre (AD-22). Le
/// contrat (`contract.attribute_filter`) refuse un opérateur hors de cette liste.
pub const KNOWN_OPERATORS: [&str; 8] = [
    "eq",
    "ne",
    "lt",
    "lte",
    "gt",
    "gte",
    "contains",
    "begins_with",
];

/// Au plus tant de noms de champs cités par la question d'un champ absent.
const MAX_FIELD_NAMES: usize = 10;

fn name_of(field: &Value) -> Option<&str> {
    field.get("name").and_then(Value::as_str)
}

/// Ce que l'inventaire contredit dans un filtre, comme question ; `None` si rien.
fn fault(filter: &AttributeFilter, params: &Value, inventory: &Value) -> Option<String> {
    let text = |name: &str| params.get(name).and_then(Value::as_str);
    let layer_id = text(&filter.layer)?;
    let field = text(&filter.field)?;
    if parse_reference(&Value::from(layer_id)).is_some() {
        return None;
    }
    let layer = inventory
        .get("layers")?
        .as_array()?
        .iter()
        .find(|l| l.get("layer_id").and_then(Value::as_str) == Some(layer_id))?;
    let layer_name = layer
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(layer_id);
    let fields = layer.get("fields")?.as_array()?;
    // QGIS cherche un champ sans tenir compte de la casse : le nom exact d'abord, puis le même
    // nom à la casse près.
    let shown = fields
        .iter()
        .find(|f| name_of(f) == Some(field))
        .or_else(|| {
            let wanted = field.to_lowercase();
            fields
                .iter()
                .find(|f| name_of(f).is_some_and(|n| n.to_lowercase() == wanted))
        });
    let Some(shown) = shown else {
        // Vue triée : les champs d'une couche détaillée ont pu être coupés sans marque.
        if inventory.get("index").is_some() {
            return None;
        }
        let names: Vec<&str> = fields.iter().filter_map(name_of).collect();
        return Some(field_question(layer_name, field, &names));
    };
    let values: Vec<&str> = shown
        .get("values")?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    // Une liste plus longue que celle que le noyau montre n'est pas sûre d'être complète
    // (inventaire écrit à la main pour `cassini plan`).
    if values.len() > MAX_VALUES {
        return None;
    }
    let operator = text(&filter.operator)?;
    let value = text(&filter.value)?;
    // La chaîne vide n'est jamais listée : l'inventaire l'ôte de `values`.
    if value.is_empty() {
        return None;
    }
    let lower = value.to_lowercase();
    // `begins_with` et `contains` deviennent un LIKE dans QGIS : `%` et `_` y sont des jokers,
    // et le fournisseur peut comparer sans la casse. Seules l'égalité et la différence restent
    // exactes.
    let like = |test: &dyn Fn(&str, &str) -> bool| {
        value.contains(['%', '_']) || values.iter().any(|v| test(&v.to_lowercase(), &lower))
    };
    let present = match operator {
        "eq" | "ne" => values.contains(&value),
        "begins_with" => like(&|v, x| v.starts_with(x)),
        "contains" => like(&|v, x| v.contains(x)),
        _ => return None,
    };
    if present {
        return None;
    }
    let field = name_of(shown).unwrap_or(field);
    Some(value_question(layer_name, field, operator, value, &values))
}

/// La question d'une valeur absente : la couche, le champ, la valeur et les valeurs connues
/// dans l'ordre de l'inventaire. Forme 0 du générateur, sans tirage.
pub fn value_question(
    layer: &str,
    field: &str,
    operator: &str,
    value: &str,
    values: &[&str],
) -> String {
    let list = values.join(", ");
    let what = match operator {
        "begins_with" => format!("aucune valeur qui commence par « {value} »"),
        "contains" => format!("aucune valeur qui contient « {value} »"),
        _ => format!("pas de valeur « {value} »"),
    };
    format!(
        "Le champ {field} de la couche « {layer} » n'a {what} (valeurs : {list}) : laquelle faut-il prendre ?"
    )
}

/// La question d'un champ absent : la couche, le champ demandé et les champs de la couche, dans
/// l'ordre de l'inventaire, au plus `MAX_FIELD_NAMES`, pour que l'utilisateur réponde sans
/// ouvrir QGIS.
pub fn field_question(layer: &str, field: &str, fields: &[&str]) -> String {
    let known = if fields.is_empty() {
        String::new()
    } else {
        let shown = &fields[..fields.len().min(MAX_FIELD_NAMES)];
        format!(" (champs : {})", shown.join(", "))
    };
    format!(
        "La couche « {layer} » n'a aucun champ « {field} »{known} : quel champ faut-il utiliser ?"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn registry() -> Registry {
        Registry::standard()
    }

    /// Un inventaire vu par le modèle : un zonage aux valeurs listées, des gares dont le nom est
    /// trop varié pour être listé.
    fn inventory(zones: &[&str]) -> Value {
        json!({
            "project": {"name": "p", "epsg": 2154},
            "layers": [
                {"layer_id": "zonage_1a2b", "name": "Zonage PLU", "kind": "vector",
                 "geometry": "MultiPolygon", "epsg": 2154, "feature_count": 12,
                 "source": "plu.gpkg",
                 "fields": [
                     {"name": "typezone", "type": "string", "values": zones},
                     {"name": "surface", "type": "number"}
                 ]},
                {"layer_id": "gares_3c4d", "name": "Gares", "kind": "vector",
                 "geometry": "Point", "epsg": 2154, "feature_count": 30, "source": "gares.gpkg",
                 "fields": [{"name": "nom", "type": "string"}]}
            ],
            "unloaded_files": []
        })
    }

    fn select(layer: &str, field: &str, operator: &str, value: &str) -> Value {
        json!({"command": "analysis.select_by_attribute",
            "params": {"layer_id": layer, "field": field, "operator": operator, "value": value}})
    }

    fn plan(calls: Vec<Value>) -> Plan {
        serde_json::from_value(json!({"kind": "calls", "calls": calls})).unwrap()
    }

    fn guard(calls: Vec<Value>, inventory: &Value) -> Plan {
        let plan = plan(calls);
        assert_eq!(registry().check_plan(&plan), Ok(()), "plan de test valide");
        guard_attributes(&registry(), plan, inventory)
    }

    fn question(text: &str) -> Plan {
        Plan::Question {
            question: text.to_string(),
            source: QuestionSource::Guard,
        }
    }

    #[test]
    fn an_absent_value_becomes_the_guard_question() {
        let got = guard(
            vec![select("zonage_1a2b", "typezone", "eq", "AU")],
            &inventory(&["N", "U"]),
        );
        assert_eq!(
            got,
            question(
                "Le champ typezone de la couche « Zonage PLU » n'a pas de valeur « AU » (valeurs : N, U) : laquelle faut-il prendre ?"
            )
        );
    }

    #[test]
    fn a_listed_value_leaves_the_plan_unchanged() {
        let calls = vec![select("zonage_1a2b", "typezone", "eq", "U")];
        assert_eq!(guard(calls.clone(), &inventory(&["N", "U"])), plan(calls));
    }

    #[test]
    fn the_comparison_is_exact_case_included() {
        let got = guard(
            vec![select("zonage_1a2b", "typezone", "eq", "u")],
            &inventory(&["N", "U"]),
        );
        assert!(matches!(
            got,
            Plan::Question {
                source: QuestionSource::Guard,
                ..
            }
        ));
    }

    #[test]
    fn ne_is_checked_like_eq() {
        let inv = inventory(&["N", "U"]);
        let calls = vec![select("zonage_1a2b", "typezone", "ne", "U")];
        assert_eq!(guard(calls.clone(), &inv), plan(calls));
        assert!(matches!(
            guard(vec![select("zonage_1a2b", "typezone", "ne", "AU")], &inv),
            Plan::Question { .. }
        ));
    }

    #[test]
    fn a_field_without_values_is_never_checked_on_its_value() {
        let calls = vec![select("gares_3c4d", "nom", "eq", "Gare X")];
        assert_eq!(guard(calls.clone(), &inventory(&["N", "U"])), plan(calls));
    }

    #[test]
    fn an_intermediate_step_is_not_checked() {
        let inv = inventory(&["N", "U"]);
        let calls = vec![
            select("zonage_1a2b", "typezone", "eq", "U"),
            select("step:1.layer_id", "typezone", "eq", "ZZ"),
        ];
        assert_eq!(guard(calls.clone(), &inv), plan(calls));
        // `step:k` seul ne passe pas `check_plan` pour cette commande, mais n'est pas non plus
        // une couche de l'inventaire.
        let whole = plan(vec![
            select("zonage_1a2b", "typezone", "eq", "U"),
            select("step:1", "absent", "eq", "ZZ"),
        ]);
        assert_eq!(guard_attributes(&registry(), whole.clone(), &inv), whole);
    }

    #[test]
    fn an_absent_field_becomes_the_guard_question() {
        let got = guard(
            vec![select("zonage_1a2b", "zone_plu", "eq", "U")],
            &inventory(&["N", "U"]),
        );
        assert_eq!(
            got,
            question(
                "La couche « Zonage PLU » n'a aucun champ « zone_plu » (champs : typezone, surface) : quel champ faut-il utiliser ?"
            )
        );
    }

    #[test]
    fn begins_with_and_contains_need_one_matching_value() {
        let inv = inventory(&["AUc", "AUs", "N", "U"]);
        for (op, value) in [("begins_with", "AU"), ("contains", "Us"), ("contains", "N")] {
            let calls = vec![select("zonage_1a2b", "typezone", op, value)];
            assert_eq!(guard(calls.clone(), &inv), plan(calls), "{op} {value}");
        }
        assert_eq!(
            guard(vec![select("zonage_1a2b", "typezone", "begins_with", "AUx")], &inv),
            question(
                "Le champ typezone de la couche « Zonage PLU » n'a aucune valeur qui commence par « AUx » (valeurs : AUc, AUs, N, U) : laquelle faut-il prendre ?"
            )
        );
        assert_eq!(
            guard(vec![select("zonage_1a2b", "typezone", "contains", "x")], &inv),
            question(
                "Le champ typezone de la couche « Zonage PLU » n'a aucune valeur qui contient « x » (valeurs : AUc, AUs, N, U) : laquelle faut-il prendre ?"
            )
        );
    }

    #[test]
    fn order_comparisons_are_never_checked() {
        let inv = inventory(&["N", "U"]);
        for op in ["lt", "lte", "gt", "gte"] {
            let calls = vec![select("zonage_1a2b", "typezone", op, "ZZ")];
            assert_eq!(guard(calls.clone(), &inv), plan(calls), "{op}");
        }
    }

    #[test]
    fn a_sorted_view_does_not_check_absent_fields_but_checks_values() {
        let mut inv = inventory(&["N", "U"]);
        inv["index"] = json!([
            {"layer_id": "zonage_1a2b", "name": "Zonage PLU", "geometry": "MultiPolygon"},
            {"layer_id": "gares_3c4d", "name": "Gares", "geometry": "Point"},
            {"layer_id": "communes_5e6f", "name": "Communes", "geometry": "MultiPolygon"}
        ]);
        // Champ coupé de la couche détaillée, ou couche montrée seulement dans `index`.
        for calls in [
            vec![select("zonage_1a2b", "zone_plu", "eq", "U")],
            vec![select("communes_5e6f", "nom", "eq", "Annecy")],
        ] {
            assert_eq!(guard(calls.clone(), &inv), plan(calls));
        }
        // Un champ montré l'est avec toutes ses valeurs.
        assert!(matches!(
            guard(vec![select("zonage_1a2b", "typezone", "eq", "AU")], &inv),
            Plan::Question { .. }
        ));
    }

    #[test]
    fn an_unknown_layer_or_a_layer_without_fields_is_not_checked() {
        let mut inv = inventory(&["N", "U"]);
        inv["layers"]
            .as_array_mut()
            .unwrap()
            .push(json!({"layer_id": "mnt_7a8b", "name": "MNT", "kind": "raster"}));
        for calls in [
            vec![select("inconnue_0000", "typezone", "eq", "AU")],
            vec![select("mnt_7a8b", "typezone", "eq", "AU")],
        ] {
            assert_eq!(guard(calls.clone(), &inv), plan(calls));
        }
    }

    #[test]
    fn with_several_faults_the_question_is_on_the_first_in_plan_order() {
        let got = guard(
            vec![
                select("zonage_1a2b", "typezone", "eq", "U"),
                select("zonage_1a2b", "typezone", "eq", "AU"),
                select("zonage_1a2b", "zone_plu", "eq", "U"),
            ],
            &inventory(&["N", "U"]),
        );
        assert_eq!(
            got,
            question(
                "Le champ typezone de la couche « Zonage PLU » n'a pas de valeur « AU » (valeurs : N, U) : laquelle faut-il prendre ?"
            )
        );
    }

    #[test]
    fn a_question_or_a_refusal_passes_unchanged() {
        let inv = inventory(&["N", "U"]);
        for p in [
            Plan::Question {
                question: "Laquelle ?".to_string(),
                source: QuestionSource::Model,
            },
            question(crate::point::GUARD_QUESTION),
            Plan::Refusal {
                reason: "Hors catalogue.".to_string(),
            },
        ] {
            assert_eq!(guard_attributes(&registry(), p.clone(), &inv), p);
        }
    }

    #[test]
    fn every_question_ends_with_a_question_mark() {
        for q in [
            field_question("L", "f", &[]),
            field_question("L", "f", &["a"]),
            value_question("L", "f", "eq", "v", &["a"]),
            value_question("L", "f", "begins_with", "v", &["a"]),
            value_question("L", "f", "contains", "v", &["a"]),
        ] {
            assert!(q.ends_with('?'), "{q}");
        }
    }

    #[test]
    fn like_wildcards_are_not_checked() {
        let inv = inventory(&["AUc", "N", "U"]);
        for (op, value) in [
            ("begins_with", "A_"),
            ("contains", "%x"),
            ("contains", "x_y"),
        ] {
            let calls = vec![select("zonage_1a2b", "typezone", op, value)];
            assert_eq!(guard(calls.clone(), &inv), plan(calls), "{op} {value}");
        }
    }

    #[test]
    fn an_empty_value_is_not_checked() {
        let calls = vec![select("zonage_1a2b", "typezone", "eq", "")];
        assert_eq!(guard(calls.clone(), &inventory(&["N", "U"])), plan(calls));
    }

    #[test]
    fn a_list_longer_than_the_shown_limit_is_not_complete() {
        let many: Vec<String> = (0..=MAX_VALUES).map(|i| format!("Z{i}")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        let calls = vec![select("zonage_1a2b", "typezone", "eq", "AU")];
        assert_eq!(guard(calls.clone(), &inventory(&many)), plan(calls));
    }

    #[test]
    fn begins_with_and_contains_ignore_case_but_eq_does_not() {
        let inv = inventory(&["AUc", "N", "U"]);
        for (op, value) in [("begins_with", "au"), ("contains", "UC")] {
            let calls = vec![select("zonage_1a2b", "typezone", op, value)];
            assert_eq!(guard(calls.clone(), &inv), plan(calls), "{op} {value}");
        }
        assert!(matches!(
            guard(
                vec![select("zonage_1a2b", "typezone", "begins_with", "ux")],
                &inv
            ),
            Plan::Question { .. }
        ));
        assert!(matches!(
            guard(vec![select("zonage_1a2b", "typezone", "eq", "auc")], &inv),
            Plan::Question { .. }
        ));
    }

    #[test]
    fn a_field_name_matches_without_case() {
        let inv = inventory(&["N", "U"]);
        let calls = vec![select("zonage_1a2b", "TypeZone", "eq", "U")];
        assert_eq!(guard(calls.clone(), &inv), plan(calls));
        // Le champ trouvé à la casse près reste contrôlé sur sa valeur.
        assert_eq!(
            guard(vec![select("zonage_1a2b", "TYPEZONE", "eq", "AU")], &inv),
            question(
                "Le champ typezone de la couche « Zonage PLU » n'a pas de valeur « AU » (valeurs : N, U) : laquelle faut-il prendre ?"
            )
        );
    }

    #[test]
    fn the_field_question_names_at_most_ten_fields() {
        let names: Vec<String> = (1..=12).map(|i| format!("c{i}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        assert_eq!(
            field_question("L", "f", &names),
            "La couche « L » n'a aucun champ « f » (champs : c1, c2, c3, c4, c5, c6, c7, c8, c9, c10) : quel champ faut-il utiliser ?"
        );
    }

    #[test]
    fn a_command_without_an_attribute_filter_is_not_checked() {
        let calls = vec![json!({"command": "analysis.buffer",
            "params": {"layer_id": "zonage_1a2b", "distance": {"value": 300, "unit": "m"}}})];
        assert_eq!(guard(calls.clone(), &inventory(&["N", "U"])), plan(calls));
    }
}
