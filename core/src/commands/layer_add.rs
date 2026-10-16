//! Commande `layer.add` : ajoute au projet QGIS une couche vectorielle issue d'un
//! fichier de l'utilisateur (`.gpkg`, `.shp`, `.geojson`). Déclaration seule : le
//! geste vit dans l'extension, QGIS lit le fichier et reprojette (AD-19).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `layer.add`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerAddParams {
    /// Fichier à ouvrir, relatif au projet : `.gpkg`, `.shp` ou `.geojson`.
    pub path: String,
    /// SCR à donner à la couche quand le fichier n'en indique aucun (ex. `EPSG:2154`) ;
    /// refusé s'il contredit celui du fichier. Le fichier n'est jamais modifié.
    #[serde(default)]
    pub crs: Option<String>,
}

/// Résultat de `layer.add`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LayerAddOutput {
    /// Id de la couche dans le projet : valeur de session, jamais une clé de rejeu.
    pub layer_id: String,
    /// Nom affiché de la couche.
    pub name: String,
    /// SCR de la couche, en code explicite (ex. `EPSG:4326`).
    pub source_crs: String,
    /// SCR du projet, en code explicite.
    pub project_crs: String,
    /// Vrai si QGIS reprojette la couche à l'affichage.
    pub reprojected: bool,
}

/// `layer.add` : commande déléguée à QGIS.
pub struct LayerAdd;

impl Delegated for LayerAdd {
    type Params = LayerAddParams;
    type Output = LayerAddOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<LayerAddParams, LayerAddOutput>(
            "layer.add",
            "Ajoute au projet une couche issue d'un fichier GeoPackage, Shapefile ou GeoJSON.",
            "Adds to the project a layer read from a GeoPackage, Shapefile or GeoJSON file.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .path_params(&["path"])
        .example(serde_json::json!({"path": "donnees/parcelles.gpkg"}))
        .example(serde_json::json!({"path": "donnees/gares.shp", "crs": "EPSG:2154"}))
        .tests(&["layer_add_is_delegated", "layer_add_takes_an_optional_crs"])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<LayerAdd>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn layer_add_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("layer.add").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        assert_eq!(spec.path_params, vec!["path".to_string()]);
        let err = registry
            .call("layer.add", json!({"path": "a.gpkg"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn layer_add_takes_an_optional_crs() {
        let registry = Registry::standard();
        let err = registry
            .call("layer.add", json!({"path": "a.shp", "crs": "EPSG:2154"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable"); // params valides, exécutant absent
    }
}
