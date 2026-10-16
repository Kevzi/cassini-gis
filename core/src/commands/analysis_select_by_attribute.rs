//! Commande `analysis.select_by_attribute` : entités d'une couche dont un champ vérifie
//! une comparaison. Déclaration seule : le geste vit dans l'extension (AD-22 : ni nom
//! d'algorithme ni expression ici). Le filtre est un champ, un opérateur et une valeur,
//! jamais un texte à interpréter. Le noyau nomme le fichier de sortie (AD-10) : `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

// Pas de commentaire par variante : schemars en ferait une liste de `oneOf`, alors que le
// panneau ne sait faire une liste déroulante que d'une énumération plate.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(inline)]
pub enum AttributeOperator {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    Contains,
    BeginsWith,
}

/// Paramètres de `analysis.select_by_attribute`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalysisSelectByAttributeParams {
    /// Id de la couche vecteur à filtrer, rendu par `layer.add`, `project.inventory` ou une
    /// analyse (valable pour la session seulement).
    pub layer_id: String,
    /// Nom d'un champ de la couche, tel que `layer.describe` le donne.
    pub field: String,
    /// Comparaison : `eq` égal, `ne` différent, `lt` `lte` `gt` `gte` plus petit ou plus
    /// grand (strict ou non), `contains` contient, `begins_with` commence par.
    pub operator: AttributeOperator,
    /// Valeur à comparer, toujours écrite en texte (ex. `"U"`, `"1000"`) ; QGIS la convertit
    /// selon le type du champ.
    pub value: String,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `analysis.select_by_attribute`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisSelectByAttributeOutput {
    /// Id de la couche créée dans le projet.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités retenues (0 si aucune ne convient).
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
}

/// `analysis.select_by_attribute` : commande déléguée à QGIS.
pub struct AnalysisSelectByAttribute;

impl Delegated for AnalysisSelectByAttribute {
    type Params = AnalysisSelectByAttributeParams;
    type Output = AnalysisSelectByAttributeOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisSelectByAttributeParams, AnalysisSelectByAttributeOutput>(
            "analysis.select_by_attribute",
            "Garde les entités d'une couche vecteur dont un champ vérifie une comparaison (champ, opérateur, valeur) et les ajoute au projet comme nouvelle couche. Ne modifie pas la couche d'entrée.",
            "Keeps the features of a vector layer whose field satisfies a comparison (field, operator, value) and adds them to the project as a new layer. Does not modify the input layer.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        .example(serde_json::json!({"layer_id": "zonage_7a8b9c", "field": "typezone", "operator": "eq", "value": "U"}))
        .example(serde_json::json!({"layer_id": "communes_0d1e2f", "field": "population", "operator": "gt", "value": "1000"}))
        .example(serde_json::json!({"layer_id": "gares_1a2b3c", "field": "nom", "operator": "begins_with", "value": "Ville"}))
        .tests(&[
            "analysis_select_by_attribute_is_delegated",
            "analysis_select_by_attribute_takes_a_field_an_operator_and_a_text_value",
            "analysis_select_by_attribute_output_is_a_gpkg_named_by_the_core",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisSelectByAttribute>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    fn params(operator: &str) -> serde_json::Value {
        json!({"layer_id": "l1", "field": "zone", "operator": operator, "value": "U"})
    }

    #[test]
    fn analysis_select_by_attribute_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.select_by_attribute").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call("analysis.select_by_attribute", params("eq"))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_select_by_attribute_takes_a_field_an_operator_and_a_text_value() {
        let registry = Registry::standard();
        for op in [
            "eq",
            "ne",
            "lt",
            "lte",
            "gt",
            "gte",
            "contains",
            "begins_with",
        ] {
            assert!(
                registry
                    .validate_params("analysis.select_by_attribute", &params(op))
                    .is_ok(),
                "{op}"
            );
        }
        // Opérateur inconnu, valeur non textuelle, paramètre en trop ou manquant : refusés.
        for bad in [
            params("is_null"),
            params("like"),
            json!({"layer_id": "l1", "field": "pop", "operator": "gt", "value": 1000}),
            json!({"layer_id": "l1", "field": "zone", "operator": "eq"}),
            json!({"layer_id": "l1", "operator": "eq", "value": "U"}),
            json!({"layer_id": "l1", "field": "zone", "operator": "eq", "value": "U", "expression": "1=1"}),
        ] {
            let err = registry
                .validate_params("analysis.select_by_attribute", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
    }

    #[test]
    fn analysis_select_by_attribute_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.select_by_attribute").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
        // L'opérateur est une énumération plate : le panneau en fait une liste.
        let operator = &spec.params_schema["properties"]["operator"];
        assert!(operator["enum"].is_array(), "{operator}");
    }
}
