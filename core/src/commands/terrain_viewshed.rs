//! Commande `terrain.viewshed` : ce qu'un observateur voit du relief d'un MNT, dans un rayon.
//! Calculée dans le noyau, sans GDAL ni GRASS (AD-1).
//!
//! Provenance : le principe est celui de l'algorithme exact « R3 » décrit par W. R. Franklin
//! et C. Ray, « Higher isn't necessarily better: visibility algorithms and experiments », dans
//! T. C. Waugh et R. G. Healey (dir.), *Advances in GIS Research: Sixth International
//! Symposium on Spatial Data Handling*, Édimbourg, 1994, p. 751-770, repris d'après la
//! publication seule, sans aucune ligne de code de Whitebox, de GRASS, de GDAL ni de QGIS :
//! chaque cellule est jugée par une ligne de vue propre, de l'observateur vers son centre.
//! Courbure et réfraction : voir `terrain_line_of_sight.rs`. Écarts mesurés contre
//! `r.viewshed` de GRASS sur un MNT réel : `docs/comparaison-terrain-qgis.md`.
//!
//! Variante : l'échantillonnage de chaque ligne n'est pas celui du papier (interpolation aux
//! croisements de la ligne avec la grille). C'est celui de `terrain.line_of_sight` (`sight`) :
//! pas constant le long du segment, altitude interpolée en bilinéaire. Ce choix rend la
//! concordance exacte par construction : une cellule vaut 1 si et seulement si
//! `terrain.line_of_sight` vers son centre rend `visible: true`, avec les mêmes paramètres.
//!
//! Le MNT est lu et la sortie écrite par `dem.rs`, l'unique porte raster du noyau (AD-20).
//! L'observateur est dans le système du MNT : un autre système est refusé, jamais reprojeté
//! (AD-19). Le rayon est borné à 500 cellules (AD-8) : le calcul, non annulable, coûte comme le
//! cube du rayon en cellules, et cette borne le garde à environ 5 s au pire (tout visible).
//! Le noyau nomme le fichier de sortie dans le dossier de travail (AD-10).

use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::commands::analysis_buffer::{Distance, DistanceUnit};
use crate::commands::terrain_line_of_sight::{
    check_crs, check_extent, default_curvature, default_observer_height, default_refraction,
    default_target_height, endpoint_elevation, refraction, sight, Height, Line,
};
use crate::dem::Dem;
use crate::point::Point;
use crate::raster_path::RasterPath;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

/// Valeur sans-donnée du champ de vision écrit.
pub const VIEWSHED_NODATA: f32 = -9999.0;
/// Rayon maximal, en nombre de plus petites cellules (AD-8).
pub const MAX_RADIUS_CELLS: f64 = 500.0;

/// Rayon par défaut, 5 km, ramené à la borne du MNT s'il la dépasse.
const DEFAULT_RADIUS: Distance = Distance {
    value: 5.0,
    unit: DistanceUnit::Km,
};

/// Un rayon donné est toujours `Some` : `None` veut dire « absent », jamais `null`.
fn given_radius<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Distance>, D::Error> {
    Distance::deserialize(d).map(Some)
}

/// Paramètres de `terrain.viewshed`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerrainViewshedParams {
    /// Fichier MNT (GeoTIFF à une bande, système projeté en mètres), chemin relatif au projet
    /// recopié de l'inventaire. DEM file path, relative to the project, copied from the inventory.
    pub dem: RasterPath,
    /// Position de l'observateur, dans le système du MNT.
    pub observer: Point,
    /// Hauteur de l'œil au-dessus du sol, avec son unité (1,6 m par défaut).
    #[serde(default = "default_observer_height")]
    pub observer_height: Height,
    /// Hauteur des cibles au-dessus du sol, avec son unité (0 m par défaut).
    #[serde(default = "default_target_height")]
    pub target_height: Height,
    /// Rayon maximal du champ de vision, avec son unité. Au plus 500 fois la taille de cellule
    /// du MNT. Absent : 5 km, ramenés à la borne du MNT.
    #[serde(default, deserialize_with = "given_radius", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Distance")]
    pub radius: Option<Distance>,
    /// Tenir compte de la courbure terrestre (vrai par défaut).
    #[serde(default = "default_curvature")]
    pub curvature: bool,
    /// Coefficient de réfraction atmosphérique, de 0 à moins de 1 (0,13 par défaut). N'agit
    /// que si `curvature` est vrai.
    #[schemars(range(min = 0.0), extend("exclusiveMaximum" = 1.0))]
    #[serde(default = "default_refraction", deserialize_with = "refraction")]
    pub refraction: f64,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `terrain.viewshed`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TerrainViewshedOutput {
    /// GeoTIFF Float32 écrit : 1 visible, 0 non visible, sans-donnée -9999 hors du rayon et sur
    /// les trous du MNT.
    pub file: String,
    /// Nombre de cellules visibles (cellule de l'observateur comprise).
    pub visible_cells: u64,
    /// Nombre de cellules du rayon masquées par le relief.
    pub hidden_cells: u64,
    /// Nombre de cellules sans donnée dans la sortie (hors du rayon ou trou du MNT).
    pub nodata_cells: u64,
}

