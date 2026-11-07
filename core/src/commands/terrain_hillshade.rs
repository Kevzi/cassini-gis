//! Commande `terrain.hillshade` : ombrage d'un MNT sous un soleil placé à un azimut et une
//! hauteur, calculé dans le noyau par la fenêtre 3 × 3 de Horn (1981), sans GDAL ni GRASS (AD-1).
//!
//! Provenance : le gradient est celui de `crate::horn` ; l'ombrage est la réflectance d'une
//! surface lambertienne, le cosinus de l'angle entre la normale au sol et la direction du
//! soleil, tel que le pose B. K. P. Horn, « Hill shading and the reflectance map »,
//! *Proceedings of the IEEE*, 69(1), 1981, p. 14-47. Aucune ligne de code de GDAL, de GRASS, de
//! QGIS ni de Whitebox n'est reprise.
//!
//! L'azimut du soleil est rapporté au **nord du quadrillage** du MNT (AD-31). La valeur écrite
//! va de 0 (ombre) à 255 (pleine lumière), sans arrondi entier (sortie Float32). Aucune
//! reprojection (AD-19) ; le noyau nomme le fichier de sortie dans le dossier de travail (AD-10).

use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::angle::{Angle, AngleUnit};
use crate::dem::{Dem, NODATA};
use crate::horn::{horn_gradient, same_grid_as};
use crate::raster_path::RasterPath;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

fn default_sun_azimuth() -> Angle {
    Angle {
        value: 315.0,
        unit: AngleUnit::Deg,
    }
}

fn default_sun_altitude() -> Angle {
    Angle {
        value: 45.0,
        unit: AngleUnit::Deg,
    }
}

fn default_z_factor() -> f64 {
    1.0
}

/// Paramètres de `terrain.hillshade`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerrainHillshadeParams {
    /// Fichier MNT (GeoTIFF à une bande, système projeté en mètres), chemin relatif au projet
    /// recopié de l'inventaire. DEM file path, relative to the project, copied from the inventory.
    pub dem: RasterPath,
    /// Direction d'où vient la lumière, depuis le nord du quadrillage du MNT, sens horaire
    /// (315° par défaut, nord-ouest).
    #[serde(default = "default_sun_azimuth")]
    pub sun_azimuth: Angle,
    /// Hauteur du soleil au-dessus de l'horizon, de 0 à 90° (45° par défaut).
    #[serde(default = "default_sun_altitude")]
    pub sun_altitude: Angle,
    /// Exagération verticale, strictement positive (1 par défaut).
    #[serde(default = "default_z_factor")]
    pub z_factor: f64,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `terrain.hillshade`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TerrainHillshadeOutput {
    /// GeoTIFF Float32 de l'ombrage écrit, de 0 (ombre) à 255 (pleine lumière), sans-donnée -9999.
    pub file: String,
    /// Nombre de cellules sans donnée dans la sortie (bord du MNT et voisines d'un trou).
    pub nodata_cells: u64,
}

/// Le soleil, déjà contrôlé : un vecteur unitaire (est, nord, haut) et l'exagération verticale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sun {
    east: f64,
    north: f64,
    up: f64,
    z_factor: f64,
}

/// Refus d'un paramètre de l'ombrage, qui le nomme (AD-21).
fn invalid(name: &str, detail: String) -> CommandFailure {
    CommandFailure::new(
        "terrain.invalid_parameter",
        format!("Paramètre « {name} » invalide : {detail}"),
    )
}

impl Sun {
    /// Contrôle la hauteur et l'exagération, puis pose la direction du soleil.
    pub fn new(azimuth: Angle, altitude: Angle, z_factor: f64) -> Result<Sun, CommandFailure> {
        let h = altitude.degrees();
        if !(0.0..=90.0).contains(&h) {
            return Err(invalid(
                "sun_altitude",
                format!(
                    "la hauteur du soleil doit être comprise entre 0 et 90° (reçu {}{}).",
                    altitude.value,
                    altitude.unit.symbol()
                ),
            ));
        }
        if !(z_factor.is_finite() && z_factor > 0.0) {
            return Err(invalid(
                "z_factor",
                format!("l'exagération verticale doit être strictement positive (reçu {z_factor})."),
            ));
        }
        // `libm` plutôt que les fonctions de la bibliothèque du système : les mêmes octets sur
        // toutes les plateformes, condition du rejeu (AD-11).
        let (a, h) = (azimuth.degrees().to_radians(), h.to_radians());
        let (sin_h, cos_h) = (libm::sin(h), libm::cos(h));
        Ok(Sun {
            east: libm::sin(a) * cos_h,
            north: libm::cos(a) * cos_h,
            up: sin_h,
            z_factor,
        })
    }

