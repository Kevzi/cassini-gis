//! Commande `network.shortest_path` : plus court chemin entre deux points sur un réseau routier
//! apporté par l'utilisateur. Déclaration seule : le geste vit dans l'extension (AD-22 : aucun nom
//! d'algorithme ici). Le noyau nomme le fichier de sortie (AD-10) : `.gpkg`. Même contrat de
//! réseau que `analysis.isochrone` (`NetworkOptions`, AD-28), lu tel quel, jamais redéfini ici.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::analysis_isochrone::TravelMode;
use super::network_options::NetworkOptions;
use crate::point::Point;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `network.shortest_path`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NetworkShortestPathParams {
    /// Id de la couche de lignes qui forme le réseau routier, rendu par `layer.add` ou
    /// `project.inventory` (valable pour la session seulement). Le réseau est apporté par
    /// l'utilisateur ; en degrés, il est mesuré dans un système en mètres choisi par le noyau.
    pub layer_id: String,
    /// Point de départ, avec son système de coordonnées.
    pub start: Point,
    /// Point d'arrivée, avec son système de coordonnées.
    pub end: Point,
    /// Mode de déplacement : `walk` à pied (5 km/h) ou `car` en voiture (50 km/h).
    pub mode: TravelMode,
    /// Contrat de réseau, facultatif : vitesse uniforme (`speed`) ou lue par tronçon
    /// (`speed_field`, prioritaire), sens de circulation (`direction`) et distance d'accrochage
    /// du départ et de l'arrivée (`tolerance`). Absent, les valeurs du `mode` : 5 km/h à pied,
    /// 50 km/h en voiture, deux sens, 100 m. À poser seulement si la phrase le demande.
    #[serde(default)]
    pub network: Option<NetworkOptions>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
    /// Système en mètres du calcul, en code EPSG : fourni par le noyau d'après l'inventaire
    /// (changement de cap du 2026-11-12), jamais par l'appelant.
    #[serde(default)]
    pub work_epsg: Option<u32>,
}

/// Unité de la longueur rendue : toujours le mètre.
#[derive(Debug, Serialize, JsonSchema)]
pub enum PathLengthUnit {
    /// Mètres.
    #[serde(rename = "m")]
    M,
}

/// Longueur du chemin, avec son unité (jamais de mesure nue).
#[derive(Debug, Serialize, JsonSchema)]
pub struct PathLength {
    /// Longueur lue sur la géométrie, dans le système en mètres du calcul.
    pub value: f64,
    /// Unité de la valeur.
    pub unit: PathLengthUnit,
}

/// Unité de la durée rendue : toujours la seconde.
#[derive(Debug, Serialize, JsonSchema)]
pub enum PathDurationUnit {
    /// Secondes.
    #[serde(rename = "s")]
    S,
}

/// Durée du trajet, avec son unité (jamais de mesure nue).
#[derive(Debug, Serialize, JsonSchema)]
pub struct PathDuration {
    /// Durée du trajet à la vitesse du contrat de réseau (stratégie « plus rapide »).
    pub value: f64,
    /// Unité de la valeur.
    pub unit: PathDurationUnit,
}

/// Résultat de `network.shortest_path`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct NetworkShortestPathOutput {
    /// Id de la couche créée dans le projet.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités de la couche créée (une ligne).
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
    /// Longueur du chemin, en mètres.
    pub length: PathLength,
    /// Durée du trajet, en secondes.
    pub duration: PathDuration,
}

/// `network.shortest_path` : commande déléguée à QGIS.
pub struct NetworkShortestPath;