/// Comptes de cellules d'un champ de vision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewshedStats {
    pub visible_cells: u64,
    pub hidden_cells: u64,
    pub nodata_cells: u64,
}

/// Centre de la cellule (`col`, `row`) du MNT, dans son système.
pub(crate) fn cell_centre(dem: &Dem, col: u32, row: u32) -> Point {
    Point {
        x: dem.origin_x + (f64::from(col) + 0.5) * dem.cell_size_x,
        y: dem.origin_y - (f64::from(row) + 0.5) * dem.cell_size_y,
        epsg: dem.epsg,
    }
}

/// Indice de colonne ou de ligne `floor(v)`, borné à `[0, max]`.
fn index(v: f64, max: u32) -> u32 {
    v.floor().clamp(0.0, f64::from(max)) as u32
}

/// Longueur lisible en français, au dixième : « 11,8 km », « 23,7 m », « 5 km ». `arrondi` choisit
/// le sens : la borne est arrondie vers le bas, pour qu'un rayon recopié du message soit accepté.
fn length_fr(metres: f64, arrondi: fn(f64) -> f64) -> String {
    let (v, unit) = if metres >= 1000.0 {
        (arrondi(metres / 100.0) / 10.0, "km")
    } else {
        (arrondi(metres * 10.0) / 10.0, "m")
    };
    let text = format!("{v:.1}").replace('.', ",");
    format!("{} {unit}", text.strip_suffix(",0").unwrap_or(&text))
}

/// Champ de vision sur un MNT déjà ouvert. Fonction pure et déterministe : calcul en f64,
/// même entrée, même grille. La grille rendue est alignée sur celle du MNT et rognée au carré
/// du cercle, intersecté avec le MNT.
pub fn viewshed(
    dem: &Dem,
    params: &TerrainViewshedParams,
) -> Result<(Dem, ViewshedStats), CommandFailure> {
    let o = &params.observer;
    // Mêmes contrôles et même ordre que `terrain.line_of_sight`.
    check_crs(dem, o, "observateur")?;
    check_extent(dem, o, "observateur")?;
    let z_obs = endpoint_elevation(dem, o, "observateur")?;
    let cell = dem.cell_size_x.min(dem.cell_size_y);
    let bound = MAX_RADIUS_CELLS * cell;
    let radius = match params.radius {
        Some(r) if r.metres() > bound => {
            return Err(CommandFailure::new(
                "terrain.viewshed_too_large",
                format!(
                    "Le rayon de {} dépasse 500 cellules du MNT : au plus {} pour des cellules de {} ; réduisez-le.",
                    length_fr(r.metres(), f64::round),
                    length_fr(bound, f64::floor),
                    length_fr(cell, f64::round),
                ),
            ));
        }
        Some(r) => r.metres(),
        None => DEFAULT_RADIUS.metres().min(bound),
    };
    let eye = z_obs + params.observer_height.metres();
    let target_height = params.target_height.metres();

    // Fenêtre : colonnes et lignes des cellules qui touchent le carré du cercle.
    let (max_col, max_row) = (dem.width - 1, dem.height - 1);
    let c0 = index((o.x - radius - dem.origin_x) / dem.cell_size_x, max_col);
    let c1 = index((o.x + radius - dem.origin_x) / dem.cell_size_x, max_col);
    let r0 = index((dem.origin_y - (o.y + radius)) / dem.cell_size_y, max_row);
    let r1 = index((dem.origin_y - (o.y - radius)) / dem.cell_size_y, max_row);
    // Cellule de l'observateur : celle qui le contient, le bord droit et le bas compris.
    let own = (
        index((o.x - dem.origin_x) / dem.cell_size_x, max_col),
        index((dem.origin_y - o.y) / dem.cell_size_y, max_row),
    );

    let (w, h) = (c1 - c0 + 1, r1 - r0 + 1);
    let mut values = vec![VIEWSHED_NODATA; w as usize * h as usize];
    let (mut visible, mut hidden) = (0u64, 0u64);
    for row in r0..=r1 {
        for col in c0..=c1 {
            let at = (row - r0) as usize * w as usize + (col - c0) as usize;
            if (col, row) == own {
                values[at] = 1.0;
                visible += 1;
                continue;
            }
            if dem.value(col, row).is_none() {
                continue;
            }
            let t = cell_centre(dem, col, row);
            let line = Line::new(dem, o, &t);
            if line.distance > radius {
                continue;
            }
            // La cellule est valide : son altitude est connue, comme pour la ligne de vue.
            let Ok(z_tgt) = endpoint_elevation(dem, &t, "cible") else {
                continue;
            };
            let seen = sight(
                dem,
                &line,
                eye,
                z_tgt,
                target_height,
                params.curvature,
                params.refraction,
                true,
            );
            if seen.obstruction.is_none() {
                values[at] = 1.0;
                visible += 1;
            } else {
                values[at] = 0.0;
                hidden += 1;
            }
        }
    }
    let out = Dem {
        width: w,
        height: h,
        origin_x: dem.origin_x + f64::from(c0) * dem.cell_size_x,
        origin_y: dem.origin_y - f64::from(r0) * dem.cell_size_y,
        cell_size_x: dem.cell_size_x,
        cell_size_y: dem.cell_size_y,
        epsg: dem.epsg,
        nodata: Some(f64::from(VIEWSHED_NODATA)),
        values,
    };
    let stats = ViewshedStats {
        visible_cells: visible,
        hidden_cells: hidden,
        nodata_cells: u64::from(w) * u64::from(h) - visible - hidden,
    };
    Ok((out, stats))
}

