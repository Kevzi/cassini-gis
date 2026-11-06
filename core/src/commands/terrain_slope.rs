//! Commande `terrain.slope` : pente d'un MNT, en degrés ou en pourcentage, calculée dans le
//! noyau par la méthode de Horn (1981), sans GDAL ni GRASS (AD-1).
//!
//! Provenance : les différences finies pondérées sur la fenêtre 3 × 3 (`crate::horn`) sont
//! celles de B. K. P. Horn, « Hill shading and the reflectance map », *Proceedings of the
//! IEEE*, 69(1), 1981, p. 14-47, écrites d'après la publication seule. Aucune ligne de code de GDAL, de
//! GRASS, de QGIS ni de Whitebox n'est reprise. Résultat comparé à `gdaldem slope -alg Horn`
//! (ticket 4.2) et à `native:slope` de QGIS (`docs/comparaison-terrain-qgis.md`).
//!
//! Le MNT est ouvert et la pente écrite par `dem.rs`, l'unique porte raster du noyau (AD-20).
//! Aucune reprojection (AD-19) : la sortie garde l'emprise, la cellule et l'EPSG du MNT. Le
//! noyau nomme le fichier de sortie dans le dossier de travail (AD-10).

use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::dem::Dem;
use crate::horn::{horn_gradient, same_grid_as};
use crate::raster_path::RasterPath;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

/// Valeur sans-donnée de la pente écrite.
pub const SLOPE_NODATA: f32 = -9999.0;

/// Unité de la pente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SlopeUnit {
    /// Degrés, de 0 (plat) à 90 (vertical).
    Degrees,
    /// Pourcentage : 100 × dénivelé / distance horizontale (45° = 100 %).
    Percent,
}

/// Paramètres de `terrain.slope`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerrainSlopeParams {
    /// Fichier MNT (GeoTIFF à une bande, système projeté en mètres), chemin relatif au projet
    /// recopié de l'inventaire. DEM file path, relative to the project, copied from the inventory.
    pub dem: RasterPath,
    /// Unité de la pente : `degrees` ou `percent`.
    pub unit: SlopeUnit,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `terrain.slope`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TerrainSlopeOutput {
    /// GeoTIFF Float32 de la pente écrit, sans-donnée -9999.
    pub file: String,
    /// Unité de la pente écrite.
    pub unit: SlopeUnit,
    /// Pente minimale sur les cellules valides : absente si aucune ne l'est.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Pente maximale sur les cellules valides : absente si aucune ne l'est.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Nombre de cellules sans donnée dans la sortie (bord du MNT et voisines d'un trou compris).
    pub nodata_cells: u64,
}

/// Statistiques d'une grille de pente.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlopeStats {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub nodata_cells: u64,
}

/// Pente de Horn de chaque cellule. Une cellule de bord, ou dont l'une des 8 voisines est sans
/// donnée, est sans donnée (comportement par défaut de GDAL et de GRASS). Calcul en f64, rendu
/// en f32 ; la grille rendue garde l'emprise, la cellule et l'EPSG du MNT.
pub fn horn_slope(dem: &Dem, unit: SlopeUnit) -> (Dem, SlopeStats) {
    let (w, h) = (dem.width, dem.height);
    let mut values = vec![SLOPE_NODATA; w as usize * h as usize];
    let mut min: Option<f64> = None;
    let mut max: Option<f64> = None;
    let mut valid = 0u64;
    for row in 1..h.saturating_sub(1) {
        for col in 1..w.saturating_sub(1) {
            let Some((dx, dy)) = horn_gradient(dem, col, row) else {
                continue;
            };
            let p = (dx * dx + dy * dy).sqrt();
            let slope = match unit {
                SlopeUnit::Percent => 100.0 * p,
                // `libm::atan` plutôt que `f64::atan`, qui appelle la libm du système : les
                // mêmes octets sur toutes les plateformes, condition du rejeu (AD-11).
                SlopeUnit::Degrees => libm::atan(p).to_degrees(),
            } as f32;
            values[row as usize * w as usize + col as usize] = slope;
            let s = f64::from(slope);
            min = Some(min.map_or(s, |m| m.min(s)));
            max = Some(max.map_or(s, |m| m.max(s)));
            valid += 1;
        }
    }
    let out = same_grid_as(dem, values, SLOPE_NODATA);
    let stats = SlopeStats {
        min,
        max,
        nodata_cells: u64::from(w) * u64::from(h) - valid,
    };
    (out, stats)
}

