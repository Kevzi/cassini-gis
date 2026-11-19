//! Commande `analysis.count_points_in_polygon` : pour chaque zone (polygone) d'une couche, nombre
//! d'entités d'une couche de points qu'elle contient, et, avec `weight_field`, la somme d'un champ
//! numérique de ces points. Résultat au gabarit commun `SummaryTable` (AD-27). Déclaration seule : le geste vit dans
//! l'extension (AD-22 : aucun nom d'algorithme, aucun nom de champ de QGIS ici). Le noyau
//! nomme le fichier de sortie (AD-10) : une couche, donc `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::summary_row::SummaryRow;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Nombre maximal de lignes détaillées dans le résultat (AD-8) ; la couche créée les garde toutes.
pub const MAX_ROWS: usize = 100;

/// Paramètres de `analysis.count_points_in_polygon`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalysisCountPointsInPolygonParams {
    /// Id de la couche de zones (polygones), rendu par `layer.add`, `project.inventory` ou une
    /// analyse (valable pour la session seulement).
    pub layer_id: String,
    /// Id de la couche de points à compter dans chaque zone. Différente de `layer_id`.
    pub points_layer_id: String,
    /// Champ de la couche de zones qui nomme chaque zone dans le résultat (ex. le nom de la
    /// commune). Facultatif : sans lui, une zone est désignée par son numéro d'entité.
    #[serde(default)]
    pub label_field: Option<String>,
    /// Champ numérique de la couche de points à sommer dans chaque zone (pondération). Facultatif :
    /// sans lui, les lignes ne portent que le décompte.
    #[serde(default)]
    pub weight_field: Option<String>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `analysis.count_points_in_polygon`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisCountPointsInPolygonOutput {
    /// Id de la couche créée dans le projet : les zones, avec un champ de décompte.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités (zones) de la couche créée.
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
    /// Une ligne par zone, 100 au plus : `count` (unité « points ») et, avec `weight_field`,
    /// `weighted_sum` (sans unité). L'étiquette est `label_field`, sinon le rang de la zone.
    #[schemars(length(max = 100))]
    pub rows: Vec<SummaryRow>,
    /// Vrai si la couche a plus de zones que `rows` n'en détaille.
    pub truncated: bool,
}

/// `analysis.count_points_in_polygon` : commande déléguée à QGIS.
pub struct AnalysisCountPointsInPolygon;

impl Delegated for AnalysisCountPointsInPolygon {
    type Params = AnalysisCountPointsInPolygonParams;
    type Output = AnalysisCountPointsInPolygonOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisCountPointsInPolygonParams, AnalysisCountPointsInPolygonOutput>(
            "analysis.count_points_in_polygon",
            "Comptage de points par polygone : compte, pour chaque zone d'une couche de polygones (communes, secteurs), les points d'une seconde couche qu'elle contient (ex. gares par commune). Le résultat est une couche des zones avec un champ de décompte, ajoutée au projet, et un résumé par zone (décompte, et somme pondérée si `weight_field` est donné). Ne modifie pas les couches d'entrée.",
            "Count points in polygon: counts, for each zone of a polygon layer (municipalities, sectors), the points of a second layer that it contains (e.g. stations per municipality). The result is a layer of the zones with a count field, added to the project, and a per-zone summary (count, and weighted sum when `weight_field` is given). Does not modify the input layers.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        .example(serde_json::json!({"layer_id": "communes_1a2b3c", "points_layer_id": "gares_4d5e6f", "label_field": "nom"}))
        .example(serde_json::json!({"layer_id": "secteurs_7a8b9c", "points_layer_id": "arrets_0d1e2f"}))
        .example(serde_json::json!({"layer_id": "communes_1a2b3c", "points_layer_id": "arrets_0d1e2f", "label_field": "nom", "weight_field": "voyageurs"}))
        .tests(&[
            "analysis_count_points_in_polygon_is_delegated",
            "analysis_count_points_in_polygon_needs_two_layers",
            "analysis_count_points_in_polygon_output_is_a_gpkg_named_by_the_core",
            "analysis_count_points_in_polygon_rows_are_bounded",
            "analysis_count_points_in_polygon_old_id_is_unknown",
            "analysis_count_points_in_polygon_legacy_zones_result_is_refused",
            "analysis_count_points_in_polygon_rows_follow_the_summary_template",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisCountPointsInPolygon>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn analysis_count_points_in_polygon_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.count_points_in_polygon").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call(
                "analysis.count_points_in_polygon",
                json!({"layer_id": "l1", "points_layer_id": "l2"}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_count_points_in_polygon_needs_two_layers() {
        let registry = Registry::standard();
        for ok in [
            json!({"layer_id": "l1", "points_layer_id": "l2"}),
            json!({"layer_id": "l1", "points_layer_id": "l2", "label_field": "nom"}),
            json!({"layer_id": "l1", "points_layer_id": "l2", "weight_field": "poids"}),
        ] {
            assert!(registry
                .validate_params("analysis.count_points_in_polygon", &ok)
                .is_ok());
        }
        for bad in [
            json!({"layer_id": "l1"}),
            json!({"points_layer_id": "l2"}),
            json!({"layer_id": "l1", "points_layer_id": "l2", "statistic": "sum"}),
            json!({"layer_id": "l1", "points_layer_id": "l2", "label_field": 3}),
            json!({"layer_id": "l1", "points_layer_id": "l2", "weight_field": 3}),
        ] {
            let err = registry
                .validate_params("analysis.count_points_in_polygon", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
    }

    #[test]
    fn analysis_count_points_in_polygon_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.count_points_in_polygon").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
    }

    #[test]
    fn analysis_count_points_in_polygon_rows_are_bounded() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.count_points_in_polygon").unwrap();
        let rows = &spec.output_schema["properties"]["rows"];
        assert_eq!(rows["maxItems"], json!(super::MAX_ROWS));
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn analysis_count_points_in_polygon_old_id_is_unknown() {
        let registry = Registry::standard();
        assert!(registry.spec("analysis.zonal_statistics").is_none());
    }

    #[test]
    fn analysis_count_points_in_polygon_legacy_zones_result_is_refused() {
        let registry = Registry::standard();
        let old = json!({"layer_id": "z1", "name": "n", "feature_count": 1, "crs": "EPSG:2154",
            "zones": [{"label": "a", "count": 1}], "truncated": false});
        let err = registry
            .validate_output("analysis.count_points_in_polygon", &old)
            .unwrap_err();
        assert_eq!(err.code(), "qgis.bad_result");
    }

    #[test]
    fn analysis_count_points_in_polygon_rows_follow_the_summary_template() {
        let registry = Registry::standard();
        let row = |values: serde_json::Value| json!({"label": "a", "values": values});
        let result = |rows: Vec<serde_json::Value>| {
            json!({"layer_id": "z1", "name": "n", "feature_count": rows.len(), "crs": "EPSG:2154",
                "rows": rows, "truncated": false})
        };
        let simple = result(vec![row(json!({"count": {"value": 3.0, "unit": "points"}}))]);
        let weighted = result(vec![row(json!({
            "count": {"value": 3.0, "unit": "points"},
            "weighted_sum": {"value": 41.5, "unit": null}}))]);
        let empty = result(vec![]);
        for ok in [&simple, &weighted, &empty] {
            assert!(registry.validate_output("analysis.count_points_in_polygon", ok).is_ok());
        }
    }
}
