//! Gabarit unique du résumé chiffré (AD-27) : `rows`, une liste de `{label, values}` où `values`
//! associe un nom à `{value, unit}`, et `truncated`, explicite. Défini une fois, ici ; les
//! commandes à venir (10.3, 10.11) le reprennent sans inventer leur forme. Pas une commande : ce
//! fichier ne s'inscrit pas au registre.
//!
//! L'étiquette vient d'un champ désigné ou du rang, jamais de l'identifiant d'entité. La borne
//! est `maxItems` du schéma de la commande, constante du noyau transmise dans l'ordre : la
//! couche produite garde tout.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Une valeur chiffrée et son unité (`null` : sans unité).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SummaryValue {
    pub value: f64,
    /// Unité de la valeur (« m », « m² », « % »…), absente quand elle n'en a pas.
    pub unit: Option<String>,
}

/// Une ligne du résumé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SummaryRow {
    /// Étiquette lisible : valeur d'un champ désigné, ou rang de la ligne.
    pub label: String,
    /// Valeurs chiffrées, par nom.
    pub values: BTreeMap<String, SummaryValue>,
}

/// Résumé chiffré d'une commande : au plus `maxItems` lignes (borne posée par la commande sur
/// son propre champ), et `truncated` vrai quand des lignes ont été écartées.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SummaryTable {
    pub rows: Vec<SummaryRow>,
    pub truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_summary_template_has_rows_values_with_unit_and_truncated() {
        let schema = serde_json::to_value(schemars::schema_for!(SummaryTable)).unwrap();
        assert_eq!(schema["required"], json!(["rows", "truncated"]));
        assert_eq!(schema["$defs"]["SummaryRow"]["required"], json!(["label", "values"]));
        assert!(schema["$defs"]["SummaryValue"]["properties"]["unit"].is_object());
    }

    #[test]
    fn a_summary_table_round_trips() {
        let table = SummaryTable {
            rows: vec![SummaryRow {
                label: "1".into(),
                values: BTreeMap::from([(
                    "area".to_string(),
                    SummaryValue { value: 12.5, unit: Some("m²".into()) },
                )]),
            }],
            truncated: false,
        };
        let json = serde_json::to_value(&table).unwrap();
        assert_eq!(serde_json::from_value::<SummaryTable>(json).unwrap(), table);
    }

    #[derive(Debug, serde::Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct NoParams {}

    #[derive(Debug, Serialize, JsonSchema)]
    struct Bounded {
        #[schemars(length(max = 2))]
        rows: Vec<SummaryRow>,
        truncated: bool,
    }

    struct Probe;
    impl crate::registry::Delegated for Probe {
        type Params = NoParams;
        type Output = Bounded;
        fn spec() -> crate::registry::CommandSpec {
            crate::registry::CommandSpec::new::<NoParams, Bounded>("test.bounded", "Borne.", "Bound.")
                .runs_in(crate::registry::RunsIn::Qgis)
                .cancellable(false)
                .tests(&["rows_beyond_max_items_are_refused"])
        }
    }

    #[test]
    fn a_malformed_nested_row_is_refused() {
        let mut registry = crate::registry::Registry::new();
        registry.register_delegated::<Probe>();
        let check = |row: serde_json::Value| {
            registry.validate_output("test.bounded", &json!({"rows": [row], "truncated": false}))
        };
        assert!(check(json!({"label": "1", "values": {"n": {"value": 1.0, "unit": "m"}}})).is_ok());
        // Étiquette absente.
        assert!(check(json!({"values": {"n": {"value": 1.0, "unit": null}}})).is_err());
        // Une chaîne là où un nombre est attendu.
        assert!(check(json!({"label": "1", "values": {"n": {"value": "12", "unit": null}}})).is_err());
    }

    #[test]
    fn rows_beyond_max_items_are_refused() {
        let mut registry = crate::registry::Registry::new();
        registry.register_delegated::<Probe>();
        let row = json!({"label": "1", "values": {"n": {"value": 1.0, "unit": null}}});
        let call = |rows: Vec<serde_json::Value>| {
            registry.validate_output("test.bounded", &json!({"rows": rows, "truncated": false}))
        };
        assert!(call(vec![row.clone(), row.clone()]).is_ok());
        let err = call(vec![row.clone(), row.clone(), row]).unwrap_err();
        assert_eq!(err.code(), "qgis.bad_result");
    }
}
