//! Commande `layer.describe` : décrit une couche du projet QGIS (champs, étendue, SCR,
//! nombre d'objets, type de géométrie). Déclaration seule : le geste vit dans
//! l'extension, en lecture seule (AD-19 : SCR en code explicite).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `layer.describe`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerDescribeParams {
    /// Id de la couche dans le projet, rendu par `layer.add` ou `project.inventory`
    /// (valable pour la session seulement).
    pub layer_id: String,
}

/// Un champ de la table d'attributs.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DescribedField {
    pub name: String,
    /// Type du champ tel que QGIS le nomme (ex. `String`, `Integer`, `Real`).
    #[serde(rename = "type")]
    pub field_type: String,
}

/// Étendue rectangulaire, dans le SCR indiqué.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DescribedExtent {
    pub xmin: f64,
    pub ymin: f64,
    pub xmax: f64,
    pub ymax: f64,
    /// SCR de l'étendue, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
}

/// Résultat de `layer.describe`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LayerDescribeOutput {
    /// Nom affiché de la couche.
    pub name: String,
    /// Champs de la table d'attributs.
    pub fields: Vec<DescribedField>,
    /// SCR de la couche, en code explicite.
    pub crs: String,
    /// Étendue de la couche dans son SCR ; `null` pour une couche vide.
    pub extent: Option<DescribedExtent>,
    /// Nombre d'objets de la couche.
    pub feature_count: u64,
    /// Type de géométrie (ex. `Point`, `Line`, `Polygon`, `NoGeometry`).
    pub geometry: String,
}

/// `layer.describe` : commande déléguée à QGIS.
pub struct LayerDescribe;

impl Delegated for LayerDescribe {
    type Params = LayerDescribeParams;
    type Output = LayerDescribeOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<LayerDescribeParams, LayerDescribeOutput>(
            "layer.describe",
            "Décrit une couche du projet : champs et leur type, étendue, système de coordonnées, nombre d'objets et type de géométrie. Ne modifie rien.",
            "Describes a project layer: fields and their types, extent, coordinate system, feature count and geometry type. Changes nothing.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(false)
        .example(serde_json::json!({"layer_id": "parcelles_1a2b3c"}))
        .tests(&["layer_describe_is_delegated", "layer_describe_requires_a_layer_id"])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<LayerDescribe>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn layer_describe_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("layer.describe").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert!(spec.path_params.is_empty());
        let err = registry
            .call("layer.describe", json!({"layer_id": "l1"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn layer_describe_requires_a_layer_id() {
        let registry = Registry::standard();
        let err = registry
            .validate_params("layer.describe", &json!({}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
    }
}
