//! Commande `project.inventory` : dresse l'inventaire du projet QGIS ouvert (couches
//! dans l'ordre de la légende, SCR du projet, étendue d'ensemble). Déclaration seule :
//! le geste vit dans l'extension, en lecture seule (AD-19 : SCR en code explicite).
//!
//! Depuis la tâche 4a du ticket 5.10, l'extension rapporte aussi des faits bruts de QGIS
//! (sorte de couche, géométrie, nombre d'objets, champs, nom du projet), tous facultatifs :
//! une extension plus ancienne ne les rend pas. C'est le noyau qui les met dans la forme
//! vue par le modèle (`crate::agent_inventory`).

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
    /// La même étendue en degrés WGS84 (`EPSG:4326`), reprojetée par QGIS ; absente d'une
    /// extension antérieure ou quand QGIS ne sait pas la reprojeter. Le noyau en déduit le
    /// fuseau UTM du projet (`crate::agent_inventory`), sans convertir de coordonnée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lonlat: Option<LonLatExtent>,
}

/// Étendue en degrés WGS84 : longitudes ouest et est, latitudes sud et nord.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LonLatExtent {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
    /// SCR de l'étendue, en code explicite : toujours `EPSG:4326` (AD-19).
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
    /// Unité du SCR de la couche, lue par QGIS : `m`, `deg` ou `other` ; absente d'une extension
    /// antérieure. Lue par le noyau pour choisir un système de calcul en mètres (entrée 9.16),
    /// jamais montrée au modèle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crs_unit: Option<String>,
    /// Sorte de couche : `vector`, `raster`, ou `other` pour une autre sorte de QGIS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Géométrie d'une couche vecteur, nom WKB de QGIS (ex. `MultiPolygon`, `PointZ`) ;
    /// `null` pour une couche qui n'est pas vecteur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Option<String>>,
    /// Nombre d'objets d'une couche vecteur (0 si le fournisseur ne sait pas compter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_count: Option<u64>,
    /// Champs d'une couche vecteur, dans l'ordre de la couche ; vide pour un raster.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<InventoryField>>,
}

/// Un champ d'une couche vecteur.
#[derive(Debug, Serialize, JsonSchema)]
pub struct InventoryField {
    /// Nom du champ.
    pub name: String,
    /// Type du champ tel que QGIS le nomme (ex. `String`, `Integer64`, `Real`).
    #[serde(rename = "type")]
    pub field_type: String,
    /// Pour un champ texte : quelques valeurs distinctes non nulles, triées, lues avec un
    /// plafond ; absent pour un autre champ.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

/// Résultat de `project.inventory`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ProjectInventoryOutput {
    /// Nom du projet (nom du fichier sans extension) ; absent d'une extension antérieure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Couches dans l'ordre de la légende, de haut en bas.
    pub layers: Vec<InventoryLayer>,
    /// SCR du projet, en code explicite.
    pub project_crs: String,
    /// Unité du SCR du projet, lue par QGIS : `m`, `deg` ou `other` ; absente d'une extension
    /// antérieure (entrée 9.16).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_crs_unit: Option<String>,
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
