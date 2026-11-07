//! Commande `terrain.peaks` : points hauts d'un MNT, calculés dans le noyau, sans GDAL ni GRASS
//! (AD-1), et écrits en GeoJSON de points par l'écrivain unique du noyau (`geojson.rs`, AD-30).
//!
//! Méthode : inondation descendante. Les cellules valides sont parcourues de la plus haute à la
//! plus basse, dans un ordre total fixé (altitude décroissante, puis ordre de lecture : ligne,
//! puis colonne). Une cellule dont aucune voisine (8-connexité) n'est encore traitée ouvre une
//! composante : c'est un sommet candidat, racine de sa composante. Sinon elle rejoint les
//! composantes de ses voisines traitées ; si elle en relie plusieurs, elle est leur col :
//! chaque composante dont le sommet n'est pas le premier dans l'ordre reçoit la proéminence
//! `altitude du sommet − altitude du col` et se fond dans la première. Une composante jamais
//! rejointe, et celle du point culminant, reçoivent le dénivelé jusqu'à la cellule valide la
//! plus basse du MNT.
//!
//! Égalité : une cellule n'est sommet que si aucune voisine n'est plus haute et qu'aucune
//! voisine de même altitude ne la précède dans l'ordre de lecture ; un plateau donne donc un
//! sommet, sa première cellule, sauf un plateau non convexe, qui peut donner un second
//! candidat de proéminence nulle, gardé seulement si `min_prominence` vaut 0. Bord : une cellule du bord du MNT, ou voisine d'un trou, n'est
//! jamais un sommet, mais compte comme terrain dans l'inondation.
//!
//! Filtres, dans cet ordre : proéminence au moins `min_prominence` ; écart (distance
//! planimétrique entre centres de cellules) au moins `min_distance` avec chaque sommet déjà
//! gardé, sinon seul le premier dans l'ordre reste ; puis `max_count` premiers, `truncated`
//! vrai si un sommet admis par les deux filtres est écarté. Le point est au centre de la
//! cellule, dans l'EPSG du MNT : aucune reprojection (AD-19).
//!
//! Coût : un tri des cellules valides et une union de composantes (union-find, division de
//! chemin), sans parallélisme, pour un ordre et des octets figés. Mémoire : deux `Vec<u32>`
//! (ordre et parent, 8 octets par cellule), plus les sommets candidats (`summits`, 4 octets,
//! et `found`, 24 octets, chacun jusqu'à un quart des cellules sur un MNT très accidenté),
//! soit au pire environ 15 octets par cellule en plus du MNT. Au pire, un MNT au plafond de
//! 1 Gio de `dem.rs` (268 millions de cellules Float32) demande environ 3,7 Gio de plus et de
//! l'ordre d'une à deux minutes de calcul, presque toutes dans le tri ; un MNT d'une commune à
//! 5 m (4 millions de cellules) se traite en moins d'une seconde.

use std::path::Path;

use schemars::JsonSchema;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::commands::analysis_buffer::{Distance, DistanceUnit};
use crate::commands::terrain_line_of_sight::{Height, MetreUnit};
use crate::dem::Dem;
use crate::geojson::{self, Feature, Geometry, Property};
use crate::point::Point;
use crate::raster_path::RasterPath;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

/// Borne haute de `max_count`, et donc de la liste `peaks` du résultat (AD-8).
pub const MAX_PEAKS: u32 = 100;

/// Marque d'une cellule pas encore traitée dans le tableau des parents.
const UNSEEN: u32 = u32::MAX;

fn default_min_prominence() -> Height {
    Height {
        value: 10.0,
        unit: DistanceUnit::M,
    }
}

fn default_min_distance() -> Distance {
    Distance {
        value: 100.0,
        unit: DistanceUnit::M,
    }
}

fn default_max_count() -> u32 {
    20
}

fn max_count<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let n = u32::deserialize(d)?;
    if n == 0 || n > MAX_PEAKS {
        return Err(D::Error::custom(format!(
            "`max_count` demande de 1 à {MAX_PEAKS} sommets, pas {n}"
        )));
    }
    Ok(n)
}

