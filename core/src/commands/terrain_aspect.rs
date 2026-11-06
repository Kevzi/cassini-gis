//! Commande `terrain.aspect` : exposition de chaque cellule d'un MNT, c'est-à-dire la direction
//! vers laquelle le sol descend, calculée dans le noyau par la fenêtre 3 × 3 de Horn (1981),
//! sans GDAL ni GRASS (AD-1).
//!
//! Provenance : le gradient est celui de `crate::horn` (B. K. P. Horn, « Hill shading and the
//! reflectance map », *Proceedings of the IEEE*, 69(1), 1981, p. 14-47) ; l'exposition est la
//! direction de la plus grande descente, l'opposé du gradient, mesurée depuis le nord dans le
//! sens horaire. Aucune ligne de code de GDAL, de GRASS, de QGIS ni de Whitebox n'est reprise.
//!
//! L'angle est rapporté au **nord du quadrillage** du MNT (l'axe des lignes de la grille), pas
//! au nord vrai : aucune convergence des méridiens n'est appliquée (AD-31). Une cellule plate
//! (gradient nul) n'a pas d'exposition : elle est sans donnée. Aucune reprojection (AD-19) : la
//! sortie garde l'emprise, la cellule et l'EPSG du MNT. Le noyau nomme le fichier de sortie
//! dans le dossier de travail (AD-10).

use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::dem::Dem;
use crate::horn::{horn_gradient, same_grid_as};
use crate::raster_path::RasterPath;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

/// Valeur sans-donnée de l'exposition écrite.
pub const ASPECT_NODATA: f32 = -9999.0;

/// Paramètres de `terrain.aspect`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerrainAspectParams {
    /// Fichier MNT (GeoTIFF à une bande, système projeté en mètres), chemin relatif au projet
    /// recopié de l'inventaire. DEM file path, relative to the project, copied from the inventory.
    pub dem: RasterPath,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `terrain.aspect`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TerrainAspectOutput {
    /// GeoTIFF Float32 de l'exposition écrit, en degrés depuis le nord du quadrillage du MNT,
    /// sens horaire, dans [0, 360) ; sans-donnée -9999.
    pub file: String,
    /// Nombre de cellules plates (gradient nul), mises sans donnée.
    pub flat_cells: u64,
    /// Nombre de cellules sans donnée en tout (bord, voisines d'un trou et cellules plates).
    pub nodata_cells: u64,
}

/// Comptes d'une grille d'exposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AspectStats {
    pub flat_cells: u64,
    pub nodata_cells: u64,
}

/// Exposition d'un gradient de Horn `(dx, dy)` (`dx` vers l'est, `dy` vers le sud), en degrés
/// depuis le nord du quadrillage, sens horaire, dans [0, 360) ; `None` pour un gradient nul.
///
/// En (est, nord), la pente vaut `p = dx`, `q = -dy` ; la descente va vers `(-p, -q)`, dont
/// l'azimut est `atan2(-p, -q) = atan2(-dx, dy)`.
pub fn aspect_of(dx: f64, dy: f64) -> Option<f32> {
    if dx == 0.0 && dy == 0.0 {
        return None;
    }
    // `libm::atan2` plutôt que `f64::atan2`, qui appelle la libm du système : les mêmes octets
    // sur toutes les plateformes, condition du rejeu (AD-11).
    let mut degrees = libm::atan2(-dx, dy).to_degrees();
    if degrees < 0.0 {
        degrees += 360.0;
    }
    // Un angle à peine sous 360° s'arrondit à 360 en f32 ; `+ 0.0` efface un `-0.0`.
    let value = degrees as f32 + 0.0;
    Some(if value >= 360.0 { 0.0 } else { value })
}

