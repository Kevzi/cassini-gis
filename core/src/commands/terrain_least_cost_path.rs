//! Commande `terrain.least_cost_path` : chemin de moindre coût du départ à l'arrivée sur un
//! raster de coût, calculé dans le noyau, sans GDAL ni GRASS (AD-1), par le module de coût
//! unique (`cost.rs`, AD-20), et écrit en GeoJSON d'une ligne par l'écrivain unique du noyau
//! (`geojson.rs`, AD-30).
//!
//! Mêmes règles que `terrain.cost_distance` : le coût est cumulé depuis le départ, puis le
//! chemin remonté de l'arrivée au départ par les prédécesseurs, et écrit du départ à l'arrivée,
//! dans cet ordre. Sommets aux centres des cellules, de la cellule de départ à celle
//! d'arrivée, les sommets intermédiaires alignés (même pas de grille avant et après) retirés.
//! Départ et arrivée dans la même cellule : une ligne de deux sommets confondus, de coût et de
//! longueur nuls. `total_cost` vaut le coût cumulé à la cellule d'arrivée, celui qu'écrirait
//! `terrain.cost_distance` depuis le départ seul ; `length` est la longueur de la ligne écrite,
//! en mètres, dans le système du raster, sans reprojection (AD-19).
//!
//! Coût et mémoire : ceux de `terrain.cost_distance` (au pire 144 octets par cellule), sans
//! annulation ni parallélisme, pour un ordre et des octets figés.

use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::commands::analysis_buffer::{Distance, DistanceUnit};
use crate::commands::terrain_cost_distance::locate;
use crate::cost::{accumulate, path};
use crate::dem::Dem;
use crate::geojson::{self, Feature, Geometry, Property};
use crate::point::Point;
use crate::raster_path::RasterPath;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

/// Paramètres de `terrain.least_cost_path`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerrainLeastCostPathParams {
    /// Raster de coût (GeoTIFF à une bande, système projeté en mètres) : coût par mètre
    /// parcouru, cellule sans donnée, nulle ou négative infranchissable. Chemin relatif au
    /// projet recopié de l'inventaire. Cost raster file path, relative to the project.
    pub cost: RasterPath,
    /// Point de départ, dans le système du raster de coût.
    pub start: Point,
    /// Point d'arrivée, dans le système du raster de coût.
    pub end: Point,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `terrain.least_cost_path`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TerrainLeastCostPathOutput {
    /// GeoJSON écrit : une ligne du départ à l'arrivée (format de 2008, `crs` du raster),
    /// propriétés `total_cost` et `length`.
    pub file: String,
    /// Nombre d'entités écrites : toujours 1.
    pub feature_count: u64,
    /// Coût total du chemin, dans l'unité du raster multipliée par des mètres.
    pub total_cost: f64,
    /// Longueur de la ligne, en mètres.
    pub length: Distance,
}

/// Chemin de moindre coût trouvé : sommets de la ligne, coût total et longueur en mètres.
#[derive(Debug, Clone, PartialEq)]
pub struct LeastCostPath {
    pub vertices: Vec<(f64, f64)>,
    pub total_cost: f64,
    pub length: f64,
}

