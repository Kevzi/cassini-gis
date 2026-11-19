//! Commande `analysis.sum_line_lengths` : pour chaque polygone d'une couche, longueur totale des
//! lignes d'une seconde couche qu'il contient (découpées à son bord). Résultat au gabarit commun
//! `SummaryRow` (AD-27). Une couche en degrés est mesurée dans un système en mètres choisi par
//! le noyau (`work_epsg`, changement de cap du 2026-11-12), comme `analysis.nearest`.
//! Déclaration seule : le geste vit dans l'extension (AD-22 : aucun nom d'algorithme, aucun nom
//! de champ de QGIS ici). Le noyau nomme le fichier de sortie (AD-10) : une couche, donc `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::summary_row::SummaryRow;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

// Unité des longueurs. Pas de commentaire par variante : schemars en ferait une liste de `oneOf`,
// alors que le panneau ne sait faire une liste déroulante que d'une énumération plate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum LengthUnit {
    #[default]
    M,
    Km,
}

/// Paramètres de `analysis.sum_line_lengths`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalysisSumLineLengthsParams {
    /// Id de la couche de polygones, rendu par `layer.add`, `project.inventory` ou une analyse
    /// (valable pour la session seulement).
    pub layer_id: String,
    /// Id de la couche de lignes à mesurer dans chaque polygone. Différente de `layer_id`.
    pub lines_layer_id: String,
    /// Champ de la couche de polygones qui nomme chaque polygone dans le résultat (ex. le nom de
    /// la commune). Facultatif : sans lui, un polygone est désigné par son numéro d'entité.
    #[serde(default)]
    pub label_field: Option<String>,
    /// Unité des longueurs : `m` (par défaut) ou `km`. À poser seulement si la phrase en donne une.
    #[serde(default)]
    pub unit: LengthUnit,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
    /// Système en mètres du calcul, en code EPSG : fourni par le noyau d'après l'inventaire
    /// (changement de cap du 2026-11-12), jamais par l'appelant.
    #[serde(default)]
    pub work_epsg: Option<u32>,
}

/// Résultat de `analysis.sum_line_lengths`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisSumLineLengthsOutput {
    /// Id de la couche créée dans le projet : les polygones, avec un champ de longueur.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités (polygones) de la couche créée.
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
    /// Une ligne par polygone, 100 au plus : `length`, dans l'unité demandée (`m` ou `km`), 0 pour
    /// un polygone sans ligne. L'étiquette est `label_field`, sinon le rang du polygone.
    #[schemars(length(max = 100))]
    pub rows: Vec<SummaryRow>,
    /// Vrai si la couche a plus de polygones que `rows` n'en détaille.
    pub truncated: bool,
}

/// `analysis.sum_line_lengths` : commande déléguée à QGIS.
pub struct AnalysisSumLineLengths;

