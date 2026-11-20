//! Commande `analysis.difference` : première couche vecteur privée des parties que la seconde recouvre.
//! Déclaration seule : le geste vit dans l'extension, qui lance l'algorithme du cadre de
//! traitement de QGIS depuis sa table de traduction (AD-22 : aucun nom d'algorithme ici).
//! Le noyau nomme le fichier de sortie (AD-10) ; c'est une couche, donc `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `analysis.difference`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalysisDifferenceParams {
    /// Id de la couche vecteur à découper, rendu par `layer.add`, `project.inventory` ou une
    /// analyse (valable pour la session seulement). Ses entités sont gardées là où la
    /// seconde couche ne les recouvre pas.
    pub layer_id: String,
    /// Id de la seconde couche vecteur, celle qu'on retranche. Différente de `layer_id`.
    pub overlay_layer_id: String,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `analysis.difference`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisDifferenceOutput {
    /// Id de la couche créée dans le projet.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités de la couche créée.
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
}

/// `analysis.difference` : commande déléguée à QGIS.
pub struct AnalysisDifference;

impl Delegated for AnalysisDifference {
    type Params = AnalysisDifferenceParams;
    type Output = AnalysisDifferenceOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisDifferenceParams, AnalysisDifferenceOutput>(
            "analysis.difference",
            "Calcule la différence de deux couches vecteur : les parties de la première couche que la seconde ne recouvre pas (les parcelles hors d'une zone). Un recouvrement total donne une couche vide. Le résultat est ajouté au projet comme nouvelle couche. Ne modifie pas les couches d'entrée.",
            "Computes the difference of two vector layers: the parts of the first layer not covered by the second (the parcels outside a zone). Full overlap gives an empty layer. The result is added to the project as a new layer. Does not modify the input layers.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        .example(serde_json::json!({"layer_id": "parcelles_1a2b3c", "overlay_layer_id": "ebc_4d5e6f"}))
        .example(serde_json::json!({"layer_id": "parcelles_7a8b9c", "overlay_layer_id": "zones_humides_0d1e2f"}))
        .tests(&[
            "analysis_difference_is_delegated",
            "analysis_difference_needs_two_layers",
            "analysis_difference_output_is_a_gpkg_named_by_the_core",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisDifference>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn analysis_difference_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.difference").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call(
                "analysis.difference",
                json!({"layer_id": "l1", "overlay_layer_id": "l2"}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_difference_needs_two_layers() {
        let registry = Registry::standard();
        assert!(registry
            .validate_params(
                "analysis.difference",
                &json!({"layer_id": "l1", "overlay_layer_id": "l2"}),
            )
            .is_ok());
        for bad in [
            json!({"layer_id": "l1"}),
            json!({"overlay_layer_id": "l2"}),
            json!({"layer_id": "l1", "overlay_layer_id": "l2", "expression": "1=1"}),
        ] {
            let err = registry
                .validate_params("analysis.difference", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
    }

    #[test]
    fn analysis_difference_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.difference").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
    }
}