/// `terrain.slope` : commande du noyau.
pub struct TerrainSlope;

impl Command for TerrainSlope {
    type Params = TerrainSlopeParams;
    type Output = TerrainSlopeOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<TerrainSlopeParams, TerrainSlopeOutput>(
            "terrain.slope",
            "Pente d'un MNT : calcule la pente de chaque cellule d'un modèle numérique d'élévation (fichier GeoTIFF désigné par `dem`, chemin de fichier MNT recopié de l'inventaire, pas un id de couche), en degrés ou en pourcentage, et écrit un nouveau GeoTIFF de même emprise. Les cellules du bord et les voisines d'un trou sont sans donnée. Ne modifie pas le MNT.",
            "DEM slope: computes the slope of every cell of a digital elevation model (GeoTIFF file given by `dem`, a DEM file path copied from the inventory, not a layer id), in degrees or in percent, and writes a new GeoTIFF with the same extent. Edge cells and the neighbours of a hole have no data. Does not modify the DEM.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["dem"])
        .output_params(&["out"])
        .output_ext(".tif")
        .example(serde_json::json!({"dem": "mnt-plan-incline-2154.tif", "unit": "degrees"}))
        .example(serde_json::json!({"dem": "mnt-colline-2154.tif", "unit": "percent"}))
        .tests(&[
            "a_tilted_plane_has_a_uniform_slope_in_degrees",
            "a_tilted_plane_has_a_ten_percent_slope",
            "the_hill_is_flat_at_the_top_and_symmetric",
            "a_hole_spreads_to_its_eight_neighbours",
            "a_tiny_dem_has_no_valid_cell",
            "a_refused_dem_writes_nothing_and_keeps_the_module_error",
            "an_unknown_unit_is_invalid",
            "two_runs_give_identical_bytes",
            "terrain_slope_runs_in_the_core_and_the_contract_accepts_it",
        ])
    }

    fn run(&self, params: TerrainSlopeParams) -> Result<TerrainSlopeOutput, CommandFailure> {
        let dem = Dem::open(Path::new(params.dem.as_str())).map_err(CommandFailure::from)?;
        let (slope, stats) = horn_slope(&dem, params.unit);
        slope
            .write_geotiff(Path::new(&params.out))
            .map_err(CommandFailure::from)?;
        Ok(TerrainSlopeOutput {
            file: params.out,
            unit: params.unit,
            min: stats.min,
            max: stats.max,
            nodata_cells: stats.nodata_cells,
        })
    }
}