/// `terrain.viewshed` : commande du noyau.
pub struct TerrainViewshed;

impl Command for TerrainViewshed {
    type Params = TerrainViewshedParams;
    type Output = TerrainViewshedOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<TerrainViewshedParams, TerrainViewshedOutput>(
            "terrain.viewshed",
            "Champ de vision depuis un point sur un MNT : calcule quelles cellules d'un modèle numérique d'élévation (fichier GeoTIFF désigné par `dem`, chemin de fichier MNT recopié de l'inventaire, pas un id de couche) un observateur voit, dans un rayon donné (5 km par défaut, ramenés à la borne). Le rayon est borné à 500 fois la taille de cellule du MNT : 12,5 km pour des cellules de 25 m, 2,5 km pour 5 m, 500 m pour 1 m ; au-delà, refus. L'observateur est dans le système du MNT, avec la hauteur de l'œil et des cibles au-dessus du sol ; la courbure terrestre et la réfraction sont prises en compte par défaut. Écrit un nouveau GeoTIFF : 1 visible, 0 masqué, sans donnée hors du rayon. Ne modifie pas le MNT.",
            "Viewshed from a point on a DEM: computes which cells of a digital elevation model (GeoTIFF file given by `dem`, a DEM file path copied from the inventory, not a layer id) an observer sees, within a given radius (5 km by default, capped at the bound). The radius is bounded to 500 times the DEM cell size: 12.5 km for 25 m cells, 2.5 km for 5 m, 500 m for 1 m; beyond that, refused. The observer is in the DEM's coordinate system, with eye and target heights above the ground; Earth curvature and refraction are taken into account by default. Writes a new GeoTIFF: 1 visible, 0 hidden, no data outside the radius. Does not modify the DEM.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["dem"])
        .output_params(&["out"])
        .output_ext(".tif")
        .example(serde_json::json!({
            "dem": "mnt-colline-2154.tif",
            "observer": {"x": 652030, "y": 6850250, "epsg": 2154},
            "observer_height": {"value": 2, "unit": "m"},
            "radius": {"value": 1, "unit": "km"}
        }))
        .example(serde_json::json!({
            "dem": "mnt-colline-2154.tif",
            "observer": {"x": 652250, "y": 6850250, "epsg": 2154},
            "radius": {"value": 200, "unit": "m"}
        }))
        .tests(&[
            "the_hill_hides_its_far_side",
            "every_cell_agrees_with_the_line_of_sight",
            "the_radius_crops_the_output",
            "the_agreement_holds_on_a_cropped_window_and_a_holed_dem",
            "the_observer_cell_is_visible_even_with_a_tiny_radius",
            "the_default_radius_is_capped_on_a_fine_dem",
            "rectangular_cells_bound_on_the_smaller_and_crop_each_axis_apart",
            "a_hole_is_nodata_and_hides_nothing",
            "a_too_large_radius_writes_nothing",
            "a_bad_observer_writes_nothing",
            "defaults_and_invalid_params",
            "two_runs_give_identical_bytes",
            "terrain_viewshed_runs_in_the_core_and_the_contract_accepts_it",
        ])
    }

    fn run(&self, params: TerrainViewshedParams) -> Result<TerrainViewshedOutput, CommandFailure> {
        let dem = Dem::open(Path::new(params.dem.as_str())).map_err(CommandFailure::from)?;
        let (grid, stats) = viewshed(&dem, &params)?;
        grid.write_geotiff(Path::new(&params.out))
            .map_err(CommandFailure::from)?;
        Ok(TerrainViewshedOutput {
            file: params.out,
            visible_cells: stats.visible_cells,
            hidden_cells: stats.hidden_cells,
            nodata_cells: stats.nodata_cells,
        })
    }
}

