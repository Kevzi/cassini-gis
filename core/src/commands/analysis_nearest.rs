//! Commande `analysis.nearest` : jointure au plus proche voisin. Chaque entité d'une couche
//! reçoit les attributs de l'entité la plus proche d'une seconde couche, et la distance qui
//! les sépare, en mètres. Déclaration seule : le geste vit dans l'extension (AD-22 : aucun
//! nom d'algorithme, aucun nom de champ de QGIS ici). Le noyau nomme le fichier de sortie
//! (AD-10) : une couche, donc `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::analysis_buffer::Distance;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `analysis.nearest`, tels que les écrit l'appelant.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawParams {
    layer_id: String,
    target_layer_id: String,
    #[serde(default)]
    max_distance: Option<Distance>,
    #[serde(default)]
    out: String,
    #[serde(default)]
    work_epsg: Option<u32>,
}

/// Paramètres de `analysis.nearest`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(try_from = "RawParams")]
pub struct AnalysisNearestParams {
    /// Id de la couche qui reçoit le voisin, rendu par `layer.add`, `project.inventory` ou une
    /// analyse (valable pour la session seulement).
    pub layer_id: String,
    /// Id de la couche des voisins. Différente de `layer_id`.
    pub target_layer_id: String,
    /// Distance au-delà de laquelle aucun voisin n'est joint (l'entité est gardée, sans voisin).
    /// Absente : sans limite. À poser seulement si la phrase en donne une.
    #[serde(default)]
    pub max_distance: Option<Distance>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
    /// Système en mètres du calcul, en code EPSG : fourni par le noyau d'après l'inventaire
    /// (changement de cap du 2026-11-12), jamais par l'appelant.
    #[serde(default)]
    pub work_epsg: Option<u32>,
}

impl TryFrom<RawParams> for AnalysisNearestParams {
    type Error = String;

    /// Refuse la même couche des deux côtés, avant tout ordre à QGIS.
    fn try_from(raw: RawParams) -> Result<Self, String> {
        if raw.layer_id == raw.target_layer_id {
            return Err(
                "`layer_id` et `target_layer_id` doivent désigner deux couches différentes".into(),
            );
        }
        Ok(Self {
            layer_id: raw.layer_id,
            target_layer_id: raw.target_layer_id,
            max_distance: raw.max_distance,
            out: raw.out,
            work_epsg: raw.work_epsg,
        })
    }
}

/// Résultat de `analysis.nearest`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisNearestOutput {
    /// Id de la couche créée dans le projet.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités de la couche créée (toutes celles de `layer_id`).
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
    /// Nom du champ de la couche créée qui porte la distance au voisin.
    pub distance_field: String,
    /// Unité du champ de distance (`m`).
    pub unit: String,
    /// Nombre d'entités sans voisin à moins de `max_distance`.
    pub unmatched_count: u64,
}

/// `analysis.nearest` : commande déléguée à QGIS.
pub struct AnalysisNearest;

impl Delegated for AnalysisNearest {
    type Params = AnalysisNearestParams;
    type Output = AnalysisNearestOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisNearestParams, AnalysisNearestOutput>(
            "analysis.nearest",
            "Jointure au plus proche voisin : chaque entité d'une couche reçoit les attributs de l'entité la plus proche d'une seconde couche (ex. la gare la plus proche de chaque parcelle), et la distance qui les sépare, en mètres, dans un champ. Une distance maximale facultative laisse sans voisin les entités trop éloignées. Le résultat est une nouvelle couche ajoutée au projet. Ne modifie pas les couches d'entrée.",
            "Nearest neighbour join: each feature of a layer receives the attributes of the closest feature of a second layer (e.g. the nearest station to each parcel), and the distance between them, in meters, in a field. An optional maximum distance leaves features that are too far without a neighbour. The result is a new layer added to the project. Does not modify the input layers.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        // Une couche en degrés est reprojetée par QGIS, dans la même étape, vers le système en
        // mètres que choisit le noyau (`crate::work_crs`, entrée 9.16) : la distance est en mètres.
        .work_crs_param("work_epsg", "layer_id")
        .example(serde_json::json!({"layer_id": "parcelles_1a2b3c", "target_layer_id": "gares_4d5e6f"}))
        .example(serde_json::json!({"layer_id": "parcelles_1a2b3c", "target_layer_id": "gares_4d5e6f", "max_distance": {"value": 500, "unit": "m"}}))
        .tests(&[
            "analysis_nearest_is_delegated",
            "analysis_nearest_needs_two_different_layers",
            "analysis_nearest_max_distance_is_optional_and_positive",
            "analysis_nearest_output_is_a_gpkg_named_by_the_core",
            "analysis_nearest_declares_its_work_crs",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisNearest>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn analysis_nearest_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.nearest").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call(
                "analysis.nearest",
                json!({"layer_id": "l1", "target_layer_id": "l2"}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_nearest_needs_two_different_layers() {
        let registry = Registry::standard();
        assert!(registry
            .validate_params(
                "analysis.nearest",
                &json!({"layer_id": "l1", "target_layer_id": "l2"})
            )
            .is_ok());
        for bad in [
            json!({"layer_id": "l1"}),
            json!({"target_layer_id": "l2"}),
            json!({"layer_id": "l1", "target_layer_id": "l1"}),
            json!({"layer_id": "l1", "target_layer_id": "l2", "count": 3}),
        ] {
            let err = registry
                .validate_params("analysis.nearest", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
    }

    #[test]
    fn analysis_nearest_max_distance_is_optional_and_positive() {
        let registry = Registry::standard();
        for ok in [
            json!({"layer_id": "l1", "target_layer_id": "l2", "max_distance": {"value": 500, "unit": "m"}}),
            json!({"layer_id": "l1", "target_layer_id": "l2", "max_distance": {"value": 0.5, "unit": "km"}}),
        ] {
            assert!(registry.validate_params("analysis.nearest", &ok).is_ok());
        }
        for bad in [
            json!({"layer_id": "l1", "target_layer_id": "l2", "max_distance": {"value": 0, "unit": "m"}}),
            json!({"layer_id": "l1", "target_layer_id": "l2", "max_distance": {"value": -5, "unit": "m"}}),
            json!({"layer_id": "l1", "target_layer_id": "l2", "max_distance": {"value": 1, "unit": "mi"}}),
            json!({"layer_id": "l1", "target_layer_id": "l2", "max_distance": 500}),
        ] {
            assert!(registry.validate_params("analysis.nearest", &bad).is_err());
        }
    }

    #[test]
    fn analysis_nearest_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.nearest").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn analysis_nearest_declares_its_work_crs() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.nearest").unwrap();
        assert_eq!(
            spec.work_crs_param,
            Some(("work_epsg".to_string(), "layer_id".to_string()))
        );
        let ok = json!({"layer_id": "l1", "target_layer_id": "l2", "work_epsg": 2154});
        assert!(registry.validate_params("analysis.nearest", &ok).is_ok());
        let bad = json!({"layer_id": "l1", "target_layer_id": "l2", "work_epsg": -1});
        assert!(registry.validate_params("analysis.nearest", &bad).is_err());
    }
}
