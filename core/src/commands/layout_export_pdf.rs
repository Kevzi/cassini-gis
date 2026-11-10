//! Commande `layout.export_pdf` : la carte du projet en PDF, avec titre, légende, échelle,
//! flèche du nord et sources. Déclaration seule : le geste vit dans l'extension, qui monte la
//! mise en page (AD-22 : aucun nom de classe de QGIS ici). Le noyau nomme le fichier de sortie
//! (AD-10) : un PDF.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::angle::Angle;
use crate::commands::analysis_buffer::Distance;
use crate::point::Point;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Nombre maximal de sources détaillées dans le résultat (AD-8).
pub const MAX_SOURCES: usize = 50;

/// Longueur maximale, en caractères, d'un en-tête ou d'un pied de page (D8, changement de cap
/// du 2026-11-10).
pub const MAX_MARGIN_TEXT: usize = 120;

/// Nombre maximal de carrés de 100 km nommés au cadre du carroyage MGRS ; au-delà, les premiers
/// puis « … » (tranché en party le 2026-11-10, à confirmer par le propriétaire).
pub const MAX_SQUARES: usize = 12;

/// Paramètre que le noyau complète pour un carroyage MGRS (`epsg`, `squares`).
pub const GRID_PARAM: &str = "grid";

/// Bornes du dénominateur d'échelle (décision du propriétaire, 2026-11-03).
pub const MIN_SCALE: u32 = 100;
/// Borne haute du dénominateur d'échelle.
pub const MAX_SCALE: u32 = 100_000_000;

/// Refuse une échelle hors bornes avant tout ordre à QGIS : le schéma les annonce, la
/// validation (par le type Rust) les tient.
fn bounded_scale<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
    let v = Option::<u32>::deserialize(d)?;
    match v {
        Some(s) if !(MIN_SCALE..=MAX_SCALE).contains(&s) => Err(serde::de::Error::custom(
            "l'échelle doit être comprise entre 1:100 et 1:100 000 000",
        )),
        _ => Ok(v),
    }
}

// Format de page. Pas de commentaire par variante : schemars en ferait une liste de `oneOf`,
// alors que le panneau ne sait faire une liste déroulante que d'une énumération plate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum PageSize {
    A4,
    A3,
    A2,
    A1,
    A0,
}

// Orientation de la page (même remarque).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum PageOrientation {
    Landscape,
    Portrait,
}

/// Carroyage de la carte : un graticule en WGS84 (`degrees`), un quadrillage dans un
/// système projeté nommé (`projected`), étiquettes en décimal, ou un carroyage MGRS (`mgrs`,
/// grille USNG : quadrillage UTM étiqueté par le kilomètre dans le carré de 100 km).
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MapGrid {
    Degrees {
        /// Écart entre deux lignes du graticule, strictement positif.
        #[serde(deserialize_with = "positive_angle")]
        interval: Angle,
    },
    Projected {
        /// Code EPSG du système de la grille (32631 pour UTM 31 N).
        epsg: u32,
        /// Écart entre deux lignes, en mètres ou kilomètres.
        interval: Distance,
    },
    Mgrs {
        /// Écart entre deux lignes : 1 km, 10 km ou 100 km.
        #[serde(deserialize_with = "mgrs_interval")]
        interval: Distance,
        /// Fourni par le noyau d'après l'emprise du projet : ne pas l'écrire.
        #[serde(default)]
        epsg: Option<u32>,
        /// Fourni par le noyau d'après l'emprise du projet : ne pas l'écrire.
        #[serde(default)]
        squares: Option<Vec<String>>,
    },
}

/// Un carroyage MGRS se trace à 1, 10 ou 100 km (D7), en mètres ou en kilomètres.
fn mgrs_interval<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Distance, D::Error> {
    let interval = Distance::deserialize(d)?;
    let metres = interval.metres();
    if [1_000.0, 10_000.0, 100_000.0].contains(&metres) {
        Ok(interval)
    } else {
        Err(serde::de::Error::custom(
            "l'intervalle d'un carroyage MGRS est de 1 km, 10 km ou 100 km",
        ))
    }
}