/// Paramètres de `terrain.peaks`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerrainPeaksParams {
    /// Fichier MNT (GeoTIFF à une bande, système projeté en mètres), chemin relatif au projet
    /// recopié de l'inventaire. DEM file path, relative to the project, copied from the inventory.
    pub dem: RasterPath,
    /// Proéminence minimale d'un sommet gardé, avec son unité (10 m par défaut).
    #[serde(default = "default_min_prominence")]
    pub min_prominence: Height,
    /// Écart horizontal minimal entre deux sommets gardés, avec son unité (100 m par défaut).
    #[serde(default = "default_min_distance")]
    pub min_distance: Distance,
    /// Nombre maximal de sommets écrits, de 1 à 100 (20 par défaut).
    #[serde(default = "default_max_count", deserialize_with = "max_count")]
    #[schemars(range(min = 1, max = 100))]
    pub max_count: u32,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Un sommet du résultat.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct PeakOutput {
    /// Centre de la cellule du sommet, dans le système du MNT.
    pub point: Point,
    /// Altitude du sommet, en mètres.
    pub elevation: f64,
    /// Proéminence du sommet, en mètres.
    pub prominence: f64,
}

/// Résultat de `terrain.peaks`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TerrainPeaksOutput {
    /// GeoJSON de points écrit (format de 2008, `crs` du MNT), propriétés `rank`, `elevation`
    /// et `prominence`.
    pub file: String,
    /// Nombre de sommets écrits.
    pub feature_count: u64,
    /// Sommets écrits, du plus haut au plus bas (100 au plus).
    #[schemars(length(max = 100))]
    pub peaks: Vec<PeakOutput>,
    /// Vrai si des sommets au-delà de `max_count` ont été écartés.
    pub truncated: bool,
    /// Unité des altitudes et des proéminences : toujours `m`.
    pub unit: MetreUnit,
}

/// Un sommet trouvé : sa cellule, son altitude et sa proéminence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peak {
    pub col: u32,
    pub row: u32,
    pub elevation: f64,
    pub prominence: f64,
}

/// Sommets gardés et indicateur de troncature.
#[derive(Debug, Clone, PartialEq)]
pub struct Peaks {
    pub peaks: Vec<Peak>,
    pub truncated: bool,
}

/// Racine de la composante de `i`, avec division de chemin.
fn find(parent: &mut [u32], mut i: u32) -> u32 {
    loop {
        let p = parent[i as usize];
        if p == i {
            return i;
        }
        let g = parent[p as usize];
        parent[i as usize] = g;
        i = g;
    }
}

