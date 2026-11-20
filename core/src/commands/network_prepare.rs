//! Commande `network.prepare` : prépare une couche de chemins bruts en réseau que
//! `analysis.isochrone` et `network.shortest_path` lisent sans autre traitement (AD-28) :
//! accrochage des extrémités, découpe aux croisements, suppression des géométries nulles et en
//! double, sur une couche nouvelle. Une tolérance en mètres sur une couche en degrés se mesure
//! dans un système en mètres choisi par le noyau (`work_epsg`, décision du 2026-11-12), comme
//! `analysis.buffer`.
//! Déclaration seule : le geste vit dans l'extension (AD-22 : aucun nom d'algorithme ici). Le
//! noyau nomme le fichier de sortie (AD-10) : une couche, donc `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::analysis_buffer::Distance;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Tolérance d'accrochage par défaut de la préparation, en mètres. Propre à cette commande : le
/// défaut de `network_options` (100 m) est celui de l'accrochage d'un point au réseau.
pub const DEFAULT_PREPARE_TOLERANCE_M: f64 = 1.0;

/// Paramètres de `network.prepare`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NetworkPrepareParams {
    /// Id de la couche de lignes brutes (chemins), rendu par `layer.add` ou `project.inventory`
    /// (valable pour la session seulement). Apportée par l'utilisateur, jamais modifiée.
    pub layer_id: String,
    /// Distance d'accrochage des extrémités, facultative : absente, 1 m. Deux extrémités plus
    /// proches que cette distance sont reliées ; plus éloignées, elles restent disjointes.
    #[serde(default)]
    pub tolerance: Option<Distance>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
    /// Système en mètres du calcul, en code EPSG : fourni par le noyau d'après l'inventaire
    /// (changement de cap du 2026-11-12), jamais par l'appelant.
    #[serde(default)]
    pub work_epsg: Option<u32>,
}

/// Résultat de `network.prepare`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct NetworkPrepareOutput {
    /// Id de la couche créée dans le projet : le réseau préparé, des lignes.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`) : celui de la couche brute.
    pub crs: String,
    /// Nombre de tronçons de la couche créée.
    pub feature_count: u64,
}

/// `network.prepare` : commande déléguée à QGIS.
pub struct NetworkPrepare;