/// En-tête ou pied de page : une ligne libre, 120 caractères au plus, sans saut de ligne, non
/// vide (D8). Absent, rien n'est imprimé.
fn margin_text<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let text = Option::<String>::deserialize(d)?;
    if let Some(t) = &text {
        if t.trim().is_empty() {
            return Err(serde::de::Error::custom(
                "un en-tête ou un pied de page ne doit pas être vide : ne l'écris pas",
            ));
        }
        // Tout caractère de contrôle (saut de ligne, tabulation…) ou séparateur de ligne Unicode.
        if t.chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
        {
            return Err(serde::de::Error::custom(
                "un en-tête ou un pied de page tient sur une ligne, sans saut de ligne",
            ));
        }
        if t.chars().count() > MAX_MARGIN_TEXT {
            return Err(serde::de::Error::custom(format!(
                "un en-tête ou un pied de page fait {MAX_MARGIN_TEXT} caractères au plus"
            )));
        }
    }
    Ok(text)
}

/// Zone et carrés de 100 km d'un carroyage MGRS, d'après l'emprise du projet en degrés
/// (`extent.lonlat` de l'inventaire lu auprès de QGIS) : le code EPSG de la zone UTM du centre
/// (même règle que `utm_epsg` de l'inventaire, donc rien au-delà de deux fuseaux), et les
/// désignations (`31T GL`) des carrés que l'emprise touche dans cette zone, triées, 12 au plus
/// puis « … ». Le bord de l'emprise est échantillonné au kilomètre près, son intérieur à moins
/// d'un carré : un carré que l'emprise ne fait qu'effleurer de moins d'un kilomètre peut manquer.
/// `None` sans emprise lisible.
pub fn mgrs_grid_of(inventory: &Value) -> Option<(u32, Vec<String>)> {
    use crate::geodesy::Mgrs;
    let extent = inventory.get("extent")?;
    let epsg = crate::agent_inventory::utm_epsg(extent)?;
    let lonlat = extent.get("lonlat")?;
    let get = |key: &str| lonlat.get(key).and_then(Value::as_f64);
    let (west, south, east, north) = (get("west")?, get("south")?, get("east")?, get("north")?);
    let zone = (epsg % 100) as u8;
    // Pas d'échantillonnage : au plus un centième de degré (environ 1 km) sur le bord.
    let steps = |span: f64| ((span / 0.01).ceil() as usize).clamp(1, 2_000);
    let (nx, ny) = (steps(east - west), steps(north - south));
    let lon_at = |i: usize| west + (east - west) * i as f64 / nx as f64;
    let lat_at = |j: usize| south + (north - south) * j as f64 / ny as f64;
    let mut points: Vec<(f64, f64)> = Vec::new();
    for i in 0..=nx {
        points.push((south, lon_at(i)));
        points.push((north, lon_at(i)));
    }
    for j in 0..=ny {
        points.push((lat_at(j), west));
        points.push((lat_at(j), east));
    }
    // Intérieur : un point tous les 25 pas (moins d'un carré de 100 km).
    for i in (0..=nx).step_by(25) {
        for j in (0..=ny).step_by(25) {
            points.push((lat_at(j), lon_at(i)));
        }
    }
    let designated: Vec<Option<String>> = points
        .into_iter()
        .map(|(lat, lon)| Mgrs::square_in_zone(lat, lon, zone))
        .collect();
    // Un point hors des huit colonnes de la zone (emprise sur deux fuseaux) laisse des carrés
    // de l'autre zone hors de la liste : « … » le dit, comme une liste tronquée.
    let dropped = designated.iter().any(Option::is_none);
    let found: std::collections::BTreeSet<String> = designated.into_iter().flatten().collect();
    let mut squares: Vec<String> = found.into_iter().collect();
    if squares.len() > MAX_SQUARES || dropped {
        squares.truncate(MAX_SQUARES);
        squares.push("…".to_string());
    }
    Some((epsg, squares))
}

/// Refuse un intervalle de graticule nul ou négatif avant tout ordre à QGIS (une `Distance`
/// le refuse déjà par son type).
fn positive_angle<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Angle, D::Error> {
    let a = Angle::deserialize(d)?;
    if a.value > 0.0 {
        Ok(a)
    } else {
        Err(serde::de::Error::custom(
            "l'intervalle du graticule doit être strictement positif",
        ))
    }
}

