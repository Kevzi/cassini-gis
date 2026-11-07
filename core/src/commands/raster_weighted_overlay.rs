//! Commande `raster.weighted_overlay` : superposition pondérée de rasters de même grille,
//! calculée dans le noyau, sans GDAL ni GRASS (AD-1, amendement du 2026-11-05 d'AD-20).
//!
//! Score d'une cellule : somme des valeurs × poids, divisée par la somme des poids, en f64,
//! rendue en f32. Une cellule sans donnée dans un critère donne sans donnée ; une cellule
//! valide et non nulle d'un masque d'exclusion donne sans donnée (un masque sans donnée
//! n'exclut pas).
//!
//! Tous les rasters sont ouverts par `dem.rs`, l'unique porte raster du noyau (AD-20), puis
//! leurs grilles contrôlées contre celle du premier critère, avant tout calcul : une grille
//! différente rend `raster.grid_mismatch`, qui nomme le raster et l'écart. Le noyau ne
//! rééchantillonne jamais (AD-19) ; l'alignement est un geste QGIS.

use std::path::Path;

use schemars::JsonSchema;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::dem::{Dem, NODATA};
use crate::horn::same_grid_as;
use crate::raster_path::RasterPath;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

/// Nombres de critères permis (AD-8).
pub const MIN_CRITERIA: usize = 2;
pub const MAX_CRITERIA: usize = 8;

/// Nombre maximal de masques d'exclusion (AD-8).
pub const MAX_EXCLUSIONS: usize = 8;

fn weight<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    let w = f64::deserialize(d)?;
    if !(w.is_finite() && w > 0.0) {
        return Err(D::Error::custom(format!(
            "`weight` doit être un nombre fini strictement positif, pas {w}"
        )));
    }
    Ok(w)
}

/// Un critère : un raster et son poids.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OverlayCriterion {
    /// Raster du critère (GeoTIFF à une bande), chemin relatif au projet recopié de
    /// l'inventaire. Criterion raster path, relative to the project.
    pub raster: RasterPath,
    /// Poids du critère, strictement positif.
    #[serde(deserialize_with = "weight")]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub weight: f64,
}

fn criteria<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<OverlayCriterion>, D::Error> {
    let criteria = Vec::<OverlayCriterion>::deserialize(d)?;
    if criteria.len() < MIN_CRITERIA || criteria.len() > MAX_CRITERIA {
        return Err(D::Error::custom(format!(
            "`criteria` demande de {MIN_CRITERIA} à {MAX_CRITERIA} critères, pas {}",
            criteria.len()
        )));
    }
    Ok(criteria)
}

fn exclusions<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<RasterPath>, D::Error> {
    let exclusions = Vec::<RasterPath>::deserialize(d)?;
    if exclusions.len() > MAX_EXCLUSIONS {
        return Err(D::Error::custom(format!(
            "`exclusions` demande au plus {MAX_EXCLUSIONS} masques, pas {}",
            exclusions.len()
        )));
    }
    Ok(exclusions)
}

/// Paramètres de `raster.weighted_overlay`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RasterWeightedOverlayParams {
    /// Critères, de 2 à 8 : `{raster, weight}`, tous de même grille.
    #[serde(deserialize_with = "criteria")]
    #[schemars(length(min = MIN_CRITERIA, max = MAX_CRITERIA))]
    pub criteria: Vec<OverlayCriterion>,
    /// Masques d'exclusion, de 0 à 8, de même grille que les critères : toute cellule valide
    /// et non nulle d'un masque est exclue.
    #[serde(default, deserialize_with = "exclusions")]
    #[schemars(length(max = MAX_EXCLUSIONS))]
    pub exclusions: Vec<RasterPath>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `raster.weighted_overlay`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RasterWeightedOverlayOutput {
    /// GeoTIFF Float32 du score écrit, sans-donnée -9999.
    pub file: String,
    /// Score le plus bas sur les cellules valides, `null` si aucune ne l'est.
    pub min: Option<f64>,
    /// Score le plus haut sur les cellules valides, `null` si aucune ne l'est.
    pub max: Option<f64>,
    /// Cellules sans donnée (critère sans donnée ou exclusion).
    pub nodata_cells: u64,
}

/// Statistiques d'un score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayStats {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub nodata_cells: u64,
}

