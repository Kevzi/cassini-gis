//! Commande `data.convert` : écriture d'une couche vecteur entière dans un autre format, par
//! QGIS. Déclaration seule : le geste vit dans l'extension, qui lance un algorithme du cadre de
//! traitement de QGIS depuis sa table de traduction (AD-22 : aucun nom d'algorithme ici). La
//! couche d'origine n'est jamais modifiée (§2.6), et le fichier écrit n'est pas ajouté au
//! projet : le résultat est un fichier, comme celui d'un export PDF.
//!
//! Le noyau nomme le fichier de sortie (AD-10) et en déduit l'extension du `format` demandé
//! (AD-24) : `.gpkg`, `.shp` ou `.geojson`. Un Shapefile dépose ses fichiers compagnons
//! (`.shx`, `.dbf`, `.prj`, `.cpg`, `.qix`) à côté du `.shp`, sous le même radical. Aucun autre format
//! n'est permis : un tableur (Excel, CSV) est refusé par le noyau, avant QGIS.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

// Format de sortie. Pas de commentaire par variante : schemars en ferait une liste de `oneOf`,
// alors que le panneau ne sait faire une liste déroulante que d'une énumération plate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum VectorFormat {
    Gpkg,
    Shp,
    Geojson,
}

/// Extension du fichier écrit pour chaque format, point compris.
const EXTENSIONS: &[(&str, &str)] = &[("gpkg", ".gpkg"), ("shp", ".shp"), ("geojson", ".geojson")];

/// Paramètres de `data.convert`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataConvertParams {
    /// Id de la couche vecteur à écrire, rendu par `layer.add`, `project.inventory` ou une
    /// analyse (valable pour la session seulement). Elle n'est pas modifiée.
    pub layer_id: String,
    /// Format du fichier écrit : `gpkg` (GeoPackage), `shp` (Shapefile) ou `geojson` (GeoJSON).
    pub format: VectorFormat,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail, avec l'extension du
    /// format.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `data.convert`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DataConvertOutput {
    /// Fichier écrit (le `.shp` pour un Shapefile).
    pub file: String,
    /// Format écrit.
    pub format: VectorFormat,
    /// Nombre d'entités écrites (celui de la couche d'origine).
    pub feature_count: u64,
}

/// `data.convert` : commande déléguée à QGIS.
pub struct DataConvert;

impl Delegated for DataConvert {
    type Params = DataConvertParams;
    type Output = DataConvertOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<DataConvertParams, DataConvertOutput>(
            "data.convert",
            "Écrit une couche vecteur entière dans un fichier d'un autre format, par QGIS : GeoPackage (gpkg), Shapefile (shp) ou GeoJSON (geojson). Le fichier est écrit dans le dossier de travail, il n'est pas ajouté au projet. Ne modifie pas la couche d'origine. Un Shapefile coupe les noms de champ à 10 caractères et les textes à 254, garde les dates sans l'heure et demande un seul type de géométrie. Le GeoJSON garde le système de la couche : pour du WGS84, voir data.reproject.",
            "Writes a whole vector layer to a file in another format, through QGIS: GeoPackage (gpkg), Shapefile (shp) or GeoJSON (geojson). The file is written in the working folder and is not added to the project. Does not modify the original layer. A Shapefile cuts field names to 10 characters and text to 254, keeps dates without the time and needs a single geometry type. GeoJSON keeps the layer's system: for WGS84, see data.reproject.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext_from("format", EXTENSIONS)
        .example(serde_json::json!({"layer_id": "gares_1a2b3c", "format": "shp"}))
        .example(serde_json::json!({"layer_id": "communes_4d5e6f", "format": "gpkg"}))
        .tests(&[
            "data_convert_is_delegated",
            "data_convert_params_are_a_layer_and_a_format",
            "data_convert_output_extension_follows_the_format",
            "a_one_step_conversion_plan_passes_the_check",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<DataConvert>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn data_convert_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("data.convert").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        assert!(!spec.dictated_by_core);
        let err = registry
            .call("data.convert", json!({"layer_id": "gares", "format": "shp"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn data_convert_params_are_a_layer_and_a_format() {
        let registry = Registry::standard();
        for good in [
            json!({"layer_id": "gares", "format": "gpkg"}),
            json!({"layer_id": "gares", "format": "shp"}),
            json!({"layer_id": "gares", "format": "geojson"}),
            json!({"layer_id": "gares", "format": "shp", "out": "1-data.convert.shp"}),
        ] {
            assert!(
                registry.validate_params("data.convert", &good).is_ok(),
                "{good}"
            );
        }
        // Un tableur n'est pas un format de la fiche : refusé par le noyau, avant QGIS.
        for bad in [
            json!({"layer_id": "gares"}),
            json!({"format": "shp"}),
            json!({"layer_id": "gares", "format": "xlsx"}),
            json!({"layer_id": "gares", "format": "csv"}),
            json!({"layer_id": "gares", "format": "SHP"}),
            json!({"layer_id": "gares", "format": ".shp"}),
            json!({"layer_id": "gares", "format": 1}),
            json!({"layer_id": "gares", "format": "shp", "encoding": "UTF-8"}),
        ] {
            let err = registry.validate_params("data.convert", &bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn data_convert_output_extension_follows_the_format() {
        let registry = Registry::standard();
        let spec = registry.spec("data.convert").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        for (format, ext) in [("gpkg", ".gpkg"), ("shp", ".shp"), ("geojson", ".geojson")] {
            let params = json!({"layer_id": "gares", "format": format});
            assert_eq!(spec.output_ext_for(&params), ext, "{format}");
        }
        // Chaque valeur permise du schéma a son extension, et aucune autre.
        let allowed: Vec<&str> = spec.params_schema["properties"]["format"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let mut table: Vec<&str> = super::EXTENSIONS.iter().map(|(v, _)| *v).collect();
        let mut allowed_sorted = allowed.clone();
        allowed_sorted.sort_unstable();
        table.sort_unstable();
        assert_eq!(allowed_sorted, table);
        // Un format absent ou inconnu retombe sur l'extension par défaut : la validation, qui
        // suit le nommage, le refuse de toute façon.
        assert_eq!(spec.output_ext_for(&json!({"format": "xlsx"})), spec.output_ext);
        assert_eq!(spec.output_ext_for(&json!({})), spec.output_ext);
        // Les autres commandes gardent leur extension fixe.
        let reproject = registry.spec("data.reproject").unwrap();
        assert_eq!(reproject.output_ext_for(&json!({"format": "shp"})), ".gpkg");
    }

    #[test]
    fn a_one_step_conversion_plan_passes_the_check() {
        let registry = Registry::standard();
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "data.convert", "params": {"layer_id": "gares_1", "format": "gpkg"}}
        ]}))
        .unwrap();
        let checked = registry.check_plan(&plan);
        assert!(checked.is_ok(), "{checked:?}");
        // Après une analyse : la couche produite, par référence d'étape.
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "analysis.buffer", "params": {"layer_id": "gares_1",
                "distance": {"value": 500, "unit": "m"}}},
            {"command": "data.convert", "params": {"layer_id": "step:1.layer_id",
                "format": "shp"}}
        ]}))
        .unwrap();
        let checked = registry.check_plan(&plan);
        assert!(checked.is_ok(), "{checked:?}");
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "data.convert", "params": {"layer_id": "gares_1", "format": "xlsx"}}
        ]}))
        .unwrap();
        assert!(registry.check_plan(&plan).is_err());
    }
}