/// Chemin de moindre coût sur un raster de coût déjà ouvert. Fonction pure et déterministe.
pub fn least_cost_path(
    cost: &Dem,
    start: &Point,
    end: &Point,
) -> Result<LeastCostPath, CommandFailure> {
    let from = locate(cost, start, "départ")?;
    let to = locate(cost, end, "arrivée")?;
    let acc = accumulate(cost, &[from]);
    let cells = path(&acc, to);
    let Some(total_cost) = acc.at(to.0, to.1).filter(|_| !cells.is_empty()) else {
        return Err(CommandFailure::new(
            "terrain.no_path",
            format!(
                "L'arrivée ({}, {}) est inatteignable depuis le départ ({}, {}) : toute route traverse des cellules infranchissables du raster de coût.",
                end.x, end.y, start.x, start.y
            ),
        ));
    };
    drop(acc);
    // Sommets gardés : extrémités, et chaque cellule où le pas de grille change.
    let mut kept: Vec<(u32, u32)> = Vec::new();
    for (k, &cell) in cells.iter().enumerate() {
        if k > 0 && k + 1 < cells.len() {
            let (p, n) = (cells[k - 1], cells[k + 1]);
            let before = (i64::from(cell.0) - i64::from(p.0), i64::from(cell.1) - i64::from(p.1));
            let after = (i64::from(n.0) - i64::from(cell.0), i64::from(n.1) - i64::from(cell.1));
            if before == after {
                continue;
            }
        }
        kept.push(cell);
    }
    if kept.len() == 1 {
        kept.push(kept[0]);
    }
    let vertices: Vec<(f64, f64)> = kept
        .iter()
        .map(|&(c, r)| {
            (
                cost.origin_x + (f64::from(c) + 0.5) * cost.cell_size_x,
                cost.origin_y - (f64::from(r) + 0.5) * cost.cell_size_y,
            )
        })
        .collect();
    let length = vertices
        .windows(2)
        .map(|s| (s[1].0 - s[0].0).hypot(s[1].1 - s[0].1))
        .sum();
    Ok(LeastCostPath {
        vertices,
        total_cost,
        length,
    })
}

/// `terrain.least_cost_path` : commande du noyau.
pub struct TerrainLeastCostPath;

impl Command for TerrainLeastCostPath {
    type Params = TerrainLeastCostPathParams;
    type Output = TerrainLeastCostPathOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<TerrainLeastCostPathParams, TerrainLeastCostPathOutput>(
            "terrain.least_cost_path",
            "Chemin de moindre coût d'un point de départ à un point d'arrivée sur un raster de coût (fichier GeoTIFF désigné par `cost`, chemin de fichier recopié de l'inventaire ou sortie d'une étape, pas un id de couche) : la route la moins coûteuse, aux 8 voisins, au coût moyen des deux cellules multiplié par la distance ; un pas en diagonale demande que les deux cellules qu'il frôle soient franchissables. La valeur d'une cellule est un coût par mètre parcouru ; une cellule sans donnée, nulle ou négative est infranchissable. Départ et arrivée sont dans le système du raster. Écrit un GeoJSON d'une ligne du départ à l'arrivée, avec son coût total et sa longueur en mètres ; refus si l'arrivée est inatteignable. Calcul non annulable. Ne modifie pas le raster de coût.",
            "Least-cost path from a start point to an end point on a cost raster (GeoTIFF file given by `cost`, a file path copied from the inventory or a step output, not a layer id): the cheapest route, to the 8 neighbours, at the mean cost of the two cells times the distance; a diagonal step needs both cells it brushes to be passable. A cell value is a cost per metre travelled; a no-data, zero or negative cell is impassable. Start and end are in the raster's coordinate system. Writes a GeoJSON of one line from start to end, with its total cost and its length in metres; refused if the end cannot be reached. Not cancellable. Does not modify the cost raster.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["cost"])
        .output_params(&["out"])
        .output_ext(".geojson")
        .example(serde_json::json!({
            "cost": "mnt-colline-2154.tif",
            "start": {"x": 652030, "y": 6850250, "epsg": 2154},
            "end": {"x": 652470, "y": 6850250, "epsg": 2154}
        }))
        .example(serde_json::json!({
            "cost": "mnt-plan-incline-2154.tif",
            "start": {"x": 652005, "y": 6850295, "epsg": 2154},
            "end": {"x": 652395, "y": 6850005, "epsg": 2154}
        }))
        .tests(&[
            "a_uniform_cost_gives_a_straight_line_whose_cost_is_its_length",
            "the_path_goes_around_a_wall_through_its_opening",
            "the_total_cost_is_the_accumulated_cost_at_the_end",
            "an_unreachable_end_is_no_path_and_writes_nothing",
            "a_bad_endpoint_is_named_and_writes_nothing",
            "the_geojson_is_one_line_from_start_to_end_in_the_raster_crs",
            "two_runs_give_identical_bytes",
            "terrain_least_cost_path_runs_in_the_core_and_the_contract_accepts_it",
        ])
    }

