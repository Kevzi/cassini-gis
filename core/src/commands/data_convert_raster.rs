//! Commande `data.convert_raster` : écriture d'un raster lisible par QGIS (ASCII Grid `.asc`,
//! ERDAS `.img`, JPEG 2000 `.jp2`, GeoTIFF, VRT…) en GeoTIFF que le noyau sait lire (AD-20),
//! par QGIS (changement de cap du 2026-11-10, epic 10, entrée 30). Déclaration seule : le geste
//! vit dans l'extension, qui lance l'algorithme du cadre de traitement de QGIS depuis sa table
//! fermée (AD-22, AD-24 : aucun nom d'algorithme ici).
//!
//! Le GeoTIFF écrit est en Float32, sans-donnée -9999, première bande seulement ; l'original
//! n'est jamais modifié (§2.6). `assign_epsg` déclare le système d'une source qui n'en a pas
//! (ASCII Grid sans `.prj`) ; il ne reprojette jamais (AD-19) : une source qui a déjà un autre
//! système est refusée par le geste. Le noyau nomme le fichier de sortie (AD-10), un `.tif`, et
//! dicte ensuite `layer.open_raster` sur lui (amendement du 2026-11-05 d'AD-24), comme après un
//! GeoTIFF du noyau : un plan peut enchaîner la conversion et une commande de terrain par
//! `step:k`.

use schemars::JsonSchema;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::raster_path::RasterPath;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `data.convert_raster`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataConvertRasterParams {
    /// Raster à convertir : chemin relatif au projet. Il n'est pas modifié.
    pub raster: RasterPath,
    /// Code EPSG à déclarer, seulement pour une source qui n'a pas de système (ex. 2154 pour
    /// un ASCII Grid du Lambert-93 livré sans `.prj`). Jamais une reprojection.
    #[serde(default, deserialize_with = "epsg_code")]
    #[schemars(range(min = 1))]
    pub assign_epsg: Option<u32>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Un code EPSG est strictement positif : 0 est refusé par le noyau, pas par le geste.
fn epsg_code<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
    let code = Option::<u32>::deserialize(d)?;
    if code == Some(0) {
        return Err(D::Error::custom("`assign_epsg` est un code EPSG, au moins 1"));
    }
    Ok(code)
}

/// Résultat de `data.convert_raster`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DataConvertRasterOutput {
    /// GeoTIFF écrit.
    pub file: String,
    /// Code EPSG du GeoTIFF écrit, ou `null` s'il n'a pas de système (les commandes du noyau
    /// le refuseront alors, `dem.no_epsg`).
    pub epsg: Option<u32>,
}

/// `data.convert_raster` : commande déléguée à QGIS.
pub struct DataConvertRaster;

impl Delegated for DataConvertRaster {
    type Params = DataConvertRasterParams;
    type Output = DataConvertRasterOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<DataConvertRasterParams, DataConvertRasterOutput>(
            "data.convert_raster",
            "Écrit un raster d'un autre format (ASCII Grid .asc, ERDAS .img, JPEG 2000 .jp2, VRT…) en GeoTIFF, par QGIS, pour que les commandes de terrain le lisent : Float32, sans-donnée -9999, première bande. Le GeoTIFF est ouvert dans le projet ; l'original n'est pas modifié. assign_epsg déclare le système d'une source qui n'en a pas (un .asc sans .prj) ; ne reprojette jamais. Pour enchaîner une pente, passer step:k au MNT.",
            "Writes a raster of another format (ASCII Grid .asc, ERDAS .img, JPEG 2000 .jp2, VRT…) to GeoTIFF, through QGIS, so that terrain commands can read it: Float32, no-data -9999, first band. The GeoTIFF is opened in the project; the original is not modified. assign_epsg declares the system of a source that has none (an .asc without .prj); never reprojects. To chain a slope, pass step:k as the DEM.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .path_params(&["raster"])
        .output_params(&["out"])
        .output_ext(".tif")
        .example(serde_json::json!({"raster": "mnt/BDALTIV2_25M_0900_6550.asc", "assign_epsg": 2154}))
        .example(serde_json::json!({"raster": "mnt/mnt_annecy.img"}))
        .tests(&[
            "data_convert_raster_is_delegated",
            "data_convert_raster_params_are_a_raster_and_an_optional_code",
            "data_convert_raster_output_is_a_tif_named_by_the_core",
            "a_conversion_then_slope_plan_passes_the_check",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<DataConvertRaster>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn data_convert_raster_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("data.convert_raster").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        assert!(!spec.dictated_by_core);
        let err = registry
            .call("data.convert_raster", json!({"raster": "mnt.asc"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn data_convert_raster_params_are_a_raster_and_an_optional_code() {
        let registry = Registry::standard();
        for good in [
            json!({"raster": "mnt.asc"}),
            json!({"raster": "mnt.asc", "assign_epsg": 2154}),
            json!({"raster": "mnt.img", "out": "1-data.convert_raster.tif"}),
        ] {
            assert!(
                registry.validate_params("data.convert_raster", &good).is_ok(),
                "{good}"
            );
        }
        // Aucun paramètre de reprojection, de type de données ou de bande : le geste les fixe
        // (Float32, -9999, première bande).
        for bad in [
            json!({}),
            json!({"assign_epsg": 2154}),
            json!({"raster": "mnt.asc", "assign_epsg": 0}),
            json!({"raster": "mnt.asc", "assign_epsg": -2154}),
            json!({"raster": "mnt.asc", "assign_epsg": "2154"}),
            json!({"raster": "mnt.asc", "assign_epsg": 2154.5}),
            json!({"raster": "mnt.asc", "target_epsg": 4326}),
            json!({"raster": "mnt.asc", "band": 2}),
            json!({"raster": 12}),
        ] {
            let err = registry
                .validate_params("data.convert_raster", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn data_convert_raster_output_is_a_tif_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("data.convert_raster").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".tif");
        assert_eq!(spec.path_params, vec!["raster".to_string()]);
        assert!(crate::raster_path::writes_one_raster(spec));
    }

    #[test]
    fn a_conversion_then_slope_plan_passes_the_check() {
        let registry = Registry::standard();
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "data.convert_raster", "params": {"raster": "mnt/dalle.asc",
                "assign_epsg": 2154}},
            {"command": "terrain.slope", "params": {"dem": "step:1", "unit": "degrees"}}
        ]}))
        .unwrap();
        let checked = registry.check_plan(&plan);
        assert!(checked.is_ok(), "{checked:?}");
        // La conversion seule.
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "data.convert_raster", "params": {"raster": "mnt/dalle.img"}}
        ]}))
        .unwrap();
        assert!(registry.check_plan(&plan).is_ok());
    }
}