/// Sommets d'un MNT déjà ouvert, filtrés puis classés. Fonction pure et déterministe.
pub fn peaks(dem: &Dem, min_prominence: f64, min_distance: f64, max_count: usize) -> Peaks {
    let (w, h) = (dem.width as usize, dem.height as usize);
    let n = w * h;
    // `dem.rs` plafonne la grille à 1 Gio, soit 2^28 cellules : l'index tient en u32, et
    // `UNSEEN` (u32::MAX) n'est jamais un index de cellule.
    assert!(n < UNSEEN as usize, "MNT de plus de 2^32 - 1 cellules");
    let z = |i: usize| dem.value((i % w) as u32, (i / w) as u32);

    let mut order: Vec<u32> = (0..n).filter(|&i| z(i).is_some()).map(|i| i as u32).collect();
    let Some(&lowest) = order
        .iter()
        .min_by(|&&a, &&b| dem.values[a as usize].total_cmp(&dem.values[b as usize]))
    else {
        return Peaks {
            peaks: Vec::new(),
            truncated: false,
        };
    };
    let zmin = f64::from(dem.values[lowest as usize]);
    // Ordre total : altitude décroissante, puis ordre de lecture (l'index croît avec la ligne,
    // puis la colonne).
    let key = |a: u32, b: u32| {
        dem.values[b as usize]
            .total_cmp(&dem.values[a as usize])
            .then(a.cmp(&b))
    };
    order.sort_unstable_by(|&a, &b| key(a, b));

    let mut parent = vec![UNSEEN; n];
    let mut summits: Vec<u32> = Vec::new();
    let mut found: Vec<Peak> = Vec::new();
    let peak_of = |i: u32, prominence: f64| Peak {
        col: i % w as u32,
        row: i / w as u32,
        elevation: f64::from(dem.values[i as usize]),
        prominence,
    };
    let mut roots: Vec<u32> = Vec::with_capacity(8);
    for &cell in &order {
        let (col, row) = (cell as usize % w, cell as usize / w);
        roots.clear();
        for dr in -1i64..=1 {
            for dc in -1i64..=1 {
                if dr == 0 && dc == 0 {
                    continue;
                }
                let (c, r) = (col as i64 + dc, row as i64 + dr);
                if c < 0 || r < 0 || c >= w as i64 || r >= h as i64 {
                    continue;
                }
                let j = (r as usize * w + c as usize) as u32;
                if parent[j as usize] == UNSEEN {
                    continue;
                }
                let root = find(&mut parent, j);
                if !roots.contains(&root) {
                    roots.push(root);
                }
            }
        }
        if roots.is_empty() {
            parent[cell as usize] = cell;
            summits.push(cell);
            continue;
        }
        // La composante dont le sommet vient le premier dans l'ordre l'emporte ; les autres
        // ont leur col ici.
        let winner = *roots.iter().min_by(|&&a, &&b| key(a, b)).unwrap();
        let col_z = f64::from(dem.values[cell as usize]);
        for &r in &roots {
            if r != winner {
                found.push(peak_of(r, f64::from(dem.values[r as usize]) - col_z));
                parent[r as usize] = winner;
            }
        }
        parent[cell as usize] = winner;
    }
    for &s in &summits {
        if parent[s as usize] == s {
            found.push(peak_of(s, f64::from(dem.values[s as usize]) - zmin));
        }
    }
    drop(parent);
    drop(order);

    // Bord du MNT ou voisine d'un trou : jamais un sommet.
    let interior = |p: &Peak| {
        let (c, r) = (p.col, p.row);
        c > 0
            && r > 0
            && (c as usize) < w - 1
            && (r as usize) < h - 1
            && (r - 1..=r + 1).all(|rr| (c - 1..=c + 1).all(|cc| dem.value(cc, rr).is_some()))
    };
    let mut admitted: Vec<Peak> = found
        .into_iter()
        .filter(|p| interior(p) && p.prominence >= min_prominence)
        .collect();
    admitted.sort_unstable_by(|a, b| {
        key(
            a.row * w as u32 + a.col,
            b.row * w as u32 + b.col,
        )
    });

    let centre = |p: &Peak| {
        (
            dem.origin_x + (f64::from(p.col) + 0.5) * dem.cell_size_x,
            dem.origin_y - (f64::from(p.row) + 0.5) * dem.cell_size_y,
        )
    };
    // Un sommet de plus que `max_count` suffit à savoir s'il y a troncature.
    let mut kept: Vec<Peak> = Vec::new();
    for p in admitted {
        let (x, y) = centre(&p);
        let near = kept.iter().any(|k| {
            let (kx, ky) = centre(k);
            (x - kx).hypot(y - ky) < min_distance
        });
        if !near {
            kept.push(p);
            if kept.len() > max_count {
                break;
            }
        }
    }
    let truncated = kept.len() > max_count;
    kept.truncate(max_count);
    Peaks {
        peaks: kept,
        truncated,
    }
}

/// `terrain.peaks` : commande du noyau.
pub struct TerrainPeaks;

