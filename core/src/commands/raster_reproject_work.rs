//! Commande `raster.reproject_work` : reprojection d'un MNT en degrés vers le système en mètres
//! que le noyau a choisi pour le calcul (ticket 33, première moitié : déclaration seule, le geste
//! de l'extension est la seconde moitié). Déclaration seule : le geste vit dans l'extension, qui
//! lance l'algorithme du cadre de traitement de QGIS depuis sa table fermée (AD-22, AD-24 : aucun
//! nom d'algorithme ici).
//!
//! Elle est **dictée par le noyau**, comme `layer.open_raster` : juste **avant** une commande de
//! terrain dont le MNT est en degrés (`CommandSpec::work_crs_param` sur un `RasterPath`), la
//! session lit le système du MNT (`crate::dem::crs_of`), choisit le code EPSG
//! (`crate::work_crs::work_epsg_for_dem`) et ordonne cette reprojection ; la commande de terrain
//! lit ensuite la sortie, désignée par `out:<k>.1`. Le noyau nomme le fichier (AD-10). Aucune
//! étape n'est ajoutée au plan montré (AD-23) ; le journal garde la ligne, que le rejeu rejoue.
//! Elle n'est offerte ni au modèle, ni au client MCP, ni au formulaire du panneau.
//!
//! Le MNT d'origine n'est jamais modifié (§2.6). Un MNT déjà en mètres n'est jamais reprojeté
//! (amendement RA06 d'AD-24) : le noyau ne dicte pas cette commande dans ce cas.

use schemars::JsonSchema;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::raster_path::RasterPath;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Identifiant de la commande, que la session dicte avant une commande de terrain.
pub const REPROJECT_WORK: &str = "raster.reproject_work";

/// Paramètres de `raster.reproject_work`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RasterReprojectWorkParams {
    /// MNT en degrés à reprojeter : chemin relatif au projet, ou `out:<k>.<n>`. Il n'est pas
    /// modifié.
    pub raster: RasterPath,
    /// Code EPSG du système en mètres du calcul : choisi par le noyau, jamais par l'appelant.
    #[serde(deserialize_with = "epsg_code")]
    #[schemars(range(min = 1))]
    pub work_epsg: u32,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Un code EPSG est strictement positif : 0 est refusé par le noyau, pas par le geste.
fn epsg_code<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let code = u32::deserialize(d)?;
    if code == 0 {
        return Err(D::Error::custom("`work_epsg` est un code EPSG, au moins 1"));
    }
    Ok(code)
}

/// Résultat de `raster.reproject_work`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RasterReprojectWorkOutput {
    /// GeoTIFF écrit, dans le système en mètres du calcul.
    pub file: String,
    /// Code EPSG du GeoTIFF écrit (celui de `work_epsg`).
    pub epsg: Option<u32>,
}

/// `raster.reproject_work` : commande déléguée à QGIS, dictée par le noyau.
pub struct RasterReprojectWork;

impl Delegated for RasterReprojectWork {
    type Params = RasterReprojectWorkParams;
    type Output = RasterReprojectWorkOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<RasterReprojectWorkParams, RasterReprojectWorkOutput>(
            REPROJECT_WORK,
            "Reprojette un MNT en degrés vers le système en mètres choisi par le noyau pour le calcul, et écrit un GeoTIFF Float32, sans-donnée -9999 ; dictée par le noyau avant une commande de terrain, jamais offerte. L'original n'est pas modifié.",
            "Reprojects a DEM in degrees to the metric system the core chose for the computation, and writes a Float32 GeoTIFF, no-data -9999; dictated by the core before a terrain command, never offered. The original is not modified.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(false)
        .path_params(&["raster"])
        .output_params(&["out"])
        .output_ext(".tif")
        .dictated_by_core()
        .example(serde_json::json!({"raster": "mnt/annecy-4326.tif", "work_epsg": 32632}))
        .tests(&[
            "raster_reproject_work_is_delegated_and_dictated",
            "raster_reproject_work_takes_a_raster_and_a_positive_code",
            "raster_reproject_work_output_is_a_tif_named_by_the_core",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<RasterReprojectWork>()) }

#[cfg(test)]
mod tests {
    use super::REPROJECT_WORK;
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn raster_reproject_work_is_delegated_and_dictated() {
        let registry = Registry::standard();
        let spec = registry.spec(REPROJECT_WORK).unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(false));
        assert!(spec.dictated_by_core);
        let err = registry
            .call(REPROJECT_WORK, json!({"raster": "mnt.tif", "work_epsg": 2154}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn raster_reproject_work_takes_a_raster_and_a_positive_code() {
        let registry = Registry::standard();
        for good in [
            json!({"raster": "mnt.tif", "work_epsg": 32632}),
            json!({"raster": "out:2.1", "work_epsg": 2154, "out": "3-raster.reproject_work.tif"}),
        ] {
            assert!(
                registry.validate_params(REPROJECT_WORK, &good).is_ok(),
                "{good}"
            );
        }
        // Aucun paramètre de rééchantillonnage ni de taille de cellule : le geste les fixe.
        for bad in [
            json!({}),
            json!({"raster": "mnt.tif"}),
            json!({"work_epsg": 2154}),
            json!({"raster": "mnt.tif", "work_epsg": 0}),
            json!({"raster": "mnt.tif", "work_epsg": -2154}),
            json!({"raster": "mnt.tif", "work_epsg": "2154"}),
            json!({"raster": "mnt.tif", "work_epsg": 2154, "resampling": "cubic"}),
            json!({"raster": 12, "work_epsg": 2154}),
        ] {
            let err = registry.validate_params(REPROJECT_WORK, &bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn raster_reproject_work_output_is_a_tif_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec(REPROJECT_WORK).unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".tif");
        assert_eq!(spec.path_params, vec!["raster".to_string()]);
        assert!(crate::raster_path::writes_one_raster(spec));
    }
}
