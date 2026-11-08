//! Commande `data.reproject` : reprojection d'une couche vecteur entière vers un code EPSG, par
//! QGIS, en nouvelle couche. Déclaration seule : le geste vit dans l'extension, qui lance
//! l'algorithme du cadre de traitement de QGIS depuis sa table de traduction (AD-22 : aucun nom
//! d'algorithme ici). La reprojection est celle de QGIS, dans le contexte de transformation du
//! projet, jamais du noyau (AD-19). La couche d'origine n'est jamais modifiée (§2.6).
//!
//! Le noyau nomme le fichier de sortie (AD-10) ; c'est une couche, donc `.gpkg`. Erreur du
//! geste : `data.unknown_crs` (code cible inconnu de QGIS, nommé dans le message), avant tout
//! lancement. À distinguer de `data.transform_point`, qui reprojette un seul point sans créer de
//! couche.

use schemars::JsonSchema;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `data.reproject`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataReprojectParams {
    /// Id de la couche vecteur à reprojeter, rendu par `layer.add`, `project.inventory` ou une
    /// analyse (valable pour la session seulement). Elle n'est pas modifiée.
    pub layer_id: String,
    /// Code EPSG du système d'arrivée (ex. 2154 pour le Lambert-93, 4326 pour le WGS84).
    #[serde(deserialize_with = "epsg_code")]
    #[schemars(range(min = 1))]
    pub target_epsg: u32,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Un code EPSG est strictement positif : 0 est refusé par le noyau, pas par le geste.
fn epsg_code<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let code = u32::deserialize(d)?;
    if code == 0 {
        return Err(D::Error::custom("`target_epsg` est un code EPSG, au moins 1"));
    }
    Ok(code)
}

/// Résultat de `data.reproject`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DataReprojectOutput {
    /// Id de la couche créée dans le projet.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre d'entités de la couche créée (celui de la couche d'origine).
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
}

/// `data.reproject` : commande déléguée à QGIS.
pub struct DataReproject;

impl Delegated for DataReproject {
    type Params = DataReprojectParams;
    type Output = DataReprojectOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<DataReprojectParams, DataReprojectOutput>(
            "data.reproject",
            "Reprojette une couche vecteur entière vers un code EPSG, par QGIS : par exemple une couche en degrés (EPSG:4326) vers le Lambert-93 (EPSG:2154) avant une analyse en mètres. Le résultat est ajouté au projet comme nouvelle couche. Ne modifie pas la couche d'origine. Pour un seul point, voir data.transform_point.",
            "Reprojects a whole vector layer to an EPSG code, through QGIS: for instance a layer in degrees (EPSG:4326) to Lambert-93 (EPSG:2154) before an analysis in metres. The result is added to the project as a new layer. Does not modify the original layer. For a single point, see data.transform_point.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        .example(serde_json::json!({"layer_id": "gares_1a2b3c", "target_epsg": 2154}))
        .example(serde_json::json!({"layer_id": "communes_4d5e6f", "target_epsg": 4326}))
        .tests(&[
            "data_reproject_is_delegated",
            "data_reproject_params_are_a_layer_and_a_code",
            "data_reproject_output_is_a_gpkg_named_by_the_core",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<DataReproject>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn data_reproject_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("data.reproject").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        assert!(!spec.dictated_by_core);
        let err = registry
            .call(
                "data.reproject",
                json!({"layer_id": "gares", "target_epsg": 2154}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn data_reproject_params_are_a_layer_and_a_code() {
        let registry = Registry::standard();
        for good in [
            json!({"layer_id": "gares", "target_epsg": 2154}),
            json!({"layer_id": "gares", "target_epsg": 4326}),
            json!({"layer_id": "gares", "target_epsg": 2154, "out": "r.gpkg"}),
        ] {
            assert!(
                registry.validate_params("data.reproject", &good).is_ok(),
                "{good}"
            );
        }
        for bad in [
            json!({"layer_id": "gares"}),
            json!({"target_epsg": 2154}),
            json!({"layer_id": "gares", "target_epsg": -2154}),
            json!({"layer_id": "gares", "target_epsg": 0}),
            json!({"layer_id": "gares", "target_epsg": 2154.5}),
            json!({"layer_id": "gares", "target_epsg": "2154"}),
            json!({"layer_id": "gares", "target_epsg": 2154, "operation": "+proj=noop"}),
        ] {
            let err = registry
                .validate_params("data.reproject", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn data_reproject_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("data.reproject").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
    }
}
