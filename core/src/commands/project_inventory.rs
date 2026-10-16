//! Commande `project.inventory` : dresse l'inventaire du projet QGIS ouvert (couches
//! dans l'ordre de la légende, SCR du projet, étendue d'ensemble). Déclaration seule :
//! le geste vit dans l'extension, en lecture seule (AD-19 : SCR en code explicite).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `project.inventory` : aucun.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectInventoryParams {}

/// Étendue rectangulaire, dans le SCR indiqué.
#[derive(Debug, Serialize, JsonSchema)]
pub struct InventoryExtent {
    pub xmin: f64,
    pub ymin: f64,
    pub xmax: f64,
    pub ymax: f64,
    /// SCR de l'étendue, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
}

/// Une couche du projet.
#[derive(Debug, Serialize, JsonSchema)]
pub struct InventoryLayer {
    /// Id de la couche dans le projet : valeur de session, jamais une clé de rejeu.
    pub layer_id: String,
    /// Nom affiché de la couche.
    pub name: String,
    /// Nom du fichier source, sans chemin ; absent pour une couche sans fichier.
    pub source: Option<String>,
    /// SCR de la couche, en code explicite.
    pub crs: String,
}

/// Résultat de `project.inventory`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ProjectInventoryOutput {
    /// Couches dans l'ordre de la légende, de haut en bas.
    pub layers: Vec<InventoryLayer>,
    /// SCR du projet, en code explicite.
    pub project_crs: String,
    /// Étendue de l'ensemble des couches, dans le SCR du projet ; `null` si aucune couche n'a d'étendue.
    pub extent: Option<InventoryExtent>,
}

/// `project.inventory` : commande déléguée à QGIS.
pub struct ProjectInventory;

impl Delegated for ProjectInventory {
    type Params = ProjectInventoryParams;
    type Output = ProjectInventoryOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<ProjectInventoryParams, ProjectInventoryOutput>(
            "project.inventory",
            "Dresse l'inventaire du projet ouvert : couches dans l'ordre de la légende, système de coordonnées du projet et étendue d'ensemble. Ne modifie rien.",
            "Lists the open project: layers in legend order, project coordinate system and overall extent. Changes nothing.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(false)
        .example(serde_json::json!({}))
        .tests(&["project_inventory_is_delegated", "project_inventory_takes_no_parameter"])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<ProjectInventory>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn project_inventory_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("project.inventory").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert!(spec.path_params.is_empty());
        let err = registry.call("project.inventory", json!({})).unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn project_inventory_takes_no_parameter() {
        let registry = Registry::standard();
        let err = registry
            .validate_params("project.inventory", &json!({"path": "a"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
    }
}