/// Paramètres de `layout.export_pdf`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayoutExportPdfParams {
    /// Titre de la carte, imprimé en haut de la page. Ne doit pas être vide.
    pub title: String,
    /// Dénominateur de l'échelle (25000 pour 1:25 000). À poser seulement si la demande donne
    /// une échelle.
    #[serde(default, deserialize_with = "bounded_scale")]
    #[schemars(range(min = 100, max = 100000000))]
    pub scale: Option<u32>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
    /// Ids de couche à laisser hors de la carte : fournis par le noyau au rejeu seulement,
    /// ce sont les couches posées par une exécution précédente du même journal (ticket 5.36).
    /// Layer ids to leave out of the map: given by the core on replay only.
    #[serde(default)]
    pub leave_out: Vec<String>,
    /// Centre de la carte d'un export à l'échelle : fourni par le noyau au rejeu seulement,
    /// c'est le `center` du résultat journalisé (ticket 5.38). Le geste l'emploie au lieu de
    /// la carte de QGIS. Centre of a map at a scale: given by the core on replay only.
    #[serde(default)]
    pub center: Option<Point>,
    /// Format de page : a4 si absent.
    #[serde(default)]
    pub page_size: Option<PageSize>,
    /// Orientation de la page : landscape si absente.
    #[serde(default)]
    pub orientation: Option<PageOrientation>,
    /// Carroyage de la carte, seulement si la demande en donne un ; absent, aucun.
    #[serde(default)]
    pub grid: Option<MapGrid>,
    /// En-tête : une ligne libre imprimée en haut de la page (120 caractères au plus), recopiée
    /// de la demande, seulement si elle en donne un.
    #[serde(default, deserialize_with = "margin_text")]
    pub header: Option<String>,
    /// Pied de page : une ligne libre imprimée en bas de la page (120 caractères au plus),
    /// recopiée de la demande, seulement si elle en donne un.
    #[serde(default, deserialize_with = "margin_text")]
    pub footer: Option<String>,
}

// Origine de la date d'une source. Pas de commentaire par variante (même remarque que
// `PageSize`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum DateOrigin {
    Metadata,
    File,
}

/// Détail d'une source de la carte : résolution et date, avec l'origine de la date (D9).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SourceDetail {
    /// Nom du fichier de la source.
    pub source: String,
    /// Taille de cellule d'un raster dans un système en mètres ; `null` pour un vecteur ou un
    /// raster en degrés.
    pub resolution: Option<Distance>,
    /// Date des données (AAAA-MM-JJ) : celle des métadonnées de la couche dans QGIS, sinon la
    /// dernière modification du fichier ; `null` si QGIS ne la sait pas, ou pour une sortie de
    /// Cassini.
    pub date: Option<String>,
    /// D'où vient la date : `metadata` ou `file` ; `null` sans date.
    pub date_origin: Option<DateOrigin>,
}

/// Résultat de `layout.export_pdf`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LayoutExportPdfOutput {
    /// Fichier PDF écrit.
    pub file: String,
    /// Nombre de pages du PDF (1).
    pub page_count: u64,
    /// Nombre de couches visibles dessinées sur la carte.
    pub layer_count: u64,
    /// Noms de fichiers des sources des couches visibles, 50 au plus.
    #[schemars(length(max = 50))]
    pub sources: Vec<String>,
    /// Vrai si les couches visibles ont plus de sources que `sources` n'en détaille.
    pub truncated: bool,
    /// Résolution et date de chaque source de `sources`, dans le même ordre, 50 au plus ; la
    /// ligne des sources imprimée les reprend.
    #[schemars(length(max = 50))]
    pub source_details: Vec<SourceDetail>,
    /// Dénominateur de l'échelle tenue par la carte : présent seulement si `scale` a été
    /// demandé (un journal écrit sans échelle rejoue à l'identique).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<u32>,
    /// Centre employé pour la carte à l'échelle, dans le système du projet : présent seulement
    /// si `scale` a été demandé. Journalisé, il est repassé au geste au rejeu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub center: Option<Point>,
}

