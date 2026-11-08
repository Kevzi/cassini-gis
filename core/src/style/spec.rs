//! `StyleSpec` : le style d'une couche, neutre (AD-22).
//!
//! Des couleurs, des épaisseurs, des classes par valeur de champ, un dégradé, une rampe
//! d'altitudes : jamais un nom de moteur de rendu ni une expression de QGIS. Le noyau le compose
//! depuis sa table de sémiologie ([`super::table`]) ; l'extension le traduit en rendu de QGIS
//! (entrée 21). La forme suit, champ pour champ, le bloc ` ```json type ` du catalogue des
//! commandes (§3, « Description de style »), que la recette `core/tests/catalogue.rs` compare
//! au schéma engendré.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::CommandFailure;

/// Code d'un style mal formé (AD-21).
pub const STYLE_INVALID: &str = "style.invalid";

/// Plus petit et plus grand nombre de classes d'un dégradé (`graduated`).
pub const GRADUATED_CLASSES: (u32, u32) = (2, 12);

/// Le style d'une couche, l'une de cinq formes, étiquetée par `kind`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StyleSpec {
    /// Un seul symbole. `fill` absent : trait seul ; `stroke` absent : surface sans contour.
    Single {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stroke: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fill: Option<String>,
        /// Épaisseur du trait, en millimètres de carte.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width_mm: Option<f64>,
        /// Opacité, de 0 à 1 (1 par défaut).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        opacity: Option<f64>,
        /// Dénominateur d'échelle : la couche n'est dessinée qu'aux échelles plus grandes que
        /// 1:`max_scale` ; absent, à toute échelle (ajout du 2026-11-08, décision 8).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_scale: Option<u64>,
    },
    /// Un symbole par valeur du champ `field` ; `other` dessine les valeurs non listées.
    Categories {
        field: String,
        classes: Vec<StyleClass>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        other: Option<StyleOther>,
        /// Opacité de la couche, de 0 à 1 (1 par défaut) : les zones d'un PLU laissent voir le
        /// fond (ajout du 2026-11-08).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        opacity: Option<f64>,
        /// Dénominateur d'échelle : la couche n'est dessinée qu'aux échelles plus grandes que
        /// 1:`max_scale` ; absent, à toute échelle (ajout du 2026-11-08, décision 8).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_scale: Option<u64>,
    },
    /// Dégradé selon un champ numérique d'un vecteur : QGIS calcule les seuils sur les vraies
    /// valeurs ; le noyau ne choisit que le champ, la méthode, le nombre de classes et les deux
    /// couleurs extrêmes (décision du propriétaire, 2026-11-08).
    Graduated {
        field: String,
        method: GraduatedMethod,
        class_count: u32,
        ramp: ColorRamp,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stroke: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width_mm: Option<f64>,
        /// Dénominateur d'échelle : la couche n'est dessinée qu'aux échelles plus grandes que
        /// 1:`max_scale` ; absent, à toute échelle (ajout du 2026-11-08, décision 8).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_scale: Option<u64>,
    },
    /// Teintes d'altitude d'un MNT : `stops` en mètres, valeurs croissantes.
    ElevationRamp {
        stops: Vec<RampStop>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        opacity: Option<f64>,
    },
    /// Un raster d'ombrage déjà présent, rendu en gris et posé en fondu sur les couches du dessous.
    Hillshade {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        opacity: Option<f64>,
    },
}

/// Une classe d'un style `categories`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StyleClass {
    /// Valeurs du champ qui prennent ce symbole, comparées comme du texte.
    pub values: Vec<String>,
    pub label: String,
    pub stroke: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f64>,
    /// Dénominateur d'échelle propre à la classe : elle n'est dessinée qu'aux échelles plus
    /// grandes que 1:`max_scale` ; le `max_scale` de la couche s'applique en plus (ajout du
    /// 2026-11-08).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_scale: Option<u64>,
}

/// Le symbole des valeurs non listées d'un style `categories`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StyleOther {
    pub label: String,
    pub stroke: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_mm: Option<f64>,
    /// Dénominateur d'échelle propre à la classe : elle n'est dessinée qu'aux échelles plus
    /// grandes que 1:`max_scale` ; le `max_scale` de la couche s'applique en plus (ajout du
    /// 2026-11-08).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_scale: Option<u64>,
}

