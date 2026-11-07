//! Commande `raster.reclassify` : reclasse un raster mono-bande par une table de classes,
//! calculée dans le noyau, sans GDAL ni GRASS (AD-1, amendement du 2026-11-05 d'AD-20).
//!
//! Une classe couvre `[min, max[` ; une borne absente est ouverte. La première classe qui
//! convient l'emporte ; une cellule hors de toute classe devient sans donnée ; une cellule sans
//! donnée le reste. La table est lue telle quelle : aucun seuil métier par défaut.
//!
//! Le raster est ouvert et le résultat écrit par `dem.rs`, l'unique porte raster du noyau
//! (AD-20). Aucune reprojection ni aucun rééchantillonnage (AD-19) : la sortie garde l'emprise,
//! la cellule et l'EPSG de l'entrée. Le noyau nomme le fichier de sortie (AD-10).

use std::path::Path;

use schemars::JsonSchema;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::dem::{Dem, NODATA};
use crate::horn::same_grid_as;
use crate::raster_path::RasterPath;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

/// Nombre maximal de classes (AD-8).
pub const MAX_CLASSES: usize = 32;

/// Une classe : les valeurs de `[min, max[` deviennent `value`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RasterClass {
    /// Borne basse, incluse, dans l'unité du raster ; absente : pas de borne basse.
    #[serde(default)]
    pub min: Option<f64>,
    /// Borne haute, exclue, dans l'unité du raster ; absente : pas de borne haute.
    #[serde(default)]
    pub max: Option<f64>,
    /// Valeur écrite pour les cellules de la classe.
    pub value: f64,
}

impl RasterClass {
    /// La valeur `v` tombe-t-elle dans `[min, max[` ?
    fn contains(&self, v: f64) -> bool {
        self.min.is_none_or(|m| v >= m) && self.max.is_none_or(|m| v < m)
    }
}

fn classes<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<RasterClass>, D::Error> {
    let classes = Vec::<RasterClass>::deserialize(d)?;
    if classes.is_empty() || classes.len() > MAX_CLASSES {
        return Err(D::Error::custom(format!(
            "`classes` demande de 1 à {MAX_CLASSES} classes, pas {}",
            classes.len()
        )));
    }
    Ok(classes)
}

/// Paramètres de `raster.reclassify`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RasterReclassifyParams {
    /// Raster à reclasser (GeoTIFF à une bande, système projeté en mètres), chemin relatif au
    /// projet recopié de l'inventaire. Raster file path, relative to the project.
    pub raster: RasterPath,
    /// Table de classes, de 1 à 32, dans l'unité du raster : `{min?, max?, value}`, la
    /// classe couvrant `[min, max[`. La première classe qui convient l'emporte.
    #[serde(deserialize_with = "classes")]
    #[schemars(length(min = 1, max = MAX_CLASSES))]
    pub classes: Vec<RasterClass>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Cellules d'une valeur de sortie.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct ClassCount {
    /// Valeur de sortie, telle que donnée par la première classe qui l'écrit.
    pub value: f64,
    /// Nombre de cellules qui la portent.
    pub cells: u64,
}

/// Résultat de `raster.reclassify`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RasterReclassifyOutput {
    /// GeoTIFF Float32 reclassé écrit, sans-donnée -9999.
    pub file: String,
    /// Cellules par valeur de sortie, dans l'ordre des classes (une entrée par valeur
    /// distincte, zéro compris).
    pub counts: Vec<ClassCount>,
    /// Cellules sans donnée (entrée sans donnée ou hors de toute classe).
    pub nodata_cells: u64,
}

/// Contrôle la table : `min` inférieur à `max`, valeur différente du sans-donnée écrit.
/// L'erreur `raster.invalid_class` nomme la classe fautive par son rang, compté de 1.
pub fn check_classes(classes: &[RasterClass]) -> Result<(), CommandFailure> {
    for (i, class) in classes.iter().enumerate() {
        let rank = i + 1;
        if let (Some(min), Some(max)) = (class.min, class.max) {
            if min >= max {
                return Err(CommandFailure::new(
                    "raster.invalid_class",
                    format!(
                        "La classe {rank} est invalide : son minimum ({min}) doit être inférieur à son maximum ({max})."
                    ),
                ));
            }
        }
        if class.value as f32 == NODATA {
            return Err(CommandFailure::new(
                "raster.invalid_class",
                format!(
                    "La classe {rank} est invalide : sa valeur {} est la valeur sans-donnée du raster écrit (-9999) ; choisissez-en une autre.",
                    class.value
                ),
            ));
        }
    }
    Ok(())
}

