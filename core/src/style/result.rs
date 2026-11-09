//! Style des rasters de résultat du noyau (bogue du fond noir, 2026-11-09).
//!
//! Un raster que le noyau écrit (champ de vision, pente, coût…) s'ouvrait dans le rendu gris
//! par défaut de QGIS : les cellules à 0 étaient noires et opaques, et cachaient le fond de
//! carte. Juste après l'ouverture dictée (`layer.open_raster`), la session dicte un
//! `layer.set_style` dont le style vient d'ici, composé d'après la commande et son résultat.
//! Le noyau décide, l'extension applique (AD-4, AD-22).
//!
//! Les couleurs sont celles des dégradés ColorBrewer de la table ([`super::table::RAMPS`]), plus
//! une classe de YlOrRd ([`CUMULATIVE_LOW`]) ; le
//! sans-donnée écrit, -9999 ([`crate::dem::NODATA`]), est toujours transparent.

use serde_json::Value;

use super::spec::{RampInterpolation, ResultStop, StyleSpec};
use super::table::{ramp, NamedRamp, HILLSHADE_OPACITY};
/// Le sans-donnée de tout raster écrit par le noyau, -9999.
const NODATA: f64 = crate::dem::NODATA as f64;

/// Opacité d'un champ de vision : les cellules visibles « colorées à demi » laissent lire la
/// carte (décision du propriétaire, 2026-11-09).
pub const VIEWSHED_OPACITY: f64 = 0.5;
/// Opacité des autres rasters de résultat, dégradés couvrant toute leur emprise : le fond de
/// carte reste lisible dessous. Choisie le 2026-11-09, sonde QGIS sur Annecy.
pub const RESULT_OPACITY: f64 = 0.7;

/// Couleur d'une cellule vue par un seul observateur : la 5e des 9 classes de YlOrRd. La plus
/// claire (`#ffffcc`) se perdait sur le fond de carte, en voile pâle (sonde d'Annecy du
/// 2026-11-09) ; le dégradé va de là au rouge sombre de la dernière classe.
pub const CUMULATIVE_LOW: &str = "#fd8d3c";
/// Source de [`CUMULATIVE_LOW`].
pub const CUMULATIVE_LOW_SOURCE: &str = "ColorBrewer 2.0, schéma séquentiel YlOrRd à 9 classes, 5e classe, https://colorbrewer2.org, licence Apache-2.0 (lu le 2026-11-09)";

/// Classes d'exposition : borne haute en degrés depuis le nord (une valeur prend la classe de la
/// première borne qui lui est supérieure ou égale), couleur, libellé. Couleurs : les 8 classes du
/// schéma qualitatif Set1 de ColorBrewer, dans l'ordre du cercle des teintes, le nord en rouge.
pub const ASPECT_CLASSES: [(f64, &str, &str); 9] = [
    (22.5, "#e41a1c", "Nord"),
    (67.5, "#ff7f00", "Nord-est"),
    (112.5, "#ffff33", "Est"),
    (157.5, "#4daf4a", "Sud-est"),
    (202.5, "#377eb8", "Sud"),
    (247.5, "#984ea3", "Sud-ouest"),
    (292.5, "#f781bf", "Ouest"),
    (337.5, "#a65628", "Nord-ouest"),
    (360.0, "#e41a1c", "Nord"),
];
/// Source de [`ASPECT_CLASSES`].
pub const ASPECT_CLASSES_SOURCE: &str = "ColorBrewer 2.0, schéma qualitatif Set1 à 8 classes, https://colorbrewer2.org, licence Apache-2.0 (lu le 2026-11-09)";

