//! Commande `analysis.zonal_statistics` : pour chaque zone (polygone) d'une couche, nombre
//! d'entités d'une couche de points qu'elle contient. Déclaration seule : le geste vit dans
//! l'extension (AD-22 : aucun nom d'algorithme, aucun nom de champ de QGIS ici). Le noyau
//! nomme le fichier de sortie (AD-10) : une couche, donc `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Nombre maximal de zones détaillées dans le résultat (AD-8) ; la couche créée les garde toutes.
pub const MAX_ZONES: usize = 100;

/// Paramètres de `analysis.zonal_statistics`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalysisZonalStatisticsParams {
    /// Id de la couche de zones (polygones), rendu par `layer.add`, `project.inventory` ou une
    /// analyse (valable pour la session seulement).
    pub layer_id: String,
    /// Id de la couche de points à compter dans chaque zone. Différente de `layer_id`.
    pub points_layer_id: String,
    /// Champ de la couche de zones qui nomme chaque zone dans le résultat (ex. le nom de la
    /// commune). Facultatif : sans lui, une zone est désignée par son numéro d'entité.
    #[serde(default)]
    pub label_field: Option<String>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Décompte d'une zone.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ZoneCount {
    /// Nom de la zone (valeur de `label_field`) ou son numéro d'entité.
    pub label: String,
    /// Nombre de points contenus dans la zone (0 si elle n'en contient aucun).
    pub count: u64,
}

/// Résultat de `analysis.zonal_statistics`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisZonalStatisticsOutput {
    /// Id de la couche créée dans le projet : les zones, avec un champ de décompte.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités (zones) de la couche créée.
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
    /// Décomptes des premières zones, 100 au plus.
    #[schemars(length(max = 100))]
    pub zones: Vec<ZoneCount>,
    /// Vrai si la couche a plus de zones que `zones` n'en détaille.
    pub truncated: bool,
}

/// `analysis.zonal_statistics` : commande déléguée à QGIS.
pub struct AnalysisZonalStatistics;

impl Delegated for AnalysisZonalStatistics {
    type Params = AnalysisZonalStatisticsParams;
    type Output = AnalysisZonalStatisticsOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisZonalStatisticsParams, AnalysisZonalStatisticsOutput>(
            "analysis.zonal_statistics",
            "Statistiques par zone : compte, pour chaque zone d'une couche de polygones (communes, secteurs), les points d'une seconde couche qu'elle contient (ex. gares par commune). Le résultat est une couche des zones avec un champ de décompte, ajoutée au projet, et la liste des décomptes. Ne modifie pas les couches d'entrée.",
            "Zonal statistics: counts, for each zone of a polygon layer (municipalities, sectors), the points of a second layer that it contains (e.g. stations per municipality). The result is a layer of the zones with a count field, added to the project, and the list of counts. Does not modify the input layers.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        .example(serde_json::json!({"layer_id": "communes_1a2b3c", "points_layer_id": "gares_4d5e6f", "label_field": "nom"}))
        .example(serde_json::json!({"layer_id": "secteurs_7a8b9c", "points_layer_id": "arrets_0d1e2f"}))
        .tests(&[
            "analysis_zonal_statistics_is_delegated",
            "analysis_zonal_statistics_needs_two_layers",
            "analysis_zonal_statistics_output_is_a_gpkg_named_by_the_core",
            "analysis_zonal_statistics_zones_are_bounded",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisZonalStatistics>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn analysis_zonal_statistics_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.zonal_statistics").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call(
                "analysis.zonal_statistics",
                json!({"layer_id": "l1", "points_layer_id": "l2"}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_zonal_statistics_needs_two_layers() {
        let registry = Registry::standard();
        for ok in [
            json!({"layer_id": "l1", "points_layer_id": "l2"}),
            json!({"layer_id": "l1", "points_layer_id": "l2", "label_field": "nom"}),
        ] {
            assert!(registry
                .validate_params("analysis.zonal_statistics", &ok)
                .is_ok());
        }
        for bad in [
            json!({"layer_id": "l1"}),
            json!({"points_layer_id": "l2"}),
            json!({"layer_id": "l1", "points_layer_id": "l2", "statistic": "sum"}),
            json!({"layer_id": "l1", "points_layer_id": "l2", "label_field": 3}),
        ] {
            let err = registry
                .validate_params("analysis.zonal_statistics", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
    }

    #[test]
    fn analysis_zonal_statistics_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.zonal_statistics").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
    }

    #[test]
    fn analysis_zonal_statistics_zones_are_bounded() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.zonal_statistics").unwrap();
        let zones = &spec.output_schema["properties"]["zones"];
        assert_eq!(zones["maxItems"], json!(super::MAX_ZONES));
        assert!(registry.check_contract().is_empty());
    }
}
