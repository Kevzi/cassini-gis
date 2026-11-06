//! Commande `data.transform_point` : reprojection d'un seul point vers un code EPSG, par QGIS.
//! Déclaration seule : le geste vit dans l'extension (`point_transform.py`), qui reprojette par
//! `QgsCoordinateTransform` dans le contexte de transformation du projet (AD-19 : la
//! reprojection est celle de QGIS, jamais du noyau). Aucune couche créée, aucun fichier écrit :
//! pas de `out`.
//!
//! C'est le pont entre un point Lambert-93 (ou tout autre système) et la géodésie du noyau
//! (`coord.convert`, `geo.*`, qui n'acceptent que WGS84 et UTM), et entre un point WGS84 et le
//! système d'un MNT. Erreurs du geste : `data.unknown_crs` (code source ou cible inconnu de
//! QGIS, nommé dans le message) et `data.transform_failed` (point que QGIS ne sait pas
//! reprojeter).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::point::Point;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `data.transform_point`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataTransformPointParams {
    /// Point à reprojeter, écrit dans la demande ou rendu par une étape (tout code EPSG connu
    /// de QGIS).
    pub point: Point,
    /// Code EPSG du système d'arrivée (ex. 4326 pour le WGS84, 2154 pour le Lambert-93).
    pub target_epsg: u32,
}

/// Résultat de `data.transform_point`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DataTransformPointOutput {
    /// Point reprojeté, `epsg` égal à `target_epsg`.
    pub point: Point,
}

/// `data.transform_point` : commande déléguée à QGIS.
pub struct DataTransformPoint;

impl Delegated for DataTransformPoint {
    type Params = DataTransformPointParams;
    type Output = DataTransformPointOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<DataTransformPointParams, DataTransformPointOutput>(
            "data.transform_point",
            "Reprojette un seul point vers un code EPSG, par QGIS ; relie un point Lambert-93 (EPSG:2154) à coord.convert et geo.*, qui n'acceptent que WGS84 et UTM, ou un point WGS84 au système d'un MNT. Ne crée aucune couche.",
            "Reprojects a single point to an EPSG code, through QGIS; links a Lambert-93 point (EPSG:2154) to coord.convert and geo.*, which only accept WGS84 and UTM, or a WGS84 point to the system of a DEM. Creates no layer.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .example(serde_json::json!({"point": {"x": 652000.0, "y": 6853000.0, "epsg": 2154}, "target_epsg": 4326}))
        .example(serde_json::json!({"point": {"x": 2.4069, "y": 48.7353, "epsg": 4326}, "target_epsg": 2154}))
        .tests(&[
            "data_transform_point_is_delegated",
            "data_transform_point_params_are_a_point_and_a_code",
            "data_transform_point_writes_no_file",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<DataTransformPoint>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    fn lambert() -> serde_json::Value {
        json!({"x": 700000.0, "y": 6600000.0, "epsg": 2154})
    }

    #[test]
    fn data_transform_point_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("data.transform_point").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call(
                "data.transform_point",
                json!({"point": lambert(), "target_epsg": 4326}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn data_transform_point_params_are_a_point_and_a_code() {
        let registry = Registry::standard();
        for good in [
            json!({"point": lambert(), "target_epsg": 4326}),
            json!({"point": {"x": 3.0, "y": 46.5, "epsg": 4326}, "target_epsg": 2154}),
            json!({"point": lambert(), "target_epsg": 2154}),
        ] {
            assert!(
                registry
                    .validate_params("data.transform_point", &good)
                    .is_ok(),
                "{good}"
            );
        }
        for bad in [
            json!({"point": lambert()}),
            json!({"target_epsg": 4326}),
            json!({"point": lambert(), "target_epsg": -4326}),
            json!({"point": lambert(), "target_epsg": 4326.5}),
            json!({"point": lambert(), "target_epsg": "4326"}),
            json!({"point": lambert(), "target_epsg": 4326, "out": "p.gpkg"}),
            json!({"point": {"x": 700000.0, "y": 6600000.0}, "target_epsg": 4326}),
            json!({"point": {"x": 700000.0, "y": 6600000.0, "epsg": -2154}, "target_epsg": 4326}),
        ] {
            let err = registry
                .validate_params("data.transform_point", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn data_transform_point_writes_no_file() {
        let registry = Registry::standard();
        let spec = registry.spec("data.transform_point").unwrap();
        assert!(spec.output_params.is_empty());
        assert!(!spec.dictated_by_core);
    }
}
