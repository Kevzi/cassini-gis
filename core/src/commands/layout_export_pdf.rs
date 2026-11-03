//! Commande `layout.export_pdf` : la carte du projet en PDF, avec titre, légende, échelle,
//! flèche du nord et sources. Déclaration seule : le geste vit dans l'extension, qui monte la
//! mise en page (AD-22 : aucun nom de classe de QGIS ici). Le noyau nomme le fichier de sortie
//! (AD-10) : un PDF.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Nombre maximal de sources détaillées dans le résultat (AD-8).
pub const MAX_SOURCES: usize = 50;

/// Paramètres de `layout.export_pdf`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayoutExportPdfParams {
    /// Titre de la carte, imprimé en haut de la page. Ne doit pas être vide.
    pub title: String,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
    /// Ids de couche à laisser hors de la carte : fournis par le noyau au rejeu seulement,
    /// ce sont les couches posées par une exécution précédente du même journal (ticket 5.36).
    /// Layer ids to leave out of the map: given by the core on replay only.
    #[serde(default)]
    pub leave_out: Vec<String>,
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
}

/// `layout.export_pdf` : commande déléguée à QGIS.
pub struct LayoutExportPdf;

impl Delegated for LayoutExportPdf {
    type Params = LayoutExportPdfParams;
    type Output = LayoutExportPdfOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<LayoutExportPdfParams, LayoutExportPdfOutput>(
            "layout.export_pdf",
            "Export PDF de la carte : produit une page A4 paysage montrant les couches visibles du projet, avec un titre, une légende, une barre d'échelle, une flèche du nord et les sources (noms des fichiers des couches). Ne modifie ni le projet ni les couches.",
            "PDF map export: produces an A4 landscape page showing the visible layers of the project, with a title, a legend, a scale bar, a north arrow and the sources (file names of the layers). Does not modify the project or the layers.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(false)
        .output_params(&["out"])
        .output_ext(".pdf")
        .leave_out_param("leave_out")
        .example(serde_json::json!({"title": "Parcelles constructibles à moins de 500 m d'une gare"}))
        .example(serde_json::json!({"title": "Réseau ferré de Villeneuve-le-Roi"}))
        .tests(&[
            "layout_export_pdf_is_delegated",
            "layout_export_pdf_needs_a_title",
            "layout_export_pdf_output_is_a_pdf_named_by_the_core",
            "layout_export_pdf_sources_are_bounded",
            "layout_export_pdf_leave_out_is_an_optional_list_of_ids",
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
}