/// Le style que la session dicte après l'ouverture du raster d'une commande du noyau.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultStyle {
    /// `terrain.viewshed` : 1 visible, 0 masqué.
    Viewshed,
    /// `terrain.viewshed_cumulative` : nombre d'observateurs, de 0 à `max_count`.
    ViewshedCumulative,
    /// `terrain.slope` : pente, en degrés ou en pourcentage (`unit`).
    Slope,
    /// `terrain.aspect` : exposition, de 0 à 360°.
    Aspect,
    /// `terrain.hillshade` : ombrage, posé en fondu comme celui de la mise en forme.
    Hillshade,
    /// `terrain.cost_distance` : coût cumulé, de 0 à `max`.
    CostDistance,
    /// `raster.weighted_overlay` : score, de `min` à `max`.
    WeightedOverlay,
}

impl ResultStyle {
    /// Le style du raster écrit, d'après le résultat de la commande (`unit`, `min`, `max`,
    /// `max_count`). Un champ absent ou un raster sans valeur valide donne une rampe par défaut :
    /// le style est toujours valide.
    pub fn compose(self, result: &Value) -> StyleSpec {
        let number = |key: &str| {
            result
                .get(key)
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite())
        };
        match self {
            ResultStyle::Viewshed => StyleSpec::ResultRamp {
                stops: vec![stop(1.0, named("red_blue").from, Some("Visible"))],
                interpolation: RampInterpolation::Discrete,
                transparent: vec![0.0, NODATA],
                unit: None,
                opacity: Some(VIEWSHED_OPACITY),
            },
            ResultStyle::ViewshedCumulative => {
                let top = number("max_count").unwrap_or(1.0).max(1.0);
                let colors = named("yellow_red");
                let label = |n: f64| {
                    let n = n as u64;
                    Some(if n <= 1 {
                        format!("{n} observateur")
                    } else {
                        format!("{n} observateurs")
                    })
                };
                let (stops, interpolation) = if top > 1.0 {
                    (
                        vec![
                            stop_owned(1.0, CUMULATIVE_LOW, label(1.0)),
                            stop_owned(top, colors.to, label(top)),
                        ],
                        RampInterpolation::Linear,
                    )
                } else {
                    (
                        vec![stop_owned(1.0, CUMULATIVE_LOW, label(1.0))],
                        RampInterpolation::Discrete,
                    )
                };
                StyleSpec::ResultRamp {
                    stops,
                    interpolation,
                    transparent: vec![0.0, NODATA],
                    unit: None,
                    opacity: Some(RESULT_OPACITY),
                }
            }
            ResultStyle::Slope => {
                let percent = result.get("unit").and_then(Value::as_str) == Some("percent");
                let (unit, fallback) = if percent { ("%", 100.0) } else { ("°", 90.0) };
                let span = (
                    number("min").unwrap_or(0.0),
                    number("max").unwrap_or(fallback),
                );
                continuous(span, named("yellow_red"), Some(unit))
            }
            ResultStyle::Aspect => StyleSpec::ResultRamp {
                // Une classe par direction, de 45° centrée sur elle ; le nord est coupé en deux
                // autour de 0 et 360°. Par classes : est et ouest ne se confondent pas.
                stops: ASPECT_CLASSES
                    .iter()
                    .map(|&(upper, color, label)| stop(upper, color, Some(label)))
                    .collect(),
                interpolation: RampInterpolation::Discrete,
                transparent: vec![NODATA],
                unit: Some("°".to_string()),
                opacity: Some(RESULT_OPACITY),
            },
            ResultStyle::Hillshade => StyleSpec::Hillshade {
                opacity: Some(HILLSHADE_OPACITY),
            },
            ResultStyle::CostDistance => continuous(
                (0.0, number("max").unwrap_or(1.0)),
                named("yellow_red"),
                None,
            ),
            ResultStyle::WeightedOverlay => continuous(
                (number("min").unwrap_or(0.0), number("max").unwrap_or(1.0)),
                named("greens"),
                None,
            ),
        }
    }
}