/// Nom de fichier d'un chemin, pour un message : le dossier n'y figure pas.
fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_string(), |n| n.to_string_lossy().into_owned())
}

/// Contrôle que chaque grille (`label`, chemin, grille) est celle du premier critère, avant
/// tout calcul. La première différence rend `raster.grid_mismatch`.
fn check_grids(reference: &Dem, others: &[(String, &str, &Dem)]) -> Result<(), CommandFailure> {
    for (label, path, grid) in others {
        if let Some(gap) = reference.grid_mismatch(grid) {
            return Err(CommandFailure::new(
                "raster.grid_mismatch",
                format!(
                    "{label} (« {} ») n'a pas la grille du critère 1 : {gap}. Alignez-le d'abord sur le critère 1 dans QGIS ; Cassini ne rééchantillonne pas.",
                    file_name(path)
                ),
            ));
        }
    }
    Ok(())
}

/// Score pondéré de grilles déjà contrôlées. Fonction pure et déterministe : somme dans l'ordre
/// des critères, en f64, rendue en f32. La grille rendue est celle du premier critère.
pub fn overlay(criteria: &[(&Dem, f64)], exclusions: &[&Dem]) -> (Dem, OverlayStats) {
    let reference = criteria[0].0;
    let (w, h) = (reference.width, reference.height);
    let total: f64 = criteria.iter().map(|(_, weight)| weight).sum();
    let mut values = vec![NODATA; w as usize * h as usize];
    let mut min: Option<f64> = None;
    let mut max: Option<f64> = None;
    let mut valid = 0u64;
    for row in 0..h {
        for col in 0..w {
            let excluded = exclusions
                .iter()
                .any(|mask| mask.value(col, row).is_some_and(|v| v != 0.0));
            if excluded {
                continue;
            }
            let mut sum = 0.0f64;
            let mut complete = true;
            for (grid, weight) in criteria {
                match grid.value(col, row) {
                    Some(v) => sum += f64::from(v) * weight,
                    None => {
                        complete = false;
                        break;
                    }
                }
            }
            if !complete {
                continue;
            }
            let score = (sum / total) as f32;
            values[row as usize * w as usize + col as usize] = score;
            let s = f64::from(score);
            min = Some(min.map_or(s, |m| m.min(s)));
            max = Some(max.map_or(s, |m| m.max(s)));
            valid += 1;
        }
    }
    let stats = OverlayStats {
        min,
        max,
        nodata_cells: u64::from(w) * u64::from(h) - valid,
    };
    (same_grid_as(reference, values, NODATA), stats)
}

/// `raster.weighted_overlay` : commande du noyau.
pub struct RasterWeightedOverlay;

impl Command for RasterWeightedOverlay {
    type Params = RasterWeightedOverlayParams;
    type Output = RasterWeightedOverlayOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<RasterWeightedOverlayParams, RasterWeightedOverlayOutput>(
            "raster.weighted_overlay",
            "Superposition pondérée de rasters : combine de 2 à 8 rasters de critères de même grille (`criteria`, liste de `{raster, weight}`, chemins de fichiers GeoTIFF recopiés de l'inventaire, pas des ids de couche) en un score, somme des valeurs × poids divisée par la somme des poids ; une cellule sans donnée dans un critère reste sans donnée. Des masques `exclusions` facultatifs retirent toute cellule non nulle. Écrit un nouveau GeoTIFF de même emprise. Ne rééchantillonne pas : des grilles différentes sont refusées.",
            "Weighted raster overlay: combines 2 to 8 criterion rasters on the same grid (`criteria`, a list of `{raster, weight}`, GeoTIFF file paths copied from the inventory, not layer ids) into a score, the sum of values × weights divided by the sum of weights; a cell without data in any criterion stays without data. Optional `exclusions` masks remove every non-zero cell. Writes a new GeoTIFF with the same extent. Does not resample: different grids are refused.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["criteria.raster", "exclusions"])
        .output_params(&["out"])
        .output_ext(".tif")
        .example(serde_json::json!({"criteria": [
            {"raster": "mnt-colline-2154.tif", "weight": 2.0},
            {"raster": "mnt-colline-2154.tif", "weight": 1.0}
        ]}))
        .example(serde_json::json!({"criteria": [
            {"raster": "mnt-plan-incline-2154.tif", "weight": 1.0},
            {"raster": "mnt-plan-incline-2154.tif", "weight": 3.0}
        ]}))
        .tests(&[
            "two_criteria_give_the_weighted_mean_cell_by_cell",
            "a_hole_and_an_exclusion_give_no_data",
            "an_exclusion_everywhere_gives_null_extremes_and_writes_the_file",
            "different_grids_are_refused_naming_the_raster_and_the_gap",
            "the_lists_and_weights_are_bounded_by_the_registry",
            "a_refused_criterion_keeps_the_module_error",
            "two_overlays_give_identical_bytes",
            "raster_weighted_overlay_runs_in_the_core_and_the_contract_accepts_it",
        ])
    }

