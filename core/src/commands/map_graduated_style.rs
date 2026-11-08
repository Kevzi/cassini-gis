//! Commande `map.graduated_style` : colore une couche vecteur selon un champ numérique, en
//! dégradé (décision du propriétaire du 2026-11-08, entrée 11.20 ; le geste QGIS vient en 21).
//!
//! Le noyau ne voit pas les valeurs : il compose un `StyleSpec` `graduated` (champ, méthode,
//! nombre de classes, couleurs extrêmes du dégradé nommé, prises dans sa table), et QGIS calcule
//! les seuils sur les vraies valeurs. La session dicte ensuite un seul `layer.set_style` juste
//! après la ligne de la commande, comme pour `map.topographic_style` (amendement d'AD-30). Aucun
//! inventaire n'est lu : l'extension vérifie que le champ existe et qu'il est numérique
//! (`style.field_missing`).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::commands::layer_set_style::SET_STYLE;
use crate::commands::map_topographic_style::Order;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};
use crate::style::spec::{ColorRamp, GraduatedMethod, StyleSpec};
use crate::style::table::ramp;

/// Identifiant de la commande.
pub const GRADUATED_STYLE: &str = "map.graduated_style";

/// Nombre de classes par défaut.
pub const DEFAULT_CLASSES: u32 = 5;

/// Dégradé nommé, dont les couleurs sont dans la table (`crate::style::table::RAMPS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RampName {
    #[default]
    YellowRed,
    Blues,
    Greens,
    Reds,
    Purples,
    Greys,
    RedBlue,
}

impl RampName {
    fn id(self) -> &'static str {
        match self {
            RampName::YellowRed => "yellow_red",
            RampName::Blues => "blues",
            RampName::Greens => "greens",
            RampName::Reds => "reds",
            RampName::Purples => "purples",
            RampName::Greys => "greys",
            RampName::RedBlue => "red_blue",
        }
    }
}

/// Paramètres de `map.graduated_style`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MapGraduatedStyleParams {
    /// Couche vecteur à colorer.
    pub layer_id: String,
    /// Champ numérique de la couche.
    pub field: String,
    /// Méthode de calcul des seuils ; défaut `jenks`.
    #[serde(default)]
    pub method: Option<GraduatedMethod>,
    /// Nombre de classes, de 2 à 12 ; défaut 5.
    #[serde(default)]
    pub class_count: Option<u32>,
    /// Dégradé nommé ; défaut `yellow_red`.
    #[serde(default)]
    pub ramp: Option<RampName>,
}

/// Résultat de `map.graduated_style`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct MapGraduatedStyleOutput {
    /// La couche colorée (une couche existante : genre de sortie `none`).
    pub layer_id: String,
    pub field: String,
    pub class_count: u32,
    /// Ordre à dicter (champ caché, ôté avant journalisation) : absent du schéma public.
    #[schemars(skip)]
    pub orders: Vec<Order>,
}

/// `map.graduated_style` : commande du noyau.
pub struct MapGraduatedStyle;

impl Command for MapGraduatedStyle {
    type Params = MapGraduatedStyleParams;
    type Output = MapGraduatedStyleOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<MapGraduatedStyleParams, MapGraduatedStyleOutput>(
            GRADUATED_STYLE,
            "Colore une couche vecteur en dégradé selon un champ numérique (par exemple les communes selon la population) : `method` (equal_interval, quantile, jenks ; défaut jenks), `class_count` (2 à 12 ; défaut 5), `ramp` (dégradé nommé ; défaut yellow_red). QGIS calcule les seuils ; aucun fichier n'est modifié.",
            "Colours a vector layer with a graduated ramp along a numeric field (for instance municipalities by population): `method` (equal_interval, quantile, jenks; default jenks), `class_count` (2 to 12; default 5), `ramp` (named ramp; default yellow_red). QGIS computes the breaks; no file is modified.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .dictates(&[SET_STYLE])
        .output_none()
        .example(json!({"layer_id": "communes_1", "field": "population"}))
        .example(json!({"layer_id": "communes_1", "field": "population", "method": "quantile",
            "class_count": 7, "ramp": "blues"}))
        .tests(&[
            "the_defaults_are_jenks_five_classes_yellow_red",
            "the_class_count_is_bounded",
            "an_unknown_ramp_or_method_is_refused_by_the_schema",
            "every_named_ramp_gives_its_table_colors",
            "map_graduated_style_dictates_one_set_style",
        ])
    }

