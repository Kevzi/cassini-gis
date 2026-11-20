//! Commande `raster.contour` : courbes de niveau d'un MNT à une équidistance donnée. Déclaration
//! seule : le geste vit dans l'extension (AD-22 : aucun nom d'algorithme, aucun nom de champ de
//! QGIS ici). Le MNT se désigne par un `RasterPath` (amendement du 2026-11-05 d'AD-24) : chemin
//! relatif au projet, jamais un identifiant de couche. Un `step:k` est refusé par le contrôle de
//! plan : les valeurs d'un raster chaîné (pente, coût) ne sont pas des mètres. Le noyau nomme le
//! fichier de sortie (AD-10) : une couche de lignes, donc `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::analysis_buffer::Distance;
use crate::raster_path::RasterPath;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Paramètres de `raster.contour`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RasterContourParams {
    /// MNT à une bande (fichier GeoTIFF), chemin copié de l'inventaire. Pas un id de couche, et
    /// pas `step:k` : seules des altitudes en mètres donnent des courbes justes.
    pub raster: RasterPath,
    /// Équidistance verticale entre deux courbes, strictement positive, en `m` ou `km`.
    pub interval: Distance,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `raster.contour`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RasterContourOutput {
    /// Id de la couche créée dans le projet : les courbes, avec le champ `elevation`.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre de courbes de la couche créée.
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
}

/// `raster.contour` : commande déléguée à QGIS.
pub struct RasterContour;

impl Delegated for RasterContour {
    type Params = RasterContourParams;
    type Output = RasterContourOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<RasterContourParams, RasterContourOutput>(
            "raster.contour",
            "Trace les courbes de niveau d'un modèle numérique de terrain (fichier GeoTIFF à une bande désigné par `raster`, chemin recopié de l'inventaire, pas un id de couche ni `step:k`) à une équidistance donnée (`interval`, en m ou km, par exemple « tous les 5 mètres »). Le résultat est une nouvelle couche de lignes, une par palier d'altitude, avec le champ `elevation` en mètres, ajoutée au projet. Ne modifie pas le raster.",
            "Draws the contour lines of a digital elevation model (single-band GeoTIFF file given by `raster`, a path copied from the inventory, not a layer id nor `step:k`) at a given interval (`interval`, in m or km, e.g. \"every 5 metres\"). The result is a new line layer, one line per elevation level, with the `elevation` field in metres, added to the project. Does not modify the raster.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .path_params(&["raster"])
        .output_params(&["out"])
        .output_ext(".gpkg")
        .example(serde_json::json!({"raster": "dem/mnt.tif", "interval": {"value": 5, "unit": "m"}}))
        .example(serde_json::json!({"raster": "mnt-colline-2154.tif", "interval": {"value": 0.01, "unit": "km"}}))
        .tests(&[
            "raster_contour_is_delegated",
            "raster_contour_refuses_a_non_positive_interval",
            "raster_contour_output_is_a_gpkg_named_by_the_core",
            "raster_contour_designates_the_raster_by_path",
            "raster_contour_refuses_a_step_raster",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<RasterContour>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn raster_contour_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.contour").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call(
                "raster.contour",
                json!({"raster": "mnt.tif", "interval": {"value": 5, "unit": "m"}}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn raster_contour_refuses_a_non_positive_interval() {
        let registry = Registry::standard();
        for ok in [
            json!({"raster": "mnt.tif", "interval": {"value": 5, "unit": "m"}}),
            json!({"raster": "mnt.tif", "interval": {"value": 0.005, "unit": "km"}}),
        ] {
            assert!(registry.validate_params("raster.contour", &ok).is_ok());
        }
        for bad in [
            json!({"raster": "mnt.tif", "interval": {"value": 0, "unit": "m"}}),
            json!({"raster": "mnt.tif", "interval": {"value": -5, "unit": "m"}}),
            json!({"raster": "mnt.tif", "interval": 5}),
            json!({"raster": "mnt.tif"}),
            json!({"interval": {"value": 5, "unit": "m"}}),
            json!({"raster": "mnt.tif", "interval": {"value": 5, "unit": "m"}, "field": "x"}),
        ] {
            let err = registry.validate_params("raster.contour", &bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
        }
    }

    #[test]
    fn raster_contour_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.contour").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
    }

    #[test]
    fn raster_contour_designates_the_raster_by_path() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.contour").unwrap();
        assert_eq!(spec.path_params, vec!["raster".to_string()]);
    }

    fn plan_of(raster: &str) -> crate::plan::Plan {
        serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "terrain.slope", "params": {"dem": "dem/mnt.tif", "unit": "degrees"}},
            {"command": "raster.contour", "params": {
                "raster": raster, "interval": {"value": 5, "unit": "m"}}}
        ]}))
        .unwrap()
    }

    #[test]
    fn raster_contour_refuses_a_step_raster() {
        let registry = Registry::standard();
        let err = registry.check_plan(&plan_of("step:1")).unwrap_err();
        assert_eq!(err.step, 2);
        assert_eq!(err.code, "plan.bad_reference");
        assert!(err.message.contains("raster.contour"));
        assert_eq!(registry.check_plan(&plan_of("dem/mnt.tif")), Ok(()));
    }
}