    /// Éclairement d'un gradient de Horn `(dx, dy)` (`dx` vers l'est, `dy` vers le sud), de 0 à
    /// 255. En (est, nord), `p = dx`, `q = -dy` ; la normale au sol est `(-z·p, -z·q, 1)`.
    pub fn shade(&self, dx: f64, dy: f64) -> f32 {
        let (nx, ny) = (-self.z_factor * dx, self.z_factor * dy);
        let norm = libm::sqrt(nx * nx + ny * ny + 1.0);
        let cos = (nx * self.east + ny * self.north + self.up) / norm;
        (255.0 * cos.max(0.0)) as f32
    }
}

/// Ombrage de chaque cellule. Une cellule de bord, ou dont l'une des 8 voisines est sans donnée,
/// est sans donnée, comme pour la pente. Rend aussi le nombre de cellules sans donnée.
pub fn horn_hillshade(dem: &Dem, sun: &Sun) -> (Dem, u64) {
    let (w, h) = (dem.width, dem.height);
    let mut values = vec![NODATA; w as usize * h as usize];
    let mut valid = 0u64;
    for row in 1..h.saturating_sub(1) {
        for col in 1..w.saturating_sub(1) {
            if let Some((dx, dy)) = horn_gradient(dem, col, row) {
                values[row as usize * w as usize + col as usize] = sun.shade(dx, dy);
                valid += 1;
            }
        }
    }
    let nodata = u64::from(w) * u64::from(h) - valid;
    (same_grid_as(dem, values, NODATA), nodata)
}

/// `terrain.hillshade` : commande du noyau.
pub struct TerrainHillshade;

impl Command for TerrainHillshade {
    type Params = TerrainHillshadeParams;
    type Output = TerrainHillshadeOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<TerrainHillshadeParams, TerrainHillshadeOutput>(
            "terrain.hillshade",
            "Ombrage d'un MNT : éclaire un modèle numérique d'élévation (fichier GeoTIFF désigné par `dem`, chemin de fichier MNT recopié de l'inventaire, pas un id de couche) par un soleil placé à un azimut (depuis le nord du quadrillage, 315° par défaut) et une hauteur (45° par défaut), et écrit un nouveau GeoTIFF de même emprise, de 0 (ombre) à 255 (pleine lumière). Les cellules du bord et les voisines d'un trou sont sans donnée. Ne modifie pas le MNT.",
            "DEM hillshade: lights a digital elevation model (GeoTIFF file given by `dem`, a DEM file path copied from the inventory, not a layer id) with a sun at an azimuth (from grid north, 315° by default) and an altitude (45° by default), and writes a new GeoTIFF with the same extent, from 0 (shadow) to 255 (full light). Edge cells and the neighbours of a hole have no data. Does not modify the DEM.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["dem"])
        .output_params(&["out"])
        .output_ext(".tif")
        .example(serde_json::json!({"dem": "mnt-colline-2154.tif"}))
        .example(serde_json::json!({
            "dem": "mnt-plan-incline-2154.tif",
            "sun_azimuth": {"value": 90, "unit": "deg"},
            "sun_altitude": {"value": 30, "unit": "deg"},
            "z_factor": 2
        }))
        .tests(&[
            "the_default_shade_of_a_plane_is_the_analytic_value",
            "a_sun_facing_the_slope_is_brighter_and_a_flat_dem_gets_the_sine",
            "an_azimuth_in_gon_gives_the_same_bytes",
            "a_z_factor_doubles_the_slope",
            "an_invalid_altitude_or_z_factor_is_named_and_nothing_is_written",
            "a_hole_spreads_to_its_eight_neighbours",
            "a_refused_dem_writes_nothing_and_keeps_the_module_error",
            "the_hill_shade_has_frozen_bytes",
            "two_runs_give_identical_bytes",
            "terrain_hillshade_runs_in_the_core_and_the_contract_accepts_it",
        ])
    }

