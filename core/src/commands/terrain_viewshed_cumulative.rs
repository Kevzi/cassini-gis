//! Commande `terrain.viewshed_cumulative` : combien d'observateurs voient chaque cellule d'un
//! MNT. Calculée dans le noyau, sans GDAL ni GRASS (AD-1).
//!
//! Le calcul est celui de `terrain.viewshed` (`terrain_viewshed.rs`), répété pour chaque
//! observateur avec les mêmes hauteurs, rayon, courbure et réfraction ; les grilles 0/1 sont
//! additionnées sur une grille commune, alignée sur le MNT et réduite à l'enveloppe des
//! fenêtres individuelles. Une cellule vaut le nombre d'observateurs pour lesquels
//! `terrain.viewshed` rend 1, et -9999 si aucun ne la couvre (hors de tous les rayons, ou trou).
//! Un observateur répété compte deux fois.
//!
//! La liste est bornée à 32 observateurs (AD-8), par le schéma et par la désérialisation. Tous
//! les observateurs sont contrôlés, puis toutes les fenêtres calculées, avant d'écrire quoi que
//! ce soit : la première erreur arrête tout. Pas de parallélisme : ordre et octets figés.
//!
//! Coût : celui de `terrain.viewshed` (environ 5 s au pire, tout visible au rayon de 500
//! cellules) multiplié par le nombre d'observateurs, soit environ 2 min 30 s au pire pour 32,
//! sans annulation. Décision du propriétaire (2026-11-06) : gardé tel quel, et dit.

use std::path::Path;

use schemars::JsonSchema;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::commands::analysis_buffer::Distance;
use crate::commands::terrain_line_of_sight::{
    default_curvature, default_observer_height, default_refraction, default_target_height,
    refraction, Height,
};
use crate::commands::terrain_viewshed::{
    check_observer, given_radius, viewshed_of, TerrainViewshedParams,
};
use crate::dem::{Dem, NODATA};
use crate::point::Point;
use crate::raster_path::RasterPath;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

/// Nombre maximal d'observateurs (AD-8).
pub const MAX_OBSERVERS: usize = 32;

fn observers<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Point>, D::Error> {
    let observers = Vec::<Point>::deserialize(d)?;
    if observers.is_empty() || observers.len() > MAX_OBSERVERS {
        return Err(D::Error::custom(format!(
            "`observers` demande de 1 à {MAX_OBSERVERS} points, pas {}",
            observers.len()
        )));
    }
    Ok(observers)
}

/// Paramètres de `terrain.viewshed_cumulative`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerrainViewshedCumulativeParams {
    /// Fichier MNT (GeoTIFF à une bande, système projeté en mètres), chemin relatif au projet
    /// recopié de l'inventaire. DEM file path, relative to the project, copied from the inventory.
    pub dem: RasterPath,
    /// Positions des observateurs, de 1 à 32, dans le système du MNT.
    #[serde(deserialize_with = "observers")]
    #[schemars(length(min = 1, max = MAX_OBSERVERS))]
    pub observers: Vec<Point>,
    /// Hauteur de l'œil au-dessus du sol, la même pour chacun (1,6 m par défaut).
    #[serde(default = "default_observer_height")]
    pub observer_height: Height,
    /// Hauteur des cibles au-dessus du sol, avec son unité (0 m par défaut).
    #[serde(default = "default_target_height")]
    pub target_height: Height,
    /// Rayon autour de chaque observateur, avec son unité. Au plus 500 fois la taille de
    /// cellule du MNT. Absent : 5 km, ramenés à la borne du MNT.
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

/// Résultat de `terrain.viewshed_cumulative`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TerrainViewshedCumulativeOutput {
    /// GeoTIFF Float32 écrit : nombre d'observateurs qui voient la cellule (0 à n), sans-donnée
    /// -9999 hors de tous les rayons et sur les trous du MNT.
    pub file: String,
    /// Nombre d'observateurs calculés.
    pub observer_count: u64,
    /// Compte le plus élevé atteint.
    pub max_count: u64,
    /// Nombre de cellules vues par au moins un observateur.
    pub visible_cells: u64,
    /// Nombre de cellules sans donnée dans la sortie.
    pub nodata_cells: u64,
}

