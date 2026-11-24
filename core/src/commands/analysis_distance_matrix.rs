//! Commande `analysis.distance_matrix` : distances, en mètres, entre les entités de deux couches
//! de points. Le résultat est un tableau sans géométrie, une ligne par paire (départ, cible,
//! distance), écrit dans le dossier de travail (AD-10) et résumé au gabarit commun `SummaryRow`
//! (AD-27) ; il n'est pas ajouté au projet. Déclaration seule : le geste vit dans l'extension
//! (AD-22 : aucun nom d'algorithme, aucun nom de champ de QGIS ici). Le noyau nomme le fichier de
//! sortie : un tableau, donc `.csv`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::summary_row::SummaryRow;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `analysis.distance_matrix`, tels que les écrit l'appelant.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawParams {
    layer_id: String,
    label_field: String,
    target_layer_id: String,
    target_label_field: String,
    #[serde(default)]
    #[schemars(range(min = 1))]
    neighbors: Option<u32>,
    #[serde(default)]
    out: String,
    #[serde(default)]
    work_epsg: Option<u32>,
}

/// Paramètres de `analysis.distance_matrix`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(try_from = "RawParams")]
pub struct AnalysisDistanceMatrixParams {
    /// Id de la couche de points de départ, rendu par `layer.add`, `project.inventory` ou une
    /// analyse (valable pour la session seulement). Elle n'est pas modifiée.
    pub layer_id: String,
    /// Champ de la couche de départ qui nomme chaque entité dans le tableau (ex. le nom de l'école).
    pub label_field: String,
    /// Id de la couche de points d'arrivée. Différente de `layer_id`.
    pub target_layer_id: String,
    /// Champ de la couche d'arrivée qui nomme chaque entité dans le tableau (ex. le nom de la gare).
    pub target_label_field: String,
    /// Nombre de cibles gardées par départ, les plus proches d'abord (au moins 1). Absent : toutes
    /// les paires. À poser seulement si la phrase en donne un, ou si les couches ont beaucoup
    /// d'entités (au-delà de 100 000 paires, la commande exige ce nombre).
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub neighbors: Option<u32>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
    /// Système en mètres du calcul, en code EPSG : fourni par le noyau d'après l'inventaire
    /// (changement de cap du 2026-11-12), jamais par l'appelant.
    #[serde(default)]
    pub work_epsg: Option<u32>,
}

impl TryFrom<RawParams> for AnalysisDistanceMatrixParams {
    type Error = String;

    /// Refuse la même couche des deux côtés et un nombre de voisins nul, avant tout ordre à QGIS.
    fn try_from(raw: RawParams) -> Result<Self, String> {
        if raw.layer_id == raw.target_layer_id {
            return Err(
                "`layer_id` et `target_layer_id` doivent désigner deux couches différentes".into(),
            );
        }
        if raw.neighbors == Some(0) {
            return Err("`neighbors` doit valoir au moins 1".into());
        }
        Ok(Self {
            layer_id: raw.layer_id,
            label_field: raw.label_field,
            target_layer_id: raw.target_layer_id,
            target_label_field: raw.target_label_field,
            neighbors: raw.neighbors,
            out: raw.out,
            work_epsg: raw.work_epsg,
        })
    }
}

/// Résultat de `analysis.distance_matrix`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisDistanceMatrixOutput {
    /// Tableau écrit (CSV sans géométrie), toutes les paires gardées ; il n'est pas ajouté au projet.
    pub file: String,
    /// Nombre de paires du tableau (départ, cible).
    pub pair_count: u64,
    /// Une ligne par paire, 100 au plus : `distance` en mètres. L'étiquette est « départ → cible ».
    #[schemars(length(max = 100))]
    pub rows: Vec<SummaryRow>,
    /// Vrai si le tableau a plus de paires que `rows` n'en détaille.
    pub truncated: bool,
}

/// `analysis.distance_matrix` : commande déléguée à QGIS.
pub struct AnalysisDistanceMatrix;