impl Delegated for NetworkPrepare {
    type Params = NetworkPrepareParams;
    type Output = NetworkPrepareOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<NetworkPrepareParams, NetworkPrepareOutput>(
            "network.prepare",
            "Prépare un réseau : à partir d'une couche de chemins bruts (lignes aux extrémités disjointes, croisements sans nœud, doublons), accroche les extrémités proches (tolérance de 1 m par défaut, ou `tolerance`), découpe les lignes à leurs croisements et retire les géométries nulles et en double, sur une couche de lignes nouvelle ajoutée au projet. À lancer avant analysis.isochrone ou network.shortest_path sur des chemins bruts : elles lisent la couche rendue telle quelle. Une couche en degrés est mesurée dans un système en mètres choisi par le noyau ; le résultat reste dans le système de la couche. Des extrémités plus éloignées que la tolérance ne sont pas reliées. Ne modifie pas la couche d'entrée.",
            "Prepares a network: from a layer of raw paths (lines with disjoint endpoints, crossings without a node, duplicates), snaps nearby endpoints (1 m tolerance by default, or `tolerance`), splits the lines at their crossings and removes null and duplicate geometries, on a new line layer added to the project. To run before analysis.isochrone or network.shortest_path on raw paths: they read the returned layer as is. A layer in degrees is measured in a system in meters chosen by the core; the result stays in the layer's system. Endpoints farther apart than the tolerance are not joined. Does not modify the input layer.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        // Une couche en degrés est reprojetée par QGIS, dans la même étape, vers le système en
        // mètres que choisit le noyau (`crate::work_crs`) : la tolérance est en mètres.
        .work_crs_param("work_epsg", "layer_id")
        // Le défaut est celui du noyau, écrit dans les paramètres journalisés (AD-28) ; l'extension
        // n'en porte aucun.
        .default_param(
            "tolerance",
            serde_json::json!({"value": DEFAULT_PREPARE_TOLERANCE_M, "unit": "m"}),
        )
        .example(serde_json::json!({"layer_id": "chemins_1a2b3c"}))
        .example(serde_json::json!({"layer_id": "chemins_1a2b3c", "tolerance": {"value": 5, "unit": "m"}}))
        .example(serde_json::json!({"layer_id": "step:1.layer_id"}))
        .tests(&[
            "network_prepare_is_delegated",
            "network_prepare_requires_layer_id",
            "network_prepare_tolerance_is_optional_and_closed",
            "network_prepare_default_tolerance_is_one_metre",
            "network_prepare_output_is_a_gpkg_named_by_the_core",
            "network_prepare_declares_its_work_crs",
            "network_prepare_takes_a_layer_from_a_previous_step",
            "network_prepare_result_is_the_four_fields",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<NetworkPrepare>()) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn network_prepare_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("network.prepare").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call("network.prepare", json!({"layer_id": "l1"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn network_prepare_requires_layer_id() {
        let registry = Registry::standard();
        let err = registry
            .validate_params("network.prepare", &json!({}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
        let err = registry
            .validate_params("network.prepare", &json!({"layer_id": "l1", "champ": 1}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
    }

    #[test]
    fn network_prepare_tolerance_is_optional_and_closed() {
        let registry = Registry::standard();
        let with = |tolerance: serde_json::Value| {
            registry.validate_params(
                "network.prepare",
                &json!({"layer_id": "l1", "tolerance": tolerance}),
            )
        };
        assert!(registry
            .validate_params("network.prepare", &json!({"layer_id": "l1"}))
            .is_ok());
        assert!(with(json!({"value": 5, "unit": "m"})).is_ok());
        assert!(with(json!({"value": 0.01, "unit": "km"})).is_ok());
        assert!(with(json!({"value": 0, "unit": "m"})).is_err());
        assert!(with(json!({"value": 5})).is_err());
        assert!(with(json!({"value": 5, "unit": "ft"})).is_err());
    }

    #[test]
    fn network_prepare_default_tolerance_is_one_metre() {
        let registry = Registry::standard();
        let spec = registry.spec("network.prepare").unwrap();
        let (name, value) = spec.default_param.clone().unwrap();
        assert_eq!(name, "tolerance");
        assert_eq!(value, json!({"value": 1.0, "unit": "m"}));
        // Le défaut déclaré est lui-même une distance valide.
        let d: Distance = serde_json::from_value(value).unwrap();
        assert_eq!(d.metres(), 1.0);
    }

    #[test]
    fn network_prepare_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("network.prepare").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
    }

    #[test]
    fn network_prepare_declares_its_work_crs() {
        let registry = Registry::standard();
        let spec = registry.spec("network.prepare").unwrap();
        assert_eq!(
            spec.work_crs_param,
            Some(("work_epsg".to_string(), "layer_id".to_string()))
        );
        let ok = json!({"layer_id": "l1", "work_epsg": 2154});
        assert!(registry.validate_params("network.prepare", &ok).is_ok());
        let bad = json!({"layer_id": "l1", "work_epsg": -1});
        assert!(registry.validate_params("network.prepare", &bad).is_err());
    }

    #[test]
    fn network_prepare_takes_a_layer_from_a_previous_step() {
        // La couche préparée alimente l'isochrone : accepté par le contrôle de plan (AD-8).
        let registry = Registry::standard();
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "layer.add", "params": {"path": "chemins.gpkg"}},
            {"command": "network.prepare", "params": {"layer_id": "step:1.layer_id"}},
            {"command": "analysis.isochrone", "params": {
                "layer_id": "step:2.layer_id",
                "origin": {"x": 652100, "y": 6850100, "epsg": 2154},
                "mode": "walk", "duration": {"value": 10, "unit": "min"}}}
        ]}))
        .unwrap();
        let checked = registry.check_plan(&plan);
        assert!(checked.is_ok(), "{checked:?}");
    }

    #[test]
    fn network_prepare_result_is_the_four_fields() {
        let out = NetworkPrepareOutput {
            layer_id: "reseau_1".into(),
            name: "reseau".into(),
            crs: "EPSG:2154".into(),
            feature_count: 12,
        };
        let v = serde_json::to_value(out).unwrap();
        let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["crs", "feature_count", "layer_id", "name"]);
    }
}