    fn run(
        &self,
        params: TerrainLeastCostPathParams,
    ) -> Result<TerrainLeastCostPathOutput, CommandFailure> {
        let cost = Dem::open(Path::new(params.cost.as_str())).map_err(CommandFailure::from)?;
        let found = least_cost_path(&cost, &params.start, &params.end)?;
        let feature = Feature {
            geometry: Geometry::LineString(found.vertices),
            properties: vec![
                ("total_cost", Property::Number(found.total_cost)),
                ("length", Property::Number(found.length)),
            ],
        };
        geojson::write(Path::new(&params.out), cost.epsg, &[feature])
            .map_err(CommandFailure::from)?;
        Ok(TerrainLeastCostPathOutput {
            file: params.out,
            feature_count: 1,
            total_cost: found.total_cost,
            length: Distance {
                value: found.length,
                unit: DistanceUnit::M,
            },
        })
    }
}

inventory::submit! { Registration(|r| r.register(TerrainLeastCostPath)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::{fixture, Scratch};
    use crate::commands::terrain_cost_distance::cost_distance;
    use crate::cost::passable;
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

    fn centre(dem: &Dem, col: u32, row: u32) -> Point {
        pt(
            dem.origin_x + (f64::from(col) + 0.5) * dem.cell_size_x,
            dem.origin_y - (f64::from(row) + 0.5) * dem.cell_size_y,
        )
    }

    /// Cellule (colonne, ligne) d'un sommet placé au centre d'une cellule.
    fn cell(dem: &Dem, v: (f64, f64)) -> (u32, u32) {
        (
            ((v.0 - dem.origin_x) / dem.cell_size_x).floor() as u32,
            ((dem.origin_y - v.1) / dem.cell_size_y).floor() as u32,
        )
    }

    /// Mur à la colonne 4, de NoData en haut et de 0 en bas, ouvert à la ligne 6.
    fn walled() -> Dem {
        let (w, h) = (9u32, 8u32);
        let mut values = vec![1.0f32; (w * h) as usize];
        for row in 0..h {
            if row != 6 {
                values[(row * w + 4) as usize] = if row < 3 { -9999.0 } else { 0.0 };
            }
        }
        grid(w, h, values)
    }

    fn call(p: Value) -> Result<Value, crate::registry::RegistryError> {
        Registry::standard().call("terrain.least_cost_path", p)
    }

    #[test]
    fn a_uniform_cost_gives_a_straight_line_whose_cost_is_its_length() {
        let dem = grid(9, 5, vec![1.0; 45]);
        let found = least_cost_path(&dem, &pt(12.0, 27.0), &pt(78.0, 21.0)).unwrap();
        // Sommets aux centres des cellules de départ et d'arrivée, rien entre eux.
        assert_eq!(found.vertices, vec![(15.0, 25.0), (75.0, 25.0)]);
        assert_eq!(found.length, 60.0);
        assert_eq!(found.total_cost, found.length);
        // Même cellule : deux sommets confondus, coût et longueur nuls.
        let same = least_cost_path(&dem, &pt(12.0, 27.0), &pt(14.0, 21.0)).unwrap();
        assert_eq!(same.vertices, vec![(15.0, 25.0), (15.0, 25.0)]);
        assert_eq!((same.length, same.total_cost), (0.0, 0.0));
    }

    #[test]
    fn the_path_goes_around_a_wall_through_its_opening() {
        let dem = walled();
        let found = least_cost_path(&dem, &centre(&dem, 1, 1), &centre(&dem, 7, 1)).unwrap();
        let cells: Vec<(u32, u32)> = found.vertices.iter().map(|&v| cell(&dem, v)).collect();
        assert_eq!(cells.first(), Some(&(1, 1)));
        assert_eq!(cells.last(), Some(&(7, 1)));
        // Chaque segment suit un pas de grille constant : ses cellules sont toutes
        // franchissables, et l'une d'elles est l'ouverture.
        let mut through = false;
        for s in cells.windows(2) {
            let (dc, dr) = (
                i64::from(s[1].0) - i64::from(s[0].0),
                i64::from(s[1].1) - i64::from(s[0].1),
            );
            let n = dc.abs().max(dr.abs());
            assert!(dc.abs() == dr.abs() || dc == 0 || dr == 0, "{s:?}");
            for k in 0..=n {
                let c = (i64::from(s[0].0) + dc / n.max(1) * k) as u32;
                let r = (i64::from(s[0].1) + dr / n.max(1) * k) as u32;
                assert!(passable(&dem, c, r).is_some(), "({c}, {r})");
                through |= (c, r) == (4, 6);
            }
        }
        assert!(through, "{cells:?}");
        assert!(found.length > 60.0);
        assert!((found.total_cost - found.length).abs() < 1e-9);
    }

    #[test]
    fn the_total_cost_is_the_accumulated_cost_at_the_end() {
        let values = (0..80).map(|i| 1.0 + ((i * 7) % 5) as f32).collect();
        let dem = grid(10, 8, values);
        let (start, end) = (centre(&dem, 0, 7), centre(&dem, 9, 0));
        let found = least_cost_path(&dem, &start, &end).unwrap();
        let (grid_out, _) = cost_distance(&dem, &[start]).unwrap();
        assert_eq!(found.total_cost as f32, grid_out.value(9, 0).unwrap());
        let acc = accumulate(&dem, &[(0, 7)]);
        assert_eq!(Some(found.total_cost), acc.at(9, 0));
        // La ligne va du départ à l'arrivée, dans cet ordre.
        assert_eq!(found.vertices.first(), Some(&(5.0, 5.0)));
        assert_eq!(found.vertices.last(), Some(&(95.0, 75.0)));
    }

    #[test]
    fn an_unreachable_end_is_no_path_and_writes_nothing() {
        let dir = Scratch::new("inatteignable");
        let out = dir.join("chemin.geojson");
        // Anneau de NoData autour des cellules (5..=7, 1..=3).
        let (w, h) = (10u32, 6u32);
        let mut values = vec![1.0f32; (w * h) as usize];
        for row in 0..=4 {
            for col in 4..=8 {
                if row == 0 || row == 4 || col == 4 || col == 8 {
                    values[(row * w + col) as usize] = -9999.0;
                }
            }
        }
        let input = dir.join("cout.tif");
        grid(w, h, values).write_geotiff(&input).unwrap();
        let err = call(json!({
            "cost": input.to_string_lossy(),
            "start": {"x": 5, "y": 5, "epsg": 2154},
            "end": {"x": 65, "y": 35, "epsg": 2154},
            "out": out.to_string_lossy(),
        }))
        .unwrap_err();
        assert_eq!(err.code(), "terrain.no_path");
        assert!(!out.exists());
    }

    #[test]
    fn a_bad_endpoint_is_named_and_writes_nothing() {
        let dir = Scratch::new("extremite");
        let out = dir.join("chemin.geojson");
        let input = dir.join("cout.tif");
        walled().write_geotiff(&input).unwrap();
        let good = json!({"x": 15, "y": 65, "epsg": 2154});
        let cases = [
            (json!({"x": 2.3, "y": 48.7, "epsg": 4326}), "terrain.crs_mismatch"),
            (json!({"x": 95, "y": 65, "epsg": 2154}), "terrain.outside_dem"),
            (json!({"x": 45, "y": 75, "epsg": 2154}), "terrain.impassable_source"),
            (json!({"x": 45, "y": 35, "epsg": 2154}), "terrain.impassable_source"),
        ];
        for (bad, code) in cases {
            for (start, end, name) in [(&bad, &good, "départ"), (&good, &bad, "arrivée")] {
                let err = call(json!({
                    "cost": input.to_string_lossy(),
                    "start": start,
                    "end": end,
                    "out": out.to_string_lossy(),
                }))
                .unwrap_err();
                assert_eq!(err.code(), code, "{name}");
                assert!(err.to_string().contains(name), "{err}");
                assert!(!out.exists());
            }
        }
    }

    #[test]
    fn the_geojson_is_one_line_from_start_to_end_in_the_raster_crs() {
        let dir = Scratch::new("geojson");
        let out = dir.join("chemin.geojson");
        let r = call(json!({
            "cost": fixture("mnt-plan-incline-2154.tif").to_string_lossy(),
            "start": {"x": 652005, "y": 6850295, "epsg": 2154},
            "end": {"x": 652395, "y": 6850005, "epsg": 2154},
            "out": out.to_string_lossy(),
        }))
        .unwrap();
        let written: Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
        assert_eq!(written["type"], "FeatureCollection");
        assert_eq!(
            written["crs"]["properties"]["name"],
            "urn:ogc:def:crs:EPSG::2154"
        );
        let features = written["features"].as_array().unwrap();
        assert_eq!(features.len(), 1);
        let geometry = &features[0]["geometry"];
        assert_eq!(geometry["type"], "LineString");
        let coords = geometry["coordinates"].as_array().unwrap();
        assert_eq!(coords.first().unwrap(), &json!([652005.0, 6850295.0]));
        assert_eq!(coords.last().unwrap(), &json!([652395.0, 6850005.0]));
        let props = &features[0]["properties"];
        assert_eq!(props["total_cost"], r["total_cost"]);
        assert_eq!(props["length"], r["length"]["value"]);
        assert_eq!(r["length"]["unit"], "m");
        assert_eq!(r["feature_count"], 1);
        assert_eq!(r["file"], json!(out.to_string_lossy()));
    }

    #[test]
    fn two_runs_give_identical_bytes() {
        let dir = Scratch::new("deux");
        let (a, b) = (dir.join("a.geojson"), dir.join("b.geojson"));
        let p = |out: &Path| {
            json!({
                "cost": fixture("mnt-colline-2154.tif").to_string_lossy(),
                "start": {"x": 652030, "y": 6850250, "epsg": 2154},
                "end": {"x": 652470, "y": 6850250, "epsg": 2154},
                "out": out.to_string_lossy(),
            })
        };
        let ra = call(p(&a)).unwrap();
        let rb = call(p(&b)).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        assert_eq!(ra, json!({
            "file": a.to_string_lossy(),
            "feature_count": 1,
            "total_cost": rb["total_cost"],
            "length": rb["length"],
        }));
        // Le résultat est celui de la fonction pure sur le raster ouvert.
        let hill = Dem::open(&fixture("mnt-colline-2154.tif")).unwrap();
        let found =
            least_cost_path(&hill, &pt(652030.0, 6850250.0), &pt(652470.0, 6850250.0)).unwrap();
        assert_eq!(ra["total_cost"], json!(found.total_cost));
        assert_eq!(ra["length"]["value"], json!(found.length));
    }

    #[test]
    fn terrain_least_cost_path_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("terrain.least_cost_path").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert_eq!(spec.output_ext, ".geojson");
        assert_eq!(spec.path_params, vec!["cost".to_string()]);
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        let violations = registry.check_contract();
        assert!(
            violations.iter().all(|v| v.id != "terrain.least_cost_path"),
            "{violations:?}"
        );
        let props = spec.output_schema["properties"].as_object().unwrap();
        let mut keys: Vec<&str> = props.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["feature_count", "file", "length", "total_cost"]);
        let params = &spec.params_schema;
        for p in ["start", "end"] {
            assert!(crate::point::is_point(&params["properties"][p], params), "{p}");
        }
    }
}
