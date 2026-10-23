//! Commande `analysis.isochrone` : zone atteignable en un temps donné depuis un point, sur un
//! réseau routier apporté par l'utilisateur. Déclaration seule : le geste vit dans l'extension
//! (AD-22 : aucun nom d'algorithme ici). Le noyau nomme le fichier de sortie (AD-10) : `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::point::Point;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

// Pas de commentaire par variante : schemars en ferait une liste de `oneOf`, alors que le
// panneau ne sait faire une liste déroulante que d'une énumération plate.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(inline)]
pub enum TravelMode {
    Walk,
    Car,
}

/// Unité d'une durée.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DurationUnit {
    /// Secondes.
    S,
    /// Minutes.
    Min,
}

/// Durée mesurée, avec son unité (jamais de mesure nue).
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Duration {
    /// Valeur strictement positive.
    #[schemars(range(min = 0.0))]
    #[serde(deserialize_with = "positive")]
    pub value: f64,
    /// Unité de la valeur.
    pub unit: DurationUnit,
}

/// Refuse une valeur nulle, négative ou non finie, avant tout ordre à QGIS.
fn positive<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    let v = f64::deserialize(d)?;
    if v.is_finite() && v > 0.0 {
        Ok(v)
    } else {
        Err(serde::de::Error::custom(
            "la durée doit être strictement positive",
        ))
    }
}

/// Paramètres de `analysis.isochrone`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalysisIsochroneParams {
    /// Id de la couche de lignes qui forme le réseau routier, rendu par `layer.add` ou
    /// `project.inventory` (valable pour la session seulement). Le réseau est apporté par
    /// l'utilisateur et doit être dans un système métrique.
    pub layer_id: String,
    /// Point de départ, avec son système de coordonnées.
    pub origin: Point,
    /// Mode de déplacement : `walk` à pied (5 km/h) ou `car` en voiture (50 km/h).
    pub mode: TravelMode,
    /// Durée maximale du trajet.
    pub duration: Duration,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `analysis.isochrone`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisIsochroneOutput {
    /// Id de la couche créée dans le projet.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités de la couche créée (un polygone).
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
}

/// `analysis.isochrone` : commande déléguée à QGIS.
pub struct AnalysisIsochrone;

impl Delegated for AnalysisIsochrone {
    type Params = AnalysisIsochroneParams;
    type Output = AnalysisIsochroneOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisIsochroneParams, AnalysisIsochroneOutput>(
            "analysis.isochrone",
            "Calcule la zone atteignable en un temps donné, à pied ou en voiture, depuis un point, sur un réseau routier (couche de lignes) apporté par l'utilisateur, et l'ajoute au projet comme une couche d'un polygone. Vitesses fixes (marche 5 km/h, voiture 50 km/h), tronçons parcourus dans les deux sens. Le polygone est l'enveloppe convexe des tronçons atteints : il peut surestimer la zone sur un réseau en étoile ou coupé par une rivière. Le réseau doit être en mètres. L'origine doit être à moins de 100 m du réseau, sinon la commande est refusée. Ne modifie pas le réseau.",
            "Computes the area reachable within a given time, on foot or by car, from a point, on a road network (line layer) provided by the user, and adds it to the project as a one-polygon layer. Fixed speeds (walking 5 km/h, car 50 km/h), segments travelled both ways. The polygon is the convex hull of the reached segments: it may overestimate the area on a star-shaped network or one cut by a river. The network must use a metric coordinate system. The origin must be within 100 m of the network, otherwise the command is refused. Does not modify the network.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        .example(serde_json::json!({"layer_id": "routes_1a2b3c", "origin": {"x": 652000, "y": 6850000, "epsg": 2154}, "mode": "walk", "duration": {"value": 10, "unit": "min"}}))
        .example(serde_json::json!({"layer_id": "routes_1a2b3c", "origin": {"x": 652000, "y": 6850000, "epsg": 2154}, "mode": "car", "duration": {"value": 300, "unit": "s"}}))
        .tests(&[
            "analysis_isochrone_is_delegated",
            "analysis_isochrone_refuses_a_non_positive_duration",
            "analysis_isochrone_mode_is_closed",
            "analysis_isochrone_output_is_a_gpkg_named_by_the_core",
            "analysis_isochrone_description_states_the_snap_distance",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisIsochrone>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    fn params(mode: &str, value: serde_json::Value, unit: &str) -> serde_json::Value {
        json!({
            "layer_id": "l1",
            "origin": {"x": 652000, "y": 6850000, "epsg": 2154},
            "mode": mode,
            "duration": {"value": value, "unit": unit},
        })
    }

    #[test]
    fn analysis_isochrone_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.isochrone").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call("analysis.isochrone", params("walk", json!(3), "min"))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_isochrone_refuses_a_non_positive_duration() {
        let registry = Registry::standard();
        for value in [0, -5] {
            let err = registry
                .validate_params("analysis.isochrone", &params("walk", json!(value), "min"))
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
        for unit in ["s", "min"] {
            assert!(registry
                .validate_params("analysis.isochrone", &params("car", json!(180), unit))
                .is_ok());
        }
        // Une unité inconnue est refusée.
        assert!(registry
            .validate_params("analysis.isochrone", &params("walk", json!(1), "h"))
            .is_err());
    }

    #[test]
    fn analysis_isochrone_mode_is_closed() {
        let registry = Registry::standard();
        for mode in ["walk", "car"] {
            assert!(registry
                .validate_params("analysis.isochrone", &params(mode, json!(3), "min"))
                .is_ok());
        }
        assert!(registry
            .validate_params("analysis.isochrone", &params("bike", json!(3), "min"))
            .is_err());
        // Pas de coordonnée nue.
        assert!(registry
            .validate_params(
                "analysis.isochrone",
                &json!({"layer_id": "l1", "origin": {"x": 1, "y": 2}, "mode": "walk",
                        "duration": {"value": 3, "unit": "min"}}),
            )
            .is_err());
    }

    #[test]
    fn analysis_isochrone_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.isochrone").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
    }

    #[test]
    fn analysis_isochrone_description_states_the_snap_distance() {
        // L'IA lit la description : elle doit savoir que l'origine est refusée à plus de 100 m
        // du réseau (`_ORIGIN_SNAP_METERS` de l'extension).
        let registry = Registry::standard();
        let spec = registry.spec("analysis.isochrone").unwrap();
        assert!(spec.description_fr.contains("100 m du réseau"));
        assert!(spec.description_en.contains("100 m of the network"));
    }
}