    fn run(&self, params: TerrainHillshadeParams) -> Result<TerrainHillshadeOutput, CommandFailure> {
        let sun = Sun::new(params.sun_azimuth, params.sun_altitude, params.z_factor)?;
        let dem = Dem::open(Path::new(params.dem.as_str())).map_err(CommandFailure::from)?;
        let (shade, nodata_cells) = horn_hillshade(&dem, &sun);
        shade
            .write_geotiff(Path::new(&params.out))
            .map_err(CommandFailure::from)?;
        Ok(TerrainHillshadeOutput {
            file: params.out,
            nodata_cells,
        })
    }
}

inventory::submit! { Registration(|r| r.register(TerrainHillshade)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::{fixture, Scratch};
    use crate::registry::Registry;
    use serde_json::{json, Value};

    /// Lance `terrain.hillshade` avec `extra` ajouté aux paramètres.
    fn hillshade(
        dem: &Path,
        out: &Path,
        extra: Value,
    ) -> Result<Value, crate::registry::RegistryError> {
        let mut params = json!({"dem": dem.to_string_lossy(), "out": out.to_string_lossy()});
        for (k, v) in extra.as_object().unwrap() {
            params[k] = v.clone();
        }
        Registry::standard().call("terrain.hillshade", params)
    }

    /// MNT `width` × `height` de cellules de 10 m, en Lambert-93.
    fn grid(width: u32, height: u32, values: Vec<f32>) -> Dem {
        Dem {
            width,
            height,
            origin_x: 652000.0,
            origin_y: 6850070.0,
            cell_size_x: 10.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values,
        }
    }

    /// Ombrage, calculé à la main, d'un plan qui descend vers l'est de `p` m par m, sous un
    /// soleil à l'azimut `a` et à la hauteur `h` (degrés) : normale `(p, 0, 1) / √(1 + p²)`.
    fn plane_shade(p: f64, a: f64, h: f64) -> f64 {
        let (a, h) = (a.to_radians(), h.to_radians());
        let sun = [a.sin() * h.cos(), a.cos() * h.cos(), h.sin()];
        let dot = (p * sun[0] + sun[2]) / (1.0 + p * p).sqrt();
        255.0 * dot.max(0.0)
    }

    /// Vérifie que toute cellule intérieure vaut `expected` et que le bord est sans donnée.
    fn assert_uniform_interior(grid: &Dem, expected: f64) {
        for row in 0..grid.height {
            for col in 0..grid.width {
                let edge = row == 0 || col == 0 || row == grid.height - 1 || col == grid.width - 1;
                match grid.value(col, row) {
                    None => assert!(edge, "({col}, {row}) sans donnée"),
                    Some(v) => {
                        assert!(!edge, "bord ({col}, {row}) = {v}");
                        assert!((f64::from(v) - expected).abs() < 1e-3, "({col}, {row}) = {v}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_default_shade_of_a_plane_is_the_analytic_value() {
        let dir = Scratch::new("defaut");
        let out = dir.join("ombrage.tif");
        let source = fixture("mnt-plan-incline-2154.tif");
        let result = hillshade(&source, &out, json!({})).unwrap();
        assert_eq!(result["file"], out.to_string_lossy().as_ref());
        assert_eq!(result["nodata_cells"], 136);
        // Soleil 315°, 45° : (−½, ½, √2/2) ; normale (0,1 ; 0 ; 1) / √1,01.
        let expected = 255.0 * (-0.05 + 0.5f64.sqrt()) / 1.01f64.sqrt();
        assert!((expected - plane_shade(0.1, 315.0, 45.0)).abs() < 1e-9);
        assert!((expected - 166.7306).abs() < 1e-3, "{expected}");
        let grid = Dem::open(&out).unwrap();
        let source = Dem::open(&source).unwrap();
        assert_eq!(grid.extent(), source.extent());
        assert_eq!((grid.epsg, grid.nodata), (2154, Some(-9999.0)));
        assert_uniform_interior(&grid, expected);
    }

    #[test]
    fn a_sun_facing_the_slope_is_brighter_and_a_flat_dem_gets_the_sine() {
        let dir = Scratch::new("soleil");
        let (east, west) = (dir.join("est.tif"), dir.join("ouest.tif"));
        let plane = fixture("mnt-plan-incline-2154.tif");
        hillshade(&plane, &east, json!({"sun_azimuth": {"value": 90}})).unwrap();
        hillshade(&plane, &west, json!({"sun_azimuth": {"value": 270}})).unwrap();
        let (east, west) = (Dem::open(&east).unwrap(), Dem::open(&west).unwrap());
        let (e, w) = (east.value(5, 5).unwrap(), west.value(5, 5).unwrap());
        assert!(e > w, "{e} ≤ {w}");
        assert_uniform_interior(&east, plane_shade(0.1, 90.0, 45.0));
        assert_uniform_interior(&west, plane_shade(0.1, 270.0, 45.0));

        let (flat, out) = (dir.join("plat.tif"), dir.join("plat-ombrage.tif"));
        grid(5, 5, vec![7.0; 25]).write_geotiff(&flat).unwrap();
        let result = hillshade(&flat, &out, json!({"sun_altitude": {"value": 30}})).unwrap();
        assert_eq!(result["nodata_cells"], 16);
        assert_uniform_interior(&Dem::open(&out).unwrap(), 255.0 * 0.5);
        // Soleil à l'horizon : un sol plat est dans l'ombre ; au zénith, en pleine lumière.
        hillshade(&flat, &out, json!({"sun_altitude": {"value": 0}})).unwrap();
        assert_uniform_interior(&Dem::open(&out).unwrap(), 0.0);
        hillshade(&flat, &out, json!({"sun_altitude": {"value": 100, "unit": "gon"}})).unwrap();
        assert_uniform_interior(&Dem::open(&out).unwrap(), 255.0);
    }

    #[test]
    fn an_azimuth_in_gon_gives_the_same_bytes() {
        let dir = Scratch::new("grades");
        let (deg, gon) = (dir.join("deg.tif"), dir.join("gon.tif"));
        let hill = fixture("mnt-colline-2154.tif");
        hillshade(&hill, &deg, json!({"sun_azimuth": {"value": 90, "unit": "deg"}})).unwrap();
        hillshade(&hill, &gon, json!({"sun_azimuth": {"value": 100, "unit": "gon"}})).unwrap();
        assert_eq!(std::fs::read(&deg).unwrap(), std::fs::read(&gon).unwrap());
    }

    #[test]
    fn a_z_factor_doubles_the_slope() {
        let dir = Scratch::new("facteur-z");
        let (doubled, steeper) = (dir.join("z2.tif"), dir.join("raide.tif"));
        let plane = fixture("mnt-plan-incline-2154.tif");
        hillshade(&plane, &doubled, json!({"z_factor": 2})).unwrap();
        // Le même plan, à pente doublée : 20 %.
        let source = Dem::open(&plane).unwrap();
        let input = dir.join("plan-20.tif");
        Dem {
            values: source.values.iter().map(|z| 2.0 * z).collect(),
            ..source
        }
        .write_geotiff(&input)
        .unwrap();
        hillshade(&input, &steeper, json!({})).unwrap();
        let (doubled, steeper) = (Dem::open(&doubled).unwrap(), Dem::open(&steeper).unwrap());
        assert_eq!(doubled.values, steeper.values);
        assert_uniform_interior(&doubled, plane_shade(0.2, 315.0, 45.0));
    }

    #[test]
    fn an_invalid_altitude_or_z_factor_is_named_and_nothing_is_written() {
        let dir = Scratch::new("refus-parametre");
        let out = dir.join("ombrage.tif");
        let plane = fixture("mnt-plan-incline-2154.tif");
        for (extra, name) in [
            (json!({"sun_altitude": {"value": 95}}), "sun_altitude"),
            (json!({"sun_altitude": {"value": -1}}), "sun_altitude"),
            (json!({"sun_altitude": {"value": 101, "unit": "gon"}}), "sun_altitude"),
            (json!({"z_factor": 0}), "z_factor"),
            (json!({"z_factor": -1}), "z_factor"),
        ] {
            let err = hillshade(&plane, &out, extra.clone()).unwrap_err();
            assert_eq!(err.code(), "terrain.invalid_parameter", "{extra}");
            assert!(err.to_string().contains(name), "{extra} : {err}");
            assert!(!out.exists(), "{extra}");
        }
        // Le refus précède la lecture du MNT : un fichier absent n'est pas lu.
        let err = hillshade(&dir.join("absent.tif"), &out, json!({"z_factor": 0})).unwrap_err();
        assert_eq!(err.code(), "terrain.invalid_parameter");
        assert!(Sun::new(default_sun_azimuth(), default_sun_altitude(), f64::NAN).is_err());
        assert!(Sun::new(default_sun_azimuth(), default_sun_altitude(), f64::INFINITY).is_err());
    }

    #[test]
    fn a_hole_spreads_to_its_eight_neighbours() {
        let dir = Scratch::new("trou");
        let (input, out) = (dir.join("mnt.tif"), dir.join("ombrage.tif"));
        let mut values = vec![100.0f32; 49];
        values[3 * 7 + 3] = -9999.0;
        grid(7, 7, values).write_geotiff(&input).unwrap();
        let result = hillshade(&input, &out, json!({})).unwrap();
        let written = Dem::open(&out).unwrap();
        for row in 1..6u32 {
            for col in 1..6u32 {
                let near = col.abs_diff(3) <= 1 && row.abs_diff(3) <= 1;
                assert_eq!(written.value(col, row).is_none(), near, "({col}, {row})");
            }
        }
        assert_eq!(result["nodata_cells"], 24 + 9);
    }

    #[test]
    fn a_refused_dem_writes_nothing_and_keeps_the_module_error() {
        let dir = Scratch::new("refus");
        let out = dir.join("ombrage.tif");
        let absent = dir.join("absent.tif");
        let expected = Dem::open(&absent).unwrap_err();
        let err = hillshade(&absent, &out, json!({})).unwrap_err();
        assert_eq!(
            (err.code(), err.to_string()),
            (expected.code, expected.message)
        );
        assert!(!out.exists());

        let text = dir.join("texte.tif");
        std::fs::write(&text, "pas un TIFF").unwrap();
        let err = hillshade(&text, &out, json!({})).unwrap_err();
        assert_eq!(err.code(), "dem.unreadable");
        assert!(!out.exists());

        let mercator = dir.join("3857.tif");
        Dem {
            epsg: 3857,
            ..grid(5, 5, vec![1.0; 25])
        }
        .write_geotiff(&mercator)
        .unwrap();
        let expected = Dem::open(&mercator).unwrap_err();
        assert_eq!(expected.code, "dem.not_projected");
        let err = hillshade(&mercator, &out, json!({})).unwrap_err();
        assert_eq!(
            (err.code(), err.to_string()),
            (expected.code, expected.message)
        );
        assert!(!out.exists());
    }

    /// Empreinte FNV-1a 64 bits des octets d'un fichier.
    fn fnv1a(path: &Path) -> u64 {
        std::fs::read(path).unwrap().iter().fold(0xcbf29ce484222325, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        })
    }

    /// Octets figés de l'ombrage de la colline, relevés sous Windows (`cassini.exe`, 2026-11-06)
    /// et vérifiés sous Linux : les fonctions trigonométriques de `libm` donnent les mêmes octets
    /// partout (AD-11). Un échec fait diverger le rejeu des journaux existants.
    #[test]
    fn the_hill_shade_has_frozen_bytes() {
        let dir = Scratch::new("octets-figes");
        let out = dir.join("ombrage.tif");
        hillshade(&fixture("mnt-colline-2154.tif"), &out, json!({})).unwrap();
        assert_eq!(format!("{:016x}", fnv1a(&out)), "0c5233717ef348de");
    }

    #[test]
    fn two_runs_give_identical_bytes() {
        let dir = Scratch::new("deux");
        let (a, b) = (dir.join("a.tif"), dir.join("b.tif"));
        let input = fixture("mnt-colline-2154.tif");
        hillshade(&input, &a, json!({})).unwrap();
        hillshade(&input, &b, json!({})).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    }

    #[test]
    fn terrain_hillshade_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("terrain.hillshade").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert_eq!(spec.output_ext, ".tif");
        assert_eq!(spec.path_params, vec!["dem".to_string()]);
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        let violations = registry.check_contract();
        assert!(
            violations.iter().all(|v| v.id != "terrain.hillshade"),
            "{violations:?}"
        );
    }
}