inventory::submit! { Registration(|r| r.register(TerrainViewshed)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::terrain_line_of_sight::{line_of_sight, TerrainLineOfSightParams};
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
            let dir = std::env::temp_dir()
                .join(format!("cassini-viewshed-{}-{name}", std::process::id()));
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

    fn m(value: f64) -> Height {
        Height {
            value,
            unit: DistanceUnit::M,
        }
    }

    fn pt(x: f64, y: f64) -> Point {
        Point { x, y, epsg: 2154 }
    }

    fn params(observer: Point, radius_m: f64) -> TerrainViewshedParams {
        TerrainViewshedParams {
            dem: RasterPath(String::new()),
            observer,
            observer_height: m(1.6),
            target_height: m(0.0),
            radius: Some(Distance {
                value: radius_m,
                unit: DistanceUnit::M,
            }),
            curvature: true,
            refraction: 0.13,
            out: String::new(),
        }
    }

    fn hill() -> Dem {
        Dem::open(&fixture("mnt-colline-2154.tif")).unwrap()
    }

    /// Valeur de la cellule (`col`, `row`) du MNT dans la grille rognée, `None` hors fenêtre
    /// ou sans donnée.
    fn at(grid: &Dem, dem: &Dem, col: u32, row: u32) -> Option<f32> {
        let c = ((grid.origin_x - dem.origin_x) / dem.cell_size_x).round() as u32;
        let r = ((dem.origin_y - grid.origin_y) / dem.cell_size_y).round() as u32;
        grid.value(col.checked_sub(c)?, row.checked_sub(r)?)
    }

    fn call(p: Value) -> Result<Value, crate::registry::RegistryError> {
        Registry::standard().call("terrain.viewshed", p)
    }

    #[test]
    fn the_hill_hides_its_far_side() {
        let dem = hill();
        let mut p = params(pt(652030.0, 6850250.0), 1000.0);
        p.observer_height = m(2.0);
        let (grid, stats) = viewshed(&dem, &p).unwrap();
        // Le cercle de 1 km déborde le MNT de 500 m : la sortie est rognée au MNT entier.
        assert_eq!(grid.extent(), dem.extent());
        assert_eq!((grid.width, grid.height, grid.epsg), (50, 50, 2154));
        assert_eq!(grid.nodata, Some(-9999.0));
        // L'observateur est dans la cellule (3, 25) ; le sommet au coin des cellules 24 et 25.
        assert_eq!(grid.value(3, 25), Some(1.0));
        // Versant ouest, entre l'observateur et le sommet : visible.
        for col in 4..15 {
            assert_eq!(grid.value(col, 25), Some(1.0), "colonne {col}");
        }
        // Versant est, derrière le sommet : masqué.
        for col in 30..50 {
            assert_eq!(grid.value(col, 25), Some(0.0), "colonne {col}");
        }
        assert_eq!(stats.visible_cells + stats.hidden_cells, 2500);
        assert_eq!(stats.nodata_cells, 0);
        assert!(stats.visible_cells > 0 && stats.hidden_cells > 0);
    }

    #[test]
    fn every_cell_agrees_with_the_line_of_sight() {
        let dem = hill();
        let mut p = params(pt(652030.0, 6850250.0), 1000.0);
        p.observer_height = m(2.0);
        let (grid, _) = viewshed(&dem, &p).unwrap();
        let mut checked = 0;
        // Échantillon déterministe : une cellule sur trois, 834 en tout.
        for k in (0..2500u32).step_by(3) {
            let (col, row) = (k % 50, k / 50);
            let v = grid.value(col, row).unwrap();
            let los = TerrainLineOfSightParams {
                dem: RasterPath(String::new()),
                observer: p.observer,
                target: cell_centre(&dem, col, row),
                observer_height: p.observer_height,
                target_height: p.target_height,
                curvature: p.curvature,
                refraction: p.refraction,
            };
            let r = line_of_sight(&dem, &los).unwrap();
            assert_eq!(v == 1.0, r.visible, "({col}, {row})");
            checked += 1;
        }
        assert!(checked >= 200);
        // Même accord sans courbure, avec une cible haute.
        p.curvature = false;
        p.target_height = m(5.0);
        let (grid, _) = viewshed(&dem, &p).unwrap();
        for k in (1..2500u32).step_by(7) {
            let (col, row) = (k % 50, k / 50);
            let los = TerrainLineOfSightParams {
                dem: RasterPath(String::new()),
                observer: p.observer,
                target: cell_centre(&dem, col, row),
                observer_height: p.observer_height,
                target_height: p.target_height,
                curvature: false,
                refraction: p.refraction,
            };
            let r = line_of_sight(&dem, &los).unwrap();
            assert_eq!(grid.value(col, row) == Some(1.0), r.visible, "({col}, {row})");
        }
    }

    /// Verdict de `terrain.line_of_sight` vers le centre de la cellule (`col`, `row`), avec les
    /// paramètres du champ de vision.
    fn los_verdict(dem: &Dem, p: &TerrainViewshedParams, col: u32, row: u32) -> bool {
        let los = TerrainLineOfSightParams {
            dem: RasterPath(String::new()),
            observer: p.observer,
            target: cell_centre(dem, col, row),
            observer_height: p.observer_height,
            target_height: p.target_height,
            curvature: p.curvature,
            refraction: p.refraction,
        };
        line_of_sight(dem, &los).unwrap().visible
    }

    #[test]
    fn the_agreement_holds_on_a_cropped_window_and_a_holed_dem() {
        // Fenêtre rognée au centre de la colline : chaque cellule est retrouvée par `at`.
        let dem = hill();
        let p = params(pt(652255.0, 6850255.0), 150.0);
        let (grid, stats) = viewshed(&dem, &p).unwrap();
        assert_eq!((grid.width, grid.height), (31, 31));
        assert!(grid.origin_x > dem.origin_x && grid.origin_y < dem.origin_y);
        let mut checked = 0u64;
        for row in 0..dem.height {
            for col in 0..dem.width {
                if let Some(v) = at(&grid, &dem, col, row) {
                    assert_eq!(v == 1.0, los_verdict(&dem, &p, col, row), "({col}, {row})");
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, stats.visible_cells + stats.hidden_cells);
        assert!(checked >= 200);
        // MNT troué : chaque cellule valide concorde, le trou est sans donnée.
        let holed = holed();
        let p = params(pt(5.0, 35.0), 1000.0);
        let (grid, _) = viewshed(&holed, &p).unwrap();
        for row in 0..7 {
            for col in 0..7 {
                if (col, row) == (3, 3) {
                    assert_eq!(grid.value(col, row), None);
                } else {
                    let v = grid.value(col, row).unwrap();
                    assert_eq!(v == 1.0, los_verdict(&holed, &p, col, row), "({col}, {row})");
                }
            }
        }
    }

    #[test]
    fn the_observer_cell_is_visible_even_with_a_tiny_radius() {
        // Rayon de 1 m : aucun centre de cellule n'est assez près, sauf la cellule forcée.
        let (grid, stats) = viewshed(&hill(), &params(pt(652030.0, 6850250.0), 1.0)).unwrap();
        assert_eq!((grid.width, grid.height), (2, 2));
        assert_eq!((grid.origin_x, grid.origin_y), (652020.0, 6850260.0));
        // L'observateur est sur le coin haut-gauche de la cellule (3, 25) : bas droit ici.
        assert_eq!(grid.value(1, 1), Some(1.0));
        for (col, row) in [(0, 0), (1, 0), (0, 1)] {
            assert_eq!(grid.value(col, row), None, "({col}, {row})");
        }
        assert_eq!(
            stats,
            ViewshedStats {
                visible_cells: 1,
                hidden_cells: 0,
                nodata_cells: 3
            }
        );
    }

    #[test]
    fn the_default_radius_is_capped_on_a_fine_dem() {
        // Bande plate de 1 101 × 3 cellules de 1 m : la borne vaut 500 m, sous les 5 km.
        let fine = Dem {
            width: 1101,
            height: 3,
            origin_x: 0.0,
            origin_y: 3.0,
            cell_size_x: 1.0,
            cell_size_y: 1.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values: vec![50.0; 3303],
        };
        let mut p = params(pt(550.5, 1.5), 500.0);
        p.radius = None;
        let (grid, stats) = viewshed(&fine, &p).unwrap();
        // Colonnes 50 à 1 050 : un rayon de 500 m, pas de 5 km ni de refus.
        assert_eq!((grid.width, grid.height), (1001, 3));
        assert_eq!(grid.origin_x, 50.0);
        let explicit = viewshed(&fine, &params(pt(550.5, 1.5), 500.0)).unwrap();
        assert_eq!((grid, stats), explicit);
        // Donnés explicitement, 5 km sont refusés.
        let err = viewshed(&fine, &params(pt(550.5, 1.5), 5000.0)).unwrap_err();
        assert_eq!(err.code, "terrain.viewshed_too_large");
    }

    /// MNT plat à 50 m de 40 colonnes de 20 m sur 80 lignes de 10 m, coin haut-gauche (0, 800).
    fn rectangular() -> Dem {
        Dem {
            width: 40,
            height: 80,
            origin_x: 0.0,
            origin_y: 800.0,
            cell_size_x: 20.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values: vec![50.0; 3200],
        }
    }

    #[test]
    fn rectangular_cells_bound_on_the_smaller_and_crop_each_axis_apart() {
        let dem = rectangular();
        // Centre de la cellule (20, 40).
        let o = pt(410.0, 395.0);
        let mut p = params(o, 200.0);
        p.curvature = false;
        let (grid, stats) = viewshed(&dem, &p).unwrap();
        // 200 m font 10 colonnes de 20 m et 20 lignes de 10 m de chaque côté.
        assert_eq!((grid.width, grid.height), (21, 41));
        assert_eq!((grid.origin_x, grid.origin_y), (200.0, 600.0));
        assert_eq!((grid.cell_size_x, grid.cell_size_y), (20.0, 10.0));
        // L'observateur, au centre de la fenêtre ; un coin, à 283 m, hors du cercle.
        assert_eq!(grid.value(10, 20), Some(1.0));
        assert_eq!(grid.value(0, 0), None);
        // Le long de chaque axe, jusqu'au rayon : visible.
        assert_eq!(grid.value(0, 20), Some(1.0));
        assert_eq!(grid.value(10, 0), Some(1.0));
        assert_eq!(stats.hidden_cells, 0);
        // La borne se prend sur la plus petite cellule : 500 × 10 m, pas 500 × 20 m.
        let err = viewshed(&dem, &params(o, 5100.0)).unwrap_err();
        assert_eq!(err.code, "terrain.viewshed_too_large");
        assert_eq!(
            err.message,
            "Le rayon de 5,1 km dépasse 500 cellules du MNT : au plus 5 km pour des cellules de 10 m ; réduisez-le."
        );
        assert!(viewshed(&dem, &params(o, 5000.0)).is_ok());
    }

    #[test]
    fn the_radius_crops_the_output() {
        let dem = hill();
        // Près du bord ouest : le carré du cercle est coupé par le MNT.
        let (grid, stats) = viewshed(&dem, &params(pt(652030.0, 6850250.0), 100.0)).unwrap();
        assert_eq!((grid.width, grid.height), (14, 21));
        assert_eq!((grid.origin_x, grid.origin_y), (652000.0, 6850350.0));
        // Au centre : le carré entier, 21 × 21 cellules alignées sur la grille du MNT.
        let o = pt(652155.0, 6850255.0);
        let (grid, stats_c) = viewshed(&dem, &params(o, 100.0)).unwrap();
        assert_eq!((grid.width, grid.height), (21, 21));
        assert_eq!((grid.origin_x, grid.origin_y), (652050.0, 6850360.0));
        for row in 0..grid.height {
            for col in 0..grid.width {
                let c = cell_centre(&grid, col, row);
                let d = ((c.x - o.x).powi(2) + (c.y - o.y).powi(2)).sqrt();
                assert_eq!(grid.value(col, row).is_some(), d <= 100.0, "({col}, {row})");
            }
        }
        // Les coins sont sans donnée ; le bord du cercle est compris (cellule à 100 m pile).
        assert_eq!(grid.value(0, 0), None);
        assert!(grid.value(0, 10).is_some());
        for (s, g) in [(stats, (14u64, 21u64)), (stats_c, (21, 21))] {
            assert!(s.nodata_cells > 0);
            assert_eq!(s.visible_cells + s.hidden_cells + s.nodata_cells, g.0 * g.1);
        }
    }

    /// MNT plat 7 × 7 de cellules de 10 m, coin haut-gauche (0, 70), avec un trou en (3, 3)
    /// dont la sans-donnée vaut 500 : lue comme une altitude, elle masquerait l'est.
    fn holed() -> Dem {
        let mut values = vec![100.0f32; 49];
        values[3 * 7 + 3] = 500.0;
        Dem {
            width: 7,
            height: 7,
            origin_x: 0.0,
            origin_y: 70.0,
            cell_size_x: 10.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(500.0),
            values,
        }
    }

    #[test]
    fn a_hole_is_nodata_and_hides_nothing() {
        let (grid, stats) = viewshed(&holed(), &params(pt(5.0, 35.0), 1000.0)).unwrap();
        assert_eq!((grid.width, grid.height), (7, 7));
        assert_eq!(grid.value(3, 3), None);
        for col in 4..7 {
            assert_eq!(grid.value(col, 3), Some(1.0), "colonne {col}");
        }
        assert_eq!(
            stats,
            ViewshedStats {
                visible_cells: 48,
                hidden_cells: 0,
                nodata_cells: 1
            }
        );
        // La sortie garde sa propre sans-donnée, -9999.
        let dir = Scratch::new("trou");
        let (input, out) = (dir.join("mnt.tif"), dir.join("vue.tif"));
        holed().write_geotiff(&input).unwrap();
        let r = call(json!({
            "dem": input.to_string_lossy(),
            "observer": {"x": 5, "y": 35, "epsg": 2154},
            "out": out.to_string_lossy(),
        }))
        .unwrap();
        assert_eq!(r["nodata_cells"], 1);
        let written = Dem::open(&out).unwrap();
        assert_eq!(written.nodata, Some(-9999.0));
        assert_eq!(written.values[3 * 7 + 3], -9999.0);
    }

    #[test]
    fn a_too_large_radius_writes_nothing() {
        let err = viewshed(&hill(), &params(pt(652030.0, 6850250.0), 20000.0)).unwrap_err();
        assert_eq!(err.code, "terrain.viewshed_too_large");
        assert_eq!(
            err.message,
            "Le rayon de 20 km dépasse 500 cellules du MNT : au plus 5 km pour des cellules de 10 m ; réduisez-le."
        );
        // Lisible et arrondi vers le bas pour la borne : 11 828,2 m donnent « 11,8 km ».
        assert_eq!(length_fr(11828.224, f64::floor), "11,8 km");
        assert_eq!(length_fr(23.656, f64::round), "23,7 m");
        assert_eq!(length_fr(15000.0, f64::round), "15 km");
        let err = viewshed(&hill(), &params(pt(652030.0, 6850250.0), 5000.1)).unwrap_err();
        assert_eq!(err.code, "terrain.viewshed_too_large");
        // 500 cellules de 10 m pile : accepté.
        assert!(viewshed(&hill(), &params(pt(652030.0, 6850250.0), 5000.0)).is_ok());
        let dir = Scratch::new("trop-grand");
        let out = dir.join("vue.tif");
        let err = call(json!({
            "dem": fixture("mnt-colline-2154.tif").to_string_lossy(),
            "observer": {"x": 652030, "y": 6850250, "epsg": 2154},
            "radius": {"value": 20, "unit": "km"},
            "out": out.to_string_lossy(),
        }))
        .unwrap_err();
        assert_eq!(err.code(), "terrain.viewshed_too_large");
        assert!(!out.exists());
    }

    #[test]
    fn a_pseudo_metre_dem_is_refused_and_nothing_is_written() {
        let dir = Scratch::new("pseudo-metres");
        let out = dir.join("vue.tif");
        for epsg in [3857, 3395] {
            let input = dir.join(&format!("{epsg}.tif"));
            Dem { epsg, ..holed() }.write_geotiff(&input).unwrap();
            let err = call(json!({
                "dem": input.to_string_lossy(),
                "observer": {"x": 15, "y": 35, "epsg": epsg},
                "radius": {"value": 100, "unit": "m"},
                "out": out.to_string_lossy(),
            }))
            .unwrap_err();
            assert_eq!(err.code(), "dem.not_projected");
            assert!(err.to_string().contains(&format!("EPSG:{epsg}")), "{err}");
            assert!(err.to_string().contains("QGIS"), "{err}");
            assert!(!out.exists());
        }
    }

    #[test]
    fn a_bad_observer_writes_nothing() {
        let dir = Scratch::new("observateur");
        let out = dir.join("vue.tif");
        let mut holed_dem = holed();
        holed_dem.nodata = Some(-9999.0);
        holed_dem.values[3 * 7 + 3] = -9999.0;
        let input = dir.join("troue.tif");
        holed_dem.write_geotiff(&input).unwrap();
        let hill = fixture("mnt-colline-2154.tif");
        for (dem, observer, code) in [
            (&hill, json!({"x": 2.3, "y": 48.7, "epsg": 4326}), "terrain.crs_mismatch"),
            (&hill, json!({"x": 651990, "y": 6850250, "epsg": 2154}), "terrain.outside_dem"),
            (&input, json!({"x": 35, "y": 35, "epsg": 2154}), "terrain.no_elevation"),
        ] {
            let err = call(json!({
                "dem": dem.to_string_lossy(),
                "observer": observer,
                "radius": {"value": 100, "unit": "m"},
                "out": out.to_string_lossy(),
            }))
            .unwrap_err();
            assert_eq!(err.code(), code);
            assert!(!out.exists());
        }
        // Un mauvais système l'emporte sur un rayon trop grand : l'observateur est jugé d'abord.
        let err = call(json!({
            "dem": hill.to_string_lossy(),
            "observer": {"x": 2.3, "y": 48.7, "epsg": 4326},
            "radius": {"value": 20, "unit": "km"},
            "out": out.to_string_lossy(),
        }))
        .unwrap_err();
        assert_eq!(err.code(), "terrain.crs_mismatch");
        assert!(!out.exists());
    }

    #[test]
    fn defaults_and_invalid_params() {
        let base = json!({
            "dem": "mnt.tif",
            "observer": {"x": 1, "y": 2, "epsg": 2154},
        });
        let p: TerrainViewshedParams = serde_json::from_value(base.clone()).unwrap();
        assert_eq!(
            (p.observer_height, p.target_height, p.curvature, p.refraction),
            (m(1.6), m(0.0), true, 0.13)
        );
        assert_eq!(p.radius, None);
        for (key, bad) in [
            ("radius", json!({"value": 0, "unit": "m"})),
            ("radius", json!({"value": -1, "unit": "km"})),
            ("radius", json!(1000)),
            ("radius", json!({"value": 1000})),
            ("radius", json!(null)),
            ("observer_height", json!({"value": -1, "unit": "m"})),
            ("target_height", json!({"value": -0.5, "unit": "m"})),
            ("refraction", json!(1.0)),
            ("observer", json!({"x": 1, "y": 2})),
        ] {
            let mut v = base.clone();
            v[key] = bad;
            let err = call(v).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{key}");
        }
        let mut v = base;
        v["target"] = json!({"x": 1, "y": 2, "epsg": 2154});
        assert_eq!(call(v).unwrap_err().code(), "registry.invalid_params");
    }

    #[test]
    fn two_runs_give_identical_bytes() {
        let dir = Scratch::new("deux");
        let (a, b) = (dir.join("a.tif"), dir.join("b.tif"));
        let p = |out: &Path| {
            json!({
                "dem": fixture("mnt-colline-2154.tif").to_string_lossy(),
                "observer": {"x": 652030, "y": 6850250, "epsg": 2154},
                "observer_height": {"value": 2, "unit": "m"},
                "radius": {"value": 1, "unit": "km"},
                "out": out.to_string_lossy(),
            })
        };
        let ra = call(p(&a)).unwrap();
        let rb = call(p(&b)).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        assert_eq!(ra["visible_cells"], rb["visible_cells"]);
        assert_eq!(ra["nodata_cells"], 0);
        let written = Dem::open(&a).unwrap();
        let (grid, _) = viewshed(&hill(), &{
            let mut q = params(pt(652030.0, 6850250.0), 1000.0);
            q.observer_height = m(2.0);
            q
        })
        .unwrap();
        assert_eq!(written, grid);
        assert_eq!(at(&written, &hill(), 3, 25), Some(1.0));
    }

    #[test]
    fn terrain_viewshed_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("terrain.viewshed").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert_eq!(spec.output_ext, ".tif");
        assert_eq!(spec.path_params, vec!["dem".to_string()]);
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        let violations = registry.check_contract();
        assert!(
            violations.iter().all(|v| v.id != "terrain.viewshed"),
            "{violations:?}"
        );
        let props = spec.output_schema["properties"].as_object().unwrap();
        let mut keys: Vec<&str> = props.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["file", "hidden_cells", "nodata_cells", "visible_cells"]
        );
        let params = &spec.params_schema;
        assert!(crate::point::is_point(
            &params["properties"]["observer"],
            params
        ));
        // Pas de défaut écrit : 5 km recopiés tels quels seraient refusés sur un MNT fin.
        assert!(params["properties"]["radius"].get("default").is_none());
    }

    /// Mesure à titre d'information : MNT de 25 m, rayon de 10 km, puis 12,5 km (la borne) sur
    /// MNT plat. Lancer en `--release`.
    #[test]
    #[ignore]
    fn viewshed_timing_at_ten_km_on_a_25_m_dem() {
        let n = 1001u32;
        let mut values = Vec::with_capacity((n * n) as usize);
        for row in 0..n {
            for col in 0..n {
                let (x, y) = (f64::from(col) * 25.0, f64::from(row) * 25.0);
                let z = 100.0 + 40.0 * (x / 900.0).sin() * (y / 1300.0).cos() + 0.002 * x;
                values.push(z as f32);
            }
        }
        let dem = Dem {
            width: n,
            height: n,
            origin_x: 600000.0,
            origin_y: 6900000.0,
            cell_size_x: 25.0,
            cell_size_y: 25.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values,
        };
        let o = cell_centre(&dem, 500, 500);
        let start = std::time::Instant::now();
        let (grid, stats) = viewshed(&dem, &params(o, 10000.0)).unwrap();
        let elapsed = start.elapsed();
        eprintln!(
            "champ de vision 10 km sur 25 m : {:?}, grille {} × {}, {stats:?}",
            elapsed, grid.width, grid.height
        );
        assert_eq!((grid.width, grid.height), (801, 801));
        // Cas lent : MNT plat, seule la courbure masque, les lignes s'arrêtent tard.
        let flat = Dem {
            values: vec![100.0; (n * n) as usize],
            ..dem
        };
        let start = std::time::Instant::now();
        let (_, stats) = viewshed(&flat, &params(o, 10000.0)).unwrap();
        eprintln!(
            "champ de vision 10 km sur 25 m, plat : {:?}, {stats:?}",
            start.elapsed()
        );
        let start = std::time::Instant::now();
        let (grid, stats) = viewshed(&flat, &params(o, 12500.0)).unwrap();
        eprintln!(
            "champ de vision 12,5 km (borne) sur 25 m, plat : {:?}, grille {} × {}, {stats:?}",
            start.elapsed(),
            grid.width,
            grid.height
        );
        // Pire cas : sans courbure, tout est visible et chaque ligne va jusqu'au bout.
        let mut q = params(o, 12500.0);
        q.curvature = false;
        let start = std::time::Instant::now();
        let (_, stats) = viewshed(&flat, &q).unwrap();
        eprintln!(
            "champ de vision 12,5 km (borne) sur 25 m, plat, sans courbure : {:?}, {stats:?}",
            start.elapsed()
        );
    }
}
