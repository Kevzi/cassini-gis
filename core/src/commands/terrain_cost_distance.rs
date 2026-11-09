//! Commande `terrain.cost_distance` : coût cumulé depuis une ou plusieurs sources sur un raster
//! de coût, calculé dans le noyau, sans GDAL ni GRASS (AD-1), par le module de coût unique
//! (`cost.rs`, AD-20).
//!
//! Le raster de coût est ouvert et le résultat écrit par `dem.rs`, l'unique porte raster du
//! noyau (AD-20) ; la sortie garde l'emprise, la cellule et l'EPSG du raster de coût, sans
//! reprojection ni rééchantillonnage (AD-19). Une cellule vaut le coût cumulé depuis la source
//! la moins coûteuse à atteindre, dans l'unité du raster multipliée par des mètres ; -9999 sur
//! l'infranchissable et l'inatteignable. Une source vaut 0 ; une source répétée ne change rien.
//!
//! La liste est bornée à 32 sources (AD-8), par le schéma et par la désérialisation. Toutes
//! les sources sont contrôlées avant tout calcul et toute écriture : la première fautive,
//! nommée par son rang (« source n° 2 »), arrête tout. Pas de parallélisme : ordre et octets
//! figés.
//!
//! Coût : un parcours de Dijkstra sur la grille, sans annulation. Mémoire, en plus du raster :
//! 12 octets par cellule (coût cumulé `f64`, prédécesseur `u32`), 4 octets par cellule pour la
//! grille écrite, qui coexiste avec le cumul, et au pire 8 entrées de 16 octets par cellule
//! franchissable dans le tas, soit au pire 144 octets par cellule : environ 36 Gio au plafond de
//! lecture de 1 Gio de `dem.rs` (2^28 cellules). En pratique une cellule entre rarement plus de
//! deux ou trois fois au tas, et un raster d'une commune à 5 m (4 millions de cellules) se traite
//! en quelques secondes.

use std::path::Path;

use schemars::JsonSchema;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::commands::terrain_line_of_sight::{check_crs, check_extent};
use crate::cost::{accumulate, passable};
use crate::dem::{Dem, NODATA};
use crate::horn::same_grid_as;
use crate::point::Point;
use crate::raster_path::RasterPath;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

/// Nombre maximal de sources (AD-8).
pub const MAX_SOURCES: usize = 32;

fn sources<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Point>, D::Error> {
    let sources = Vec::<Point>::deserialize(d)?;
    if sources.is_empty() || sources.len() > MAX_SOURCES {
        return Err(D::Error::custom(format!(
            "`sources` demande de 1 à {MAX_SOURCES} points, pas {}",
            sources.len()
        )));
    }
    Ok(sources)
}

/// Paramètres de `terrain.cost_distance`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerrainCostDistanceParams {
    /// Raster de coût (GeoTIFF à une bande, système projeté en mètres) : coût par mètre
    /// parcouru, cellule sans donnée, nulle ou négative infranchissable. Chemin relatif au
    /// projet recopié de l'inventaire. Cost raster file path, relative to the project.
    pub cost: RasterPath,
    /// Sources, de 1 à 32, dans le système du raster de coût : leur coût cumulé est nul.
    #[serde(deserialize_with = "sources")]
    #[schemars(length(min = 1, max = MAX_SOURCES))]
    pub sources: Vec<Point>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `terrain.cost_distance`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TerrainCostDistanceOutput {
    /// GeoTIFF Float32 du coût cumulé écrit (unité du raster × mètres), sans-donnée -9999 sur
    /// l'infranchissable et l'inatteignable.
    pub file: String,
    /// Coût cumulé le plus élevé atteint.
    pub max: Option<f64>,
    /// Nombre de cellules atteintes, sources comprises.
    pub reachable_cells: u64,
    /// Nombre de cellules franchissables mais inatteignables depuis toute source.
    pub unreachable_cells: u64,
}

/// Comptes d'un coût cumulé.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CostStats {
    pub max: Option<f64>,
    pub reachable_cells: u64,
    pub unreachable_cells: u64,
}

fn refusal(code: &str, message: String) -> CommandFailure {
    CommandFailure::new(code, message)
}

