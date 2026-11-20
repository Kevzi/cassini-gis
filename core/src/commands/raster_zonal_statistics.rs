//! Commande `raster.zonal_statistics` : pour chaque polygone d'une couche, moyenne, minimum et
//! maximum des valeurs d'un raster (altitudes d'un MNT, par exemple). Résultat au gabarit commun
//! `SummaryRow` (AD-27). Déclaration seule : le geste vit dans l'extension (AD-22 : aucun nom
//! d'algorithme, aucun nom de champ de QGIS ici). Le raster se désigne par un `RasterPath`
//! (amendement du 2026-11-05 d'AD-24) : chemin relatif au projet, ou `step:k` dans un plan, jamais
//! par un identifiant de couche, qui ne se rejoue pas. Le noyau nomme le fichier de sortie
//! (AD-10) : une couche, donc `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::summary_row::SummaryRow;
use crate::raster_path::RasterPath;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Nombre maximal de lignes détaillées dans le résultat (AD-8) ; la couche créée les garde toutes.
pub const MAX_ROWS: usize = 100;

/// Paramètres de `raster.zonal_statistics`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RasterZonalStatisticsParams {
    /// Raster à résumer (une bande), copié de l'inventaire ; `step:k` dans un plan pour le raster
    /// écrit par l'étape `k`. Pas un id de couche.
    pub raster: RasterPath,
    /// Id de la couche de polygones (zones), rendu par `layer.add`, `project.inventory` ou une
    /// analyse (valable pour la session seulement). Son système de coordonnées peut différer de celui du
    /// raster : QGIS reprojette les polygones, et la couche créée garde le système des polygones.
    pub layer_id: String,
    /// Champ de la couche de polygones qui nomme chaque zone dans le résultat (ex. le nom de la
    /// commune). Facultatif : sans lui, une zone est désignée par son numéro d'entité.
    #[serde(default)]
    pub label_field: Option<String>,
    /// Unité des valeurs du raster (ex. `m` pour un MNT, `°` pour une pente), si le fichier ne la
    /// porte pas. Facultatif pour un raster désigné par un chemin du projet (sinon l'unité du fichier, à
    /// défaut le mètre) ; obligatoire quand `raster` est une référence `step:k`, dont les valeurs n'ont
    /// pas d'unité connue (« ° » ou « % » pour une pente).
    #[serde(default)]
    pub unit: Option<String>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `raster.zonal_statistics`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RasterZonalStatisticsOutput {
    /// Id de la couche créée dans le projet : les polygones, avec les champs de statistiques.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités (polygones) de la couche créée.
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
    /// Une ligne par polygone, 100 au plus : `mean`, `min` et `max` avec l'unité du raster, absents
    /// pour un polygone qui ne recouvre aucune cellule. L'étiquette est `label_field`, sinon le
    /// rang du polygone.
    #[schemars(length(max = 100))]
    pub rows: Vec<SummaryRow>,
    /// Vrai si la couche a plus de polygones que `rows` n'en détaille.
    pub truncated: bool,
}

/// `raster.zonal_statistics` : commande déléguée à QGIS.
pub struct RasterZonalStatistics;

