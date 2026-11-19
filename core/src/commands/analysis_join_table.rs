//! Commande `analysis.join_table` : jointure d'un tableau à une couche. Chaque entité d'une couche
//! reçoit les colonnes de la ligne d'un tableau (CSV sans géométrie, ouvert par `layer.add` avec
//! `table`) qui porte la même valeur dans un champ commun (ex. le code INSEE). Déclaration seule :
//! le geste vit dans l'extension (AD-22 : aucun nom d'algorithme ni de champ de QGIS ici). Le noyau
//! nomme le fichier de sortie (AD-10) : une couche, donc `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `analysis.join_table`, tels que les écrit l'appelant.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawParams {
    layer_id: String,
    table_layer_id: String,
    field: String,
    table_field: String,
    #[serde(default)]
    out: String,
}

/// Paramètres de `analysis.join_table`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(try_from = "RawParams")]
pub struct AnalysisJoinTableParams {
    /// Id de la couche qui reçoit les colonnes, rendu par `layer.add`, `project.inventory` ou une
    /// analyse (valable pour la session seulement).
    pub layer_id: String,
    /// Id du tableau (couche sans géométrie ouverte par `layer.add` avec `table`). Différent de
    /// `layer_id`.
    pub table_layer_id: String,
    /// Champ commun, côté couche : un champ de l'inventaire de `layer_id`.
    pub field: String,
    /// Champ commun, côté tableau : un champ de l'inventaire de `table_layer_id`. Ses valeurs sont
    /// lues en texte : une clé à zéro de tête (`01001`) se joint à la même clé en texte.
    pub table_field: String,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

impl TryFrom<RawParams> for AnalysisJoinTableParams {
    type Error = String;

    /// Refuse la même couche des deux côtés, avant tout ordre à QGIS.
    fn try_from(raw: RawParams) -> Result<Self, String> {
        if raw.layer_id == raw.table_layer_id {
            return Err(
                "`layer_id` et `table_layer_id` doivent désigner deux couches différentes".into(),
            );
        }
        Ok(Self {
            layer_id: raw.layer_id,
            table_layer_id: raw.table_layer_id,
            field: raw.field,
            table_field: raw.table_field,
            out: raw.out,
        })
    }
}

/// Résultat de `analysis.join_table`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisJoinTableOutput {
    /// Id de la couche créée dans le projet.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités de la couche créée (toutes celles de `layer_id`).
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
    /// Nombre d'entités de `layer_id` sans ligne correspondante dans le tableau : gardées, leurs
    /// colonnes jointes sont vides.
    pub unmatched_count: u64,
}

/// `analysis.join_table` : commande déléguée à QGIS.
pub struct AnalysisJoinTable;

impl Delegated for AnalysisJoinTable {
    type Params = AnalysisJoinTableParams;
    type Output = AnalysisJoinTableOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisJoinTableParams, AnalysisJoinTableOutput>(
            "analysis.join_table",
            "Jointure d'un tableau à une couche : chaque entité d'une couche reçoit les colonnes de la ligne du tableau (CSV sans géométrie, ouvert par layer.add avec table) qui porte la même valeur dans un champ commun (ex. le code INSEE d'une commune, le numéro d'une parcelle). Une entité sans ligne correspondante est gardée, colonnes vides, et comptée ; si le tableau a plusieurs lignes pour une clé, la première est retenue. Le résultat est une nouvelle couche ajoutée au projet. Ne modifie pas les entrées.",
            "Table join: each feature of a layer receives the columns of the row of a table (CSV without geometry, opened by layer.add with table) that holds the same value in a common field (e.g. a commune INSEE code, a parcel number). A feature with no matching row is kept, with empty columns, and counted; if the table has several rows for a key, the first one is kept. The result is a new layer added to the project. Does not modify the inputs.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        .example(serde_json::json!({"layer_id": "parcelles_1a2b3c", "table_layer_id": "prix_4d5e6f", "field": "id_parcelle", "table_field": "id_parcelle"}))
        .example(serde_json::json!({"layer_id": "communes_1a2b3c", "table_layer_id": "population_4d5e6f", "field": "code_insee", "table_field": "CODGEO"}))
        .tests(&[
            "analysis_join_table_is_delegated",
            "analysis_join_table_needs_two_different_layers_and_two_fields",
            "analysis_join_table_output_is_a_gpkg_named_by_the_core",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisJoinTable>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    fn good() -> serde_json::Value {
        json!({"layer_id": "l1", "table_layer_id": "t1", "field": "code", "table_field": "CODGEO"})
    }

    #[test]
    fn analysis_join_table_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.join_table").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry.call("analysis.join_table", good()).unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_join_table_needs_two_different_layers_and_two_fields() {
        let registry = Registry::standard();
        assert!(registry.validate_params("analysis.join_table", &good()).is_ok());
        for bad in [
            json!({"layer_id": "l1", "table_layer_id": "t1", "field": "code"}),
            json!({"layer_id": "l1", "table_layer_id": "t1", "table_field": "code"}),
            json!({"layer_id": "l1", "field": "code", "table_field": "code"}),
            json!({"layer_id": "l1", "table_layer_id": "l1", "field": "code", "table_field": "code"}),
            json!({"layer_id": "l1", "table_layer_id": "t1", "field": "code", "table_field": "code", "how": "left"}),
        ] {
            let err = registry
                .validate_params("analysis.join_table", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
    }

    #[test]
    fn analysis_join_table_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.join_table").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
        assert!(registry.check_contract().is_empty());
    }
}