/// Reclasse une grille déjà ouverte. Fonction pure et déterministe : la comparaison se fait en
/// f64 sur la valeur f32 de la cellule, la valeur écrite est celle de la classe rendue en f32.
pub fn reclassify(raster: &Dem, classes: &[RasterClass]) -> (Dem, Vec<ClassCount>, u64) {
    // Une entrée par valeur de sortie distincte (en f32, la valeur écrite), dans l'ordre de
    // première apparition ; `slot[i]` est l'entrée de la classe `i`.
    let mut counts: Vec<ClassCount> = Vec::new();
    let mut slot = Vec::with_capacity(classes.len());
    for class in classes {
        let written = class.value as f32;
        let at = match counts.iter().position(|c| c.value as f32 == written) {
            Some(at) => at,
            None => {
                counts.push(ClassCount {
                    value: class.value,
                    cells: 0,
                });
                counts.len() - 1
            }
        };
        slot.push(at);
    }
    let (w, h) = (raster.width, raster.height);
    let mut values = vec![NODATA; w as usize * h as usize];
    let mut nodata = 0u64;
    for row in 0..h {
        for col in 0..w {
            let found = raster.value(col, row).and_then(|v| {
                let v = f64::from(v);
                classes.iter().position(|c| c.contains(v))
            });
            match found {
                Some(i) => {
                    values[row as usize * w as usize + col as usize] = classes[i].value as f32;
                    counts[slot[i]].cells += 1;
                }
                None => nodata += 1,
            }
        }
    }
    (same_grid_as(raster, values, NODATA), counts, nodata)
}

/// `raster.reclassify` : commande du noyau.
pub struct RasterReclassify;

impl Command for RasterReclassify {
    type Params = RasterReclassifyParams;
    type Output = RasterReclassifyOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<RasterReclassifyParams, RasterReclassifyOutput>(
            "raster.reclassify",
            "Reclassement d'un raster : remplace les valeurs d'un raster à une bande (fichier GeoTIFF désigné par `raster`, chemin recopié de l'inventaire, pas un id de couche) par une table de classes `{min, max, value}`, chaque classe couvrant [min, max[, une borne absente restant ouverte ; la première classe qui convient l'emporte, une cellule hors de toute classe devient sans donnée. Écrit un nouveau GeoTIFF de même emprise. Ne modifie pas le raster d'origine.",
            "Raster reclassification: replaces the values of a single-band raster (GeoTIFF file given by `raster`, a path copied from the inventory, not a layer id) through a table of classes `{min, max, value}`, each class covering [min, max[, a missing bound staying open; the first matching class wins, a cell outside every class gets no data. Writes a new GeoTIFF with the same extent. Does not modify the original raster.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["raster"])
        .output_params(&["out"])
        .output_ext(".tif")
        .example(serde_json::json!({"raster": "mnt-colline-2154.tif", "classes": [
            {"max": 80.0, "value": 1.0},
            {"min": 80.0, "max": 120.0, "value": 2.0},
            {"min": 120.0, "value": 3.0}
        ]}))
        .tests(&[
            "a_slope_in_three_classes_counts_like_the_slope_itself",
            "cells_outside_every_class_get_no_data",
            "an_empty_class_is_counted_with_zero_cells",
            "the_first_matching_class_wins_and_equal_values_share_a_count",
            "an_invalid_class_is_named_by_its_rank_and_nothing_is_written",
            "the_number_of_classes_is_bounded_by_the_registry",
            "a_refused_raster_keeps_the_module_error",
            "two_reclassifications_give_identical_bytes",
            "raster_reclassify_runs_in_the_core_and_the_contract_accepts_it",
        ])
    }

    fn run(&self, params: RasterReclassifyParams) -> Result<RasterReclassifyOutput, CommandFailure> {
        check_classes(&params.classes)?;
        let raster = Dem::open(Path::new(params.raster.as_str())).map_err(CommandFailure::from)?;
        let (grid, counts, nodata_cells) = reclassify(&raster, &params.classes);
        grid.write_geotiff(Path::new(&params.out))
            .map_err(CommandFailure::from)?;
        Ok(RasterReclassifyOutput {
            file: params.out,
            counts,
            nodata_cells,
        })
    }
}