/// Méthode de calcul des seuils d'un dégradé, appliquée par QGIS aux valeurs réelles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GraduatedMethod {
    EqualInterval,
    Quantile,
    Jenks,
}

/// Les deux couleurs extrêmes d'un dégradé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ColorRamp {
    pub from: String,
    pub to: String,
}

/// Un palier d'une rampe d'altitudes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RampStop {
    /// Altitude, en mètres.
    pub value: f64,
    pub color: String,
}

/// Sorte de couche à laquelle un style s'applique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleTarget {
    Vector,
    Raster,
}

impl StyleSpec {
    /// La sorte de couche que ce style vise : `single`, `categories` et `graduated` vont aux
    /// vecteurs, `elevation_ramp` et `hillshade` aux rasters (`style.geometry_mismatch` sinon,
    /// relevé par l'extension).
    pub fn target(&self) -> StyleTarget {
        match self {
            StyleSpec::Single { .. }
            | StyleSpec::Categories { .. }
            | StyleSpec::Graduated { .. } => StyleTarget::Vector,
            StyleSpec::ElevationRamp { .. } | StyleSpec::Hillshade { .. } => StyleTarget::Raster,
        }
    }

    /// Le dénominateur d'échelle au-delà duquel la couche n'est plus dessinée, pour les formes
    /// vecteur ; None : visible à toute échelle (et toujours pour les rasters).
    pub fn max_scale(&self) -> Option<u64> {
        match self {
            StyleSpec::Single { max_scale, .. }
            | StyleSpec::Categories { max_scale, .. }
            | StyleSpec::Graduated { max_scale, .. } => *max_scale,
            StyleSpec::ElevationRamp { .. } | StyleSpec::Hillshade { .. } => None,
        }
    }

    /// Toutes les couleurs du style, dans l'ordre où elles apparaissent.
    pub fn colors(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        match self {
            StyleSpec::Single { stroke, fill, .. } => {
                out.extend(stroke.as_deref());
                out.extend(fill.as_deref());
            }
            StyleSpec::Categories { classes, other, .. } => {
                for class in classes {
                    out.push(class.stroke.as_str());
                    if let Some(f) = &class.fill {
                        out.push(f.as_str());
                    }
                }
                if let Some(o) = other {
                    out.push(o.stroke.as_str());
                    if let Some(f) = &o.fill {
                        out.push(f.as_str());
                    }
                }
            }
            StyleSpec::Graduated { ramp, stroke, .. } => {
                out.push(ramp.from.as_str());
                out.push(ramp.to.as_str());
                if let Some(s) = stroke {
                    out.push(s.as_str());
                }
            }
            StyleSpec::ElevationRamp { stops, .. } => {
                out.extend(stops.iter().map(|s| s.color.as_str()));
            }
            StyleSpec::Hillshade { .. } => {}
        }
        out
    }

