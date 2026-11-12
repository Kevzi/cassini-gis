//! Commande `analysis.buffer` : zone tampon autour des entités d'une couche vecteur.
//! Déclaration seule : le geste vit dans l'extension, qui lance l'algorithme du cadre de
//! traitement de QGIS depuis sa table de traduction (AD-22 : aucun nom d'algorithme ici).
//! Le noyau nomme le fichier de sortie (AD-10) ; c'est une couche, donc `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Unité d'une distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DistanceUnit {
    /// Mètres.
    M,
    /// Kilomètres.
    Km,
}

/// Distance mesurée, avec son unité (jamais de mesure nue).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Distance {
    /// Valeur strictement positive.
    #[schemars(range(min = 0.0))]
    #[serde(deserialize_with = "positive")]
    pub value: f64,
    /// Unité de la valeur.
    pub unit: DistanceUnit,
}

impl Distance {
    /// La distance en mètres.
    pub fn metres(&self) -> f64 {
        match self.unit {
            DistanceUnit::M => self.value,
            DistanceUnit::Km => self.value * 1000.0,
        }
    }
}

/// Refuse une valeur nulle, négative ou non finie, avant tout ordre à QGIS.
fn positive<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    let v = f64::deserialize(d)?;
    if v.is_finite() && v > 0.0 {
        Ok(v)
    } else {
        Err(serde::de::Error::custom(
            "la distance doit être strictement positive",
        ))
    }
}

/// Paramètres de `analysis.buffer`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalysisBufferParams {
    /// Id de la couche vecteur à entourer, rendu par `layer.add` ou `project.inventory`
    /// (valable pour la session seulement). La couche doit être dans un système métrique.
    pub layer_id: String,
    /// Distance du tampon.
    pub distance: Distance,
    /// Fusionner tous les tampons en une seule entité. À poser seulement si la phrase le demande.
    #[serde(default)]
    pub dissolve: Option<bool>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
    /// Système en mètres du calcul, en code EPSG : fourni par le noyau d'après l'inventaire
    /// (changement de cap du 2026-11-12), jamais par l'appelant.
    #[serde(default)]
    pub work_epsg: Option<u32>,
}

/// Résultat de `analysis.buffer`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisBufferOutput {
    /// Id de la couche créée dans le projet.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités de la couche créée.
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
}

/// `analysis.buffer` : commande déléguée à QGIS.
pub struct AnalysisBuffer;

impl Delegated for AnalysisBuffer {
    type Params = AnalysisBufferParams;
    type Output = AnalysisBufferOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisBufferParams, AnalysisBufferOutput>(
            "analysis.buffer",
            "Crée une zone tampon à une distance donnée autour des entités d'une couche vecteur, et l'ajoute au projet comme nouvelle couche. La couche d'entrée doit être dans un système de coordonnées en mètres. Ne modifie pas la couche d'entrée.",
            "Creates a buffer at a given distance around the features of a vector layer and adds it to the project as a new layer. The input layer must use a metric coordinate system. Does not modify the input layer.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        // Une couche en degrés est reprojetée par QGIS, dans la même étape, vers le système en
        // mètres que choisit le noyau (`crate::work_crs`, entrée 9.16).
        .work_crs_param("work_epsg", "layer_id")
        .example(serde_json::json!({"layer_id": "gares_1a2b3c", "distance": {"value": 500, "unit": "m"}}))
        .example(serde_json::json!({"layer_id": "gares_1a2b3c", "distance": {"value": 0.5, "unit": "km"}, "dissolve": true}))
        .tests(&[
            "analysis_buffer_is_delegated",
            "analysis_buffer_refuses_a_non_positive_distance",
            "analysis_buffer_output_is_a_gpkg_named_by_the_core",
            "analysis_buffer_declares_its_work_crs",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisBuffer>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn analysis_buffer_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.buffer").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call(
                "analysis.buffer",
                json!({"layer_id": "l1", "distance": {"value": 500, "unit": "m"}}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_buffer_refuses_a_non_positive_distance() {
        let registry = Registry::standard();
        for value in [0, -5] {
            let err = registry
                .validate_params(
                    "analysis.buffer",
                    &json!({"layer_id": "l1", "distance": {"value": value, "unit": "m"}}),
                )
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
        assert!(registry
            .validate_params(
                "analysis.buffer",
                &json!({"layer_id": "l1", "distance": {"value": 0.5, "unit": "km"}, "dissolve": true}),
            )
            .is_ok());
        // Une unité inconnue est refusée.
        assert!(registry
            .validate_params(
                "analysis.buffer",
                &json!({"layer_id": "l1", "distance": {"value": 1, "unit": "mi"}}),
            )
            .is_err());
    }

    #[test]
    fn analysis_buffer_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.buffer").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
        // Les commandes existantes gardent `.out`.
        assert_eq!(registry.spec("layer.describe").unwrap().output_ext, ".out");
    }

    #[test]
    fn analysis_buffer_declares_its_work_crs() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.buffer").unwrap();
        assert_eq!(
            spec.work_crs_param,
            Some(("work_epsg".to_string(), "layer_id".to_string()))
        );
        // Le code est un entier facultatif ; une valeur négative est refusée par le schéma.
        let ok = json!({"layer_id": "l1", "distance": {"value": 1, "unit": "m"}, "work_epsg": 2154});
        assert!(registry.validate_params("analysis.buffer", &ok).is_ok());
        let bad = json!({"layer_id": "l1", "distance": {"value": 1, "unit": "m"}, "work_epsg": -1});
        assert!(registry.validate_params("analysis.buffer", &bad).is_err());
        // Le texte lu par le modèle ne change pas (décision du propriétaire, 2026-11-12).
        assert!(spec.description_fr.contains("doit être dans un système de coordonnées en mètres"));
    }
}
