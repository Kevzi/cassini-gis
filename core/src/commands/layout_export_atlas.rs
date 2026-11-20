//! Commande `layout.export_atlas` : un atlas PDF, une page par entité d'une couche de couverture,
//! avec les éléments de `layout.export_pdf` (titre, légende, échelle, flèche du nord, sources).
//! Déclaration seule : le geste vit dans l'extension, qui l'exécute depuis sa table fermée (AD-24,
//! genre `layout`) ; aucun nom d'algorithme ni de classe de QGIS ici (AD-22). Aucune expression de
//! filtre ou de tri n'est portée (AD-23). Le noyau nomme le fichier de sortie (AD-10) : un PDF.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Nombre maximal de sources détaillées dans le résultat (AD-8), le même que `layout.export_pdf`.
pub const MAX_SOURCES: usize = super::layout_export_pdf::MAX_SOURCES;

/// Un titre ne doit pas être vide : il est imprimé en tête de chaque page.
fn non_blank<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let title = String::deserialize(d)?;
    if title.trim().is_empty() {
        return Err(serde::de::Error::custom(
            "le titre de l'atlas ne doit pas être vide",
        ));
    }
    Ok(title)
}

/// Paramètres de `layout.export_atlas`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayoutExportAtlasParams {
    /// Titre imprimé en haut de chaque page. Ne doit pas être vide.
    #[serde(deserialize_with = "non_blank")]
    pub title: String,
    /// Id de la couche de couverture (vecteur) : une page par entité. Rendu par `layer.add`,
    /// `project.inventory` ou une analyse (valable pour la session seulement).
    pub coverage_layer_id: String,
    /// Champ de la couche de couverture dont la valeur est ajoutée au titre de chaque page. À
    /// poser seulement si la demande nomme un champ.
    #[serde(default)]
    pub label_field: Option<String>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `layout.export_atlas`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LayoutExportAtlasOutput {
    /// Fichier PDF écrit : une page par entité de la couche de couverture.
    pub file: String,
    /// Nombre de pages du PDF.
    pub page_count: u64,
    /// Nombre de couches visibles dessinées sur chaque page.
    pub layer_count: u64,
    /// Noms des couches distantes visibles (cartes en ligne) écartées : un export n'ouvre aucune
    /// connexion réseau.
    pub skipped_layers: Vec<String>,
    /// Noms de fichiers des sources des couches dessinées, 50 au plus.
    #[schemars(length(max = 50))]
    pub sources: Vec<String>,
    /// Vrai si les couches dessinées ont plus de sources que `sources` n'en détaille.
    pub truncated: bool,
}

/// `layout.export_atlas` : commande déléguée à QGIS.
pub struct LayoutExportAtlas;