    /// Vérifie le style : couleurs `#rrggbb`, épaisseurs positives ou nulles, opacités dans
    /// [0, 1], paliers croissants, un `single` qui a un trait ou un remplissage, un dégradé de 2 à
    /// 12 classes, une échelle de visibilité d'au moins 1. Refus : `style.invalid` (AD-21), avec
    /// la cause en français.
    pub fn validate(&self) -> Result<(), CommandFailure> {
        let invalid = |why: String| Err(CommandFailure::new(STYLE_INVALID, why));
        for color in self.colors() {
            if !is_color(color) {
                return invalid(format!(
                    "La couleur « {color} » n'est pas de la forme #rrggbb."
                ));
            }
        }
        let class_scales: Vec<Option<u64>> = match self {
            StyleSpec::Categories { classes, other, .. } => classes
                .iter()
                .map(|c| c.max_scale)
                .chain(other.iter().map(|o| o.max_scale))
                .collect(),
            _ => Vec::new(),
        };
        if class_scales.into_iter().flatten().any(|scale| scale < 1) {
            return invalid(
                "L'échelle de visibilité `max_scale` d'une classe vaut au moins 1.".into(),
            );
        }
        if self.max_scale().is_some_and(|scale| scale < 1) {
            return invalid("L'échelle de visibilité `max_scale` vaut au moins 1.".to_string());
        }
        let mut widths: Vec<Option<f64>> = Vec::new();
        let mut opacities: Vec<Option<f64>> = Vec::new();
        match self {
            StyleSpec::Single {
                stroke,
                fill,
                width_mm,
                opacity,
                ..
            } => {
                if stroke.is_none() && fill.is_none() {
                    return invalid(
                        "Un style simple doit avoir un trait, un remplissage, ou les deux."
                            .to_string(),
                    );
                }
                widths.push(*width_mm);
                opacities.push(*opacity);
            }
            StyleSpec::Categories {
                field,
                classes,
                other,
                opacity,
                ..
            } => {
                if field.trim().is_empty() {
                    return invalid("Le champ des classes est vide.".to_string());
                }
                if classes.is_empty() {
                    return invalid("Un style par classes doit avoir au moins une classe.".into());
                }
                if classes.iter().any(|c| c.values.is_empty()) {
                    return invalid("Une classe n'a aucune valeur.".to_string());
                }
                widths.extend(classes.iter().map(|c| c.width_mm));
                widths.push(other.as_ref().and_then(|o| o.width_mm));
                opacities.push(*opacity);
            }
            StyleSpec::Graduated {
                field,
                class_count,
                width_mm,
                ..
            } => {
                if field.trim().is_empty() {
                    return invalid("Le champ du dégradé est vide.".to_string());
                }
                let (low, high) = GRADUATED_CLASSES;
                if !(low..=high).contains(class_count) {
                    return invalid(format!(
                        "Un dégradé compte de {low} à {high} classes, pas {class_count}."
                    ));
                }
                widths.push(*width_mm);
            }
            StyleSpec::ElevationRamp { stops, opacity } => {
                if stops.len() < 2 {
                    return invalid("Une rampe d'altitudes a au moins deux paliers.".to_string());
                }
                if stops.iter().any(|s| !s.value.is_finite())
                    || stops.windows(2).any(|w| w[0].value >= w[1].value)
                {
                    return invalid(
                        "Les paliers d'une rampe d'altitudes doivent être strictement croissants."
                            .to_string(),
                    );
                }
                opacities.push(*opacity);
            }
            StyleSpec::Hillshade { opacity } => opacities.push(*opacity),
        }
        if widths
            .into_iter()
            .flatten()
            .any(|w| !w.is_finite() || w < 0.0)
        {
            return invalid("Une épaisseur de trait est négative.".to_string());
        }
        if opacities
            .into_iter()
            .flatten()
            .any(|o| !(0.0..=1.0).contains(&o))
        {
            return invalid("Une opacité sort de l'intervalle de 0 à 1.".to_string());
        }
        Ok(())
    }
}