/// Exposition de chaque cellule. Une cellule de bord, ou dont l'une des 8 voisines est sans
/// donnée, est sans donnée, comme pour la pente ; une cellule plate aussi. La grille rendue
/// garde l'emprise, la cellule et l'EPSG du MNT.
pub fn horn_aspect(dem: &Dem) -> (Dem, AspectStats) {
    let (w, h) = (dem.width, dem.height);
    let mut values = vec![ASPECT_NODATA; w as usize * h as usize];
    let (mut valid, mut flat) = (0u64, 0u64);
    for row in 1..h.saturating_sub(1) {
        for col in 1..w.saturating_sub(1) {
            let Some((dx, dy)) = horn_gradient(dem, col, row) else {
                continue;
            };
            match aspect_of(dx, dy) {
                Some(a) => {
                    values[row as usize * w as usize + col as usize] = a;
                    valid += 1;
                }
                None => flat += 1,
            }
        }
    }
    let stats = AspectStats {
        flat_cells: flat,
        nodata_cells: u64::from(w) * u64::from(h) - valid,
    };
    (same_grid_as(dem, values, ASPECT_NODATA), stats)
}

/// `terrain.aspect` : commande du noyau.
pub struct TerrainAspect;

impl Command for TerrainAspect {
    type Params = TerrainAspectParams;
    type Output = TerrainAspectOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<TerrainAspectParams, TerrainAspectOutput>(
            "terrain.aspect",
            "Exposition d'un MNT : calcule, pour chaque cellule d'un modèle numérique d'élévation (fichier GeoTIFF désigné par `dem`, chemin de fichier MNT recopié de l'inventaire, pas un id de couche), la direction vers laquelle le sol descend, en degrés depuis le nord du quadrillage, dans le sens horaire (0 nord, 90 est, 180 sud, 270 ouest), et écrit un nouveau GeoTIFF de même emprise. Les cellules plates, celles du bord et les voisines d'un trou sont sans donnée. Ne modifie pas le MNT.",
            "DEM aspect: computes, for every cell of a digital elevation model (GeoTIFF file given by `dem`, a DEM file path copied from the inventory, not a layer id), the direction the ground faces downhill, in degrees from grid north, clockwise (0 north, 90 east, 180 south, 270 west), and writes a new GeoTIFF with the same extent. Flat cells, edge cells and the neighbours of a hole have no data. Does not modify the DEM.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["dem"])
        .output_params(&["out"])
        .output_ext(".tif")
        .example(serde_json::json!({"dem": "mnt-plan-incline-2154.tif"}))
        .example(serde_json::json!({"dem": "mnt-colline-2154.tif"}))
        .tests(&[
            "a_plane_falling_east_faces_ninety_degrees",
            "the_hill_faces_away_from_its_top_and_is_symmetric",
            "a_flat_dem_has_only_flat_cells",
            "a_hole_spreads_to_its_eight_neighbours",
            "a_refused_dem_writes_nothing_and_keeps_the_module_error",
            "an_aspect_is_always_in_one_turn",
            "the_hill_aspect_has_frozen_bytes",
            "two_runs_give_identical_bytes",
            "terrain_aspect_runs_in_the_core_and_the_contract_accepts_it",
        ])
    }

    fn run(&self, params: TerrainAspectParams) -> Result<TerrainAspectOutput, CommandFailure> {
        let dem = Dem::open(Path::new(params.dem.as_str())).map_err(CommandFailure::from)?;
        let (aspect, stats) = horn_aspect(&dem);
        aspect
            .write_geotiff(Path::new(&params.out))
            .map_err(CommandFailure::from)?;
        Ok(TerrainAspectOutput {
            file: params.out,
            flat_cells: stats.flat_cells,
            nodata_cells: stats.nodata_cells,
        })
    }
}

