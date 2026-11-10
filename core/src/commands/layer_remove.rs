//! Commande `layer.remove` : retire une couche du projet QGIS ouvert (changement de cap du
//! 2026-11-10, `change-commande-layer-remove`). Déclaration seule : le geste vit dans
//! l'extension, qui appelle `QgsProject.removeMapLayer` (AD-24, amendement du 2026-11-08 :
//! entrée `project`, sortie `none`, geste fixe de l'API de projet).
//!
//! Le fichier de la couche n'est ni effacé, ni déplacé, ni modifié (principe 6). Une couche déjà
//! absente n'est pas une erreur : le résultat est le même, ce qui garde identique le rejeu dans la
//! même session (AD-11). Une étape suivante qui cite la couche retirée échoue par le code de son
//! propre geste. Offerte au modèle, au client MCP et au formulaire (non dictée).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Identifiant de la commande.
pub const REMOVE: &str = "layer.remove";

/// Paramètres de `layer.remove`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerRemoveParams {
    /// Id de la couche à retirer du projet, rendu par `layer.add`, `project.inventory` ou une
    /// analyse (valable pour la session seulement). Son fichier n'est pas touché.
    pub layer_id: String,
}

/// Résultat de `layer.remove`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LayerRemoveOutput {
    /// La couche retirée du projet (ou déjà absente).
    pub layer_id: String,
}

/// `layer.remove` : commande déléguée à QGIS, offerte au modèle.
pub struct LayerRemove;

impl Delegated for LayerRemove {
    type Params = LayerRemoveParams;
    type Output = LayerRemoveOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<LayerRemoveParams, LayerRemoveOutput>(
            REMOVE,
            "Retire une couche du projet QGIS ouvert, sans effacer ni modifier son fichier. Une couche déjà absente n'est pas une erreur.",
            "Removes a layer from the open QGIS project, without deleting or modifying its file. A layer already absent is not an error.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(false)
        .output_none()
        .example(serde_json::json!({"layer_id": "routes_1a2b3c"}))
        .tests(&[
            "layer_remove_is_delegated_offered_and_creates_no_layer",
            "layer_remove_takes_a_layer_id_only",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<LayerRemove>()) }

#[cfg(test)]
mod tests {
    use super::REMOVE;
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn layer_remove_is_delegated_offered_and_creates_no_layer() {
        let registry = Registry::standard();
        let spec = registry.spec(REMOVE).unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(false));
        assert!(!spec.dictated_by_core);
        assert!(spec.output_none);
        assert!(spec.output_params.is_empty());
        assert!(spec.path_params.is_empty());
        assert_eq!(spec.output_schema["required"], json!(["layer_id"]));
        let err = registry
            .call(REMOVE, json!({"layer_id": "routes_1"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn layer_remove_takes_a_layer_id_only() {
        let registry = Registry::standard();
        for bad in [
            json!({}),
            json!({"layer_id": 3}),
            json!({"layer_id": "routes_1", "delete_file": true}),
            json!({"layer": "routes_1"}),
        ] {
            let err = registry.validate_params(REMOVE, &bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
        registry
            .validate_params(REMOVE, &json!({"layer_id": "routes_1"}))
            .unwrap();
    }
}