/// Cellule (colonne, ligne) d'un point déjà dans l'emprise : le bord droit et le bord bas de
/// l'emprise appartiennent à la dernière cellule.
pub(crate) fn cell_of(dem: &Dem, p: &Point) -> (u32, u32) {
    let col = ((p.x - dem.origin_x) / dem.cell_size_x)
        .floor()
        .clamp(0.0, f64::from(dem.width - 1)) as u32;
    let row = ((dem.origin_y - p.y) / dem.cell_size_y)
        .floor()
        .clamp(0.0, f64::from(dem.height - 1)) as u32;
    (col, row)
}

/// Contrôle un point du raster de coût, dans cet ordre : système, emprise, cellule
/// franchissable ; rend sa cellule. `name` le nomme dans les messages (« source n° 2 »,
/// « départ », « arrivée »).
pub(crate) fn locate(cost: &Dem, p: &Point, name: &str) -> Result<(u32, u32), CommandFailure> {
    check_crs(cost, p, name)?;
    check_extent(cost, p, name)?;
    let (col, row) = cell_of(cost, p);
    if passable(cost, col, row).is_none() {
        return Err(refusal(
            "terrain.impassable_source",
            format!(
                "Le point {name} ({}, {}) tombe sur une cellule infranchissable du raster de coût (sans donnée, nulle ou négative) : déplacez-le sur une cellule de coût positif.",
                p.x, p.y
            ),
        ));
    }
    Ok((col, row))
}

/// Nom d'une source dans les messages : « source n° 2 », rang compté de 1.
fn rank_name(i: usize) -> String {
    format!("source n° {}", i + 1)
}

/// Coût cumulé sur un raster de coût déjà ouvert. Fonction pure et déterministe.
pub fn cost_distance(cost: &Dem, sources: &[Point]) -> Result<(Dem, CostStats), CommandFailure> {
    let mut cells = Vec::with_capacity(sources.len());
    for (i, s) in sources.iter().enumerate() {
        cells.push(locate(cost, s, &rank_name(i))?);
    }
    let acc = accumulate(cost, &cells);
    let (w, h) = (cost.width, cost.height);
    let mut values = Vec::with_capacity(acc.cost.len());
    let (mut max, mut reachable, mut unreachable) = (None::<f64>, 0u64, 0u64);
    for row in 0..h {
        for col in 0..w {
            match acc.at(col, row) {
                Some(v) => {
                    reachable += 1;
                    max = Some(max.map_or(v, |m| m.max(v)));
                    values.push(v as f32);
                }
                None => {
                    if passable(cost, col, row).is_some() {
                        unreachable += 1;
                    }
                    values.push(NODATA);
                }
            }
        }
    }
    drop(acc);
    let stats = CostStats {
        max,
        reachable_cells: reachable,
        unreachable_cells: unreachable,
    };
    Ok((same_grid_as(cost, values, NODATA), stats))
}

/// `terrain.cost_distance` : commande du noyau.
pub struct TerrainCostDistance;