impl Delegated for LayoutExportAtlas {
    type Params = LayoutExportAtlasParams;
    type Output = LayoutExportAtlasOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<LayoutExportAtlasParams, LayoutExportAtlasOutput>(
            "layout.export_atlas",
            "Atlas PDF : un seul PDF, une page par entité de la couche de couverture `coverage_layer_id` (id d'une couche vecteur), chaque page centrée sur son entité et portant le titre, une légende, une barre d'échelle, une flèche du nord et les sources, comme `layout.export_pdf` (A4 paysage). `label_field` (champ de la couche de couverture) ajoute la valeur du champ au titre de chaque page. Seules les couches locales visibles sont dessinées : les couches distantes (cartes en ligne) sont écartées et nommées au résultat. Refusé si la couche de couverture n'a aucune entité. Ne modifie ni le projet ni les couches.",
            "PDF atlas: a single PDF, one page per feature of the coverage layer `coverage_layer_id` (id of a vector layer), each page centred on its feature and carrying the title, a legend, a scale bar, a north arrow and the sources, like `layout.export_pdf` (A4 landscape). `label_field` (a field of the coverage layer) adds the field value to the title of each page. Only the visible local layers are drawn: remote layers (online maps) are left out and named in the result. Refused if the coverage layer has no feature. Does not modify the project or the layers.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(false)
        .output_params(&["out"])
        .output_ext(".pdf")
        .example(serde_json::json!({"title": "Atlas des communes", "coverage_layer_id": "communes_1a2b3c"}))
        .example(serde_json::json!({"title": "Fiche de parcelle", "coverage_layer_id": "parcelles_4d5e6f", "label_field": "section"}))
        .tests(&[
            "layout_export_atlas_is_delegated",
            "layout_export_atlas_needs_a_title_and_a_coverage_layer",
            "layout_export_atlas_label_field_is_optional",
            "layout_export_atlas_output_is_a_pdf_named_by_the_core",
            "layout_export_atlas_result_names_the_skipped_layers",
            "layout_export_atlas_covers_the_result_of_an_earlier_step",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<LayoutExportAtlas>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn layout_export_atlas_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("layout.export_atlas").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(false));
        let err = registry
            .call(
                "layout.export_atlas",
                json!({"title": "Atlas", "coverage_layer_id": "communes"}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn layout_export_atlas_needs_a_title_and_a_coverage_layer() {
        let registry = Registry::standard();
        assert!(registry
            .validate_params(
                "layout.export_atlas",
                &json!({"title": "Atlas", "coverage_layer_id": "communes"})
            )
            .is_ok());
        for bad in [
            json!({}),
            json!({"title": "Atlas"}),
            json!({"coverage_layer_id": "communes"}),
            json!({"title": "", "coverage_layer_id": "communes"}),
            json!({"title": "   ", "coverage_layer_id": "communes"}),
            json!({"title": 3, "coverage_layer_id": "communes"}),
            json!({"title": "Atlas", "coverage_layer_id": "communes", "filter": "x > 1"}),
            json!({"title": "Atlas", "coverage_layer_id": "communes", "sort_by": "nom"}),
        ] {
            let err = registry
                .validate_params("layout.export_atlas", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn layout_export_atlas_label_field_is_optional() {
        let registry = Registry::standard();
        let spec = registry.spec("layout.export_atlas").unwrap();
        let required = spec.params_schema["required"].as_array().unwrap();
        assert!(!required.contains(&json!("label_field")));
        assert!(!required.contains(&json!("out")));
        for good in [
            json!({"title": "Atlas", "coverage_layer_id": "c", "label_field": "nom"}),
            json!({"title": "Atlas", "coverage_layer_id": "c", "label_field": null}),
        ] {
            assert!(registry
                .validate_params("layout.export_atlas", &good)
                .is_ok());
        }
        let err = registry
            .validate_params(
                "layout.export_atlas",
                &json!({"title": "Atlas", "coverage_layer_id": "c", "label_field": 3}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
    }

    #[test]
    fn layout_export_atlas_output_is_a_pdf_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("layout.export_atlas").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".pdf");
    }

    #[test]
    fn layout_export_atlas_result_names_the_skipped_layers() {
        let registry = Registry::standard();
        let spec = registry.spec("layout.export_atlas").unwrap();
        let sources = &spec.output_schema["properties"]["sources"];
        assert_eq!(sources["maxItems"], json!(super::MAX_SOURCES));
        let skipped = &spec.output_schema["properties"]["skipped_layers"];
        assert_eq!(skipped["type"], json!("array"));
        assert_eq!(skipped["items"]["type"], json!("string"));
        let required = spec.output_schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("skipped_layers")));
    }

    #[test]
    fn layout_export_atlas_covers_the_result_of_an_earlier_step() {
        // « Un atlas des parcelles en zone U » : la couverture est la couche d'une étape, désignée
        // par `step:k.layer_id` comme tout paramètre `…_layer_id`.
        let registry = Registry::standard();
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "analysis.select_by_attribute", "params": {
                "layer_id": "plu", "field": "typezone", "operator": "eq", "value": "U"}},
            {"command": "layout.export_atlas", "params": {
                "title": "Zones U", "coverage_layer_id": "step:1.layer_id"}}
        ]}))
        .unwrap();
        assert_eq!(registry.check_plan(&plan), Ok(()));
        let ahead: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "layout.export_atlas", "params": {
                "title": "Zones", "coverage_layer_id": "step:1.layer_id"}}
        ]}))
        .unwrap();
        let err = registry.check_plan(&ahead).unwrap_err();
        assert_eq!(err.step, 1);
    }
}