    fn run(
        &self,
        params: RasterWeightedOverlayParams,
    ) -> Result<RasterWeightedOverlayOutput, CommandFailure> {
        // Tous les rasters ouverts, puis toutes les grilles contrôlées, avant tout calcul.
        let open = |p: &RasterPath| Dem::open(Path::new(p.as_str())).map_err(CommandFailure::from);
        let criteria = params
            .criteria
            .iter()
            .map(|c| open(&c.raster))
            .collect::<Result<Vec<_>, _>>()?;
        let masks = params
            .exclusions
            .iter()
            .map(open)
            .collect::<Result<Vec<_>, _>>()?;
        let others: Vec<(String, &str, &Dem)> = params
            .criteria
            .iter()
            .zip(&criteria)
            .enumerate()
            .skip(1)
            .map(|(i, (c, g))| (format!("Le critère {}", i + 1), c.raster.as_str(), g))
            .chain(
                params
                    .exclusions
                    .iter()
                    .zip(&masks)
                    .enumerate()
                    .map(|(i, (p, g))| (format!("Le masque d'exclusion {}", i + 1), p.as_str(), g)),
            )
            .collect();
        check_grids(&criteria[0], &others)?;

        let weighted: Vec<(&Dem, f64)> = criteria
            .iter()
            .zip(&params.criteria)
            .map(|(g, c)| (g, c.weight))
            .collect();
        let mask_refs: Vec<&Dem> = masks.iter().collect();
        let (score, stats) = overlay(&weighted, &mask_refs);
        score
            .write_geotiff(Path::new(&params.out))
            .map_err(CommandFailure::from)?;
        Ok(RasterWeightedOverlayOutput {
            file: params.out,
            min: stats.min,
            max: stats.max,
            nodata_cells: stats.nodata_cells,
        })
    }
}

