//! Commande `layer.open_raster` : ouvre dans le projet QGIS un raster écrit par le noyau
//! (ticket 4.9). Déclaration seule : le geste vit dans l'extension.
//!
//! Elle est **dictée par le noyau** : juste après une étape du noyau qui écrit un GeoTIFF
//! (`terrain.slope`, `terrain.viewshed`), la session l'appelle elle-même sur la sortie de cette
//! étape (`out:<k>.<n>`), si l'extension l'annonce, et la journalise avec son `layer_id`. Le
//! rejeu la rejoue donc comme toute étape et laisse la couche d'origine hors de la carte
//! (ticket 5.36). Elle n'est offerte ni au modèle, ni au client MCP, ni au formulaire du panneau.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::raster_path::RasterPath;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Identifiant de la commande, que la session dicte après une sortie raster.
pub const OPEN_RASTER: &str = "layer.open_raster";

/// Paramètres de `layer.open_raster`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerOpenRasterParams {
    /// Raster à ouvrir : la sortie d'une étape précédente, désignée par `out:<k>.<n>`.
    pub path: RasterPath,
}

/// Résultat de `layer.open_raster`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LayerOpenRasterOutput {
    /// Id de la couche dans le projet : valeur de session, jamais une clé de rejeu.
    pub layer_id: String,
    /// Nom affiché de la couche : le radical du fichier (`1-terrain.slope`).
    pub name: String,
}

/// `layer.open_raster` : commande déléguée à QGIS, dictée par le noyau.
pub struct LayerOpenRaster;

impl Delegated for LayerOpenRaster {
    type Params = LayerOpenRasterParams;
    type Output = LayerOpenRasterOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<LayerOpenRasterParams, LayerOpenRasterOutput>(
            OPEN_RASTER,
            "Ouvre dans le projet un raster écrit par le noyau, nommé du radical de son fichier ; dictée par le noyau après une sortie raster.",
            "Opens in the project a raster written by the core, named after its file stem; dictated by the core after a raster output.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(false)
        .path_params(&["path"])
        .dictated_by_core()
        .example(serde_json::json!({"path": "out:1.1"}))
        .tests(&[
            "layer_open_raster_is_delegated_and_dictated",
            "layer_open_raster_takes_only_a_path",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<LayerOpenRaster>()) }

#[cfg(test)]
mod tests {
    use super::OPEN_RASTER;
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn layer_open_raster_is_delegated_and_dictated() {
        let registry = Registry::standard();
        let spec = registry.spec(OPEN_RASTER).unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(false));
        assert_eq!(spec.path_params, vec!["path".to_string()]);
        assert!(spec.dictated_by_core);
        // Le catalogue le dit ; aucune autre commande ne porte la marque.
        let entry = serde_json::to_value(spec).unwrap();
        assert_eq!(entry["dictated_by_core"], json!(true));
        for other in registry.specs().filter(|s| s.id != OPEN_RASTER) {
            assert!(!other.dictated_by_core, "{}", other.id);
            let entry = serde_json::to_value(other).unwrap();
            assert!(entry.get("dictated_by_core").is_none(), "{}", other.id);
        }
        assert_eq!(
            spec.output_schema["required"],
            json!(["layer_id", "name"]),
            "{}",
            spec.output_schema
        );
        let err = registry
            .call(OPEN_RASTER, json!({"path": "out:1.1"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn layer_open_raster_takes_only_a_path() {
        let registry = Registry::standard();
        for bad in [json!({}), json!({"path": 3}), json!({"path": "a.tif", "x": 1})] {
            let err = registry.validate_params(OPEN_RASTER, &bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
        registry
            .validate_params(OPEN_RASTER, &json!({"path": "out:2.1"}))
            .unwrap();
    }
}