impl Delegated for RasterZonalStatistics {
    type Params = RasterZonalStatisticsParams;
    type Output = RasterZonalStatisticsOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<RasterZonalStatisticsParams, RasterZonalStatisticsOutput>(
            "raster.zonal_statistics",
            "Statistiques d'un raster par polygone : moyenne, minimum et maximum des valeurs d'un raster à une bande (fichier GeoTIFF désigné par `raster`, chemin recopié de l'inventaire ou `step:k`, pas un id de couche, ex. les altitudes d'un MNT) pour chaque zone d'une couche de polygones (`layer_id`, communes, secteurs), , quel que soit leur système de coordonnées (QGIS reprojette les polygones). Le résultat est une couche des polygones avec ces trois champs, ajoutée au projet, et un résumé par zone avec l'unité du raster. Ne modifie ni le raster ni la couche d'entrée.",
            "Raster statistics by polygon: mean, minimum and maximum of the values of a single-band raster (GeoTIFF file given by `raster`, a path copied from the inventory or `step:k`, not a layer id, e.g. the elevations of a DEM) for each zone of a polygon layer (`layer_id`, municipalities, sectors), , whatever their coordinate system (QGIS reprojects the polygons). The result is a layer of the polygons with these three fields, added to the project, and a per-zone summary with the raster's unit. Does not modify the raster nor the input layer.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .path_params(&["raster"])
        .output_params(&["out"])
        .output_ext(".gpkg")
        .example(serde_json::json!({"raster": "dem/mnt.tif", "layer_id": "communes_1a2b3c", "label_field": "nom"}))
        .example(serde_json::json!({"raster": "mnt-colline-2154.tif", "layer_id": "secteurs_7a8b9c"}))
        .example(serde_json::json!({"raster": "step:1", "layer_id": "communes_1a2b3c", "label_field": "nom", "unit": "°"}))
        .tests(&[
            "raster_zonal_statistics_is_delegated",
            "raster_zonal_statistics_needs_a_raster_and_a_layer",
            "raster_zonal_statistics_output_is_a_gpkg_named_by_the_core",
            "raster_zonal_statistics_rows_are_bounded",
            "raster_zonal_statistics_designates_the_raster_by_path",
            "raster_zonal_statistics_rows_follow_the_summary_template",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<RasterZonalStatistics>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn raster_zonal_statistics_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.zonal_statistics").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call(
                "raster.zonal_statistics",
                json!({"raster": "mnt.tif", "layer_id": "l1"}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn raster_zonal_statistics_needs_a_raster_and_a_layer() {
        let registry = Registry::standard();
        for ok in [
            json!({"raster": "mnt.tif", "layer_id": "l1"}),
            json!({"raster": "mnt.tif", "layer_id": "l1", "label_field": "nom"}),
            json!({"raster": "mnt.tif", "layer_id": "l1", "unit": "m"}),
        ] {
            assert!(registry.validate_params("raster.zonal_statistics", &ok).is_ok());
        }
        for bad in [
            json!({"raster": "mnt.tif"}),
            json!({"layer_id": "l1"}),
            json!({"raster": "mnt.tif", "layer_id": "l1", "statistic": "mean"}),
            json!({"raster": "mnt.tif", "layer_id": "l1", "label_field": 3}),
            json!({"raster": 3, "layer_id": "l1"}),
        ] {
            let err = registry.validate_params("raster.zonal_statistics", &bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
    }

    fn plan_of(raster: &str, unit: Option<&str>) -> crate::plan::Plan {
        let mut params = json!({"raster": raster, "layer_id": "l1"});
        if let Some(u) = unit {
            params["unit"] = json!(u);
        }
        serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "terrain.slope", "params": {"dem": "dem/mnt.tif", "unit": "degrees"}},
            {"command": "raster.zonal_statistics", "params": {
                "raster": if raster == "step:1" { "step:1" } else { raster }, "layer_id": "l1",
                "unit": params.get("unit").cloned().unwrap_or(serde_json::Value::Null)}}
        ]}))
        .unwrap()
    }

    #[test]
    fn raster_zonal_statistics_refuses_a_step_raster_without_unit() {
        let registry = Registry::standard();
        let err = registry.check_plan(&plan_of("step:1", None)).unwrap_err();
        assert_eq!((err.code, err.step), ("plan.missing_unit", 2));
        assert!(err.message.contains("unit"));
        assert_eq!(registry.check_plan(&plan_of("step:1", Some("°"))), Ok(()));
    }

    #[test]
    fn raster_zonal_statistics_accepts_a_path_raster_without_unit() {
        let registry = Registry::standard();
        assert_eq!(registry.check_plan(&plan_of("dem/mnt.tif", None)), Ok(()));
    }

    #[test]
    fn raster_zonal_statistics_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.zonal_statistics").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
    }

    #[test]
    fn raster_zonal_statistics_rows_are_bounded() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.zonal_statistics").unwrap();
        let rows = &spec.output_schema["properties"]["rows"];
        assert_eq!(rows["maxItems"], json!(super::MAX_ROWS));
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn raster_zonal_statistics_designates_the_raster_by_path() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.zonal_statistics").unwrap();
        assert_eq!(spec.path_params, vec!["raster".to_string()]);
    }

    #[test]
    fn raster_zonal_statistics_rows_follow_the_summary_template() {
        let registry = Registry::standard();
        let stat = |v: f64| json!({"value": v, "unit": "m"});
        let result = |rows: Vec<serde_json::Value>| {
            json!({"layer_id": "z1", "name": "n", "feature_count": rows.len(), "crs": "EPSG:2154",
                "rows": rows, "truncated": false})
        };
        let full = result(vec![json!({"label": "a", "values": {
            "mean": stat(95.5), "min": stat(80.0), "max": stat(120.0)}})]);
        // Un polygone hors de l'emprise du raster : aucune valeur, la ligne reste rendue.
        let outside = result(vec![json!({"label": "b", "values": {}})]);
        let empty = result(vec![]);
        for ok in [&full, &outside, &empty] {
            assert!(registry.validate_output("raster.zonal_statistics", ok).is_ok());
        }
        let old = json!({"layer_id": "z1", "name": "n", "feature_count": 1, "crs": "EPSG:2154",
            "zones": [{"label": "a", "mean": 1.0}], "truncated": false});
        assert!(registry.validate_output("raster.zonal_statistics", &old).is_err());
    }
}