/// Comptes d'un champ de vision cumulé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CumulativeStats {
    pub observer_count: u64,
    pub max_count: u64,
    pub visible_cells: u64,
    pub nodata_cells: u64,
}

/// Nom d'un observateur dans les messages : « observateur n° 2 », rang compté de 1.
fn rank_name(i: usize) -> String {
    format!("observateur n° {}", i + 1)
}

/// Décalage entier, en colonnes et lignes du MNT, du coin haut-gauche d'une fenêtre alignée.
fn offset(dem: &Dem, sub: &Dem) -> (u32, u32) {
    (
        ((sub.origin_x - dem.origin_x) / dem.cell_size_x).round() as u32,
        ((dem.origin_y - sub.origin_y) / dem.cell_size_y).round() as u32,
    )
}

/// Champ de vision cumulé sur un MNT déjà ouvert. Fonction pure et déterministe.
pub fn viewshed_cumulative(
    dem: &Dem,
    params: &TerrainViewshedCumulativeParams,
) -> Result<(Dem, CumulativeStats), CommandFailure> {
    // Tous les observateurs d'abord : un observateur fautif l'emporte sur le rayon, comme dans
    // `terrain.viewshed`, et rien n'est calculé en vain.
    for (i, o) in params.observers.iter().enumerate() {
        check_observer(dem, o, &rank_name(i))?;
    }
    let single = TerrainViewshedParams {
        dem: params.dem.clone(),
        observer: params.observers[0],
        observer_height: params.observer_height,
        target_height: params.target_height,
        radius: params.radius,
        curvature: params.curvature,
        refraction: params.refraction,
        out: String::new(),
    };
    // Chaque fenêtre est ajoutée dès qu'elle est calculée à un compteur de la taille du MNT,
    // puis oubliée : une seule fenêtre en mémoire à la fois. L'enveloppe, en colonnes et
    // lignes du MNT, est suivie au passage, et le compteur rogné à la fin.
    let full_w = dem.width as usize;
    let mut counts = vec![NODATA; full_w * dem.height as usize];
    let (mut c0, mut r0, mut c1, mut r1) = (u32::MAX, u32::MAX, 0u32, 0u32);
    for (i, o) in params.observers.iter().enumerate() {
        let (g, _) = viewshed_of(dem, &single, o, &rank_name(i))?;
        let (c, r) = offset(dem, &g);
        c0 = c0.min(c);
        r0 = r0.min(r);
        c1 = c1.max(c + g.width);
        r1 = r1.max(r + g.height);
        for row in 0..g.height as usize {
            for col in 0..g.width as usize {
                let v = g.values[row * g.width as usize + col];
                let cell = &mut counts[(row + r as usize) * full_w + col + c as usize];
                if v == 1.0 {
                    *cell = if *cell == NODATA {
                        1.0
                    } else {
                        *cell + 1.0
                    };
                } else if v == 0.0 && *cell == NODATA {
                    *cell = 0.0;
                }
            }
        }
    }
    let (w, h) = (c1 - c0, r1 - r0);
    let mut values = Vec::with_capacity(w as usize * h as usize);
    for row in r0 as usize..r1 as usize {
        values.extend_from_slice(&counts[row * full_w + c0 as usize..row * full_w + c1 as usize]);
    }
    drop(counts);
    let (mut max_count, mut visible, mut nodata) = (0u64, 0u64, 0u64);
    for &v in &values {
        if v == NODATA {
            nodata += 1;
        } else if v >= 1.0 {
            visible += 1;
            max_count = max_count.max(v as u64);
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
        nodata: Some(f64::from(NODATA)),
        values,
    };
    let stats = CumulativeStats {
        observer_count: params.observers.len() as u64,
        max_count,
        visible_cells: visible,
        nodata_cells: nodata,
    };
    Ok((out, stats))
}

/// `terrain.viewshed_cumulative` : commande du noyau.
pub struct TerrainViewshedCumulative;

impl Command for TerrainViewshedCumulative {
    type Params = TerrainViewshedCumulativeParams;
    type Output = TerrainViewshedCumulativeOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<TerrainViewshedCumulativeParams, TerrainViewshedCumulativeOutput>(
            "terrain.viewshed_cumulative",
            "Champ de vision cumulé depuis plusieurs points sur un MNT : compte, pour chaque cellule d'un modèle numérique d'élévation (fichier GeoTIFF désigné par `dem`, chemin de fichier MNT recopié de l'inventaire, pas un id de couche), combien d'observateurs la voient, de 1 à 32 observateurs, au-delà refus. Même calcul que le champ de vision d'un seul point, répété pour chacun avec les mêmes hauteurs de l'œil et des cibles, le même rayon (5 km par défaut, ramenés à la borne ; au plus 500 fois la taille de cellule du MNT, au-delà refus), la courbure terrestre et la réfraction. Les observateurs sont dans le système du MNT. Écrit un nouveau GeoTIFF : nombre d'observateurs qui voient la cellule, sans donnée hors de tous les rayons. Calcul non annulable : jusqu'à environ 5 s par observateur au rayon maximal. Ne modifie pas le MNT.",
            "Cumulative viewshed from several points on a DEM: counts, for each cell of a digital elevation model (GeoTIFF file given by `dem`, a DEM file path copied from the inventory, not a layer id), how many observers see it, from 1 to 32 observers, beyond that refused. Same computation as the single-point viewshed, repeated for each with the same eye and target heights, the same radius (5 km by default, capped at the bound; at most 500 times the DEM cell size, beyond that refused), Earth curvature and refraction. Observers are in the DEM's coordinate system. Writes a new GeoTIFF: number of observers that see the cell, no data outside every radius. Not cancellable: up to about 5 s per observer at the maximum radius. Does not modify the DEM.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["dem"])
        .output_params(&["out"])
        .output_ext(".tif")
        // Style dicté après l'ouverture du raster (bogue du fond noir, 2026-11-09).
        .result_style(crate::style::ResultStyle::ViewshedCumulative)
        .example(serde_json::json!({
            "dem": "mnt-colline-2154.tif",
            "observers": [
                {"x": 652030, "y": 6850250, "epsg": 2154},
                {"x": 652470, "y": 6850250, "epsg": 2154}
            ],
            "observer_height": {"value": 2, "unit": "m"},
            "radius": {"value": 1, "unit": "km"}
        }))
        .example(serde_json::json!({
            "dem": "mnt-colline-2154.tif",
            "observers": [
                {"x": 652250, "y": 6850250, "epsg": 2154},
                {"x": 652100, "y": 6850400, "epsg": 2154},
                {"x": 652400, "y": 6850100, "epsg": 2154}
            ],
            "radius": {"value": 300, "unit": "m"}
        }))
        .tests(&[
            "two_observers_on_either_side_give_counts_zero_one_and_two",
            "every_cell_is_the_sum_of_the_single_viewsheds",
            "one_observer_matches_the_single_viewshed",
            "disjoint_radii_cover_the_envelope_with_nodata_between",
            "a_repeated_observer_counts_twice",
            "the_observer_list_is_bounded",
            "a_bad_observer_is_named_by_its_rank_and_writes_nothing",
            "a_too_large_radius_writes_nothing",
            "two_runs_give_identical_bytes",
            "terrain_viewshed_cumulative_runs_in_the_core_and_the_contract_accepts_it",
        ])
    }

    fn run(
        &self,
        params: TerrainViewshedCumulativeParams,
    ) -> Result<TerrainViewshedCumulativeOutput, CommandFailure> {
        let dem = Dem::open(Path::new(params.dem.as_str())).map_err(CommandFailure::from)?;
        let (grid, stats) = viewshed_cumulative(&dem, &params)?;
        grid.write_geotiff(Path::new(&params.out))
            .map_err(CommandFailure::from)?;
        Ok(TerrainViewshedCumulativeOutput {
            file: params.out,
            observer_count: stats.observer_count,
            max_count: stats.max_count,
            visible_cells: stats.visible_cells,
            nodata_cells: stats.nodata_cells,
        })
    }
}

inventory::submit! { Registration(|r| r.register(TerrainViewshedCumulative)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::{fixture, Scratch};
    use crate::commands::analysis_buffer::DistanceUnit;
    use crate::commands::terrain_viewshed::viewshed;
    use crate::registry::Registry;
    use serde_json::{json, Value};

    fn m(value: f64) -> Height {
        Height {
            value,
            unit: DistanceUnit::M,
        }
    }

    fn pt(x: f64, y: f64) -> Point {
        Point { x, y, epsg: 2154 }
    }

    fn params(observers: Vec<Point>, radius_m: f64) -> TerrainViewshedCumulativeParams {
        TerrainViewshedCumulativeParams {
            dem: RasterPath(String::new()),
            observers,
            observer_height: m(2.0),
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

    /// Paramètres de `terrain.viewshed` équivalents, pour l'observateur `o`.
    fn single(p: &TerrainViewshedCumulativeParams, o: Point) -> TerrainViewshedParams {
        TerrainViewshedParams {
            dem: RasterPath(String::new()),
            observer: o,
            observer_height: p.observer_height,
            target_height: p.target_height,
            radius: p.radius,
            curvature: p.curvature,
            refraction: p.refraction,
            out: String::new(),
        }
    }

    fn hill() -> Dem {
        Dem::open(&fixture("mnt-colline-2154.tif")).unwrap()
    }

    /// Ouest et est du sommet (652250, 6850250), sur la même ligne.
    fn west_east() -> Vec<Point> {
        vec![pt(652030.0, 6850250.0), pt(652470.0, 6850250.0)]
    }

    /// Valeur de la cellule (`col`, `row`) du MNT dans une grille rognée, `None` hors fenêtre
    /// ou sans donnée.
    fn at(grid: &Dem, dem: &Dem, col: u32, row: u32) -> Option<f32> {
        let (c, r) = offset(dem, grid);
        grid.value(col.checked_sub(c)?, row.checked_sub(r)?)
    }

    fn call(p: Value) -> Result<Value, crate::registry::RegistryError> {
        Registry::standard().call("terrain.viewshed_cumulative", p)
    }

    /// Accord cellule par cellule de la grille cumulée avec la somme des champs simples.
    fn assert_sum_of_singles(dem: &Dem, p: &TerrainViewshedCumulativeParams) -> Dem {
        let (grid, stats) = viewshed_cumulative(dem, p).unwrap();
        let singles: Vec<Dem> = p
            .observers
            .iter()
            .map(|o| viewshed(dem, &single(p, *o)).unwrap().0)
            .collect();
        let (mut visible, mut nodata) = (0u64, 0u64);
        for row in 0..dem.height {
            for col in 0..dem.width {
                let parts: Vec<f32> = singles
                    .iter()
                    .filter_map(|g| at(g, dem, col, row))
                    .collect();
                let expected = if parts.is_empty() {
                    None
                } else {
                    Some(parts.iter().sum::<f32>())
                };
                assert_eq!(at(&grid, dem, col, row), expected, "({col}, {row})");
                match expected {
                    Some(v) if v >= 1.0 => visible += 1,
                    _ => {}
                }
            }
        }
        for v in &grid.values {
            if *v == NODATA {
                nodata += 1;
            }
        }
        assert_eq!(stats.visible_cells, visible);
        assert_eq!(stats.nodata_cells, nodata);
        assert_eq!(stats.observer_count, p.observers.len() as u64);
        grid
    }

    /// Profil synthétique, constant du nord au sud : 51 colonnes de 10 m sur 5 lignes, coin
    /// haut-gauche (0, 50), plaine à 100 m. Au centre (colonne 25), un pic à flancs concaves
    /// culminant à 150 m ; de part et d'autre, un mur à 130 m (colonnes 7 et 43) et, derrière
    /// chaque mur, un creux à 90 m (colonnes 1 à 5 et 45 à 49). La colline du jeu d'essai,
    /// bombée, cache son propre sommet à tout observateur plus bas : d'où ce MNT.
    fn ridges() -> Dem {
        let profile = |c: i32| -> f32 {
            let spike = (1.0 - f64::from((c - 25).abs()) / 8.0).max(0.0);
            match c {
                7 | 43 => 130.0,
                1..=5 | 45..=49 => 90.0,
                _ => (100.0 + 50.0 * spike * spike) as f32,
            }
        };
        let mut values = Vec::with_capacity(51 * 5);
        for _ in 0..5 {
            values.extend((0..51).map(profile));
        }
        Dem {
            width: 51,
            height: 5,
            origin_x: 0.0,
            origin_y: 50.0,
            cell_size_x: 10.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values,
        }
    }

    #[test]
    fn two_observers_on_either_side_give_counts_zero_one_and_two() {
        let dem = ridges();
        // Observateurs aux colonnes 10 et 40, ligne 2 : de part et d'autre du sommet.
        let p = params(vec![pt(105.0, 25.0), pt(405.0, 25.0)], 1000.0);
        let (grid, stats) = viewshed_cumulative(&dem, &p).unwrap();
        // Les deux cercles de 1 km couvrent tout le MNT.
        assert_eq!(grid.extent(), dem.extent());
        assert_eq!((grid.width, grid.height, grid.epsg), (51, 5, 2154));
        assert_eq!(grid.nodata, Some(-9999.0));
        // Le sommet est vu des deux côtés.
        assert_eq!(grid.value(25, 2), Some(2.0));
        // Chaque versant éloigné n'est vu que de l'observateur de son côté.
        assert_eq!(grid.value(20, 2), Some(1.0));
        assert_eq!(grid.value(30, 2), Some(1.0));
        // Les creux derrière les murs ne sont vus de personne.
        for col in (1..=5).chain(45..=49) {
            assert_eq!(grid.value(col, 2), Some(0.0), "colonne {col}");
        }
        assert_eq!((stats.observer_count, stats.max_count), (2, 2));
        assert_eq!(stats.nodata_cells, 0);
        for count in [0.0, 1.0, 2.0] {
            assert!(grid.values.contains(&count), "compte {count}");
        }
        assert_sum_of_singles(&dem, &p);
    }

    #[test]
    fn every_cell_is_the_sum_of_the_single_viewsheds() {
        let dem = hill();
        assert_sum_of_singles(&dem, &params(west_east(), 1000.0));
        // Fenêtres rognées qui se chevauchent en partie, sans courbure, cible haute.
        let mut p = params(
            vec![
                pt(652155.0, 6850255.0),
                pt(652305.0, 6850205.0),
                pt(652250.0, 6850400.0),
            ],
            120.0,
        );
        p.curvature = false;
        p.target_height = m(3.0);
        assert_sum_of_singles(&dem, &p);
    }

    #[test]
    fn one_observer_matches_the_single_viewshed() {
        let dem = hill();
        for (o, r) in [
            (pt(652030.0, 6850250.0), 1000.0),
            (pt(652155.0, 6850255.0), 100.0),
            (pt(652030.0, 6850250.0), 1.0),
        ] {
            let p = params(vec![o], r);
            let (grid, stats) = viewshed_cumulative(&dem, &p).unwrap();
            let (simple, s) = viewshed(&dem, &single(&p, o)).unwrap();
            assert_eq!(grid, simple);
            assert_eq!(
                (stats.visible_cells, stats.nodata_cells, stats.max_count),
                (s.visible_cells, s.nodata_cells, 1)
            );
        }
    }

    #[test]
    fn disjoint_radii_cover_the_envelope_with_nodata_between() {
        let dem = hill();
        let p = params(vec![pt(652055.0, 6850255.0), pt(652445.0, 6850255.0)], 50.0);
        let grid = assert_sum_of_singles(&dem, &p);
        // Colonnes 0 à 10 pour l'un, 39 à 49 pour l'autre ; lignes 19 à 29.
        assert_eq!((grid.width, grid.height), (50, 11));
        assert_eq!((grid.origin_x, grid.origin_y), (652000.0, 6850310.0));
        for col in 11..39 {
            for row in 0..grid.height {
                assert_eq!(grid.value(col, row), None, "({col}, {row})");
            }
        }
        assert_eq!(grid.value(5, 5), Some(1.0));
        assert_eq!(grid.value(44, 5), Some(1.0));
    }

    #[test]
    fn a_repeated_observer_counts_twice() {
        let dem = hill();
        let o = pt(652030.0, 6850250.0);
        let (grid, stats) = viewshed_cumulative(&dem, &params(vec![o, o], 1000.0)).unwrap();
        let (simple, _) = viewshed(&dem, &single(&params(vec![o], 1000.0), o)).unwrap();
        for (a, b) in grid.values.iter().zip(&simple.values) {
            let doubled = if *b == NODATA { *b } else { *b * 2.0 };
            assert_eq!(*a, doubled);
        }
        assert_eq!(stats.max_count, 2);
    }

    fn hill_call(observers: Value, radius: Value, out: &Path) -> Value {
        json!({
            "dem": fixture("mnt-colline-2154.tif").to_string_lossy(),
            "observers": observers,
            "radius": radius,
            "out": out.to_string_lossy(),
        })
    }

    #[test]
    fn the_observer_list_is_bounded() {
        let dir = Scratch::new("bornes");
        let out = dir.join("vue.tif");
        let one = json!({"x": 652250, "y": 6850250, "epsg": 2154});
        let radius = json!({"value": 20, "unit": "m"});
        for n in [0usize, MAX_OBSERVERS + 1] {
            let list = Value::Array(vec![one.clone(); n]);
            let err = call(hill_call(list, radius.clone(), &out)).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{n}");
            assert!(err.to_string().contains("observers"), "{err}");
            assert!(!out.exists());
        }
        // 32 observateurs, la borne : accepté, chacun compté.
        let list = Value::Array(vec![one; MAX_OBSERVERS]);
        let r = call(hill_call(list, radius, &out)).unwrap();
        assert_eq!(r["observer_count"], 32);
        assert_eq!(r["max_count"], 32);
        assert!(out.exists());
        // Défauts : ceux de `terrain.viewshed`.
        let p: TerrainViewshedCumulativeParams =
            serde_json::from_value(json!({"dem": "mnt.tif", "observers": [{"x": 1, "y": 2, "epsg": 2154}]}))
                .unwrap();
        assert_eq!(
            (p.observer_height, p.target_height, p.curvature, p.refraction, p.radius),
            (m(1.6), m(0.0), true, 0.13, None)
        );
        // Champ inconnu, observateur sans système, liste absente : refusés.
        for bad in [
            json!({"dem": "mnt.tif", "observers": [{"x": 1, "y": 2, "epsg": 2154}], "observer": {"x": 1, "y": 2, "epsg": 2154}}),
            json!({"dem": "mnt.tif", "observers": [{"x": 1, "y": 2}]}),
            json!({"dem": "mnt.tif"}),
        ] {
            assert_eq!(call(bad).unwrap_err().code(), "registry.invalid_params");
        }
    }

    #[test]
    fn a_bad_observer_is_named_by_its_rank_and_writes_nothing() {
        let dir = Scratch::new("observateur");
        let out = dir.join("vue.tif");
        // MNT plat 7 × 7 de 10 m, troué en (3, 3).
        let mut values = vec![100.0f32; 49];
        values[3 * 7 + 3] = -9999.0;
        let holed = Dem {
            width: 7,
            height: 7,
            origin_x: 0.0,
            origin_y: 70.0,
            cell_size_x: 10.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values,
        };
        let input = dir.join("troue.tif");
        holed.write_geotiff(&input).unwrap();
        let hill = fixture("mnt-colline-2154.tif");
        let good_hill = json!({"x": 652250, "y": 6850250, "epsg": 2154});
        let good_holed = json!({"x": 5, "y": 35, "epsg": 2154});
        for (dem, first, second, code) in [
            (&hill, &good_hill, json!({"x": 2.3, "y": 48.7, "epsg": 4326}), "terrain.crs_mismatch"),
            (&hill, &good_hill, json!({"x": 651990, "y": 6850250, "epsg": 2154}), "terrain.outside_dem"),
            (&input, &good_holed, json!({"x": 35, "y": 35, "epsg": 2154}), "terrain.no_elevation"),
        ] {
            let err = call(json!({
                "dem": dem.to_string_lossy(),
                "observers": [first, second],
                // Un rayon trop grand : l'observateur fautif est jugé d'abord.
                "radius": {"value": 20, "unit": "km"},
                "out": out.to_string_lossy(),
            }))
            .unwrap_err();
            assert_eq!(err.code(), code);
            assert!(err.to_string().contains("observateur n° 2"), "{err}");
            assert!(!out.exists());
        }
    }

    #[test]
    fn a_too_large_radius_writes_nothing() {
        let dir = Scratch::new("trop-grand");
        let out = dir.join("vue.tif");
        let err = call(hill_call(
            json!([{"x": 652030, "y": 6850250, "epsg": 2154}, {"x": 652470, "y": 6850250, "epsg": 2154}]),
            json!({"value": 5001, "unit": "m"}),
            &out,
        ))
        .unwrap_err();
        assert_eq!(err.code(), "terrain.viewshed_too_large");
        assert!(!out.exists());
    }

    #[test]
    fn two_runs_give_identical_bytes() {
        let dir = Scratch::new("deux");
        let (a, b) = (dir.join("a.tif"), dir.join("b.tif"));
        let p = |out: &Path| {
            let mut v = hill_call(
                json!([{"x": 652030, "y": 6850250, "epsg": 2154}, {"x": 652470, "y": 6850250, "epsg": 2154}]),
                json!({"value": 1, "unit": "km"}),
                out,
            );
            v["observer_height"] = json!({"value": 2, "unit": "m"});
            v
        };
        let ra = call(p(&a)).unwrap();
        let rb = call(p(&b)).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        assert_eq!(ra, json!({
            "file": a.to_string_lossy(),
            "observer_count": 2,
            "max_count": rb["max_count"],
            "visible_cells": rb["visible_cells"],
            "nodata_cells": 0,
        }));
        let written = Dem::open(&a).unwrap();
        let (grid, _) = viewshed_cumulative(&hill(), &params(west_east(), 1000.0)).unwrap();
        assert_eq!(written, grid);
    }

    #[test]
    fn terrain_viewshed_cumulative_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("terrain.viewshed_cumulative").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert_eq!(spec.output_ext, ".tif");
        assert_eq!(spec.path_params, vec!["dem".to_string()]);
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        let violations = registry.check_contract();
        assert!(
            violations
                .iter()
                .all(|v| v.id != "terrain.viewshed_cumulative"),
            "{violations:?}"
        );
        let props = spec.output_schema["properties"].as_object().unwrap();
        let mut keys: Vec<&str> = props.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["file", "max_count", "nodata_cells", "observer_count", "visible_cells"]
        );
        let params = &spec.params_schema;
        let observers = &params["properties"]["observers"];
        assert_eq!(observers["maxItems"], json!(MAX_OBSERVERS));
        assert_eq!(observers["minItems"], json!(1));
        assert!(crate::point::is_point(&observers["items"], params));
        assert!(params["properties"]["radius"].get("default").is_none());
    }
}