inventory::submit! { Registration(|r| r.register(RasterWeightedOverlay)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::Scratch;
    use crate::registry::Registry;
    use serde_json::{json, Value};

    /// Grille 3 × 3 de cellules de 10 m en Lambert-93.
    fn grid(values: [f32; 9]) -> Dem {
        Dem {
            width: 3,
            height: 3,
            origin_x: 652000.0,
            origin_y: 6850030.0,
            cell_size_x: 10.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values: values.to_vec(),
        }
    }

    fn a() -> Dem {
        grid([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0])
    }

    fn b() -> Dem {
        grid([9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0])
    }

    fn call(params: Value) -> Result<Value, crate::registry::RegistryError> {
        Registry::standard().call("raster.weighted_overlay", params)
    }

    fn criteria(paths: &[(&str, f64)]) -> Value {
        paths
            .iter()
            .map(|(p, w)| json!({"raster": p, "weight": w}))
            .collect()
    }

    #[test]
    fn two_criteria_give_the_weighted_mean_cell_by_cell() {
        let dir = Scratch::new("moyenne");
        let (pa, pb) = (dir.write("a.tif", &a()), dir.write("b.tif", &b()));
        let out = dir.join("score.tif");
        let result = call(json!({
            "criteria": criteria(&[(&pa, 1.0), (&pb, 3.0)]),
            "out": out.to_string_lossy()
        }))
        .unwrap();
        let written = Dem::open(&out).unwrap();
        assert_eq!(written.extent(), a().extent());
        assert_eq!(written.epsg, 2154);
        assert_eq!(written.nodata, Some(-9999.0));
        for i in 0..9u32 {
            let (va, vb) = (f64::from(a().values[i as usize]), f64::from(b().values[i as usize]));
            let expected = ((va + 3.0 * vb) / 4.0) as f32;
            assert_eq!(written.value(i % 3, i / 3), Some(expected), "cellule {i}");
        }
        // Cellule 0 : (1 + 27) / 4 = 7 ; cellule 8 : (9 + 3) / 4 = 3.
        assert_eq!(result["min"], json!(3.0));
        assert_eq!(result["max"], json!(7.0));
        assert_eq!(result["nodata_cells"], 0);
        assert_eq!(result["file"], out.to_string_lossy().as_ref());
    }

    #[test]
    fn a_hole_and_an_exclusion_give_no_data() {
        let mut holed = b();
        holed.values[1] = -9999.0;
        // Masque : non nul en 7, sans donnée en 3 (n'exclut pas), nul ailleurs.
        let mut mask = grid([0.0; 9]);
        mask.values[7] = 2.0;
        mask.values[3] = -9999.0;
        let (score, stats) = overlay(&[(&a(), 1.0), (&holed, 1.0)], &[&mask]);
        assert_eq!(stats.nodata_cells, 2);
        assert_eq!(score.value(1, 0), None);
        assert_eq!(score.value(1, 2), None);
        assert_eq!(score.value(0, 1), Some(5.0));
        assert_eq!((stats.min, stats.max), (Some(5.0), Some(5.0)));

        let dir = Scratch::new("trou");
        let (pa, pb, pm) = (
            dir.write("a.tif", &a()),
            dir.write("b.tif", &holed),
            dir.write("m.tif", &mask),
        );
        let out = dir.join("score.tif");
        let result = call(json!({
            "criteria": criteria(&[(&pa, 1.0), (&pb, 1.0)]),
            "exclusions": [pm],
            "out": out.to_string_lossy()
        }))
        .unwrap();
        assert_eq!(result["nodata_cells"], 2);
        assert_eq!(Dem::open(&out).unwrap(), score);
    }

    #[test]
    fn an_exclusion_everywhere_gives_null_extremes_and_writes_the_file() {
        let dir = Scratch::new("tout-exclu");
        let (pa, pb, pm) = (
            dir.write("a.tif", &a()),
            dir.write("b.tif", &b()),
            dir.write("m.tif", &grid([-1.0; 9])),
        );
        let out = dir.join("score.tif");
        let result = call(json!({
            "criteria": criteria(&[(&pa, 1.0), (&pb, 1.0)]),
            "exclusions": [pm],
            "out": out.to_string_lossy()
        }))
        .unwrap();
        assert_eq!(result["min"], Value::Null);
        assert_eq!(result["max"], Value::Null);
        assert!(result.as_object().unwrap().contains_key("min"));
        assert_eq!(result["nodata_cells"], 9);
        let written = Dem::open(&out).unwrap();
        assert!((0..9).all(|i| written.value(i % 3, i / 3).is_none()));
    }

    #[test]
    fn different_grids_are_refused_naming_the_raster_and_the_gap() {
        let dir = Scratch::new("grilles");
        let pa = dir.write("a.tif", &a());
        let pb = dir.write("b.tif", &b());
        let out = dir.join("score.tif");
        let cases: Vec<(&str, Dem, &str)> = vec![
            (
                "decale.tif",
                Dem {
                    origin_x: 652005.0,
                    ..b()
                },
                "origine x décalée de 5 m (0,5 cellule) : 652005 au lieu de 652000",
            ),
            (
                "dimensions.tif",
                Dem {
                    width: 4,
                    values: vec![1.0; 12],
                    ..b()
                },
                "dimensions 4 × 3 cellules au lieu de 3 × 3",
            ),
            (
                "cellule.tif",
                Dem {
                    cell_size_x: 5.0,
                    cell_size_y: 5.0,
                    ..b()
                },
                "taille de cellule 5 × 5 m au lieu de 10 × 10 m",
            ),
            (
                "utm.tif",
                Dem {
                    epsg: 32631,
                    ..b()
                },
                "système EPSG:32631 au lieu d'EPSG:2154",
            ),
        ];
        for (name, bad, gap) in cases {
            let pbad = dir.write(name, &bad);
            let err = call(json!({
                "criteria": criteria(&[(&pa, 1.0), (&pbad, 1.0)]),
                "out": out.to_string_lossy()
            }))
            .unwrap_err();
            assert_eq!(err.code(), "raster.grid_mismatch", "{name}: {err}");
            let message = err.to_string();
            assert!(
                message.starts_with(&format!(
                    "Le critère 2 (« {name} ») n'a pas la grille du critère 1 : {gap}."
                )),
                "{message}"
            );
            assert!(!message.contains(dir.0.to_string_lossy().as_ref()), "{message}");
            assert!(!out.exists(), "{name}");

            // Le même écart sur un masque nomme le masque.
            let err = call(json!({
                "criteria": criteria(&[(&pa, 1.0), (&pb, 1.0)]),
                "exclusions": [pa, pbad],
                "out": out.to_string_lossy()
            }))
            .unwrap_err();
            assert_eq!(err.code(), "raster.grid_mismatch");
            assert!(
                err.to_string()
                    .starts_with(&format!("Le masque d'exclusion 2 (« {name} »)")),
                "{err}"
            );
            assert!(!out.exists());
        }
    }

    #[test]
    fn the_lists_and_weights_are_bounded_by_the_registry() {
        let dir = Scratch::new("bornes");
        let pa = dir.write("a.tif", &a());
        let out = dir.join("score.tif");
        let n_criteria = |n: usize| criteria(&vec![(pa.as_str(), 1.0); n]);
        let refused = [
            json!({"criteria": n_criteria(1)}),
            json!({"criteria": n_criteria(9)}),
            json!({"criteria": n_criteria(2), "exclusions": vec![pa.clone(); 9]}),
            json!({"criteria": criteria(&[(&pa, 1.0), (&pa, 0.0)])}),
            json!({"criteria": criteria(&[(&pa, -2.0), (&pa, 1.0)])}),
            json!({"criteria": [{"raster": pa, "weight": 1.0}, {"raster": pa}]}),
        ];
        for mut params in refused {
            params["out"] = json!(out.to_string_lossy());
            let err = call(params.clone()).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{params}");
            assert!(!out.exists());
        }
        for params in [
            json!({"criteria": n_criteria(8), "exclusions": vec![pa.clone(); 8]}),
            json!({"criteria": n_criteria(2)}),
        ] {
            let mut params = params;
            params["out"] = json!(out.to_string_lossy());
            call(params).unwrap();
        }
    }

    #[test]
    fn a_refused_criterion_keeps_the_module_error() {
        let dir = Scratch::new("refus");
        let pa = dir.write("a.tif", &a());
        let absent = dir.join("absent.tif");
        let expected = Dem::open(&absent).unwrap_err();
        let out = dir.join("score.tif");
        let err = call(json!({
            "criteria": criteria(&[(&pa, 1.0), (&absent.to_string_lossy(), 1.0)]),
            "out": out.to_string_lossy()
        }))
        .unwrap_err();
        assert_eq!(
            (err.code(), err.to_string()),
            (expected.code, expected.message)
        );
        assert!(!out.exists());
        let pm = dir.write("3857.tif", &Dem { epsg: 3857, ..a() });
        let err = call(json!({
            "criteria": criteria(&[(&pa, 1.0), (&pa, 1.0)]),
            "exclusions": [pm],
            "out": out.to_string_lossy()
        }))
        .unwrap_err();
        assert_eq!(err.code(), "dem.not_projected");
        assert!(!out.exists());
    }

    #[test]
    fn two_overlays_give_identical_bytes() {
        let dir = Scratch::new("deux");
        let (pa, pb) = (dir.write("a.tif", &a()), dir.write("b.tif", &b()));
        let (oa, ob) = (dir.join("1.tif"), dir.join("2.tif"));
        for out in [&oa, &ob] {
            call(json!({
                "criteria": criteria(&[(&pa, 0.3), (&pb, 0.7)]),
                "out": out.to_string_lossy()
            }))
            .unwrap();
        }
        assert_eq!(std::fs::read(&oa).unwrap(), std::fs::read(&ob).unwrap());
    }

    #[test]
    fn raster_weighted_overlay_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.weighted_overlay").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert_eq!(spec.output_ext, ".tif");
        assert_eq!(
            spec.path_params,
            vec!["criteria.raster".to_string(), "exclusions".to_string()]
        );
        let props = &spec.params_schema["properties"];
        assert_eq!(props["criteria"]["minItems"], json!(2));
        assert_eq!(props["criteria"]["maxItems"], json!(8));
        assert_eq!(props["exclusions"]["maxItems"], json!(8));
        let violations = registry.check_contract();
        assert!(
            violations.iter().all(|v| v.id != "raster.weighted_overlay"),
            "{violations:?}"
        );
    }
}