impl Delegated for AnalysisSumLineLengths {
    type Params = AnalysisSumLineLengthsParams;
    type Output = AnalysisSumLineLengthsOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisSumLineLengthsParams, AnalysisSumLineLengthsOutput>(
            "analysis.sum_line_lengths",
            "Longueur de lignes par polygone : somme, pour chaque polygone d'une couche (communes, secteurs), la longueur des lignes d'une seconde couche qu'il contient (ex. les routes par commune), en mètres ou en kilomètres. Une couche en degrés est mesurée dans un système en mètres choisi par le noyau. Le résultat est une couche des polygones avec un champ de longueur, ajoutée au projet, et un résumé par polygone. Ne modifie pas les couches d'entrée.",
            "Line length per polygon: sums, for each polygon of a layer (municipalities, sectors), the length of the lines of a second layer that it contains (e.g. roads per municipality), in meters or kilometers. A layer in degrees is measured in a system in meters chosen by the core. The result is a layer of the polygons with a length field, added to the project, and a per-polygon summary. Does not modify the input layers.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        // Une couche en degrés est reprojetée par QGIS, dans la même étape, vers le système en
        // mètres que choisit le noyau (`crate::work_crs`, entrée 9.16) : les longueurs sont en mètres.
        .work_crs_param("work_epsg", "layer_id")
        .example(serde_json::json!({"layer_id": "communes_1a2b3c", "lines_layer_id": "routes_4d5e6f", "label_field": "nom"}))
        .example(serde_json::json!({"layer_id": "communes_1a2b3c", "lines_layer_id": "routes_4d5e6f", "unit": "km"}))
        .tests(&[
            "analysis_sum_line_lengths_is_delegated",
            "analysis_sum_line_lengths_needs_two_layers",
            "analysis_sum_line_lengths_unit_is_m_or_km",
            "analysis_sum_line_lengths_output_is_a_gpkg_named_by_the_core",
            "analysis_sum_line_lengths_declares_its_work_crs",
            "analysis_sum_line_lengths_rows_follow_the_summary_template",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisSumLineLengths>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn analysis_sum_line_lengths_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.sum_line_lengths").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call(
                "analysis.sum_line_lengths",
                json!({"layer_id": "l1", "lines_layer_id": "l2"}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_sum_line_lengths_needs_two_layers() {
        let registry = Registry::standard();
        for ok in [
            json!({"layer_id": "l1", "lines_layer_id": "l2"}),
            json!({"layer_id": "l1", "lines_layer_id": "l2", "label_field": "nom"}),
        ] {
            assert!(registry
                .validate_params("analysis.sum_line_lengths", &ok)
                .is_ok());
        }
        for bad in [
            json!({"layer_id": "l1"}),
            json!({"lines_layer_id": "l2"}),
            json!({"layer_id": "l1", "lines_layer_id": "l2", "label_field": 3}),
            json!({"layer_id": "l1", "lines_layer_id": "l2", "statistic": "sum"}),
        ] {
            let err = registry
                .validate_params("analysis.sum_line_lengths", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
    }

    #[test]
    fn analysis_sum_line_lengths_unit_is_m_or_km() {
        let registry = Registry::standard();
        for unit in ["m", "km"] {
            let ok = json!({"layer_id": "l1", "lines_layer_id": "l2", "unit": unit});
            assert!(registry
                .validate_params("analysis.sum_line_lengths", &ok)
                .is_ok());
        }
        for unit in [json!("mi"), json!("KM"), json!(1), json!({"value": 1, "unit": "km"})] {
            let bad = json!({"layer_id": "l1", "lines_layer_id": "l2", "unit": unit});
            assert!(registry
                .validate_params("analysis.sum_line_lengths", &bad)
                .is_err());
        }
    }

    #[test]
    fn analysis_sum_line_lengths_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.sum_line_lengths").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
        assert_eq!(spec.output_schema["properties"]["rows"]["maxItems"], json!(100));
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn analysis_sum_line_lengths_declares_its_work_crs() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.sum_line_lengths").unwrap();
        assert_eq!(
            spec.work_crs_param,
            Some(("work_epsg".to_string(), "layer_id".to_string()))
        );
        let ok = json!({"layer_id": "l1", "lines_layer_id": "l2", "work_epsg": 2154});
        assert!(registry.validate_params("analysis.sum_line_lengths", &ok).is_ok());
        let bad = json!({"layer_id": "l1", "lines_layer_id": "l2", "work_epsg": -1});
        assert!(registry.validate_params("analysis.sum_line_lengths", &bad).is_err());
    }

    #[test]
    fn analysis_sum_line_lengths_rows_follow_the_summary_template() {
        let registry = Registry::standard();
        let result = |rows: Vec<serde_json::Value>| {
            json!({"layer_id": "z1", "name": "n", "feature_count": rows.len(), "crs": "EPSG:2154",
                "rows": rows, "truncated": false})
        };
        let row = |value: f64, unit: &str| {
            json!({"label": "a", "values": {"length": {"value": value, "unit": unit}}})
        };
        let metres = result(vec![row(6298.3, "m"), row(0.0, "m")]);
        let km = result(vec![row(6.3, "km")]);
        let empty = result(vec![]);
        for ok in [&metres, &km, &empty] {
            assert!(registry.validate_output("analysis.sum_line_lengths", ok).is_ok());
        }
        let no_rows = json!({"layer_id": "z1", "name": "n", "feature_count": 1, "crs": "EPSG:2154",
            "truncated": false});
        assert!(registry.validate_output("analysis.sum_line_lengths", &no_rows).is_err());
    }
}
