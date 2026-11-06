//! Commande `layer.open_vector` : ouvre dans le projet QGIS un GeoJSON écrit par le noyau
//! (AD-30, ticket 11.8). Déclaration seule : le geste vit dans l'extension.
//!
//! Elle est **dictée par le noyau**, comme `layer.open_raster` : juste après une étape du noyau
//! dont la sortie est un GeoJSON (écrit par [`crate::geojson`]), la session l'appelle elle-même
//! sur la sortie de cette étape (`out:<k>.<n>`), si l'extension l'annonce, et la journalise avec
//! son `layer_id`. Le rejeu la rejoue comme toute étape et laisse la couche d'origine hors de la
//! carte. Elle n'est offerte ni au modèle, ni au client MCP, ni au formulaire du panneau.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Identifiant de la commande, que la session dicte après une sortie GeoJSON.
pub const OPEN_VECTOR: &str = "layer.open_vector";

/// Paramètres de `layer.open_vector`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerOpenVectorParams {
    /// GeoJSON à ouvrir : la sortie d'une étape précédente, désignée par `out:<k>.<n>`.
    pub path: String,
}

/// Résultat de `layer.open_vector`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LayerOpenVectorOutput {
    /// Id de la couche dans le projet : valeur de session, jamais une clé de rejeu.
    pub layer_id: String,
    /// Nom affiché de la couche : le radical du fichier.
    pub name: String,
    /// Système lu par QGIS, code explicite (`EPSG:2154`) : celui que déclare le fichier.
    pub crs: String,
}

/// `layer.open_vector` : commande déléguée à QGIS, dictée par le noyau.
pub struct LayerOpenVector;

impl Delegated for LayerOpenVector {
    type Params = LayerOpenVectorParams;
    type Output = LayerOpenVectorOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<LayerOpenVectorParams, LayerOpenVectorOutput>(
            OPEN_VECTOR,
            "Ouvre dans le projet un GeoJSON écrit par le noyau, nommé du radical de son fichier ; dictée par le noyau après une sortie vecteur.",
            "Opens in the project a GeoJSON written by the core, named after its file stem; dictated by the core after a vector output.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(false)
        .path_params(&["path"])
        .dictated_by_core()
        .example(serde_json::json!({"path": "out:1.1"}))
        .tests(&[
            "layer_open_vector_is_delegated_and_dictated",
            "layer_open_vector_takes_only_a_path",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<LayerOpenVector>()) }

#[cfg(test)]
mod tests {
    use super::OPEN_VECTOR;
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn layer_open_vector_is_delegated_and_dictated() {
        let registry = Registry::standard();
        let spec = registry.spec(OPEN_VECTOR).unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(false));
        assert_eq!(spec.path_params, vec!["path".to_string()]);
        assert!(spec.dictated_by_core);
        let entry = serde_json::to_value(spec).unwrap();
        assert_eq!(entry["dictated_by_core"], json!(true));
        // La fiche dit chaîne : pas de marque raster sur `path`.
        assert_eq!(
            spec.params_schema["properties"]["path"]["type"],
            json!("string"),
            "{}",
            spec.params_schema
        );
        assert!(spec.params_schema["properties"]["path"]
            .get("$ref")
            .is_none());
        assert_eq!(
            spec.output_schema["required"],
            json!(["layer_id", "name", "crs"]),
            "{}",
            spec.output_schema
        );
        let err = registry
            .call(OPEN_VECTOR, json!({"path": "out:1.1"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn layer_open_vector_takes_only_a_path() {
        let registry = Registry::standard();
        for bad in [
            json!({}),
            json!({"path": 3}),
            json!({"path": "a.geojson", "x": 1}),
        ] {
            let err = registry.validate_params(OPEN_VECTOR, &bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
        registry
            .validate_params(OPEN_VECTOR, &json!({"path": "out:2.1"}))
            .unwrap();
    }
}
