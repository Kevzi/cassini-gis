//! Angle partagé des commandes `geo.*` (AD-31, CAP-15) : une valeur et son unité, degrés
//! (par défaut) ou grades.
//!
//! Un azimut se compte depuis le nord vrai, dans le sens horaire ; un azimut rendu est ramené à
//! [0, 360) degrés ou [0, 400) grades, et arrondi au milliardième d'unité, pour que le JSON écrit
//! au journal se relise à l'identique (AD-11).

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

// Unité d'un angle. Sans commentaire de documentation sur les valeurs : il deviendrait une
// description par valeur dans le schéma du long prompt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AngleUnit {
    #[default]
    Deg,
    Gon,
}

impl AngleUnit {
    /// Un tour complet dans cette unité.
    fn full_turn(self) -> f64 {
        match self {
            AngleUnit::Deg => 360.0,
            AngleUnit::Gon => 400.0,
        }
    }

    /// Symbole affiché dans un message.
    pub fn symbol(self) -> &'static str {
        match self {
            AngleUnit::Deg => "°",
            AngleUnit::Gon => " gon",
        }
    }
}

/// Angle avec son unité ; azimut depuis le nord vrai, sens horaire.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Angle {
    /// Valeur de l'angle.
    #[serde(deserialize_with = "finite")]
    pub value: f64,
    /// Unité : deg (défaut) ou gon.
    #[serde(default)]
    pub unit: AngleUnit,
}

/// Refuse une valeur non finie, avant tout calcul.
fn finite<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    let v = f64::deserialize(d)?;
    if v.is_finite() {
        Ok(v)
    } else {
        Err(serde::de::Error::custom(
            "la valeur d'un angle doit être un nombre fini",
        ))
    }
}

impl Angle {
    /// L'angle en degrés.
    pub fn degrees(&self) -> f64 {
        match self.unit {
            AngleUnit::Deg => self.value,
            AngleUnit::Gon => self.value * 0.9,
        }
    }

    /// Un azimut en degrés, rendu dans `unit` : ramené à [0, tour) et arrondi au milliardième
    /// d'unité (un arrondi qui atteint le tour complet redonne 0).
    pub fn azimuth(degrees: f64, unit: AngleUnit) -> Angle {
        let full = unit.full_turn();
        let value = match unit {
            AngleUnit::Deg => degrees,
            AngleUnit::Gon => degrees * 10.0 / 9.0,
        };
        let mut v = libm::round(value.rem_euclid(full) * 1e9) / 1e9;
        if v >= full {
            v -= full;
        }
        // `-0.0` s'écrirait « -0.0 » au journal.
        Angle {
            value: v + 0.0,
            unit,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::de::IntoDeserializer;
    use serde_json::json;

    use super::*;

    #[test]
    fn the_unit_defaults_to_degrees_and_a_non_finite_value_is_refused() {
        let angle: Angle = serde_json::from_value(json!({"value": 45})).unwrap();
        assert_eq!(angle.unit, AngleUnit::Deg);
        let angle: Angle = serde_json::from_value(json!({"value": 50, "unit": "gon"})).unwrap();
        assert_eq!(angle.degrees(), 45.0);
        assert!(serde_json::from_value::<Angle>(json!({"value": 1, "unit": "rad"})).is_err());
        assert!(serde_json::from_value::<Angle>(json!({"value": null})).is_err());
        for v in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            let r: Result<f64, serde::de::value::Error> = finite(v.into_deserializer());
            assert!(r.is_err(), "{v}");
        }
    }

    #[test]
    fn an_azimuth_is_brought_back_into_one_turn() {
        assert_eq!(Angle::azimuth(-90.0, AngleUnit::Deg).value, 270.0);
        assert_eq!(Angle::azimuth(360.0, AngleUnit::Deg).value, 0.0);
        assert_eq!(Angle::azimuth(-0.0, AngleUnit::Deg).value.to_string(), "0");
        assert_eq!(Angle::azimuth(359.999_999_999_9, AngleUnit::Deg).value, 0.0);
        assert_eq!(Angle::azimuth(90.0, AngleUnit::Gon).value, 100.0);
        assert_eq!(Angle::azimuth(-9.0, AngleUnit::Gon).value, 390.0);
        assert_eq!(
            Angle::azimuth(12.345_678_901_23, AngleUnit::Deg).value,
            12.345_678_901
        );
    }
}
