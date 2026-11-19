//! Commande `analysis.statistics_by_category` : compte, et somme d'un champ numérique, par valeur
//! d'un champ de catégorie. Le résultat est un tableau sans géométrie, écrit dans le dossier de
//! travail (AD-10) et résumé au gabarit commun `SummaryRow` (AD-27) ; il n'est pas ajouté au
//! projet. Déclaration seule : le geste vit dans l'extension (AD-22 : aucun nom d'algorithme,
//! aucun nom de champ de QGIS ici). Le noyau nomme le fichier de sortie : un tableau, donc `.csv`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::summary_row::SummaryRow;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `analysis.statistics_by_category`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalysisStatisticsByCategoryParams {
    /// Id de la couche à résumer, rendu par `layer.add`, `project.inventory` ou une analyse
    /// (valable pour la session seulement). Elle n'est pas modifiée.
    pub layer_id: String,
    /// Champ de la couche dont chaque valeur est une catégorie (ex. le type de zone).
    pub category_field: String,
    /// Champ numérique de la couche à sommer dans chaque catégorie. Facultatif : sans lui, les
    /// lignes ne portent que le décompte.
    #[serde(default)]
    pub sum_field: Option<String>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `analysis.statistics_by_category`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisStatisticsByCategoryOutput {
    /// Tableau écrit (CSV sans géométrie), toutes les catégories ; il n'est pas ajouté au projet.
    pub file: String,
    /// Une ligne par catégorie, 100 au plus : `count` (unité « entités ») et, avec `sum_field`,
    /// `sum` (sans unité). L'étiquette est la valeur de la catégorie, sinon son rang.
    #[schemars(length(max = 100))]
    pub rows: Vec<SummaryRow>,
    /// Vrai si la couche a plus de catégories que `rows` n'en détaille.
    pub truncated: bool,
}

/// `analysis.statistics_by_category` : commande déléguée à QGIS.
pub struct AnalysisStatisticsByCategory;

impl Delegated for AnalysisStatisticsByCategory {
    type Params = AnalysisStatisticsByCategoryParams;
    type Output = AnalysisStatisticsByCategoryOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisStatisticsByCategoryParams, AnalysisStatisticsByCategoryOutput>(
            "analysis.statistics_by_category",
            "Statistiques par catégorie : compte les entités de chaque valeur d'un champ de catégorie d'une couche (ex. les parcelles par type de zone) et, avec `sum_field`, somme un champ numérique par catégorie (ex. la surface par type de zone). Le résultat est un tableau écrit dans le dossier de travail, sans géométrie, qui n'est pas ajouté au projet, et un résumé par catégorie. Ne modifie pas la couche d'entrée.",
            "Statistics by category: counts the features of each value of a category field of a layer (e.g. parcels per zone type) and, with `sum_field`, sums a numeric field per category (e.g. the area per zone type). The result is a table written in the working folder, without geometry, which is not added to the project, and a per-category summary. Does not modify the input layer.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".csv")
        .example(serde_json::json!({"layer_id": "parcelles_1a2b3c", "category_field": "zonage"}))
        .example(serde_json::json!({"layer_id": "parcelles_1a2b3c", "category_field": "zonage", "sum_field": "surface"}))
        .tests(&[
            "analysis_statistics_by_category_is_delegated",
            "analysis_statistics_by_category_needs_a_layer_and_a_category_field",
            "analysis_statistics_by_category_output_is_a_csv_named_by_the_core",
            "analysis_statistics_by_category_rows_are_bounded",
            "analysis_statistics_by_category_rows_follow_the_summary_template",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisStatisticsByCategory>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn analysis_statistics_by_category_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.statistics_by_category").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call(
                "analysis.statistics_by_category",
                json!({"layer_id": "l1", "category_field": "zonage"}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_statistics_by_category_needs_a_layer_and_a_category_field() {
        let registry = Registry::standard();
        for ok in [
            json!({"layer_id": "l1", "category_field": "zonage"}),
            json!({"layer_id": "l1", "category_field": "zonage", "sum_field": "surface"}),
        ] {
            assert!(registry
                .validate_params("analysis.statistics_by_category", &ok)
                .is_ok());
        }
        for bad in [
            json!({"layer_id": "l1"}),
            json!({"category_field": "zonage"}),
            json!({"layer_id": "l1", "category_field": 3}),
            json!({"layer_id": "l1", "category_field": "zonage", "sum_field": 3}),
            json!({"layer_id": "l1", "category_field": "zonage", "statistic": "mean"}),
        ] {
            let err = registry
                .validate_params("analysis.statistics_by_category", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
    }

    #[test]
    fn analysis_statistics_by_category_output_is_a_csv_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.statistics_by_category").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".csv");
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn analysis_statistics_by_category_rows_are_bounded() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.statistics_by_category").unwrap();
        let rows = &spec.output_schema["properties"]["rows"];
        assert_eq!(rows["maxItems"], json!(100));
    }

    #[test]
    fn analysis_statistics_by_category_rows_follow_the_summary_template() {
        let registry = Registry::standard();
        let row = |values: serde_json::Value| json!({"label": "UA", "values": values});
        let result = |rows: Vec<serde_json::Value>, truncated: bool| {
            json!({"file": "1-analysis.statistics_by_category.csv", "rows": rows, "truncated": truncated})
        };
        let counts = result(
            vec![row(json!({"count": {"value": 3.0, "unit": "entités"}}))],
            false,
        );
        let summed = result(
            vec![row(json!({
                "count": {"value": 3.0, "unit": "entités"},
                "sum": {"value": 41.5, "unit": null}}))],
            true,
        );
        let empty = result(vec![], false);
        for ok in [&counts, &summed, &empty] {
            assert!(registry
                .validate_output("analysis.statistics_by_category", ok)
                .is_ok());
        }
        // Un résultat sans `truncated`, ou avec une couche à la place du fichier, est refusé.
        let no_flag = json!({"file": "f.csv", "rows": []});
        let layer = json!({"layer_id": "l1", "name": "n", "feature_count": 1, "crs": "EPSG:2154",
            "rows": [], "truncated": false});
        for bad in [&no_flag, &layer] {
            let err = registry
                .validate_output("analysis.statistics_by_category", bad)
                .unwrap_err();
            assert_eq!(err.code(), "qgis.bad_result");
        }
    }
}
