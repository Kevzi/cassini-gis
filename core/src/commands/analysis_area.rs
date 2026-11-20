//! Commande `analysis.area` : ajoute à une couche de polygones un champ de surface plane, dans
//! l'unité demandée, sur une couche nouvelle. Le nom du champ est fixe et porte l'unité
//! (`area_m2`, `area_ha`), pour que la sélection par attribut qui suit compare dans la bonne unité.
//! Une couche en degrés est mesurée dans un système en mètres choisi par le noyau (`work_epsg`,
//! changement de cap du 2026-11-12), comme `analysis.buffer`.
//! Déclaration seule : le geste vit dans l'extension (AD-22 : aucun nom d'algorithme, aucune
//! formule de QGIS ici). Le noyau nomme le fichier de sortie (AD-10) : une couche, donc `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

// Unité de la surface. Pas de commentaire par variante : schemars en ferait une liste de `oneOf`,
// alors que le panneau ne sait faire une liste déroulante que d'une énumération plate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum AreaUnit {
    M2,
    Ha,
}

/// Paramètres de `analysis.area`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalysisAreaParams {
    /// Id de la couche de polygones, rendu par `layer.add`, `project.inventory` ou une analyse
    /// (valable pour la session seulement).
    pub layer_id: String,
    /// Unité de la surface : `m2` (mètres carrés, champ `area_m2`) ou `ha` (hectares, champ
    /// `area_ha`). Une sélection par attribut qui suit compare ce champ, dans cette unité.
    pub unit: AreaUnit,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
    /// Système en mètres du calcul, en code EPSG : fourni par le noyau d'après l'inventaire
    /// (changement de cap du 2026-11-12), jamais par l'appelant.
    #[serde(default)]
    pub work_epsg: Option<u32>,
}

/// Résultat de `analysis.area`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisAreaOutput {
    /// Id de la couche créée dans le projet : les polygones, avec un champ de surface.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités de la couche créée.
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
    /// Nom du champ de surface ajouté : `area_m2` ou `area_ha`.
    pub field: String,
    /// Unité du champ (`m2` ou `ha`), que reprend la sélection qui suit.
    pub unit: AreaUnit,
}

/// `analysis.area` : commande déléguée à QGIS.
pub struct AnalysisArea;

impl Delegated for AnalysisArea {
    type Params = AnalysisAreaParams;
    type Output = AnalysisAreaOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisAreaParams, AnalysisAreaOutput>(
            "analysis.area",
            "Surface des entités : ajoute à une couche de polygones (parcelles, zones) un champ de surface plane, en mètres carrés (champ area_m2) ou en hectares (champ area_ha), sur une couche nouvelle ajoutée au projet. Sert à filtrer par surface : enchaîner avec analysis.select_by_attribute sur le champ area_m2 ou area_ha, avec un seuil dans la même unité. Une couche en degrés est mesurée dans un système en mètres choisi par le noyau. Ne modifie pas la couche d'entrée.",
            "Feature area: adds to a polygon layer (parcels, zones) a planar area field, in square meters (field area_m2) or hectares (field area_ha), on a new layer added to the project. Used to filter by area: chain with analysis.select_by_attribute on the field area_m2 or area_ha, with a threshold in the same unit. A layer in degrees is measured in a system in meters chosen by the core. Does not modify the input layer.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        // Une couche en degrés est reprojetée par QGIS, dans la même étape, vers le système en
        // mètres que choisit le noyau (`crate::work_crs`, entrée 9.16) : les surfaces sont en mètres.
        .work_crs_param("work_epsg", "layer_id")
        .example(serde_json::json!({"layer_id": "parcelles_1a2b3c", "unit": "m2"}))
        .example(serde_json::json!({"layer_id": "communes_4d5e6f", "unit": "ha"}))
        .tests(&[
            "analysis_area_is_delegated",
            "analysis_area_unit_is_m2_or_ha",
            "analysis_area_output_is_a_gpkg_named_by_the_core",
            "analysis_area_declares_its_work_crs",
            "analysis_area_result_names_the_field_and_the_unit",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisArea>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn analysis_area_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.area").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call("analysis.area", json!({"layer_id": "l1", "unit": "m2"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_area_unit_is_m2_or_ha() {
        let registry = Registry::standard();
        for unit in ["m2", "ha"] {
            let ok = json!({"layer_id": "l1", "unit": unit});
            assert!(registry.validate_params("analysis.area", &ok).is_ok());
        }
        for bad in [
            json!({"layer_id": "l1"}),
            json!({"unit": "m2"}),
            json!({"layer_id": "l1", "unit": "km2"}),
            json!({"layer_id": "l1", "unit": "M2"}),
            json!({"layer_id": "l1", "unit": "HA"}),
            json!({"layer_id": "l1", "unit": 1}),
            json!({"layer_id": "l1", "unit": "ha", "formula": "$area"}),
        ] {
            let err = registry.validate_params("analysis.area", &bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn analysis_area_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.area").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn analysis_area_declares_its_work_crs() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.area").unwrap();
        assert_eq!(
            spec.work_crs_param,
            Some(("work_epsg".to_string(), "layer_id".to_string()))
        );
        let ok = json!({"layer_id": "l1", "unit": "ha", "work_epsg": 2154});
        assert!(registry.validate_params("analysis.area", &ok).is_ok());
        let bad = json!({"layer_id": "l1", "unit": "ha", "work_epsg": -1});
        assert!(registry.validate_params("analysis.area", &bad).is_err());
    }

    #[test]
    fn analysis_area_result_names_the_field_and_the_unit() {
        let registry = Registry::standard();
        let result = |field: &str, unit: &str| {
            json!({"layer_id": "z1", "name": "n", "feature_count": 4, "crs": "EPSG:2154",
                "field": field, "unit": unit})
        };
        for ok in [result("area_m2", "m2"), result("area_ha", "ha")] {
            assert!(registry.validate_output("analysis.area", &ok).is_ok());
        }
        assert!(registry
            .validate_output("analysis.area", &result("area_m2", "km2"))
            .is_err());
        let no_field = json!({"layer_id": "z1", "name": "n", "feature_count": 4,
            "crs": "EPSG:2154", "unit": "m2"});
        assert!(registry.validate_output("analysis.area", &no_field).is_err());
    }
}
