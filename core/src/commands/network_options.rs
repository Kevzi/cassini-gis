//! Contrat de réseau (AD-28) : vitesse, sens de circulation et tolérance d'accrochage d'un
//! réseau de lignes, défini une seule fois ici. Ce n'est pas une commande : aucune inscription
//! au registre. `analysis.isochrone` le lit (paramètre `network`) ; `network.shortest_path`
//! le lira aussi, et `network.prepare` n'en réutilisera que `Distance`.
//!
//! Les valeurs par défaut vivent ici, dans le noyau : `resolve` les écrit dans les paramètres
//! envoyés à l'extension et journalisés, si bien que le rejeu rejoue les mêmes valeurs même
//! si un défaut change plus tard, et que l'extension n'en porte aucune.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::analysis_buffer::{Distance, DistanceUnit};

/// Vitesse par défaut à pied, en km/h.
pub const WALK_SPEED_KMH: f64 = 5.0;
/// Vitesse par défaut en voiture, en km/h.
pub const CAR_SPEED_KMH: f64 = 50.0;
/// Tolérance d'accrochage par défaut, en mètres.
pub const DEFAULT_TOLERANCE_M: f64 = 100.0;

/// Unité d'une vitesse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum SpeedUnit {
    /// Kilomètres par heure.
    #[serde(rename = "km/h")]
    Kmh,
}

/// Vitesse mesurée, avec son unité (jamais de mesure nue).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Speed {
    /// Valeur strictement positive.
    #[schemars(range(min = 0.0))]
    #[serde(deserialize_with = "positive")]
    pub value: f64,
    /// Unité de la valeur.
    pub unit: SpeedUnit,
}

/// Refuse une valeur nulle, négative ou non finie, avant tout ordre à QGIS.
fn positive<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    let v = f64::deserialize(d)?;
    if v.is_finite() && v > 0.0 {
        Ok(v)
    } else {
        Err(serde::de::Error::custom(
            "la vitesse doit être strictement positive",
        ))
    }
}

/// Sens de circulation lu dans un champ de la couche.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Direction {
    /// Champ de la couche qui porte le sens.
    pub field: String,
    /// Valeur du champ qui veut dire « sens de saisie seul ».
    pub forward: String,
    /// Valeur du champ qui veut dire « sens inverse seul » ; toute autre valeur, double sens.
    pub backward: String,
}

/// Contrat de réseau : tout est facultatif.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NetworkOptions {
    /// Vitesse uniforme. Absente : celle du mode de la commande (5 km/h à pied, 50 km/h en
    /// voiture).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<Speed>,
    /// Champ numérique de la couche qui porte la vitesse de chaque tronçon, en km/h ;
    /// prioritaire sur `speed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_field: Option<String>,
    /// Sens de circulation. Absent : double sens partout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
    /// Distance d'accrochage d'un point de départ au réseau. Absente : 100 m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<Distance>,
}

impl NetworkOptions {
    /// Vitesse par défaut d'un mode (`walk` ou `car`), `None` pour un mode inconnu.
    pub fn default_speed(mode: &str) -> Option<Speed> {
        let value = match mode {
            "walk" => WALK_SPEED_KMH,
            "car" => CAR_SPEED_KMH,
            _ => return None,
        };
        Some(Speed {
            value,
            unit: SpeedUnit::Kmh,
        })
    }

    /// Les mêmes options, défauts écrits : `speed` (celle du mode) et `tolerance` (100 m) sont
    /// toujours présentes. Idempotent : un contrat déjà résolu reste tel quel.
    pub fn resolve(&self, mode: &str) -> NetworkOptions {
        let mut resolved = self.clone();
        if resolved.speed.is_none() {
            resolved.speed = Self::default_speed(mode);
        }
        if resolved.tolerance.is_none() {
            resolved.tolerance = Some(Distance {
                value: DEFAULT_TOLERANCE_M,
                unit: DistanceUnit::M,
            });
        }
        resolved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn network_options_schema_is_closed_and_every_field_optional() {
        let schema = serde_json::to_value(schemars::schema_for!(NetworkOptions)).unwrap();
        assert_eq!(schema["additionalProperties"], json!(false));
        assert!(schema.get("required").is_none_or(|r| r.as_array().unwrap().is_empty()));
        for field in ["speed", "speed_field", "direction", "tolerance"] {
            assert!(schema["properties"].get(field).is_some(), "{field}");
        }
        assert!(serde_json::from_value::<NetworkOptions>(json!({"vitesse": 3})).is_err());
    }

    #[test]
    fn network_options_refuse_a_non_positive_speed_and_an_unknown_unit() {
        for bad in [json!({"value": 0, "unit": "km/h"}), json!({"value": -3, "unit": "km/h"}), json!({"value": 5, "unit": "mph"})] {
            assert!(serde_json::from_value::<NetworkOptions>(json!({"speed": bad})).is_err());
        }
    }

    #[test]
    fn network_options_round_trip_with_the_catalogue_shape() {
        let value = json!({
            "speed": {"value": 20.0, "unit": "km/h"},
            "speed_field": "vitesse",
            "direction": {"field": "sens", "forward": "FT", "backward": "TF"},
            "tolerance": {"value": 250.0, "unit": "m"},
        });
        let options: NetworkOptions = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&options).unwrap(), value);
    }

    #[test]
    fn network_options_resolve_writes_the_defaults_of_the_mode() {
        let walk = serde_json::to_value(NetworkOptions::default().resolve("walk")).unwrap();
        assert_eq!(
            walk,
            json!({"speed": {"value": 5.0, "unit": "km/h"}, "tolerance": {"value": 100.0, "unit": "m"}})
        );
        let car = NetworkOptions::default().resolve("car");
        assert_eq!(car.speed.unwrap().value, 50.0);
    }

    #[test]
    fn network_options_resolve_keeps_given_values_and_is_idempotent() {
        let given: NetworkOptions = serde_json::from_value(json!({
            "speed": {"value": 20, "unit": "km/h"},
            "tolerance": {"value": 250, "unit": "m"},
        }))
        .unwrap();
        let once = given.resolve("walk");
        assert_eq!(once, given);
        assert_eq!(once.resolve("car"), once);
    }
}