impl Delegated for NetworkShortestPath {
    type Params = NetworkShortestPathParams;
    type Output = NetworkShortestPathOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<NetworkShortestPathParams, NetworkShortestPathOutput>(
            "network.shortest_path",
            "Calcule le plus court chemin, à pied ou en voiture, entre un point de départ `start` et un point d'arrivée `end`, sur un réseau routier (couche de lignes) apporté par l'utilisateur, et l'ajoute au projet comme une couche d'une ligne. Rend la longueur du chemin en mètres et sa durée en secondes. Vitesse du mode par défaut (marche 5 km/h, voiture 50 km/h) ou donnée par `network` (vitesse uniforme ou champ de vitesse), tronçons parcourus dans les deux sens sauf champ de sens donné par `network`. Le chemin suit le réseau le plus rapide à la vitesse donnée. Le départ et l'arrivée doivent être chacun à moins de 100 m du réseau (ou de `network.tolerance`), sinon la commande est refusée ; si aucun chemin ne relie les deux points (réseau non connecté, sens interdit), elle est refusée. Ne modifie pas le réseau.",
            "Computes the shortest path, on foot or by car, between a start point `start` and an end point `end`, on a road network (line layer) provided by the user, and adds it to the project as a one-line layer. Returns the path length in metres and its duration in seconds. Speed of the mode by default (walking 5 km/h, car 50 km/h) or given by `network` (uniform speed or speed field), segments travelled both ways unless `network` gives a direction field. The path follows the fastest route at the given speed. The start and the end must each be within 100 m of the network (or `network.tolerance`), otherwise the command is refused; if no path joins the two points (unconnected network, one-way restriction), it is refused. Does not modify the network.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        // Le noyau écrit les valeurs résolues du contrat de réseau (vitesse du mode, 100 m) dans
        // les paramètres envoyés et journalisés ; l'extension n'en porte aucune (AD-28).
        .network_param("network", "mode")
        // Un réseau en degrés est reprojeté par QGIS, dans la même étape, vers le système en
        // mètres que choisit le noyau (`crate::work_crs`) : longueur et accrochage en mètres.
        .work_crs_param("work_epsg", "layer_id")
        .example(serde_json::json!({"layer_id": "routes_1a2b3c", "start": {"x": 652100, "y": 6850100, "epsg": 2154}, "end": {"x": 652400, "y": 6850400, "epsg": 2154}, "mode": "walk"}))
        .example(serde_json::json!({"layer_id": "routes_1a2b3c", "start": {"x": 652100, "y": 6850100, "epsg": 2154}, "end": {"x": 652400, "y": 6850400, "epsg": 2154}, "mode": "car", "network": {"speed": {"value": 30, "unit": "km/h"}, "tolerance": {"value": 250, "unit": "m"}}}))
        .tests(&[
            "network_shortest_path_is_delegated",
            "network_shortest_path_requires_both_points",
            "network_shortest_path_mode_is_closed",
            "network_shortest_path_output_is_a_gpkg_named_by_the_core",
            "network_shortest_path_description_states_the_snap_distance",
            "network_shortest_path_network_is_optional_and_closed",
            "network_shortest_path_declares_its_network_param",
            "network_shortest_path_declares_its_work_crs",
            "network_shortest_path_result_carries_length_in_m_and_duration_in_s",
            "network_shortest_path_takes_a_point_from_a_previous_step",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<NetworkShortestPath>()) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    fn params(mode: &str) -> serde_json::Value {
        json!({
            "layer_id": "l1",
            "start": {"x": 652100, "y": 6850100, "epsg": 2154},
            "end": {"x": 652400, "y": 6850400, "epsg": 2154},
            "mode": mode,
        })
    }

    #[test]
    fn network_shortest_path_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("network.shortest_path").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call("network.shortest_path", params("walk"))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn network_shortest_path_requires_both_points() {
        let registry = Registry::standard();
        for missing in ["start", "end"] {
            let mut p = params("walk");
            p.as_object_mut().unwrap().remove(missing);
            let err = registry
                .validate_params("network.shortest_path", &p)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{missing}");
        }
        // Pas de coordonnée nue, pas de champ inconnu.
        let mut bare = params("walk");
        bare["start"] = json!({"x": 1, "y": 2});
        assert!(registry.validate_params("network.shortest_path", &bare).is_err());
        let mut extra = params("walk");
        extra["origin"] = json!({"x": 1, "y": 2, "epsg": 2154});
        assert!(registry.validate_params("network.shortest_path", &extra).is_err());
    }

    #[test]
    fn network_shortest_path_mode_is_closed() {
        let registry = Registry::standard();
        for mode in ["walk", "car"] {
            assert!(registry
                .validate_params("network.shortest_path", &params(mode))
                .is_ok());
        }
        assert!(registry
            .validate_params("network.shortest_path", &params("bike"))
            .is_err());
    }

    #[test]
    fn network_shortest_path_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("network.shortest_path").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
    }

    #[test]
    fn network_shortest_path_description_states_the_snap_distance() {
        // L'IA lit la description : elle doit savoir que le départ et l'arrivée sont refusés à
        // plus de 100 m du réseau.
        let registry = Registry::standard();
        let spec = registry.spec("network.shortest_path").unwrap();
        assert!(spec.description_fr.contains("100 m du réseau"));
        assert!(spec.description_en.contains("100 m of the network"));
    }

    #[test]
    fn network_shortest_path_network_is_optional_and_closed() {
        let registry = Registry::standard();
        assert!(registry
            .validate_params("network.shortest_path", &params("walk"))
            .is_ok());
        let with = |network: serde_json::Value| {
            let mut p = params("walk");
            p["network"] = network;
            registry.validate_params("network.shortest_path", &p)
        };
        assert!(with(
            json!({"speed": {"value": 20, "unit": "km/h"}, "tolerance": {"value": 250, "unit": "m"}})
        )
        .is_ok());
        assert!(with(json!({"speed": {"value": 0, "unit": "km/h"}})).is_err());
        assert!(with(json!({"vitesse": 5})).is_err());
    }

    #[test]
    fn network_shortest_path_declares_its_network_param() {
        let registry = Registry::standard();
        let spec = registry.spec("network.shortest_path").unwrap();
        assert_eq!(
            spec.network_param,
            Some(("network".to_string(), "mode".to_string()))
        );
    }

    #[test]
    fn network_shortest_path_declares_its_work_crs() {
        let registry = Registry::standard();
        let spec = registry.spec("network.shortest_path").unwrap();
        assert_eq!(
            spec.work_crs_param,
            Some(("work_epsg".to_string(), "layer_id".to_string()))
        );
        let mut ok = params("walk");
        ok["work_epsg"] = json!(2154);
        assert!(registry.validate_params("network.shortest_path", &ok).is_ok());
        let mut bad = params("walk");
        bad["work_epsg"] = json!(-1);
        assert!(registry.validate_params("network.shortest_path", &bad).is_err());
    }

    #[test]
    fn network_shortest_path_takes_a_point_from_a_previous_step() {
        // `end` repris du point d'une étape précédente (AD-8) : accepté par le contrôle de plan.
        let registry = Registry::standard();
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "geo.destination_point", "params": {
                "from": {"x": 6.1, "y": 45.9, "epsg": 4326},
                "azimuth": {"value": 90}, "distance": {"value": 1, "unit": "km"}}},
            {"command": "network.shortest_path", "params": {
                "layer_id": "l1",
                "start": {"x": 6.1, "y": 45.9, "epsg": 4326},
                "end": "step:1.point", "mode": "walk"}}
        ]}))
        .unwrap();
        let checked = registry.check_plan(&plan);
        assert!(checked.is_ok(), "{checked:?}");
    }

    #[test]
    fn network_shortest_path_result_carries_length_in_m_and_duration_in_s() {
        let out = NetworkShortestPathOutput {
            layer_id: "chemin_1".into(),
            name: "chemin".into(),
            feature_count: 1,
            crs: "EPSG:2154".into(),
            length: PathLength {
                value: 600.0,
                unit: PathLengthUnit::M,
            },
            duration: PathDuration {
                value: 432.0,
                unit: PathDurationUnit::S,
            },
        };
        let v = serde_json::to_value(out).unwrap();
        assert_eq!(v["length"], json!({"value": 600.0, "unit": "m"}));
        assert_eq!(v["duration"], json!({"value": 432.0, "unit": "s"}));
    }
}