/// `layout.export_pdf` : commande déléguée à QGIS.
pub struct LayoutExportPdf;

impl Delegated for LayoutExportPdf {
    type Params = LayoutExportPdfParams;
    type Output = LayoutExportPdfOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<LayoutExportPdfParams, LayoutExportPdfOutput>(
            "layout.export_pdf",
            "Export PDF de la carte : produit une page (A4 paysage par défaut ; `page_size` de a4 à a0, `orientation` landscape ou portrait) montrant les couches visibles du projet, avec un titre, une légende, une barre d'échelle, une flèche du nord et les sources (noms des fichiers des couches). Avec `scale`, la carte est tenue à cette échelle (1:scale), centrée sur la carte affichée dans QGIS (à défaut, sur les couches visibles), et refusée si le cadre ne montre aucune couche ; sans `scale`, elle est cadrée sur l'emprise des couches. Avec `grid`, la carte porte un carroyage : `degrees` (graticule WGS84, intervalle en angle) ou `projected` (quadrillage dans le système `epsg`, intervalle en distance), étiquettes en décimal, ou `mgrs` (quadrillage UTM étiqueté en MGRS, comme les cartes USNG des secours, intervalle 1, 10 ou 100 km ; la zone et les carrés sont fournis par le noyau). `header` et `footer` : une ligne de texte libre en haut et en bas de la page, recopiée de la demande, 120 caractères au plus. La ligne des sources dit la résolution et la date de chaque source. Ne modifie ni le projet ni les couches.",
            "PDF map export: produces a page (A4 landscape by default; `page_size` from a4 to a0, `orientation` landscape or portrait) showing the visible layers of the project, with a title, a legend, a scale bar, a north arrow and the sources (file names of the layers). With `scale`, the map is held at that scale (1:scale), centred on the map shown in QGIS (failing that, on the visible layers), and refused if the frame shows no layer; without `scale`, it is framed on the extent of the layers. With `grid`, the map carries a grid: `degrees` (WGS84 graticule, interval as an angle) or `projected` (grid in the `epsg` system, interval as a distance), with decimal labels, or `mgrs` (UTM grid labelled in MGRS, as on rescue USNG maps, interval 1, 10 or 100 km; zone and squares are filled by the core). `header` and `footer`: one free text line at the top and bottom of the page, copied from the request, 120 characters at most. The sources line gives the resolution and date of each source. Does not modify the project or the layers.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(false)
        .output_params(&["out"])
        .output_ext(".pdf")
        .leave_out_param("leave_out")
        .replay_param("center", "center")
        .mgrs_grid_param(GRID_PARAM)
        .example(serde_json::json!({"title": "Parcelles constructibles à moins de 500 m d'une gare"}))
        .example(serde_json::json!({"title": "Réseau ferré de Villeneuve-le-Roi"}))
        .example(serde_json::json!({"title": "Gares de Villeneuve-le-Roi", "scale": 25000}))
        .example(serde_json::json!({"title": "Carte carroyée", "page_size": "a3",
            "grid": {"kind": "projected", "epsg": 32631, "interval": {"value": 1, "unit": "km"}}}))
        .example(serde_json::json!({"title": "Secteur de recherche", "header": "Document de travail",
            "grid": {"kind": "mgrs", "interval": {"value": 1, "unit": "km"}}}))
        .tests(&[
            "layout_export_pdf_is_delegated",
            "layout_export_pdf_needs_a_title",
            "layout_export_pdf_output_is_a_pdf_named_by_the_core",
            "layout_export_pdf_sources_are_bounded",
            "layout_export_pdf_leave_out_is_an_optional_list_of_ids",
            "layout_export_pdf_scale_is_an_optional_bounded_integer",
            "layout_export_pdf_center_is_filled_on_replay_from_the_result",
            "layout_export_pdf_page_and_grid_are_optional",
            "layout_export_pdf_page_values_are_closed",
            "layout_export_pdf_grid_is_a_tagged_union",
            "layout_export_pdf_mgrs_grid_takes_only_three_intervals",
            "layout_export_pdf_header_and_footer_are_one_short_line",
            "layout_export_pdf_source_details_are_bounded",
            "the_mgrs_grid_of_annecy_is_in_zone_32_with_its_squares",
            "the_mgrs_grid_needs_a_readable_extent",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<LayoutExportPdf>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn layout_export_pdf_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("layout.export_pdf").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(false));
        let err = registry
            .call("layout.export_pdf", json!({"title": "Carte"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn layout_export_pdf_needs_a_title() {
        let registry = Registry::standard();
        assert!(registry
            .validate_params("layout.export_pdf", &json!({"title": "Carte"}))
            .is_ok());
        for bad in [
            json!({}),
            json!({"title": 3}),
            json!({"title": "Carte", "format": "A3"}),
        ] {
            let err = registry
                .validate_params("layout.export_pdf", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
    }

    #[test]
    fn layout_export_pdf_output_is_a_pdf_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("layout.export_pdf").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".pdf");
    }

    #[test]
    fn layout_export_pdf_sources_are_bounded() {
        let registry = Registry::standard();
        let spec = registry.spec("layout.export_pdf").unwrap();
        let sources = &spec.output_schema["properties"]["sources"];
        assert_eq!(sources["maxItems"], json!(super::MAX_SOURCES));
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn layout_export_pdf_leave_out_is_an_optional_list_of_ids() {
        let registry = Registry::standard();
        let spec = registry.spec("layout.export_pdf").unwrap();
        assert_eq!(spec.leave_out_param.as_deref(), Some("leave_out"));
        let schema = &spec.params_schema;
        assert_eq!(schema["properties"]["leave_out"]["type"], json!("array"));
        assert_eq!(
            schema["properties"]["leave_out"]["items"]["type"],
            json!("string")
        );
        let required = schema["required"].as_array().unwrap();
        assert!(!required.contains(&json!("leave_out")));
        assert!(registry
            .validate_params(
                "layout.export_pdf",
                &json!({"title": "Carte", "leave_out": ["a", "b"]})
            )
            .is_ok());
        let err = registry
            .validate_params("layout.export_pdf", &json!({"title": "Carte", "leave_out": "a"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
    }

    #[test]
    fn layout_export_pdf_scale_is_an_optional_bounded_integer() {
        let registry = Registry::standard();
        let spec = registry.spec("layout.export_pdf").unwrap();
        let schema = &spec.params_schema;
        let required = schema["required"].as_array().unwrap();
        assert!(!required.contains(&json!("scale")));
        assert_eq!(
            schema["properties"]["scale"]["minimum"],
            json!(super::MIN_SCALE)
        );
        assert_eq!(
            schema["properties"]["scale"]["maximum"],
            json!(super::MAX_SCALE)
        );
        // Absente : la demande d'avant reste valide.
        for good in [
            json!({"title": "Carte"}),
            json!({"title": "Carte", "scale": 25000}),
            json!({"title": "Carte", "scale": 100}),
            json!({"title": "Carte", "scale": 100_000_000}),
        ] {
            assert!(
                registry.validate_params("layout.export_pdf", &good).is_ok(),
                "{good}"
            );
        }
        for bad in [
            json!(0),
            json!(-5),
            json!(99),
            json!(100_000_001),
            json!(2.5),
            json!("25000"),
            json!(true),
        ] {
            let err = registry
                .validate_params("layout.export_pdf", &json!({"title": "Carte", "scale": bad}))
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
        // Le résultat ne porte `scale` que si elle a été demandée.
        let without = super::LayoutExportPdfOutput {
            file: "a.pdf".into(),
            page_count: 1,
            layer_count: 1,
            sources: vec![],
            truncated: false,
            source_details: vec![],
            scale: None,
            center: None,
        };
        assert!(serde_json::to_value(&without).unwrap().get("scale").is_none());
        let with = super::LayoutExportPdfOutput {
            scale: Some(25000),
            ..without
        };
        assert_eq!(serde_json::to_value(&with).unwrap()["scale"], json!(25000));
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn layout_export_pdf_center_is_filled_on_replay_from_the_result() {
        let registry = Registry::standard();
        let spec = registry.spec("layout.export_pdf").unwrap();
        assert_eq!(
            spec.replay_param,
            Some(("center".to_string(), "center".to_string()))
        );
        let required = spec.params_schema["required"].as_array().unwrap();
        assert!(!required.contains(&json!("center")));
        // Le centre est un `Point` : jamais une coordonnée nue (AD-19, AD-23).
        let center = &spec.params_schema["properties"]["center"];
        assert!(center.to_string().contains("Point"), "{center}");
        assert!(spec.output_schema["properties"]["center"]
            .to_string()
            .contains("Point"));
        let point = json!({"x": 650000.0, "y": 6850000.0, "epsg": 2154});
        assert!(registry
            .validate_params(
                "layout.export_pdf",
                &json!({"title": "Carte", "scale": 25000, "center": point})
            )
            .is_ok());
        for bad in [json!({"x": 1.0, "y": 2.0}), json!([1.0, 2.0]), json!("ici")] {
            let err = registry
                .validate_params(
                    "layout.export_pdf",
                    &json!({"title": "Carte", "scale": 25000, "center": bad}),
                )
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
        // Rendu avec l'échelle, absent sans elle.
        let with = super::LayoutExportPdfOutput {
            file: "a.pdf".into(),
            page_count: 1,
            layer_count: 1,
            sources: vec![],
            truncated: false,
            source_details: vec![],
            scale: Some(25000),
            center: Some(crate::point::Point {
                x: 650000.0,
                y: 6850000.0,
                epsg: 2154,
            }),
        };
        assert_eq!(serde_json::to_value(&with).unwrap()["center"], point);
        assert!(registry.check_contract().is_empty());
    }
    #[test]
    fn layout_export_pdf_page_and_grid_are_optional() {
        let registry = Registry::standard();
        let spec = registry.spec("layout.export_pdf").unwrap();
        let schema = &spec.params_schema;
        let required = schema["required"].as_array().unwrap();
        for name in ["page_size", "orientation", "grid"] {
            assert!(schema["properties"].get(name).is_some(), "{name}");
            assert!(!required.contains(&json!(name)), "{name}");
        }
        // Absents, la demande d'avant reste valide et ne pose rien.
        let p: super::LayoutExportPdfParams =
            serde_json::from_value(json!({"title": "Carte"})).unwrap();
        assert!(p.page_size.is_none() && p.orientation.is_none() && p.grid.is_none());
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn layout_export_pdf_page_values_are_closed() {
        let registry = Registry::standard();
        for size in ["a4", "a3", "a2", "a1", "a0"] {
            for orientation in ["landscape", "portrait"] {
                let params = json!({"title": "Carte", "page_size": size, "orientation": orientation});
                assert!(
                    registry.validate_params("layout.export_pdf", &params).is_ok(),
                    "{params}"
                );
            }
        }
        for bad in [
            json!({"title": "Carte", "page_size": "a5"}),
            json!({"title": "Carte", "page_size": "A4"}),
            json!({"title": "Carte", "page_size": 4}),
            json!({"title": "Carte", "orientation": "paysage"}),
            json!({"title": "Carte", "orientation": true}),
        ] {
            let err = registry
                .validate_params("layout.export_pdf", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn layout_export_pdf_grid_is_a_tagged_union() {
        let registry = Registry::standard();
        let utm_km = json!({"kind": "projected", "epsg": 32631, "interval": {"value": 1, "unit": "km"}});
        for good in [
            utm_km.clone(),
            json!({"kind": "projected", "epsg": 2154, "interval": {"value": 500, "unit": "m"}}),
            json!({"kind": "degrees", "interval": {"value": 0.01, "unit": "deg"}}),
            json!({"kind": "degrees", "interval": {"value": 1, "unit": "gon"}}),
            json!({"kind": "degrees", "interval": {"value": 0.5}}),
        ] {
            let params = json!({"title": "Carte", "page_size": "a3", "grid": good});
            assert!(
                registry.validate_params("layout.export_pdf", &params).is_ok(),
                "{params}"
            );
        }
        let p: super::LayoutExportPdfParams =
            serde_json::from_value(json!({"title": "Carte", "grid": utm_km})).unwrap();
        match p.grid.unwrap() {
            super::MapGrid::Projected { epsg, interval } => {
                assert_eq!(epsg, 32631);
                assert_eq!(interval.metres(), 1000.0);
            }
            other => panic!("{other:?}"),
        }
        for bad in [
            json!({"kind": "mgrs", "interval": {"value": 5, "unit": "km"}}),
            json!({"interval": {"value": 1, "unit": "km"}}),
            json!({"kind": "projected", "interval": {"value": 1, "unit": "km"}}),
            json!({"kind": "projected", "epsg": 32631}),
            json!({"kind": "projected", "epsg": -1, "interval": {"value": 1, "unit": "km"}}),
            json!({"kind": "projected", "epsg": 32631, "interval": {"value": 0, "unit": "km"}}),
            json!({"kind": "projected", "epsg": 32631, "interval": {"value": -1, "unit": "m"}}),
            json!({"kind": "projected", "epsg": 32631, "interval": {"value": 1, "unit": "deg"}}),
            json!({"kind": "degrees", "interval": {"value": 0, "unit": "deg"}}),
            json!({"kind": "degrees", "interval": {"value": -0.5, "unit": "gon"}}),
            json!({"kind": "degrees", "interval": {"value": 1, "unit": "km"}}),
            json!({"kind": "degrees"}),
            json!({"kind": "degrees", "epsg": 4326, "interval": {"value": 1}}),
            json!("utm"),
        ] {
            let err = registry
                .validate_params("layout.export_pdf", &json!({"title": "Carte", "grid": bad}))
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn layout_export_pdf_mgrs_grid_takes_only_three_intervals() {
        let registry = Registry::standard();
        for (value, unit) in [(1, "km"), (10, "km"), (100, "km"), (1000, "m"), (10000, "m")] {
            let params = json!({"title": "Carte",
                "grid": {"kind": "mgrs", "interval": {"value": value, "unit": unit}}});
            assert!(
                registry.validate_params("layout.export_pdf", &params).is_ok(),
                "{params}"
            );
        }
        // Le noyau complète `epsg` et `squares` ; un journal qui les porte se relit.
        let filled = json!({"title": "Carte", "grid": {"kind": "mgrs",
            "interval": {"value": 1, "unit": "km"}, "epsg": 32632, "squares": ["32T LR"]}});
        assert!(registry.validate_params("layout.export_pdf", &filled).is_ok());
        for bad in [
            json!({"kind": "mgrs", "interval": {"value": 2, "unit": "km"}}),
            json!({"kind": "mgrs", "interval": {"value": 500, "unit": "m"}}),
            json!({"kind": "mgrs"}),
            json!({"kind": "mgrs", "interval": {"value": 1, "unit": "km"}, "zone": "32T"}),
        ] {
            let err = registry
                .validate_params("layout.export_pdf", &json!({"title": "Carte", "grid": bad}))
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
        let spec = registry.spec("layout.export_pdf").unwrap();
        assert_eq!(spec.mgrs_grid_param.as_deref(), Some(super::GRID_PARAM));
    }

    #[test]
    fn layout_export_pdf_header_and_footer_are_one_short_line() {
        let registry = Registry::standard();
        let long = "x".repeat(super::MAX_MARGIN_TEXT);
        for good in [
            json!({"title": "Carte", "header": "Document de travail"}),
            json!({"title": "Carte", "footer": "Bureau d'études Lac et Monts — confidentiel"}),
            json!({"title": "Carte", "header": long, "footer": "é".repeat(120)}),
        ] {
            assert!(
                registry.validate_params("layout.export_pdf", &good).is_ok(),
                "{good}"
            );
        }
        for bad in [
            json!({"title": "Carte", "header": "x".repeat(super::MAX_MARGIN_TEXT + 1)}),
            json!({"title": "Carte", "header": "deux\nlignes"}),
            json!({"title": "Carte", "footer": "retour\r"}),
            json!({"title": "Carte", "footer": "séparateur\u{2028}de ligne"}),
            json!({"title": "Carte", "header": "tabulation\tici"}),
            json!({"title": "Carte", "footer": "   "}),
            json!({"title": "Carte", "header": 3}),
        ] {
            let err = registry
                .validate_params("layout.export_pdf", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn layout_export_pdf_source_details_are_bounded() {
        let registry = Registry::standard();
        let spec = registry.spec("layout.export_pdf").unwrap();
        let details = &spec.output_schema["properties"]["source_details"];
        assert_eq!(details["maxItems"], json!(super::MAX_SOURCES));
        let detail = super::SourceDetail {
            source: "mnt_5m.tif".into(),
            resolution: Some(crate::commands::analysis_buffer::Distance {
                value: 5.0,
                unit: crate::commands::analysis_buffer::DistanceUnit::M,
            }),
            date: Some("2025-03-14".into()),
            date_origin: Some(super::DateOrigin::File),
        };
        assert_eq!(
            serde_json::to_value(&detail).unwrap(),
            json!({"source": "mnt_5m.tif", "resolution": {"value": 5.0, "unit": "m"},
                   "date": "2025-03-14", "date_origin": "file"})
        );
        assert!(registry.check_contract().is_empty());
    }

    fn inventory_with(west: f64, south: f64, east: f64, north: f64) -> serde_json::Value {
        json!({"extent": {"lonlat": {"west": west, "south": south, "east": east,
            "north": north, "crs": "EPSG:4326"}}})
    }

    #[test]
    fn the_mgrs_grid_of_annecy_is_in_zone_32_with_its_squares() {
        // Emprise d'Annecy et de son lac : zone 32 (6,1° E), bande T.
        let (epsg, squares) =
            super::mgrs_grid_of(&inventory_with(6.05, 45.78, 6.25, 45.95)).unwrap();
        assert_eq!(epsg, 32632);
        assert!(!squares.is_empty());
        for s in &squares {
            assert!(s.starts_with("32T "), "{s}");
        }
        // Chaque coin tombe dans un carré listé.
        for (lat, lon) in [(45.78, 6.05), (45.95, 6.25), (45.78, 6.25), (45.95, 6.05)] {
            let one = crate::geodesy::Mgrs::square_in_zone(lat, lon, 32).unwrap();
            assert!(squares.contains(&one), "{one} {squares:?}");
        }
        let sorted = {
            let mut s = squares.clone();
            s.sort();
            s
        };
        assert_eq!(squares, sorted);
        // Une grande emprise : 12 carrés puis « … ».
        let (_, many) = super::mgrs_grid_of(&inventory_with(3.0, 43.0, 8.9, 49.0)).unwrap();
        assert_eq!(many.len(), super::MAX_SQUARES + 1);
        assert_eq!(many.last().unwrap(), "…");
        // Une emprise à cheval sur un méridien de fuseau reste dans la zone de son centre, dont les
        // colonnes la couvrent : tous ses carrés y sont, sans « … ».
        let (epsg, near) = super::mgrs_grid_of(&inventory_with(5.5, 45.8, 6.4, 46.0)).unwrap();
        assert_eq!((epsg, near.clone()), (32631, vec!["31T FL".to_string(), "31T GL".to_string()]));
        // Une emprise trop large pour les huit colonnes de la zone : des carrés manquent, « … » le
        // dit.
        let (_, wide) = super::mgrs_grid_of(&inventory_with(0.5, 45.0, 12.0, 45.2)).unwrap();
        assert_eq!(wide.last().unwrap(), "…", "{wide:?}");
        assert!(wide.len() <= super::MAX_SQUARES + 1);
    }

    #[test]
    fn the_mgrs_grid_needs_a_readable_extent() {
        assert!(super::mgrs_grid_of(&json!({})).is_none());
        assert!(super::mgrs_grid_of(&json!({"extent": {}})).is_none());
        // Plus de deux fuseaux : pas de zone.
        assert!(super::mgrs_grid_of(&inventory_with(-5.0, 42.0, 10.0, 51.0)).is_none());
        // Ouest après est.
        assert!(super::mgrs_grid_of(&inventory_with(6.3, 45.8, 6.1, 45.9)).is_none());
    }
}