impl Command for TerrainCostDistance {
    type Params = TerrainCostDistanceParams;
    type Output = TerrainCostDistanceOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<TerrainCostDistanceParams, TerrainCostDistanceOutput>(
            "terrain.cost_distance",
            "Coût de déplacement cumulé depuis un ou plusieurs points sur un raster de coût (fichier GeoTIFF désigné par `cost`, chemin de fichier recopié de l'inventaire ou sortie d'une étape, pas un id de couche) : chaque cellule reçoit le coût du trajet le moins coûteux depuis la source la plus proche, de 1 à 32 sources, au-delà refus. La valeur d'une cellule est un coût par mètre parcouru ; une cellule sans donnée, nulle ou négative est infranchissable. Déplacement aux 8 voisins, au coût moyen des deux cellules multiplié par la distance ; un pas en diagonale demande que les deux cellules qu'il frôle soient franchissables. Les sources sont dans le système du raster. Écrit un nouveau GeoTIFF du coût cumulé, sans donnée sur l'infranchissable et l'inatteignable. Calcul non annulable. Ne modifie pas le raster de coût.",
            "Accumulated travel cost from one or more points on a cost raster (GeoTIFF file given by `cost`, a file path copied from the inventory or a step output, not a layer id): each cell gets the cost of the cheapest route from the nearest source, from 1 to 32 sources, beyond that refused. A cell value is a cost per metre travelled; a no-data, zero or negative cell is impassable. Moves to the 8 neighbours, at the mean cost of the two cells times the distance; a diagonal step needs both cells it brushes to be passable. Sources are in the raster's coordinate system. Writes a new GeoTIFF of the accumulated cost, no data on impassable and unreachable cells. Not cancellable. Does not modify the cost raster.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["cost"])
        .output_params(&["out"])
        .output_ext(".tif")
        // Style dicté après l'ouverture du raster (bogue du fond noir, 2026-11-09).
        .result_style(crate::style::ResultStyle::CostDistance)
        .example(serde_json::json!({
            "cost": "mnt-colline-2154.tif",
            "sources": [{"x": 652250, "y": 6850250, "epsg": 2154}]
        }))
        .example(serde_json::json!({
            "cost": "mnt-plan-incline-2154.tif",
            "sources": [
                {"x": 652005, "y": 6850295, "epsg": 2154},
                {"x": 652395, "y": 6850005, "epsg": 2154}
            ]
        }))
        .tests(&[
            "a_uniform_cost_gives_the_distance_from_the_source",
            "several_sources_give_the_minimum_and_zero_at_each_source",
            "an_enclosed_zone_is_counted_unreachable",
            "the_source_list_is_bounded",
            "a_bad_source_is_named_by_its_rank_and_writes_nothing",
            "two_runs_give_identical_bytes",
            "terrain_cost_distance_runs_in_the_core_and_the_contract_accepts_it",
        ])
    }

    fn run(
        &self,
        params: TerrainCostDistanceParams,
    ) -> Result<TerrainCostDistanceOutput, CommandFailure> {
        let cost = Dem::open(Path::new(params.cost.as_str())).map_err(CommandFailure::from)?;
        let (grid, stats) = cost_distance(&cost, &params.sources)?;
        drop(cost);
        grid.write_geotiff(Path::new(&params.out))
            .map_err(CommandFailure::from)?;
        Ok(TerrainCostDistanceOutput {
            file: params.out,
            max: stats.max,
            reachable_cells: stats.reachable_cells,
            unreachable_cells: stats.unreachable_cells,
        })
    }
}

