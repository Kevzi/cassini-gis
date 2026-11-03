//! Commande `layout.export_pdf` : la carte du projet en PDF, avec titre, légende, échelle,
//! flèche du nord et sources. Déclaration seule : le geste vit dans l'extension, qui monte la
//! mise en page (AD-22 : aucun nom de classe de QGIS ici). Le noyau nomme le fichier de sortie
//! (AD-10) : un PDF.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::point::Point;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Nombre maximal de sources détaillées dans le résultat (AD-8).
pub const MAX_SOURCES: usize = 50;

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
            "Export PDF de la carte : produit une page A4 paysage montrant les couches visibles du projet, avec un titre, une légende, une barre d'échelle, une flèche du nord et les sources (noms des fichiers des couches). Avec `scale`, la carte est tenue à cette échelle (1:scale), centrée sur la carte affichée dans QGIS (à défaut, sur les couches visibles), et refusée si le cadre ne montre aucune couche ; sans `scale`, elle est cadrée sur l'emprise des couches. Ne modifie ni le projet ni les couches.",
            "PDF map export: produces an A4 landscape page showing the visible layers of the project, with a title, a legend, a scale bar, a north arrow and the sources (file names of the layers). With `scale`, the map is held at that scale (1:scale), centred on the map shown in QGIS (failing that, on the visible layers), and refused if the frame shows no layer; without `scale`, it is framed on the extent of the layers. Does not modify the project or the layers.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(false)
        .output_params(&["out"])
        .output_ext(".pdf")
        .leave_out_param("leave_out")
        .replay_param("center", "center")
        .example(serde_json::json!({"title": "Parcelles constructibles à moins de 500 m d'une gare"}))
        .example(serde_json::json!({"title": "Réseau ferré de Villeneuve-le-Roi"}))
        .example(serde_json::json!({"title": "Gares de Villeneuve-le-Roi", "scale": 25000}))
        .tests(&[
            "layout_export_pdf_is_delegated",
            "layout_export_pdf_needs_a_title",
            "layout_export_pdf_output_is_a_pdf_named_by_the_core",
            "layout_export_pdf_sources_are_bounded",
            "layout_export_pdf_leave_out_is_an_optional_list_of_ids",
            "layout_export_pdf_scale_is_an_optional_bounded_integer",
            "layout_export_pdf_center_is_filled_on_replay_from_the_result",
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
}