impl Delegated for AnalysisDistanceMatrix {
    type Params = AnalysisDistanceMatrixParams;
    type Output = AnalysisDistanceMatrixOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisDistanceMatrixParams, AnalysisDistanceMatrixOutput>(
            "analysis.distance_matrix",
            "Matrice de distances : distances, en mètres, entre chaque entité d'une couche de points de départ et les entités d'une couche de points d'arrivée (ex. entre chaque école et chaque gare). Un champ de chaque couche nomme les entités dans le tableau. Un nombre de voisins facultatif ne garde que les cibles les plus proches de chaque départ. Le résultat est un tableau écrit dans le dossier de travail, sans géométrie, qui n'est pas ajouté au projet, et un résumé par paire. Couches de points seulement. Ne modifie pas les couches d'entrée.",
            "Distance matrix: distances, in meters, between each feature of a start point layer and the features of a target point layer (e.g. between each school and each station). A field of each layer names the features in the table. An optional number of neighbours keeps only the closest targets of each start. The result is a table written in the working folder, without geometry, which is not added to the project, and a per-pair summary. Point layers only. Does not modify the input layers.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".csv")
        // Une couche en degrés est reprojetée par QGIS, dans la même étape, vers le système en
        // mètres que choisit le noyau (`crate::work_crs`, entrée 9.16) : la distance est en mètres.
        .work_crs_param("work_epsg", "layer_id")
        .example(serde_json::json!({"layer_id": "ecoles_1a2b3c", "label_field": "nom", "target_layer_id": "gares_4d5e6f", "target_label_field": "nom"}))
        .example(serde_json::json!({"layer_id": "ecoles_1a2b3c", "label_field": "nom", "target_layer_id": "gares_4d5e6f", "target_label_field": "nom", "neighbors": 3}))
        .tests(&[
            "analysis_distance_matrix_is_delegated",
            "analysis_distance_matrix_needs_two_layers_and_two_label_fields",
            "analysis_distance_matrix_neighbors_is_optional_and_positive",
            "analysis_distance_matrix_output_is_a_csv_named_by_the_core",
            "analysis_distance_matrix_rows_are_bounded",
            "analysis_distance_matrix_declares_its_work_crs",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisDistanceMatrix>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    fn params() -> serde_json::Value {
        json!({"layer_id": "l1", "label_field": "nom",
               "target_layer_id": "l2", "target_label_field": "nom"})
    }

    #[test]
    fn analysis_distance_matrix_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.distance_matrix").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call("analysis.distance_matrix", params())
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_distance_matrix_needs_two_layers_and_two_label_fields() {
        let registry = Registry::standard();
        assert!(registry
            .validate_params("analysis.distance_matrix", &params())
            .is_ok());
        let mut same = params();
        same["target_layer_id"] = json!("l1");
        let mut extra = params();
        extra["count"] = json!(3);
        for bad in [
            json!({"layer_id": "l1", "target_layer_id": "l2"}),
            json!({"layer_id": "l1", "label_field": "nom", "target_layer_id": "l2"}),
            json!({"layer_id": "l1", "target_layer_id": "l2", "target_label_field": "nom"}),
            json!({"label_field": "nom", "target_layer_id": "l2", "target_label_field": "nom"}),
            same,
            extra,
        ] {
            let err = registry
                .validate_params("analysis.distance_matrix", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
    }

    #[test]
    fn analysis_distance_matrix_neighbors_is_optional_and_positive() {
        let registry = Registry::standard();
        let with = |neighbors: serde_json::Value| {
            let mut p = params();
            p["neighbors"] = neighbors;
            p
        };
        for ok in [with(json!(1)), with(json!(3))] {
            assert!(registry
                .validate_params("analysis.distance_matrix", &ok)
                .is_ok());
        }
        for bad in [with(json!(0)), with(json!(-2)), with(json!(1.5)), with(json!("3"))] {
            assert!(registry
                .validate_params("analysis.distance_matrix", &bad)
                .is_err());
        }
    }

    #[test]
    fn analysis_distance_matrix_output_is_a_csv_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.distance_matrix").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".csv");
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn analysis_distance_matrix_rows_are_bounded() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.distance_matrix").unwrap();
        let rows = &spec.output_schema["properties"]["rows"];
        assert_eq!(rows["maxItems"], json!(100));
    }

    #[test]
    fn analysis_distance_matrix_declares_its_work_crs() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.distance_matrix").unwrap();
        assert_eq!(
            spec.work_crs_param,
            Some(("work_epsg".to_string(), "layer_id".to_string()))
        );
        let mut ok = params();
        ok["work_epsg"] = json!(2154);
        assert!(registry
            .validate_params("analysis.distance_matrix", &ok)
            .is_ok());
        let mut bad = params();
        bad["work_epsg"] = json!(-1);
        assert!(registry
            .validate_params("analysis.distance_matrix", &bad)
            .is_err());
    }
}