inventory::submit! { Registration(|r| r.register(TerrainCostDistance)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::{fixture, Scratch};
    use crate::cost;
    use crate::registry::Registry;
    use serde_json::{json, Value};

    fn pt(x: f64, y: f64) -> Point {
        Point { x, y, epsg: 2154 }
    }

    /// Grille de 10 m, coin haut-gauche (0, h × 10), sans-donnée -9999.
    fn grid(width: u32, height: u32, values: Vec<f32>) -> Dem {
        Dem {
            width,
            height,
            origin_x: 0.0,
            origin_y: f64::from(height) * 10.0,
            cell_size_x: 10.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values,
        }
    }

    /// Centre de la cellule (`col`, `row`) d'une grille de `grid`.
    fn centre(dem: &Dem, col: u32, row: u32) -> Point {
        pt(
            dem.origin_x + (f64::from(col) + 0.5) * dem.cell_size_x,
            dem.origin_y - (f64::from(row) + 0.5) * dem.cell_size_y,
        )
    }

    fn call(p: Value) -> Result<Value, crate::registry::RegistryError> {
        Registry::standard().call("terrain.cost_distance", p)
    }

    #[test]
    fn a_uniform_cost_gives_the_distance_from_the_source() {
        let dem = grid(9, 5, vec![2.0; 45]);
        let (out, stats) = cost_distance(&dem, &[centre(&dem, 1, 2)]).unwrap();
        assert_eq!(out.extent(), dem.extent());
        assert_eq!((out.width, out.height, out.epsg), (9, 5, 2154));
        assert_eq!(out.nodata, Some(-9999.0));
        assert_eq!(out.value(1, 2), Some(0.0));
        // Six cellules de 10 m à 2 par mètre.
        assert_eq!(out.value(7, 2), Some(120.0));
        // Une diagonale : 2 × 10 × √2.
        let d = out.value(2, 3).unwrap();
        assert!((f64::from(d) - 20.0 * std::f64::consts::SQRT_2).abs() < 1e-4);
        assert_eq!((stats.reachable_cells, stats.unreachable_cells), (45, 0));
        let max = out.values.iter().copied().fold(f32::MIN, f32::max);
        assert_eq!(stats.max.map(|m| m as f32), Some(max));
        // Un point sur le bord droit de l'emprise appartient à la dernière colonne.
        assert_eq!(cell_of(&dem, &pt(90.0, 0.0)), (8, 4));
    }

    #[test]
    fn several_sources_give_the_minimum_and_zero_at_each_source() {
        let values = (0..60).map(|i| 1.0 + (i % 5) as f32).collect();
        let dem = grid(10, 6, values);
        let (a, b) = (centre(&dem, 0, 0), centre(&dem, 8, 4));
        let (both, _) = cost_distance(&dem, &[a, b]).unwrap();
        let (only_a, _) = cost_distance(&dem, &[a]).unwrap();
        let (only_b, _) = cost_distance(&dem, &[b]).unwrap();
        for i in 0..both.values.len() {
            assert_eq!(both.values[i], only_a.values[i].min(only_b.values[i]), "{i}");
        }
        assert_eq!(both.value(0, 0), Some(0.0));
        assert_eq!(both.value(8, 4), Some(0.0));
        // La grille écrite est celle du module de coût, rendue en Float32.
        let acc = cost::accumulate(&dem, &[(0, 0), (8, 4)]);
        let expected: Vec<f32> = acc.cost.iter().map(|&v| v as f32).collect();
        assert_eq!(both.values, expected);
    }

    #[test]
    fn an_enclosed_zone_is_counted_unreachable() {
        // Anneau de 0 autour de 3 × 3 cellules franchissables ; un trou de NoData dehors.
        let (w, h) = (10u32, 6u32);
        let mut values = vec![1.0f32; (w * h) as usize];
        for row in 0..=4 {
            for col in 4..=8 {
                if row == 0 || row == 4 || col == 4 || col == 8 {
                    values[(row * w + col) as usize] = 0.0;
                }
            }
        }
        values[0] = -9999.0;
        let dem = grid(w, h, values);
        let (out, stats) = cost_distance(&dem, &[centre(&dem, 0, 5)]).unwrap();
        for row in 1..=3 {
            for col in 5..=7 {
                assert_eq!(out.value(col, row), None, "({col}, {row})");
            }
        }
        // 60 cellules : 16 du mur, 1 trou, 9 enfermées.
        assert_eq!(stats.unreachable_cells, 9);
        assert_eq!(stats.reachable_cells, 60 - 16 - 1 - 9);
        let nodata = out.values.iter().filter(|v| **v == NODATA).count();
        assert_eq!(nodata, 16 + 1 + 9);
    }

    fn hill_call(sources: Value, out: &Path) -> Value {
        json!({
            "cost": fixture("mnt-colline-2154.tif").to_string_lossy(),
            "sources": sources,
            "out": out.to_string_lossy(),
        })
    }

    #[test]
    fn the_source_list_is_bounded() {
        let dir = Scratch::new("bornes");
        let out = dir.join("cout.tif");
        let one = json!({"x": 652250, "y": 6850250, "epsg": 2154});
        for n in [0usize, MAX_SOURCES + 1] {
            let list = Value::Array(vec![one.clone(); n]);
            let err = call(hill_call(list, &out)).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{n}");
            assert!(err.to_string().contains("sources"), "{err}");
            assert!(!out.exists());
        }
        // 32 sources, la borne : accepté ; une source répétée ne change rien.
        let r32 = call(hill_call(Value::Array(vec![one.clone(); MAX_SOURCES]), &out)).unwrap();
        let bytes32 = std::fs::read(&out).unwrap();
        let out1 = dir.join("un.tif");
        let r1 = call(hill_call(json!([one]), &out1)).unwrap();
        assert_eq!(bytes32, std::fs::read(&out1).unwrap());
        assert_eq!(r32["max"], r1["max"]);
        assert_eq!(r32["reachable_cells"], 2500);
        // Champ inconnu, point sans système, liste absente : refusés.
        for bad in [
            json!({"cost": "c.tif", "sources": [{"x": 1, "y": 2, "epsg": 2154}], "source": {"x": 1, "y": 2, "epsg": 2154}}),
            json!({"cost": "c.tif", "sources": [{"x": 1, "y": 2}]}),
            json!({"cost": "c.tif"}),
        ] {
            assert_eq!(call(bad).unwrap_err().code(), "registry.invalid_params");
        }
    }

    #[test]
    fn a_bad_source_is_named_by_its_rank_and_writes_nothing() {
        let dir = Scratch::new("source");
        let out = dir.join("cout.tif");
        // Raster 7 × 7 de 10 m : trou en (3, 3), zéro en (5, 3), négatif en (1, 3).
        let mut values = vec![1.0f32; 49];
        values[3 * 7 + 3] = -9999.0;
        values[3 * 7 + 5] = 0.0;
        values[3 * 7 + 1] = -2.0;
        let input = dir.join("cout-entree.tif");
        grid(7, 7, values).write_geotiff(&input).unwrap();
        let good = json!({"x": 5, "y": 65, "epsg": 2154});
        for (second, code) in [
            (json!({"x": 2.3, "y": 48.7, "epsg": 4326}), "terrain.crs_mismatch"),
            (json!({"x": 75, "y": 35, "epsg": 2154}), "terrain.outside_dem"),
            (json!({"x": 35, "y": 35, "epsg": 2154}), "terrain.impassable_source"),
            (json!({"x": 55, "y": 35, "epsg": 2154}), "terrain.impassable_source"),
            (json!({"x": 15, "y": 35, "epsg": 2154}), "terrain.impassable_source"),
        ] {
            let err = call(json!({
                "cost": input.to_string_lossy(),
                "sources": [good, second],
                "out": out.to_string_lossy(),
            }))
            .unwrap_err();
            assert_eq!(err.code(), code);
            assert!(err.to_string().contains("source n° 2"), "{err}");
            assert!(!out.exists());
        }
        // Un raster illisible garde l'erreur du module raster.
        let err = call(json!({
            "cost": dir.join("absent.tif").to_string_lossy(),
            "sources": [good],
            "out": out.to_string_lossy(),
        }))
        .unwrap_err();
        assert_eq!(err.code(), "dem.unreadable");
        assert!(!out.exists());
    }

    #[test]
    fn two_runs_give_identical_bytes() {
        let dir = Scratch::new("deux");
        let (a, b) = (dir.join("a.tif"), dir.join("b.tif"));
        let sources = json!([
            {"x": 652030, "y": 6850250, "epsg": 2154},
            {"x": 652470, "y": 6850250, "epsg": 2154}
        ]);
        let ra = call(hill_call(sources.clone(), &a)).unwrap();
        let rb = call(hill_call(sources, &b)).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        assert_eq!(
            ra,
            json!({
                "file": a.to_string_lossy(),
                "max": rb["max"],
                "reachable_cells": 2500,
                "unreachable_cells": 0,
            })
        );
        let hill = Dem::open(&fixture("mnt-colline-2154.tif")).unwrap();
        let (grid, stats) =
            cost_distance(&hill, &[pt(652030.0, 6850250.0), pt(652470.0, 6850250.0)]).unwrap();
        assert_eq!(Dem::open(&a).unwrap(), grid);
        assert_eq!(ra["max"], json!(stats.max.unwrap()));
    }

    #[test]
    fn terrain_cost_distance_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("terrain.cost_distance").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert_eq!(spec.output_ext, ".tif");
        assert_eq!(spec.path_params, vec!["cost".to_string()]);
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        let violations = registry.check_contract();
        assert!(
            violations.iter().all(|v| v.id != "terrain.cost_distance"),
            "{violations:?}"
        );
        let props = spec.output_schema["properties"].as_object().unwrap();
        let mut keys: Vec<&str> = props.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["file", "max", "reachable_cells", "unreachable_cells"]);
        let params = &spec.params_schema;
        let sources = &params["properties"]["sources"];
        assert_eq!(sources["maxItems"], json!(MAX_SOURCES));
        assert_eq!(sources["minItems"], json!(1));
        assert!(crate::point::is_point(&sources["items"], params));
    }
}