/// Dégradé de `colors.from` (valeur basse) à `colors.to` (valeur haute) sur `span`, sans-donnée
/// transparent ; une seule valeur (raster constant) donne une seule classe.
fn continuous(span: (f64, f64), colors: &NamedRamp, unit: Option<&str>) -> StyleSpec {
    let (low, high) = span;
    let (stops, interpolation) = if high > low {
        (
            vec![stop(low, colors.from, None), stop(high, colors.to, None)],
            RampInterpolation::Linear,
        )
    } else {
        (
            vec![stop(low, colors.to, None)],
            RampInterpolation::Discrete,
        )
    };
    StyleSpec::ResultRamp {
        stops,
        interpolation,
        transparent: vec![NODATA],
        unit: unit.map(str::to_string),
        opacity: Some(RESULT_OPACITY),
    }
}

fn named(name: &str) -> &'static NamedRamp {
    ramp(name).expect("dégradé de la table")
}

fn stop(value: f64, color: &str, label: Option<&str>) -> ResultStop {
    stop_owned(value, color, label.map(str::to_string))
}

fn stop_owned(value: f64, color: &str, label: Option<String>) -> ResultStop {
    ResultStop {
        value,
        color: color.to_string(),
        label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ALL: [ResultStyle; 7] = [
        ResultStyle::Viewshed,
        ResultStyle::ViewshedCumulative,
        ResultStyle::Slope,
        ResultStyle::Aspect,
        ResultStyle::Hillshade,
        ResultStyle::CostDistance,
        ResultStyle::WeightedOverlay,
    ];

    #[test]
    fn every_result_style_is_valid_whatever_the_result() {
        for result in [
            json!({}),
            json!({"min": null, "max": null}),
            json!({"min": 3.0, "max": 3.0, "max_count": 0}),
            json!({"min": 0.5, "max": 47.2, "unit": "degrees", "max_count": 4}),
            json!({"min": -2.0, "max": 1e9, "unit": "percent", "max_count": 32}),
        ] {
            for style in ALL {
                let spec = style.compose(&result);
                spec.validate()
                    .unwrap_or_else(|e| panic!("{style:?} {result} : {}", e.message));
                assert_eq!(spec.target(), crate::style::spec::StyleTarget::Raster);
            }
        }
    }

    #[test]
    fn a_viewshed_hides_its_zeros_and_colours_the_visible_cells_at_half() {
        let style = serde_json::to_value(ResultStyle::Viewshed.compose(&json!({}))).unwrap();
        assert_eq!(
            style,
            json!({"kind": "result_ramp", "interpolation": "discrete",
                   "stops": [{"value": 1.0, "color": "#b2182b", "label": "Visible"}],
                   "transparent": [0.0, -9999.0], "opacity": 0.5})
        );
    }

    #[test]
    fn a_cumulative_viewshed_ramps_from_one_to_the_highest_count() {
        let style = ResultStyle::ViewshedCumulative.compose(&json!({"max_count": 4}));
        let StyleSpec::ResultRamp {
            stops,
            interpolation,
            transparent,
            ..
        } = style
        else {
            panic!("rampe de résultat attendue");
        };
        assert_eq!(interpolation, RampInterpolation::Linear);
        assert_eq!(transparent, [0.0, NODATA]);
        let values: Vec<f64> = stops.iter().map(|s| s.value).collect();
        assert_eq!(values, [1.0, 4.0]);
        assert_eq!(stops[1].label.as_deref(), Some("4 observateurs"));
        // Un seul observateur : une seule classe.
        let StyleSpec::ResultRamp {
            stops,
            interpolation,
            ..
        } = ResultStyle::ViewshedCumulative.compose(&json!({"max_count": 1}))
        else {
            panic!();
        };
        assert_eq!(interpolation, RampInterpolation::Discrete);
        assert_eq!(stops.len(), 1);
        assert_eq!(stops[0].color, CUMULATIVE_LOW);
    }

    #[test]
    fn a_slope_ramps_over_its_real_range_with_its_unit() {
        for (unit, legend) in [("degrees", "°"), ("percent", "%")] {
            let StyleSpec::ResultRamp {
                stops,
                unit: got,
                transparent,
                ..
            } = ResultStyle::Slope.compose(&json!({"min": 0.0, "max": 38.5, "unit": unit}))
            else {
                panic!();
            };
            assert_eq!(got.as_deref(), Some(legend));
            assert_eq!(transparent, [NODATA]);
            assert_eq!((stops[0].value, stops[1].value), (0.0, 38.5));
            assert_eq!(stops[0].color, "#ffffcc");
            assert_eq!(stops[1].color, "#800026");
        }
    }

    #[test]
    fn an_aspect_has_one_class_per_direction_and_east_differs_from_west() {
        let StyleSpec::ResultRamp {
            stops,
            interpolation,
            ..
        } = ResultStyle::Aspect.compose(&json!({}))
        else {
            panic!();
        };
        assert_eq!(interpolation, RampInterpolation::Discrete);
        let class_of = |degrees: f64| stops.iter().find(|s| degrees <= s.value).unwrap();
        assert_eq!(class_of(0.0).label.as_deref(), Some("Nord"));
        assert_eq!(class_of(359.0).label.as_deref(), Some("Nord"));
        assert_eq!(class_of(0.0).color, class_of(359.0).color);
        assert_eq!(class_of(90.0).label.as_deref(), Some("Est"));
        assert_eq!(class_of(270.0).label.as_deref(), Some("Ouest"));
        assert_ne!(class_of(90.0).color, class_of(270.0).color);
        assert_eq!(class_of(180.0).label.as_deref(), Some("Sud"));
    }

    /// Commandes du noyau qui écrivent un GeoTIFF sans style de résultat, chacune avec sa raison.
    const UNSTYLED_RASTERS: &[(&str, &str)] = &[(
        "raster.reclassify",
        "ses valeurs de sortie sont celles de la table de l'utilisateur, sans sens commun",
    )];

    /// Garde : une commande du noyau qui écrit un `.tif` déclare son style, ou figure, nommée et
    /// avec sa raison, dans `UNSTYLED_RASTERS`. Une commande raster ajoutée plus tard ne
    /// retrouve pas le fond noir en silence.
    #[test]
    fn every_core_raster_command_declares_a_result_style_or_is_exempt() {
        let registry = crate::registry::Registry::standard();
        for spec in registry.specs() {
            let writes_tif = spec.runs_in == Some(crate::registry::RunsIn::Core)
                && spec.output_ext == ".tif"
                && !spec.output_params.is_empty();
            if !writes_tif {
                assert!(
                    spec.result_style.is_none(),
                    "{} : style sans raster",
                    spec.id
                );
                continue;
            }
            let exempt = UNSTYLED_RASTERS.iter().any(|(id, _)| *id == spec.id);
            assert!(
                spec.result_style.is_some() != exempt,
                "{} : un raster du noyau déclare `result_style` ou est exempté, pas les deux",
                spec.id
            );
        }
        for (id, why) in UNSTYLED_RASTERS {
            assert!(registry.spec(id).is_some(), "{id}");
            assert!(!why.is_empty());
        }
    }

    #[test]
    fn a_hillshade_keeps_the_hillshade_style() {
        assert_eq!(
            ResultStyle::Hillshade.compose(&json!({"nodata_cells": 3})),
            StyleSpec::Hillshade {
                opacity: Some(HILLSHADE_OPACITY)
            }
        );
    }

    /// Chaque couleur dictée vient des dégradés ColorBrewer de la table, sourcés.
    #[test]
    fn every_colour_comes_from_the_sourced_ramps() {
        let sourced: Vec<&str> = super::super::table::RAMPS
            .iter()
            .flat_map(|r| [r.from, r.to])
            .chain([CUMULATIVE_LOW])
            .chain(ASPECT_CLASSES.iter().map(|c| c.1))
            .collect();
        assert!(CUMULATIVE_LOW_SOURCE.contains("ColorBrewer"));
        assert!(ASPECT_CLASSES_SOURCE.contains("ColorBrewer"));
        for style in ALL {
            let spec = style.compose(&json!({"min": 1.0, "max": 5.0, "max_count": 3}));
            for color in spec.colors() {
                assert!(sourced.contains(&color), "{style:?} : {color}");
            }
        }
    }
}