impl Command for TerrainPeaks {
    type Params = TerrainPeaksParams;
    type Output = TerrainPeaksOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<TerrainPeaksParams, TerrainPeaksOutput>(
            "terrain.peaks",
            "Points hauts d'un MNT : trouve les sommets d'un modèle numérique d'élévation (fichier GeoTIFF désigné par `dem`, chemin de fichier MNT recopié de l'inventaire, pas un id de couche), cellules plus hautes que leurs 8 voisines, de proéminence minimale (10 m par défaut) et écartés d'au moins une distance (100 m par défaut), du plus haut au plus bas, 20 au plus par défaut (de 1 à 100). Les cellules du bord et les voisines d'un trou ne sont jamais des sommets. Écrit un GeoJSON de points dans le système du MNT, avec le rang, l'altitude et la proéminence. Ne modifie pas le MNT.",
            "DEM high points: finds the summits of a digital elevation model (GeoTIFF file given by `dem`, a DEM file path copied from the inventory, not a layer id), cells higher than their 8 neighbours, with a minimum prominence (10 m by default) and at least a given distance apart (100 m by default), from highest to lowest, at most 20 by default (from 1 to 100). Edge cells and the neighbours of a hole are never summits. Writes a GeoJSON of points in the DEM's coordinate system, with rank, elevation and prominence. Does not modify the DEM.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["dem"])
        .output_params(&["out"])
        .output_ext(".geojson")
        .example(serde_json::json!({"dem": "mnt-colline-2154.tif"}))
        .example(serde_json::json!({
            "dem": "mnt-plan-incline-2154.tif",
            "min_prominence": {"value": 5, "unit": "m"},
            "min_distance": {"value": 200, "unit": "m"},
            "max_count": 5
        }))
        .tests(&[
            "the_hill_has_one_summit_at_the_first_cell_of_its_plateau",
            "a_tilted_plane_has_no_summit_and_writes_an_empty_geojson",
            "the_lower_of_two_hills_has_the_prominence_of_its_col",
            "min_prominence_drops_the_lower_hill",
            "min_distance_keeps_the_higher_hill_alone",
            "max_count_truncates",
            "a_summit_next_to_a_hole_or_on_the_edge_is_not_a_summit",
            "out_of_range_params_write_nothing",
            "a_refused_dem_writes_nothing_and_keeps_the_module_error",
            "an_unwritable_output_keeps_the_geojson_error",
            "two_runs_give_identical_bytes",
            "terrain_peaks_runs_in_the_core_and_the_contract_accepts_it",
        ])
    }

    fn run(&self, params: TerrainPeaksParams) -> Result<TerrainPeaksOutput, CommandFailure> {
        let dem = Dem::open(Path::new(params.dem.as_str())).map_err(CommandFailure::from)?;
        let found = peaks(
            &dem,
            params.min_prominence.metres(),
            params.min_distance.metres(),
            params.max_count as usize,
        );
        let outputs: Vec<PeakOutput> = found
            .peaks
            .iter()
            .map(|p| PeakOutput {
                point: Point {
                    x: dem.origin_x + (f64::from(p.col) + 0.5) * dem.cell_size_x,
                    y: dem.origin_y - (f64::from(p.row) + 0.5) * dem.cell_size_y,
                    epsg: dem.epsg,
                },
                elevation: p.elevation,
                prominence: p.prominence,
            })
            .collect();
        let features: Vec<Feature> = outputs
            .iter()
            .enumerate()
            .map(|(i, p)| Feature {
                geometry: Geometry::Point(p.point.x, p.point.y),
                properties: vec![
                    ("rank", Property::Integer(i as i64 + 1)),
                    ("elevation", Property::Number(p.elevation)),
                    ("prominence", Property::Number(p.prominence)),
                ],
            })
            .collect();
        geojson::write(Path::new(&params.out), dem.epsg, &features)
            .map_err(CommandFailure::from)?;
        Ok(TerrainPeaksOutput {
            file: params.out,
            feature_count: outputs.len() as u64,
            peaks: outputs,
            truncated: found.truncated,
            unit: MetreUnit::M,
        })
    }
}