/// Une couleur `#rrggbb`, chiffres hexadécimaux en minuscules ou en majuscules.
pub fn is_color(text: &str) -> bool {
    text.len() == 7 && text.starts_with('#') && text[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(value: serde_json::Value) -> StyleSpec {
        serde_json::from_value(value).unwrap()
    }

    fn refused(value: serde_json::Value) -> String {
        let err = parse(value.clone()).validate().unwrap_err();
        assert_eq!(err.code, STYLE_INVALID, "{value}");
        err.message
    }

    #[test]
    fn the_five_forms_read_and_validate() {
        for value in [
            json!({"kind": "single", "stroke": "#aad5e9", "width_mm": 0.4}),
            json!({"kind": "single", "fill": "#DFE8D5", "opacity": 0.5}),
            json!({"kind": "categories", "field": "typezone",
                   "classes": [{"values": ["U"], "label": "U", "stroke": "#343434", "fill": "#e60000"}],
                   "other": {"label": "Autre", "stroke": "#343434"}}),
            json!({"kind": "graduated", "field": "altitude", "method": "equal_interval",
                   "class_count": 5, "ramp": {"from": "#f2d9b0", "to": "#6b3d12"}}),
            json!({"kind": "elevation_ramp", "stops": [{"value": 0.0, "color": "#9cc58a"},
                   {"value": 100.5, "color": "#c3d99a"}]}),
            json!({"kind": "hillshade", "opacity": 0.5}),
        ] {
            let style = parse(value.clone());
            style.validate().unwrap();
            assert_eq!(serde_json::to_value(&style).unwrap(), value);
        }
    }

    #[test]
    fn a_bad_style_is_refused_as_style_invalid() {
        assert!(refused(json!({"kind": "single", "stroke": "bleu"})).contains("bleu"));
        assert!(refused(json!({"kind": "single", "stroke": "#12345"})).contains("#12345"));
        refused(json!({"kind": "single"}));
        refused(json!({"kind": "single", "stroke": "#000000", "width_mm": -1.0}));
        refused(json!({"kind": "single", "fill": "#000000", "opacity": 1.5}));
        refused(json!({"kind": "hillshade", "opacity": -0.1}));
        refused(json!({"kind": "categories", "field": "f", "classes": []}));
        refused(
            json!({"kind": "elevation_ramp", "stops": [{"value": 10, "color": "#000000"},
                       {"value": 10, "color": "#ffffff"}]}),
        );
        refused(json!({"kind": "elevation_ramp", "stops": [{"value": 10, "color": "#000000"}]}));
    }

    #[test]
    fn a_graduated_style_has_two_to_twelve_classes_and_a_known_method() {
        let graduated = |count: u32| {
            json!({"kind": "graduated", "field": "altitude", "method": "quantile",
                   "class_count": count, "ramp": {"from": "#ffffff", "to": "#000000"}})
        };
        for count in [2, 7, 12] {
            parse(graduated(count)).validate().unwrap();
        }
        for count in [0, 1, 13] {
            assert!(refused(graduated(count)).contains("classes"), "{count}");
        }
        refused(
            json!({"kind": "graduated", "field": "a", "method": "jenks", "class_count": 4,
                       "ramp": {"from": "#fff", "to": "#000000"}}),
        );
        // Une méthode inconnue ne se lit même pas : le type la refuse.
        let unknown = json!({"kind": "graduated", "field": "a", "method": "pretty",
                             "class_count": 4, "ramp": {"from": "#ffffff", "to": "#000000"}});
        assert!(serde_json::from_value::<StyleSpec>(unknown).is_err());
        for method in ["equal_interval", "quantile", "jenks"] {
            let mut ok = graduated(3);
            ok["method"] = json!(method);
            parse(ok).validate().unwrap();
        }
    }

    #[test]
    fn a_style_targets_vectors_or_rasters() {
        assert_eq!(
            parse(json!({"kind": "hillshade"})).target(),
            StyleTarget::Raster
        );
        assert_eq!(
            parse(json!({"kind": "single", "stroke": "#000000"})).target(),
            StyleTarget::Vector
        );
    }

    /// Les cas partagés avec l'extension (`fixtures/style-cases.json`, lus aussi par
    /// `qgis-plugin/tests/test_style_gestures.py`) : le noyau admet et refuse les mêmes styles.
    /// Un style qui ne se lit pas (forme inconnue, champ inconnu, type faux) compte comme refusé ;
    /// `core`, quand le cas le donne, est la réponse du noyau là où l'extension est plus stricte.
    #[test]
    fn the_shared_style_cases_agree_with_the_extension() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/style-cases.json");
        let text = std::fs::read_to_string(path).expect("fixtures/style-cases.json");
        let cases: serde_json::Value = serde_json::from_str(&text).unwrap();
        let cases = cases["cases"].as_array().unwrap();
        assert!(cases.len() >= 30);
        for case in cases {
            let name = case["name"].as_str().unwrap();
            let expected = case.get("core").unwrap_or(&case["code"]);
            let got = match serde_json::from_value::<StyleSpec>(case["style"].clone()) {
                Err(_) => Some(STYLE_INVALID.to_string()),
                Ok(style) => style.validate().err().map(|e| e.code.to_string()),
            };
            assert_eq!(got.as_deref(), expected.as_str(), "{name}");
        }
    }
}