inventory::submit! { Registration(|r| r.register(RasterReclassify)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::test_support::{fixture, Scratch};
    use crate::commands::terrain_slope::{horn_slope, SlopeUnit};
    use crate::registry::Registry;
    use serde_json::{json, Value};
    use std::path::PathBuf;

    fn call(raster: &Path, classes: Value, out: &Path) -> Result<Value, crate::registry::RegistryError> {
        Registry::standard().call(
            "raster.reclassify",
            json!({"raster": raster.to_string_lossy(), "classes": classes, "out": out.to_string_lossy()}),
        )
    }

    /// Grille 3 × 3 de cellules de 10 m, valeurs 1 à 9, une sans-donnée au centre.
    fn grid() -> Dem {
        let mut values: Vec<f32> = (1..=9).map(|v| v as f32).collect();
        values[4] = -9999.0;
        Dem {
            width: 3,
            height: 3,
            origin_x: 652000.0,
            origin_y: 6850030.0,
            cell_size_x: 10.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values,
        }
    }

    /// La pente en degrés de la colline, écrite dans `dir`.
    fn hill_slope(dir: &Scratch) -> (PathBuf, Dem) {
        let dem = Dem::open(&fixture("mnt-colline-2154.tif")).unwrap();
        let (slope, _) = horn_slope(&dem, SlopeUnit::Degrees);
        let path = dir.join("pente.tif");
        slope.write_geotiff(&path).unwrap();
        let grid = Dem::open(&path).unwrap();
        (path, grid)
    }

    #[test]
    fn a_slope_in_three_classes_counts_like_the_slope_itself() {
        let dir = Scratch::new("pente");
        let (input, slope) = hill_slope(&dir);
        let out = dir.join("classes.tif");
        let classes = json!([
            {"max": 5.0, "value": 1.0},
            {"min": 5.0, "max": 15.0, "value": 2.0},
            {"min": 15.0, "value": 3.0}
        ]);
        let result = call(&input, classes, &out).unwrap();

        let (mut low, mut mid, mut high, mut none) = (0u64, 0u64, 0u64, 0u64);
        for row in 0..slope.height {
            for col in 0..slope.width {
                match slope.value(col, row).map(f64::from) {
                    None => none += 1,
                    Some(v) if v < 5.0 => low += 1,
                    Some(v) if v < 15.0 => mid += 1,
                    Some(_) => high += 1,
                }
            }
        }
        assert!(low > 0 && mid > 0 && high > 0, "{low} {mid} {high}");
        assert_eq!(none, 4 * 49, "bord de la pente");
        assert_eq!(
            result["counts"],
            json!([
                {"value": 1.0, "cells": low},
                {"value": 2.0, "cells": mid},
                {"value": 3.0, "cells": high}
            ])
        );
        assert_eq!(result["nodata_cells"], none);
        assert_eq!(result["file"], out.to_string_lossy().as_ref());

        let written = Dem::open(&out).unwrap();
        assert_eq!(written.extent(), slope.extent());
        assert_eq!(
            (written.width, written.height, written.epsg),
            (slope.width, slope.height, 2154)
        );
        assert_eq!(written.nodata, Some(-9999.0));
        for row in 0..slope.height {
            for col in 0..slope.width {
                let expected = slope.value(col, row).map(|v| match f64::from(v) {
                    v if v < 5.0 => 1.0,
                    v if v < 15.0 => 2.0,
                    _ => 3.0,
                });
                assert_eq!(written.value(col, row), expected, "({col}, {row})");
            }
        }
    }

    #[test]
    fn cells_outside_every_class_get_no_data() {
        let classes = [RasterClass {
            min: Some(3.0),
            max: Some(7.0),
            value: 10.0,
        }];
        let (out, counts, nodata) = reclassify(&grid(), &classes);
        // 3, 4, 6 dans la classe (5 est le trou) ; 1, 2, 7, 8, 9 et le trou hors classe.
        assert_eq!(
            counts,
            vec![ClassCount {
                value: 10.0,
                cells: 3
            }]
        );
        assert_eq!(nodata, 6);
        let kept: Vec<Option<f32>> = (0..9).map(|i| out.value(i % 3, i / 3)).collect();
        assert_eq!(
            kept,
            vec![
                None,
                None,
                Some(10.0),
                Some(10.0),
                None,
                Some(10.0),
                None,
                None,
                None
            ]
        );
    }

    #[test]
    fn an_empty_class_is_counted_with_zero_cells() {
        let classes = [
            RasterClass {
                min: None,
                max: Some(5.0),
                value: 1.0,
            },
            RasterClass {
                min: Some(100.0),
                max: None,
                value: 2.0,
            },
            RasterClass {
                min: Some(5.0),
                max: None,
                value: 0.0,
            },
        ];
        let (_, counts, nodata) = reclassify(&grid(), &classes);
        assert_eq!(
            counts,
            vec![
                ClassCount {
                    value: 1.0,
                    cells: 4
                },
                ClassCount {
                    value: 2.0,
                    cells: 0
                },
                ClassCount {
                    value: 0.0,
                    cells: 4
                },
            ]
        );
        assert_eq!(nodata, 1);
    }

    #[test]
    fn the_first_matching_class_wins_and_equal_values_share_a_count() {
        let classes = [
            RasterClass {
                min: Some(2.0),
                max: Some(4.0),
                value: 7.0,
            },
            // Recouvre la première : 2 et 3 y restent, la première l'emporte.
            RasterClass {
                min: None,
                max: Some(4.0),
                value: 8.0,
            },
            // Même valeur que la première : un seul compte, au rang de la première.
            RasterClass {
                min: Some(4.0),
                max: None,
                value: 7.0,
            },
        ];
        let (out, counts, nodata) = reclassify(&grid(), &classes);
        assert_eq!(
            counts,
            vec![
                ClassCount {
                    value: 7.0,
                    cells: 7
                },
                ClassCount {
                    value: 8.0,
                    cells: 1
                },
            ]
        );
        assert_eq!(nodata, 1);
        assert_eq!(out.value(0, 0), Some(8.0));
        assert_eq!(out.value(1, 0), Some(7.0));
        // `max` exclu : 4 tombe dans la troisième classe.
        assert_eq!(out.value(0, 1), Some(7.0));
    }

    #[test]
    fn an_invalid_class_is_named_by_its_rank_and_nothing_is_written() {
        let dir = Scratch::new("invalide");
        let input = dir.join("grille.tif");
        grid().write_geotiff(&input).unwrap();
        let out = dir.join("classes.tif");
        let err = call(
            &input,
            json!([{"max": 5.0, "value": 1.0}, {"min": 5.0, "max": 5.0, "value": 2.0}]),
            &out,
        )
        .unwrap_err();
        assert_eq!(err.code(), "raster.invalid_class");
        assert!(err.to_string().contains("classe 2"), "{err}");
        assert!(!out.exists());

        let err = call(
            &input,
            json!([{"min": 9.0, "max": 1.0, "value": 1.0}]),
            &out,
        )
        .unwrap_err();
        assert_eq!(err.code(), "raster.invalid_class");
        assert!(err.to_string().contains("classe 1"), "{err}");

        let err = call(
            &input,
            json!([
                {"max": 5.0, "value": 1.0},
                {"min": 5.0, "max": 6.0, "value": 2.0},
                {"min": 6.0, "value": -9999.0}
            ]),
            &out,
        )
        .unwrap_err();
        assert_eq!(err.code(), "raster.invalid_class");
        assert!(err.to_string().contains("classe 3"), "{err}");
        assert!(err.to_string().contains("-9999"), "{err}");
        assert!(!out.exists());
    }

    #[test]
    fn the_number_of_classes_is_bounded_by_the_registry() {
        let dir = Scratch::new("bornes");
        let input = dir.join("grille.tif");
        grid().write_geotiff(&input).unwrap();
        let out = dir.join("classes.tif");
        let many = |n: usize| -> Value {
            (0..n)
                .map(|i| json!({"min": i as f64, "max": i as f64 + 1.0, "value": i as f64}))
                .collect()
        };
        for n in [0, MAX_CLASSES + 1] {
            let err = call(&input, many(n), &out).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{n}");
            assert!(!out.exists());
        }
        call(&input, many(MAX_CLASSES), &out).unwrap();
        // Un champ inconnu dans une classe est refusé aussi.
        let err = call(&input, json!([{"value": 1.0, "label": "a"}]), &out).unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
        let err = call(&input, json!([{"min": 1.0}]), &out).unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
    }

    #[test]
    fn a_refused_raster_keeps_the_module_error() {
        let dir = Scratch::new("refus");
        let out = dir.join("classes.tif");
        let absent = dir.join("absent.tif");
        let expected = Dem::open(&absent).unwrap_err();
        let err = call(&absent, json!([{"value": 1.0}]), &out).unwrap_err();
        assert_eq!(
            (err.code(), err.to_string()),
            (expected.code, expected.message)
        );
        assert!(!out.exists());
        let geographic = dir.join("3857.tif");
        Dem {
            epsg: 3857,
            ..grid()
        }
        .write_geotiff(&geographic)
        .unwrap();
        let err = call(&geographic, json!([{"value": 1.0}]), &out).unwrap_err();
        assert_eq!(err.code(), "dem.not_projected");
        assert!(!out.exists());
    }

    #[test]
    fn two_reclassifications_give_identical_bytes() {
        let dir = Scratch::new("deux");
        let (input, _) = hill_slope(&dir);
        let classes = json!([{"max": 10.0, "value": 0.0}, {"min": 10.0, "value": 1.0}]);
        let (a, b) = (dir.join("a.tif"), dir.join("b.tif"));
        let ra = call(&input, classes.clone(), &a).unwrap();
        let rb = call(&input, classes, &b).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        assert_eq!(ra["counts"], rb["counts"]);
    }

    #[test]
    fn raster_reclassify_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.reclassify").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert_eq!(spec.output_ext, ".tif");
        assert_eq!(spec.path_params, vec!["raster".to_string()]);
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        let classes = &spec.params_schema["properties"]["classes"];
        assert_eq!(classes["minItems"], json!(1));
        assert_eq!(classes["maxItems"], json!(32));
        let violations = registry.check_contract();
        assert!(
            violations.iter().all(|v| v.id != "raster.reclassify"),
            "{violations:?}"
        );
    }
}