inventory::submit! { Registration(|r| r.register(TerrainPeaks)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::{fixture, Scratch};
    use crate::registry::Registry;
    use serde_json::{json, Value};

    fn call(params: Value) -> Result<Value, crate::registry::RegistryError> {
        Registry::standard().call("terrain.peaks", params)
    }

    fn run_on(dem: &Path, out: &Path, extra: Value) -> Result<Value, crate::registry::RegistryError> {
        let mut p = json!({"dem": dem.to_string_lossy(), "out": out.to_string_lossy()});
        for (k, v) in extra.as_object().unwrap() {
            p[k] = v.clone();
        }
        call(p)
    }

    fn m(v: f64) -> Value {
        json!({"value": v, "unit": "m"})
    }

    /// Grille synthétique de cellules de 10 m en EPSG:2154, origine (652000, 6850500).
    fn grid(width: u32, height: u32, f: impl Fn(u32, u32) -> f32) -> Dem {
        let mut values = Vec::with_capacity((width * height) as usize);
        for row in 0..height {
            for col in 0..width {
                values.push(f(col, row));
            }
        }
        Dem {
            width,
            height,
            origin_x: 652000.0,
            origin_y: 6850500.0,
            cell_size_x: 10.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values,
        }
    }

    /// Cône de hauteur `top` au-dessus du fond, centré sur la cellule (cx, cy), pente 1 m par
    /// cellule.
    fn cone(col: u32, row: u32, cx: u32, cy: u32, top: f32) -> f32 {
        let d = col.abs_diff(cx).max(row.abs_diff(cy)) as f32;
        (top - d).max(0.0)
    }

    /// Deux collines sur 60 × 30 : la haute (40 m) en (15, 15), la basse (30 m) en (45, 15) ;
    /// fond à 0 m. Le col entre elles tombe en colonne 35, où les deux cônes valent 20 m :
    /// proéminence de la basse, 30 - 20 = 10 m. La cellule valide la plus basse vaut 15 m (les
    /// cônes ne descendent pas jusqu'au fond sur 30 lignes) : proéminence de la haute, 25 m.
    fn two_hills() -> Dem {
        grid(60, 30, |c, r| {
            cone(c, r, 15, 15, 40.0).max(cone(c, r, 45, 15, 30.0))
        })
    }

    fn features(path: &Path) -> Vec<Value> {
        let v: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        v["features"].as_array().unwrap().clone()
    }

    #[test]
    fn the_hill_has_one_summit_at_the_first_cell_of_its_plateau() {
        let dir = Scratch::new("colline");
        let out = dir.join("sommets.geojson");
        let r = run_on(&fixture("mnt-colline-2154.tif"), &out, json!({})).unwrap();
        let dem = Dem::open(&fixture("mnt-colline-2154.tif")).unwrap();
        let top = f64::from(dem.value(24, 24).unwrap());
        let lowest = dem
            .values
            .iter()
            .map(|&v| f64::from(v))
            .fold(f64::INFINITY, f64::min);
        assert_eq!(r["feature_count"], 1);
        assert_eq!(r["truncated"], false);
        assert_eq!(r["unit"], "m");
        assert_eq!(r["file"], out.to_string_lossy().as_ref());
        assert_eq!(
            r["peaks"],
            json!([{"point": {"x": 652245.0, "y": 6850255.0, "epsg": 2154},
                    "elevation": top, "prominence": top - lowest}])
        );
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(text.contains("urn:ogc:def:crs:EPSG::2154"), "{text}");
        let f = features(&out);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0]["geometry"]["coordinates"], json!([652245.0, 6850255.0]));
        let props: Vec<&String> = f[0]["properties"].as_object().unwrap().keys().collect();
        // L'ordre des propriétés est celui du fichier : `serde_json` sans `preserve_order`
        // trie les clés, on lit donc le texte brut.
        assert_eq!(props.len(), 3);
        let (a, b, c) = (
            text.find("\"rank\":1").unwrap(),
            text.find("\"elevation\"").unwrap(),
            text.find("\"prominence\"").unwrap(),
        );
        assert!(a < b && b < c, "{text}");
    }

    #[test]
    fn a_tilted_plane_has_no_summit_and_writes_an_empty_geojson() {
        let dir = Scratch::new("plan");
        let out = dir.join("sommets.geojson");
        let r = run_on(&fixture("mnt-plan-incline-2154.tif"), &out, json!({})).unwrap();
        assert_eq!(r["feature_count"], 0);
        assert_eq!(r["peaks"], json!([]));
        assert_eq!(r["truncated"], false);
        assert!(features(&out).is_empty());
    }

    #[test]
    fn the_lower_of_two_hills_has_the_prominence_of_its_col() {
        let found = peaks(&two_hills(), 10.0, 100.0, 20);
        assert!(!found.truncated);
        assert_eq!(
            found.peaks,
            vec![
                Peak { col: 15, row: 15, elevation: 40.0, prominence: 25.0 },
                Peak { col: 45, row: 15, elevation: 30.0, prominence: 10.0 },
            ]
        );
    }

    #[test]
    fn min_prominence_drops_the_lower_hill() {
        let found = peaks(&two_hills(), 10.5, 100.0, 20);
        assert_eq!(found.peaks.len(), 1);
        assert_eq!((found.peaks[0].col, found.peaks[0].row), (15, 15));
        // Par la commande : la hauteur garde son unité.
        let dir = Scratch::new("proeminence");
        let (input, out) = (dir.join("mnt.tif"), dir.join("sommets.geojson"));
        two_hills().write_geotiff(&input).unwrap();
        let r = run_on(&input, &out, json!({"min_prominence": m(11.0)})).unwrap();
        assert_eq!(r["feature_count"], 1);
        let r = run_on(&input, &out, json!({"min_prominence": {"value": 0.01, "unit": "km"}}))
            .unwrap();
        assert_eq!(r["feature_count"], 2);
    }

    #[test]
    fn min_distance_keeps_the_higher_hill_alone() {
        // Les sommets sont à 300 m l'un de l'autre.
        assert_eq!(peaks(&two_hills(), 10.0, 300.0, 20).peaks.len(), 2);
        let found = peaks(&two_hills(), 10.0, 300.5, 20);
        assert_eq!(found.peaks.len(), 1);
        assert_eq!((found.peaks[0].col, found.peaks[0].row), (15, 15));
        assert!(!found.truncated);
    }

    #[test]
    fn max_count_truncates() {
        // Trois collines : 40, 30 et 25 m.
        let dem = grid(90, 30, |c, r| {
            cone(c, r, 15, 15, 40.0)
                .max(cone(c, r, 45, 15, 30.0))
                .max(cone(c, r, 75, 15, 25.0))
        });
        let all = peaks(&dem, 5.0, 100.0, 20);
        assert_eq!(all.peaks.len(), 3);
        assert!(!all.truncated);
        let dir = Scratch::new("nombre");
        let (input, out) = (dir.join("mnt.tif"), dir.join("sommets.geojson"));
        dem.write_geotiff(&input).unwrap();
        let r = run_on(&input, &out, json!({"min_prominence": m(5.0), "max_count": 2})).unwrap();
        assert_eq!(r["feature_count"], 2);
        assert_eq!(r["truncated"], true);
        let f = features(&out);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0]["properties"]["rank"], 1);
        assert_eq!(f[1]["properties"]["rank"], 2);
        assert_eq!(f[0]["properties"]["elevation"], 40.0);
        assert_eq!(f[1]["properties"]["elevation"], 30.0);
        // Écarté par la distance, pas par le nombre : pas de troncature.
        let r = run_on(
            &input,
            &out,
            json!({"min_prominence": m(5.0), "min_distance": m(700.0), "max_count": 1}),
        )
        .unwrap();
        assert_eq!((r["feature_count"].clone(), r["truncated"].clone()), (json!(1), json!(false)));
    }

    #[test]
    fn a_summit_next_to_a_hole_or_on_the_edge_is_not_a_summit() {
        // Un trou à côté du sommet de la haute colline : elle n'est plus un sommet, mais reste
        // du terrain ; la basse garde la proéminence de son col.
        let mut dem = two_hills();
        dem.values[15 * 60 + 16] = -9999.0;
        let found = peaks(&dem, 10.0, 100.0, 20);
        assert_eq!(found.peaks.len(), 1);
        assert_eq!(
            found.peaks[0],
            Peak { col: 45, row: 15, elevation: 30.0, prominence: 10.0 }
        );
        // Un sommet au bord : écarté, et la colline intérieure, la plus haute gardée, garde la
        // proéminence de son col, à 13 m (colonnes 27 et 28 de la ligne 15), et non le dénivelé
        // jusqu'à la cellule la plus basse, ici un coin creusé à -50 m.
        let mut dem = grid(60, 30, |c, r| cone(c, r, 0, 15, 40.0).max(cone(c, r, 45, 15, 30.0)));
        dem.values[29 * 60 + 59] = -50.0;
        let found = peaks(&dem, 10.0, 100.0, 20);
        assert_eq!(found.peaks.len(), 1);
        assert_eq!((found.peaks[0].col, found.peaks[0].prominence), (45, 17.0));
        // Un plateau donne un sommet : sa première cellule dans l'ordre de lecture.
        let dem = grid(10, 10, |c, r| if (3..6).contains(&c) && (4..6).contains(&r) { 20.0 } else { 5.0 });
        let found = peaks(&dem, 1.0, 10.0, 20);
        assert_eq!(
            found.peaks,
            vec![Peak { col: 3, row: 4, elevation: 20.0, prominence: 15.0 }]
        );
        // Tout en trous : aucun sommet, pas d'erreur.
        let dem = grid(5, 5, |_, _| -9999.0);
        assert_eq!(peaks(&dem, 0.0, 1.0, 20), Peaks { peaks: vec![], truncated: false });
    }

    #[test]
    fn out_of_range_params_write_nothing() {
        let dir = Scratch::new("bornes");
        let out = dir.join("sommets.geojson");
        let dem = fixture("mnt-colline-2154.tif");
        for bad in [
            json!({"max_count": 0}),
            json!({"max_count": 101}),
            json!({"max_count": -1}),
            json!({"min_prominence": m(-1.0)}),
            json!({"min_distance": m(0.0)}),
            json!({"min_distance": m(-5.0)}),
            json!({"min_prominence": 10}),
            json!({"radius": m(10.0)}),
        ] {
            let err = run_on(&dem, &out, bad.clone()).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
            assert!(!out.exists());
        }
        assert!(run_on(&dem, &out, json!({"max_count": 100})).is_ok());
        assert!(run_on(&dem, &out, json!({"max_count": 1, "min_prominence": m(0.0)})).is_ok());
        // Défauts : 10 m, 100 m, 20.
        let p: TerrainPeaksParams = serde_json::from_value(json!({"dem": "mnt.tif"})).unwrap();
        assert_eq!(
            (p.min_prominence.metres(), p.min_distance.metres(), p.max_count),
            (10.0, 100.0, 20)
        );
        assert_eq!(call(json!({})).unwrap_err().code(), "registry.invalid_params");
    }

    #[test]
    fn a_refused_dem_writes_nothing_and_keeps_the_module_error() {
        let dir = Scratch::new("refus");
        let out = dir.join("sommets.geojson");

        let absent = dir.join("absent.tif");
        let expected = Dem::open(&absent).unwrap_err();
        let err = run_on(&absent, &out, json!({})).unwrap_err();
        assert_eq!(err.code(), "dem.unreadable");
        assert_eq!(err.to_string(), expected.message);
        assert!(!out.exists());

        let pseudo = dir.join("3857.tif");
        Dem { epsg: 3857, ..two_hills() }.write_geotiff(&pseudo).unwrap();
        let expected = Dem::open(&pseudo).unwrap_err();
        assert_eq!(expected.code, "dem.not_projected");
        let err = run_on(&pseudo, &out, json!({})).unwrap_err();
        assert_eq!((err.code(), err.to_string()), (expected.code, expected.message));
        assert!(!out.exists());

        // Un MNT en degrés : type de modèle géographique (2) et clé géographique 4326.
        let wgs84 = dir.join("wgs84.tif");
        two_hills().write_geotiff(&wgs84).unwrap();
        patch_model_to_geographic(&wgs84);
        let expected = Dem::open(&wgs84).unwrap_err();
        assert_eq!(expected.code, "dem.not_projected");
        let err = run_on(&wgs84, &out, json!({})).unwrap_err();
        assert_eq!((err.code(), err.to_string()), (expected.code, expected.message));
        assert!(!out.exists());
    }

    /// Passe le type de modèle à géographique et la clé 3072 en 2048 (`GeographicTypeGeoKey`)
    /// d'EPSG:4326, comme dans `terrain_slope.rs`.
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
    fn an_unwritable_output_keeps_the_geojson_error() {
        let dir = Scratch::new("ecriture");
        let out = dir.join("absent").join("sommets.geojson");
        let err = run_on(&fixture("mnt-colline-2154.tif"), &out, json!({})).unwrap_err();
        assert_eq!(err.code(), "geojson.unwritable");
        assert!(err.to_string().contains("sommets.geojson"), "{err}");
        assert!(!out.exists());
    }

    #[test]
    fn two_runs_give_identical_bytes() {
        let dir = Scratch::new("deux");
        let input = dir.join("mnt.tif");
        two_hills().write_geotiff(&input).unwrap();
        let (a, b) = (dir.join("a.geojson"), dir.join("b.geojson"));
        let ra = run_on(&input, &a, json!({})).unwrap();
        let rb = run_on(&input, &b, json!({})).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        assert_eq!(ra["peaks"], rb["peaks"]);
        assert_eq!(ra["feature_count"], 2);
    }

    #[test]
    fn terrain_peaks_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("terrain.peaks").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert_eq!(spec.output_ext, ".geojson");
        assert!(geojson::VECTOR_EXTENSIONS.contains(&spec.output_ext.as_str()));
        assert_eq!(spec.path_params, vec!["dem".to_string()]);
        let violations = registry.check_contract();
        assert!(
            violations.iter().all(|v| v.id != "terrain.peaks"),
            "{violations:?}"
        );
    }
}