    fn run(&self, params: MapGraduatedStyleParams) -> Result<MapGraduatedStyleOutput, CommandFailure> {
        if params.layer_id.trim().is_empty() {
            return Err(CommandFailure::new(
                crate::style::spec::STYLE_INVALID,
                "La couche à colorer n'est pas nommée.",
            ));
        }
        let class_count = params.class_count.unwrap_or(DEFAULT_CLASSES);
        let named = params.ramp.unwrap_or_default();
        let colors = ramp(named.id()).ok_or_else(|| {
            CommandFailure::new(
                crate::style::spec::STYLE_INVALID,
                format!("Le dégradé « {} » n'est pas dans la table.", named.id()),
            )
        })?;
        let style = StyleSpec::Graduated {
            field: params.field.clone(),
            method: params.method.unwrap_or(GraduatedMethod::Jenks),
            class_count,
            ramp: ColorRamp {
                from: colors.from.to_string(),
                to: colors.to.to_string(),
            },
            stroke: None,
            width_mm: None,
        };
        style.validate()?;
        Ok(MapGraduatedStyleOutput {
            orders: vec![Order {
                command: SET_STYLE.to_string(),
                params: json!({"layer_id": params.layer_id, "style": style}),
            }],
            layer_id: params.layer_id,
            field: params.field,
            class_count,
        })
    }
}

inventory::submit! { Registration(|r| r.register(MapGraduatedStyle)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{Registry, ORDERS_FIELD};
    use crate::style::table::RAMPS;
    use serde_json::Value;

    fn graduated(params: Value) -> Result<Value, crate::registry::RegistryError> {
        Registry::standard().call(GRADUATED_STYLE, params)
    }

    fn style(out: &Value) -> &Value {
        &out[ORDERS_FIELD][0]["params"]["style"]
    }

    #[test]
    fn the_defaults_are_jenks_five_classes_yellow_red() {
        let out = graduated(json!({"layer_id": "c", "field": "population"})).unwrap();
        assert_eq!(out["layer_id"], "c");
        assert_eq!(out["field"], "population");
        assert_eq!(out["class_count"], 5);
        assert_eq!(
            *style(&out),
            json!({"kind": "graduated", "field": "population", "method": "jenks",
                   "class_count": 5, "ramp": {"from": "#ffffcc", "to": "#800026"}})
        );
    }

    #[test]
    fn the_class_count_is_bounded() {
        for n in [2, 12] {
            graduated(json!({"layer_id": "c", "field": "f", "class_count": n})).unwrap();
        }
        for n in [0, 1, 13, 40] {
            let err = graduated(json!({"layer_id": "c", "field": "f", "class_count": n})).unwrap_err();
            assert_eq!(err.code(), "style.invalid", "{n}");
        }
        let err = graduated(json!({"layer_id": "c", "field": " "})).unwrap_err();
        assert_eq!(err.code(), "style.invalid");
        let err = graduated(json!({"layer_id": "", "field": "f"})).unwrap_err();
        assert_eq!(err.code(), "style.invalid");
    }

    #[test]
    fn an_unknown_ramp_or_method_is_refused_by_the_schema() {
        let registry = Registry::standard();
        for bad in [
            json!({"layer_id": "c", "field": "f", "ramp": "rainbow"}),
            json!({"layer_id": "c", "field": "f", "method": "pretty"}),
            json!({"layer_id": "c", "field": "f", "class_count": -1}),
            json!({"layer_id": "c"}),
            json!({"layer_id": "c", "field": "f", "x": 1}),
        ] {
            let err = registry.validate_params(GRADUATED_STYLE, &bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn every_named_ramp_gives_its_table_colors() {
        for r in RAMPS {
            let out = graduated(json!({"layer_id": "c", "field": "f", "ramp": r.name,
                "method": "quantile"}))
            .unwrap();
            assert_eq!(style(&out)["ramp"], json!({"from": r.from, "to": r.to}));
            assert_eq!(style(&out)["method"], "quantile");
        }
    }

    #[test]
    fn map_graduated_style_dictates_one_set_style() {
        let registry = Registry::standard();
        let spec = registry.spec(GRADUATED_STYLE).unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert!(!spec.dictated_by_core);
        assert!(spec.output_none);
        assert!(spec.inventory_param.is_none());
        assert_eq!(spec.required_gestures(), [SET_STYLE]);
        assert!(spec.output_schema["properties"].get(ORDERS_FIELD).is_none());
        let out = graduated(json!({"layer_id": "c", "field": "f"})).unwrap();
        let orders = out[ORDERS_FIELD].as_array().unwrap();
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0]["command"], SET_STYLE);
        assert_eq!(orders[0]["params"]["layer_id"], "c");
    }
}