inventory::submit! { Registration(|r| r.register(TerrainAspect)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Registry;
    use serde_json::{json, Value};
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures")
            .join(name)
    }

    /// Dossier temporaire propre à un test, effacé à la fin, y compris s'il échoue.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("cassini-aspect-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }
        fn join(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn aspect(dem: &Path, out: &Path) -> Result<Value, crate::registry::RegistryError> {
        Registry::standard().call(
            "terrain.aspect",
            json!({"dem": dem.to_string_lossy(), "out": out.to_string_lossy()}),
        )
    }

    /// Écart entre deux azimuts, sur le cercle.
    fn circular(a: f64, b: f64) -> f64 {
        let d = (a - b).rem_euclid(360.0);
        d.min(360.0 - d)
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

    #[test]
    fn a_plane_falling_east_faces_ninety_degrees() {
        let dir = Scratch::new("plan");
        let out = dir.join("exposition.tif");
        let source = fixture("mnt-plan-incline-2154.tif");
        let result = aspect(&source, &out).unwrap();
        assert_eq!(result["file"], out.to_string_lossy().as_ref());
        assert_eq!(result["flat_cells"], 0);
        assert_eq!(result["nodata_cells"], 136);

        let grid = Dem::open(&out).unwrap();
        let source = Dem::open(&source).unwrap();
        assert_eq!(grid.extent(), source.extent());
        assert_eq!(
            (grid.width, grid.height, grid.epsg),
            (source.width, source.height, 2154)
        );
        assert_eq!(grid.nodata, Some(-9999.0));
        for row in 0..grid.height {
            for col in 0..grid.width {
                let edge = row == 0 || col == 0 || row == grid.height - 1 || col == grid.width - 1;
                match grid.value(col, row) {
                    None => assert!(edge, "({col}, {row}) sans donnée"),
                    Some(v) => {
                        assert!(!edge, "bord ({col}, {row}) = {v}");
                        assert_eq!(v, 90.0, "({col}, {row})");
                    }
                }
            }
        }
    }

    #[test]
    fn the_hill_faces_away_from_its_top_and_is_symmetric() {
        let dem = Dem::open(&fixture("mnt-colline-2154.tif")).unwrap();
        let (grid, stats) = horn_aspect(&dem);
        let a = |c: u32, r: u32| f64::from(grid.value(c, r).unwrap());
        // Le sommet est au coin commun des cellules (24, 24) à (25, 25). Loin du sommet, sur
        // les deux lignes et les deux colonnes qui l'encadrent, l'exposition est presque celle
        // des quatre points cardinaux.
        for (c, r, expected) in [
            (24, 5, 0.0),
            (25, 5, 0.0),
            (44, 24, 90.0),
            (44, 25, 90.0),
            (24, 44, 180.0),
            (25, 44, 180.0),
            (5, 24, 270.0),
            (5, 25, 270.0),
        ] {
            assert!(circular(a(c, r), expected) < 2.0, "({c}, {r}) = {}", a(c, r));
        }
        // Diagonale nord-est : 45°.
        assert!(circular(a(40, 9), 45.0) < 1e-3, "{}", a(40, 9));
        // Symétrie par miroir : est-ouest (a ↦ 360 - a), nord-sud (a ↦ 180 - a).
        for row in 1..49 {
            for col in 1..49 {
                let v = a(col, row);
                assert!(
                    circular(v, 360.0 - a(49 - col, row)) < 1e-3,
                    "({col}, {row}) miroir est-ouest"
                );
                assert!(
                    circular(v, 180.0 - a(col, 49 - row)) < 1e-3,
                    "({col}, {row}) miroir nord-sud"
                );
                assert!((0.0..360.0).contains(&v), "({col}, {row}) = {v}");
            }
        }
        assert_eq!(stats.flat_cells, 0);
        assert_eq!(stats.nodata_cells, 4 * 49);
    }

    #[test]
    fn a_flat_dem_has_only_flat_cells() {
        let dir = Scratch::new("plat");
        let (input, out) = (dir.join("mnt.tif"), dir.join("exposition.tif"));
        grid(5, 5, vec![42.0; 25]).write_geotiff(&input).unwrap();
        let result = aspect(&input, &out).unwrap();
        assert_eq!(result["flat_cells"], 9);
        assert_eq!(result["nodata_cells"], 25);
        let written = Dem::open(&out).unwrap();
        assert!((0..5).all(|r| (0..5).all(|c| written.value(c, r).is_none())));
    }

    #[test]
    fn a_hole_spreads_to_its_eight_neighbours() {
        // Plan qui descend vers le nord (altitude = 10 × ligne), trou en (3, 3).
        let dir = Scratch::new("trou");
        let (input, out) = (dir.join("mnt.tif"), dir.join("exposition.tif"));
        let mut values: Vec<f32> = (0..49).map(|i| (10 * (i / 7)) as f32).collect();
        values[3 * 7 + 3] = -9999.0;
        grid(7, 7, values).write_geotiff(&input).unwrap();
        let result = aspect(&input, &out).unwrap();
        let written = Dem::open(&out).unwrap();
        for row in 1..6u32 {
            for col in 1..6u32 {
                let near = col.abs_diff(3) <= 1 && row.abs_diff(3) <= 1;
                match written.value(col, row) {
                    None => assert!(near, "({col}, {row}) sans donnée"),
                    Some(v) => {
                        assert!(!near, "({col}, {row}) = {v}");
                        assert_eq!(v, 0.0, "({col}, {row})");
                    }
                }
            }
        }
        assert_eq!(result["nodata_cells"], 24 + 9);
        assert_eq!(result["flat_cells"], 0);
    }

    #[test]
    fn a_refused_dem_writes_nothing_and_keeps_the_module_error() {
        let dir = Scratch::new("refus");
        let out = dir.join("exposition.tif");
        let absent = dir.join("absent.tif");
        let expected = Dem::open(&absent).unwrap_err();
        let err = aspect(&absent, &out).unwrap_err();
        assert_eq!(
            (err.code(), err.to_string()),
            (expected.code, expected.message)
        );
        assert!(!out.exists());

        let text = dir.join("texte.tif");
        std::fs::write(&text, "pas un TIFF").unwrap();
        assert_eq!(aspect(&text, &out).unwrap_err().code(), "dem.unreadable");
        assert!(!out.exists());

        // Un MNT en pseudo-mètres n'est pas projeté au sens du module raster.
        let mercator = dir.join("3857.tif");
        Dem {
            epsg: 3857,
            ..grid(5, 5, vec![1.0; 25])
        }
        .write_geotiff(&mercator)
        .unwrap();
        let expected = Dem::open(&mercator).unwrap_err();
        assert_eq!(expected.code, "dem.not_projected");
        let err = aspect(&mercator, &out).unwrap_err();
        assert_eq!(
            (err.code(), err.to_string()),
            (expected.code, expected.message)
        );
        assert!(!out.exists());
    }

    #[test]
    fn an_aspect_is_always_in_one_turn() {
        assert_eq!(aspect_of(0.0, 0.0), None);
        // Descente vers le nord, l'est, le sud, l'ouest.
        assert_eq!(aspect_of(0.0, 1.0), Some(0.0));
        assert_eq!(aspect_of(-1.0, 0.0), Some(90.0));
        assert_eq!(aspect_of(0.0, -1.0), Some(180.0));
        assert_eq!(aspect_of(1.0, 0.0), Some(270.0));
        // `-0.0` en entrée ne donne jamais `-0` ni 360.
        let north = aspect_of(-0.0, 1.0).unwrap();
        assert!(north == 0.0 && north.is_sign_positive());
        assert_eq!(aspect_of(1e-12, 1.0), Some(0.0));
        assert!(aspect_of(1e-3, 1.0).unwrap() < 360.0);
    }

    /// Empreinte FNV-1a 64 bits des octets d'un fichier.
    fn fnv1a(path: &Path) -> u64 {
        std::fs::read(path).unwrap().iter().fold(0xcbf29ce484222325, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        })
    }

    /// Octets figés de l'exposition de la colline, relevés sous Windows (`cassini.exe`, 2026-11-06)
    /// et vérifiés sous Linux : les fonctions trigonométriques de `libm` donnent les mêmes octets
    /// partout (AD-11). Un échec fait diverger le rejeu des journaux existants.
    #[test]
    fn the_hill_aspect_has_frozen_bytes() {
        let dir = Scratch::new("octets-figes");
        let out = dir.join("exposition.tif");
        aspect(&fixture("mnt-colline-2154.tif"), &out).unwrap();
        assert_eq!(format!("{:016x}", fnv1a(&out)), "e1175c9845044e55");
    }

    #[test]
    fn two_runs_give_identical_bytes() {
        let dir = Scratch::new("deux");
        let (a, b) = (dir.join("a.tif"), dir.join("b.tif"));
        let input = fixture("mnt-colline-2154.tif");
        let ra = aspect(&input, &a).unwrap();
        let rb = aspect(&input, &b).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        assert_eq!(ra["nodata_cells"], rb["nodata_cells"]);
    }

    #[test]
    fn terrain_aspect_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("terrain.aspect").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert_eq!(spec.output_ext, ".tif");
        assert_eq!(spec.path_params, vec!["dem".to_string()]);
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        let violations = registry.check_contract();
        assert!(
            violations.iter().all(|v| v.id != "terrain.aspect"),
            "{violations:?}"
        );
    }
}
