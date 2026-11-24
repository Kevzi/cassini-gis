//! Commande `raster.align` : réécrit un raster sur la grille d'un raster de référence (même
//! système, même taille de cellule, même origine, mêmes dimensions), par QGIS (epic 10, entrée
//! 38). Déclaration seule : le geste vit dans l'extension (AD-22, AD-24 : aucun nom d'algorithme
//! ici). Les deux rasters se désignent par un `RasterPath` (AD-24) : chemin relatif au projet, ou
//! `step:k`. Le noyau nomme le fichier de sortie (AD-10), un `.tif`, et dicte ensuite
//! `layer.open_raster` sur lui, comme après `data.convert_raster` : un plan peut enchaîner
//! l'alignement et une commande de terrain par `step:k`. Un seul raster par commande.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::raster_path::RasterPath;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Méthode de rééchantillonnage : liste fermée.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlignResampling {
    /// Valeur de la cellule la plus proche : garde les valeurs de la source (classes, codes).
    #[default]
    Nearest,
    /// Interpolation bilinéaire : valeurs continues (altitudes).
    Bilinear,
}

/// Paramètres de `raster.align`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RasterAlignParams {
    /// Raster à aligner (fichier GeoTIFF d'une bande, avec système), chemin copié de
    /// l'inventaire, ou `step:k`. Il n'est pas modifié.
    pub raster: RasterPath,
    /// Raster de référence (GeoTIFF d'une bande, avec système) dont la sortie prend la grille :
    /// système, taille de cellule, origine, dimensions. Chemin ou `step:k`. Pas modifié.
    pub reference: RasterPath,
    /// Méthode de rééchantillonnage : `nearest` (défaut, garde les valeurs, pour des classes)
    /// ou `bilinear` (interpole, pour des altitudes).
    #[serde(default)]
    pub resampling: Option<AlignResampling>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `raster.align`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RasterAlignOutput {
    /// GeoTIFF écrit, sur la grille de la référence.
    pub file: String,
}

/// `raster.align` : commande déléguée à QGIS.
pub struct RasterAlign;

impl Delegated for RasterAlign {
    type Params = RasterAlignParams;
    type Output = RasterAlignOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<RasterAlignParams, RasterAlignOutput>(
            "raster.align",
            "Aligne un raster (`raster`, fichier GeoTIFF d'une bande, chemin recopié de l'inventaire ou `step:k`) sur la grille d'un raster de référence (`reference`, par exemple le MNT) : la sortie a le système, la taille de cellule, l'origine et les dimensions de la référence, pour que les deux se combinent. `resampling` : `nearest` (défaut, garde les valeurs, pour des classes) ou `bilinear` (interpole, pour des altitudes). Écrit un nouveau GeoTIFF, ouvert dans le projet ; les originaux ne sont pas modifiés. Un seul raster par commande.",
            "Aligns a raster (`raster`, a one-band GeoTIFF file, a path copied from the inventory or `step:k`) onto the grid of a reference raster (`reference`, for example the DEM): the output has the reference's system, cell size, origin and dimensions, so that the two can be combined. `resampling`: `nearest` (default, keeps the values, for classes) or `bilinear` (interpolates, for elevations). Writes a new GeoTIFF, opened in the project; the originals are not modified. One raster per command.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .path_params(&["raster", "reference"])
        .output_params(&["out"])
        .output_ext(".tif")
        .example(serde_json::json!({"raster": "donnees/occupation.tif", "reference": "dem/mnt.tif"}))
        .example(serde_json::json!({"raster": "donnees/imagerie.tif", "reference": "dem/mnt.tif", "resampling": "bilinear"}))
        .example(serde_json::json!({"raster": "step:1", "reference": "dem/mnt.tif"}))
        .tests(&[
            "raster_align_is_delegated",
            "raster_align_params_are_two_rasters_and_a_closed_method",
            "raster_align_output_is_a_tif_named_by_the_core",
            "raster_align_designates_both_rasters_by_path",
            "raster_align_accepts_step_rasters",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<RasterAlign>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn raster_align_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.align").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        assert!(!spec.dictated_by_core);
        let err = registry
            .call(
                "raster.align",
                json!({"raster": "a.tif", "reference": "mnt.tif"}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn raster_align_params_are_two_rasters_and_a_closed_method() {
        let registry = Registry::standard();
        for ok in [
            json!({"raster": "a.tif", "reference": "mnt.tif"}),
            json!({"raster": "a.tif", "reference": "mnt.tif", "resampling": "nearest"}),
            json!({"raster": "a.tif", "reference": "mnt.tif", "resampling": "bilinear"}),
            json!({"raster": "a.tif", "reference": "mnt.tif", "out": "1-raster.align.tif"}),
        ] {
            assert!(registry.validate_params("raster.align", &ok).is_ok(), "{ok}");
        }
        for bad in [
            json!({"raster": "a.tif"}),
            json!({"reference": "mnt.tif"}),
            json!({}),
            json!({"raster": "a.tif", "reference": "mnt.tif", "resampling": "cubic"}),
            json!({"raster": "a.tif", "reference": "mnt.tif", "resampling": "bicubic"}),
            json!({"raster": "a.tif", "reference": "mnt.tif", "resampling": 1}),
            json!({"raster": "a.tif", "reference": "mnt.tif", "band": 2}),
            json!({"raster": "a.tif", "reference": 12}),
        ] {
            let err = registry.validate_params("raster.align", &bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn raster_align_output_is_a_tif_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.align").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".tif");
        assert!(crate::raster_path::writes_one_raster(spec));
    }

    #[test]
    fn raster_align_designates_both_rasters_by_path() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.align").unwrap();
        assert_eq!(
            spec.path_params,
            vec!["raster".to_string(), "reference".to_string()]
        );
    }

    #[test]
    fn raster_align_accepts_step_rasters() {
        let registry = Registry::standard();
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "raster.align", "params": {"raster": "donnees/occupation.tif",
                "reference": "dem/mnt.tif"}},
            {"command": "terrain.slope", "params": {"dem": "step:1", "unit": "degrees"}}
        ]}))
        .unwrap();
        assert_eq!(registry.check_plan(&plan), Ok(()));
        // Une étape en entrée, une autre en référence.
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "data.convert_raster", "params": {"raster": "mnt/dalle.asc",
                "assign_epsg": 2154}},
            {"command": "raster.align", "params": {"raster": "donnees/occupation.tif",
                "reference": "step:1", "resampling": "bilinear"}}
        ]}))
        .unwrap();
        assert_eq!(registry.check_plan(&plan), Ok(()));
    }
}