inventory::submit! { Registration(|r| r.register(TerrainSlope)) }

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
                std::env::temp_dir().join(format!("cassini-slope-{}-{name}", std::process::id()));
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

    fn slope(dem: &Path, unit: &str, out: &Path) -> Result<Value, crate::registry::RegistryError> {
        Registry::standard().call(
            "terrain.slope",
            json!({"dem": dem.to_string_lossy(), "unit": unit, "out": out.to_string_lossy()}),
        )
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
                        assert!((f64::from(v) - expected).abs() < 1e-4, "({col}, {row}) = {v}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_tilted_plane_has_a_uniform_slope_in_degrees() {
        let dir = Scratch::new("degres");
        let out = dir.join("pente.tif");
        let result = slope(&fixture("mnt-plan-incline-2154.tif"), "degrees", &out).unwrap();
        let expected = 0.1f64.atan().to_degrees();
        assert!((expected - 5.7106).abs() < 1e-4);
        assert_eq!(result["unit"], "degrees");
        assert_eq!(result["nodata_cells"], 136);
        assert_eq!(result["min"], result["max"]);
        assert!((result["min"].as_f64().unwrap() - expected).abs() < 1e-4);
        assert_eq!(result["file"], out.to_string_lossy().as_ref());

        let grid = Dem::open(&out).unwrap();
        let source = Dem::open(&fixture("mnt-plan-incline-2154.tif")).unwrap();
        assert_eq!(grid.extent(), source.extent());
        assert_eq!(
            (grid.width, grid.height, grid.epsg),
            (source.width, source.height, 2154)
        );
        assert_eq!(
            (grid.cell_size_x, grid.cell_size_y),
            (source.cell_size_x, source.cell_size_y)
        );
        assert_eq!(grid.nodata, Some(-9999.0));
        assert_uniform_interior(&grid, expected);
    }

    #[test]
    fn a_tilted_plane_has_a_ten_percent_slope() {
        let dir = Scratch::new("pourcent");
        let out = dir.join("pente.tif");
        let result = slope(&fixture("mnt-plan-incline-2154.tif"), "percent", &out).unwrap();
        assert_eq!(result["unit"], "percent");
        assert!((result["max"].as_f64().unwrap() - 10.0).abs() < 1e-4);
        assert_uniform_interior(&Dem::open(&out).unwrap(), 10.0);
    }

    #[test]
    fn the_hill_is_flat_at_the_top_and_symmetric() {
        let dem = Dem::open(&fixture("mnt-colline-2154.tif")).unwrap();
        let (grid, stats) = horn_slope(&dem, SlopeUnit::Degrees);
        let s = |c: u32, r: u32| f64::from(grid.value(c, r).unwrap());
        // Le sommet de la fixture tombe au coin commun des cellules (24, 24) à (25, 25) : aucune
        // fenêtre 3 × 3 n'y est centrée, la pente y est la plus faible du MNT (3,5°) sans être
        // nulle. La pente nulle se vérifie plus bas, sur une colline centrée sur une cellule.
        let top = s(24, 24);
        assert_eq!(stats.min, Some(top));
        for (c, r) in [(25, 24), (24, 25), (25, 25)] {
            assert!((s(c, r) - top).abs() < 1e-3, "({c}, {r})");
        }
        // Symétrie par les deux axes et la diagonale du centre.
        for row in 1..49 {
            for col in 1..49 {
                let v = s(col, row);
                assert!((v - s(49 - col, row)).abs() < 1e-3, "({col}, {row}) miroir est-ouest");
                assert!((v - s(col, 49 - row)).abs() < 1e-3, "({col}, {row}) miroir nord-sud");
                assert!((v - s(row, col)).abs() < 1e-3, "({col}, {row}) diagonale");
            }
        }
        // La pente monte puis redescend le long d'un rayon : maximum sur un anneau.
        let radial: Vec<f64> = (25..49).map(|c| s(c, 25)).collect();
        let peak = radial
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap();
        assert!(peak > 2 && peak < radial.len() - 2, "maximum au rang {peak}");
        assert!(radial[0] < radial[peak] && radial[radial.len() - 1] < radial[peak]);
        assert!((stats.max.unwrap() - radial[peak]).abs() < 0.5);
        assert_eq!(stats.nodata_cells, 4 * 49);

        // Même colline sur 51 × 51 cellules : le sommet tombe au centre de la cellule (25, 25),
        // où la pente est nulle.
        let mut values = Vec::with_capacity(51 * 51);
        for row in 0..51u32 {
            for col in 0..51u32 {
                let dx = (f64::from(col) - 25.0) * 10.0;
                let dy = (f64::from(row) - 25.0) * 10.0;
                values.push((50.0 + 100.0 / (1.0 + (dx * dx + dy * dy) / 22500.0)) as f32);
            }
        }
        let centred = Dem {
            width: 51,
            height: 51,
            values,
            ..dem
        };
        let (grid, stats) = horn_slope(&centred, SlopeUnit::Degrees);
        let top = f64::from(grid.value(25, 25).unwrap());
        assert!(top.abs() < 1e-3, "sommet {top}");
        assert_eq!(stats.min, Some(top));
    }

    /// MNT plat 7 × 7 de cellules de 10 m, avec une sans-donnée en (3, 3).
    fn holed() -> Dem {
        let mut values = vec![100.0f32; 49];
        values[3 * 7 + 3] = -9999.0;
        Dem {
            width: 7,
            height: 7,
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
    fn a_hole_spreads_to_its_eight_neighbours() {
        let dir = Scratch::new("trou");
        let (input, out) = (dir.join("mnt.tif"), dir.join("pente.tif"));
        holed().write_geotiff(&input).unwrap();
        let result = slope(&input, "percent", &out).unwrap();
        let grid = Dem::open(&out).unwrap();
        for row in 1..6u32 {
            for col in 1..6u32 {
                let near = col.abs_diff(3) <= 1 && row.abs_diff(3) <= 1;
                assert_eq!(grid.value(col, row).is_none(), near, "({col}, {row})");
            }
        }
        // Bord (24) et le trou avec ses 8 voisines (9).
        assert_eq!(result["nodata_cells"], 24 + 9);
        assert_eq!(result["min"], 0.0);
        assert_eq!(result["max"], 0.0);
    }

    #[test]
    fn a_tiny_dem_has_no_valid_cell() {
        let dir = Scratch::new("minuscule");
        let (input, out) = (dir.join("mnt.tif"), dir.join("pente.tif"));
        Dem {
            width: 2,
            height: 2,
            values: vec![1.0, 2.0, 3.0, 4.0],
            ..holed()
        }
        .write_geotiff(&input)
        .unwrap();
        let result = slope(&input, "degrees", &out).unwrap();
        assert_eq!(result["nodata_cells"], 4);
        assert!(result.get("min").is_none() && result.get("max").is_none());
        let grid = Dem::open(&out).unwrap();
        assert!((0..2).all(|r| (0..2).all(|c| grid.value(c, r).is_none())));
    }

    #[test]
    fn a_refused_dem_writes_nothing_and_keeps_the_module_error() {
        let dir = Scratch::new("refus");
        let out = dir.join("pente.tif");

        let absent = dir.join("absent.tif");
        let expected = Dem::open(&absent).unwrap_err();
        let err = slope(&absent, "degrees", &out).unwrap_err();
        assert_eq!(err.code(), "dem.unreadable");
        assert_eq!(err.to_string(), expected.message);
        assert!(!out.exists());

        let text = dir.join("texte.tif");
        std::fs::write(&text, "pas un TIFF").unwrap();
        let err = slope(&text, "degrees", &out).unwrap_err();
        assert_eq!(err.code(), "dem.unreadable");
        assert!(!out.exists());

        // Un MNT sans EPSG : `write_geotiff` refuse de l'écrire, on prend donc un TIFF écrit
        // en EPSG:2154 dont on change la clé en 32767 (défini par l'utilisateur).
        let noepsg = dir.join("sans-epsg.tif");
        holed().write_geotiff(&noepsg).unwrap();
        patch_key(&noepsg, 2154, 32767);
        let expected = Dem::open(&noepsg).unwrap_err();
        assert_eq!(expected.code, "dem.no_epsg");
        let err = slope(&noepsg, "degrees", &out).unwrap_err();
        assert_eq!(
            (err.code(), err.to_string()),
            (expected.code, expected.message)
        );
        assert!(!out.exists());

        // Un MNT en degrés : type de modèle géographique (2) et clé géographique 4326.
        let wgs84 = dir.join("wgs84.tif");
        holed().write_geotiff(&wgs84).unwrap();
        patch_model_to_geographic(&wgs84);
        let expected = Dem::open(&wgs84).unwrap_err();
        assert_eq!(expected.code, "dem.not_projected");
        let err = slope(&wgs84, "degrees", &out).unwrap_err();
        assert_eq!(
            (err.code(), err.to_string()),
            (expected.code, expected.message)
        );
        assert!(!out.exists());
    }

    /// Change la valeur de la clé `ProjectedCSTypeGeoKey` (3072) dans le répertoire de clés.
    fn patch_key(path: &Path, from: u16, to: u16) {
        let mut bytes = std::fs::read(path).unwrap();
        let mut entry = Vec::new();
        for v in [3072u16, 0, 1, from] {
            entry.extend(v.to_le_bytes());
        }
        let at = bytes
            .windows(entry.len())
            .position(|w| w == entry)
            .expect("clé 3072");
        bytes[at + 6..at + 8].copy_from_slice(&to.to_le_bytes());
        std::fs::write(path, bytes).unwrap();
    }

    /// Passe le type de modèle à géographique et la clé 3072 en 2048 (`GeographicTypeGeoKey`)
    /// d'EPSG:4326.
    fn patch_model_to_geographic(path: &Path) {
        let mut bytes = std::fs::read(path).unwrap();
        let find = |bytes: &[u8], key: [u16; 4]| {
            let entry: Vec<u8> = key.iter().flat_map(|v| v.to_le_bytes()).collect();
            bytes
                .windows(entry.len())
                .position(|w| w == entry)
                .expect("clé GeoTIFF")
        };
        let model = find(&bytes, [1024, 0, 1, 1]);
        bytes[model + 6..model + 8].copy_from_slice(&2u16.to_le_bytes());
        let cs = find(&bytes, [3072, 0, 1, 2154]);
        bytes[cs..cs + 2].copy_from_slice(&2048u16.to_le_bytes());
        bytes[cs + 6..cs + 8].copy_from_slice(&4326u16.to_le_bytes());
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn an_unknown_unit_is_invalid() {
        let dir = Scratch::new("unite");
        let out = dir.join("pente.tif");
        let err = slope(&fixture("mnt-plan-incline-2154.tif"), "radians", &out).unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
        assert!(!out.exists());
        let err = Registry::standard()
            .call(
                "terrain.slope",
                json!({"dem": "mnt.tif", "out": out.to_string_lossy()}),
            )
            .unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
    }

    #[test]
    fn a_pseudo_metre_dem_is_refused_and_nothing_is_written() {
        let dir = Scratch::new("pseudo-metres");
        let out = dir.join("pente.tif");
        for epsg in [3857, 3395] {
            let input = dir.join(&format!("{epsg}.tif"));
            Dem { epsg, ..holed() }.write_geotiff(&input).unwrap();
            let err = slope(&input, "degrees", &out).unwrap_err();
            assert_eq!(err.code(), "dem.not_projected");
            assert!(err.to_string().contains(&format!("EPSG:{epsg}")), "{err}");
            assert!(err.to_string().contains("QGIS"), "{err}");
            assert!(!out.exists());
        }
    }

    /// Empreinte FNV-1a 64 bits des octets d'un fichier.
    fn fnv1a(path: &Path) -> u64 {
        std::fs::read(path).unwrap().iter().fold(0xcbf29ce484222325, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        })
    }

    /// Octets figés de la pente en degrés de la colline : la CI les vérifie sous Linux et sous
    /// Windows (vérifiés aussi à la main sous Windows le 2026-11-05). Ce test échoue dès qu'une
    /// valeur f32 écrite change, que la cause soit l'arc tangente, la méthode de Horn ou
    /// l'écriture du GeoTIFF ; un tel changement fait diverger le rejeu des journaux existants.
    /// Il ne prouve pas à lui seul l'usage de `libm` : un écart d'un ulp en f64 entre deux libm
    /// de système disparaît le plus souvent à l'arrondi en f32. La version de `libm` est figée
    /// dans `core/Cargo.toml`.
    #[test]
    fn the_hill_slope_in_degrees_has_frozen_bytes() {
        let dir = Scratch::new("octets-figes");
        let out = dir.join("pente.tif");
        slope(&fixture("mnt-colline-2154.tif"), "degrees", &out).unwrap();
        assert_eq!(format!("{:016x}", fnv1a(&out)), "287298aa0f719991");
    }

    #[test]
    fn two_runs_give_identical_bytes() {
        let dir = Scratch::new("deux");
        let (a, b) = (dir.join("a.tif"), dir.join("b.tif"));
        let input = fixture("mnt-colline-2154.tif");
        let ra = slope(&input, "degrees", &a).unwrap();
        let rb = slope(&input, "degrees", &b).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        assert_eq!(ra["max"], rb["max"]);
    }

    #[test]
    fn terrain_slope_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("terrain.slope").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert_eq!(spec.output_ext, ".tif");
        assert_eq!(spec.path_params, vec!["dem".to_string()]);
        let violations = registry.check_contract();
        assert!(
            violations.iter().all(|v| v.id != "terrain.slope"),
            "{violations:?}"
        );
    }
}
