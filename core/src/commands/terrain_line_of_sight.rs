//! Commande `terrain.line_of_sight` : un observateur voit-il une cible sur le relief d'un MNT ?
//! Calculée dans le noyau, sans GDAL ni GRASS (AD-1).
//!
//! Le MNT est lu par `dem.rs`, l'unique porte raster du noyau (AD-20). Les points sont dans le
//! système du MNT : un autre système est refusé, jamais reprojeté (AD-19). Le profil rendu est
//! borné (AD-8). Aucun fichier n'est écrit.
//!
//! Méthode : la ligne est échantillonnée au pas de la plus petite cellule, chaque altitude est
//! interpolée de façon bilinéaire, puis abaissée de la chute due à la courbure terrestre
//! corrigée de la réfraction. Le premier échantillon intermédiaire au-dessus de la ligne de
//! visée est le premier obstacle.

use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::commands::analysis_buffer::DistanceUnit;
use crate::dem::{Dem, DemError};
use crate::point::Point;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

/// Nombre maximal d'échantillons du profil rendu, premier et dernier compris (AD-8).
pub const MAX_PROFILE: usize = 256;
/// Rayon terrestre moyen, en mètres.
pub const EARTH_RADIUS: f64 = 6_371_000.0;

pub(crate) fn default_observer_height() -> Height {
    Height {
        value: 1.6,
        unit: DistanceUnit::M,
    }
}

pub(crate) fn default_target_height() -> Height {
    Height {
        value: 0.0,
        unit: DistanceUnit::M,
    }
}

pub(crate) fn default_curvature() -> bool {
    true
}

pub(crate) fn default_refraction() -> f64 {
    0.13
}

/// Hauteur au-dessus du sol, avec son unité (jamais de mesure nue sur le canal).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Height {
    /// Valeur positive ou nulle.
    #[schemars(range(min = 0.0))]
    #[serde(deserialize_with = "non_negative")]
    pub value: f64,
    /// Unité de la valeur : `m` ou `km`.
    pub unit: DistanceUnit,
}

impl Height {
    /// La hauteur en mètres.
    pub fn metres(&self) -> f64 {
        match self.unit {
            DistanceUnit::M => self.value,
            DistanceUnit::Km => self.value * 1000.0,
        }
    }
}

/// Unité des distances et altitudes du résultat : toujours le mètre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
pub enum MetreUnit {
    /// Mètres.
    #[serde(rename = "m")]
    M,
}

/// Refuse une hauteur négative ou non finie.
fn non_negative<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    let v = f64::deserialize(d)?;
    if v.is_finite() && v >= 0.0 {
        Ok(v)
    } else {
        Err(serde::de::Error::custom(
            "la hauteur au-dessus du sol doit être positive ou nulle",
        ))
    }
}

/// Refuse un coefficient de réfraction hors de [0, 1).
pub(crate) fn refraction<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    let v = f64::deserialize(d)?;
    if v.is_finite() && (0.0..1.0).contains(&v) {
        Ok(v)
    } else {
        Err(serde::de::Error::custom(
            "le coefficient de réfraction doit être compris entre 0 et 1 (exclu)",
        ))
    }
}

/// Paramètres de `terrain.line_of_sight`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerrainLineOfSightParams {
    /// Fichier MNT (GeoTIFF à une bande, système projeté en mètres), chemin relatif au projet
    /// recopié de l'inventaire. DEM file path, relative to the project, copied from the inventory.
    pub dem: String,
    /// Position de l'observateur, dans le système du MNT.
    pub observer: Point,
    /// Position de la cible, dans le système du MNT.
    pub target: Point,
    /// Hauteur de l'œil au-dessus du sol, avec son unité (1,6 m par défaut).
    #[serde(default = "default_observer_height")]
    pub observer_height: Height,
    /// Hauteur de la cible au-dessus du sol, avec son unité (0 m par défaut).
    #[serde(default = "default_target_height")]
    pub target_height: Height,
    /// Tenir compte de la courbure terrestre (vrai par défaut).
    #[serde(default = "default_curvature")]
    pub curvature: bool,
    /// Coefficient de réfraction atmosphérique, de 0 à moins de 1 (0,13 par défaut). N'agit
    /// que si `curvature` est vrai.
    #[schemars(range(min = 0.0), extend("exclusiveMaximum" = 1.0))]
    #[serde(default = "default_refraction", deserialize_with = "refraction")]
    pub refraction: f64,
}

/// Un échantillon du profil.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct ProfileSample {
    /// Distance horizontale depuis l'observateur, en mètres.
    pub distance: f64,
    /// Altitude du sol interpolée, en mètres, sans correction de courbure : `null` si une
    /// cellule voisine est sans donnée.
    pub elevation: Option<f64>,
}

/// Résultat de `terrain.line_of_sight`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct TerrainLineOfSightOutput {
    /// Vrai si aucun relief ne masque la cible.
    pub visible: bool,
    /// Unité de toutes les distances et altitudes du résultat : toujours `m`.
    pub unit: MetreUnit,
    /// Distance horizontale de l'observateur à la cible, en mètres.
    pub distance: f64,
    /// Premier obstacle depuis l'observateur, dans le système du MNT : absent si la cible est
    /// visible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obstruction: Option<Point>,
    /// Distance horizontale de l'observateur au premier obstacle, en mètres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obstruction_distance: Option<f64>,
    /// Profil du sol, de l'observateur à la cible, réduit à 256 échantillons au plus.
    #[schemars(length(max = 256))]
    pub profile: Vec<ProfileSample>,
    /// Écart entre deux échantillons rendus du profil, en mètres (0 si les points sont
    /// confondus).
    pub profile_step: f64,
    /// Nombre d'échantillons sans altitude sur toute la ligne : ils ne masquent rien.
    pub nodata_samples: u64,
}

/// Erreur typée de la commande.
fn refusal(code: &str, message: String) -> CommandFailure {
    CommandFailure::new(code, message)
}

/// Altitude interpolée de façon bilinéaire entre les centres des quatre cellules voisines,
/// `None` si l'une d'elles est sans donnée. Sur l'axe où le point tombe pile sur un centre, seule
/// cette cellule compte : la voisine, de poids nul, n'est pas lue. Le point doit être dans
/// l'emprise ; à moins d'une demi-cellule du bord, il prend la valeur du bord.
pub(crate) fn bilinear(dem: &Dem, x: f64, y: f64) -> Option<f64> {
    let max_col = f64::from(dem.width - 1);
    let max_row = f64::from(dem.height - 1);
    let fc = ((x - dem.origin_x) / dem.cell_size_x - 0.5).clamp(0.0, max_col);
    let fr = ((dem.origin_y - y) / dem.cell_size_y - 0.5).clamp(0.0, max_row);
    let (c0, r0) = (fc.floor() as u32, fr.floor() as u32);
    let (tx, ty) = (fc - f64::from(c0), fr - f64::from(r0));
    // `fc` est borné à la dernière colonne : un poids non nul garantit une voisine dans la grille.
    let c1 = if tx > 0.0 { c0 + 1 } else { c0 };
    let r1 = if ty > 0.0 { r0 + 1 } else { r0 };
    let z = |c: u32, r: u32| dem.value(c, r).map(f64::from);
    let (z00, z10, z01, z11) = (z(c0, r0)?, z(c1, r0)?, z(c0, r1)?, z(c1, r1)?);
    let top = z00 + (z10 - z00) * tx;
    let bottom = z01 + (z11 - z01) * tx;
    Some(top + (bottom - top) * ty)
}

/// Refuse un point qui n'est pas dans le système du MNT.
pub(crate) fn check_crs(dem: &Dem, p: &Point, name: &str) -> Result<(), CommandFailure> {
    if p.epsg == dem.epsg {
        return Ok(());
    }
    Err(refusal(
        "terrain.crs_mismatch",
        format!(
            "Le point {name} est en EPSG:{}, le MNT en EPSG:{} : donnez-le dans le système du MNT, Cassini ne reprojette pas.",
            p.epsg, dem.epsg
        ),
    ))
}

/// Refuse un point hors de l'emprise du MNT (le bord est dedans).
pub(crate) fn check_extent(dem: &Dem, p: &Point, name: &str) -> Result<(), CommandFailure> {
    let e = dem.extent();
    if (e.min_x..=e.max_x).contains(&p.x) && (e.min_y..=e.max_y).contains(&p.y) {
        return Ok(());
    }
    Err(refusal(
        "terrain.outside_dem",
        format!(
            "Le point {name} ({}, {}) est hors de l'emprise du MNT (x de {} à {}, y de {} à {}).",
            p.x, p.y, e.min_x, e.max_x, e.min_y, e.max_y
        ),
    ))
}

/// Altitude d'une extrémité, déjà dans l'emprise : interpolée de façon bilinéaire ; si une
/// voisine est sans donnée, celle de la cellule qui contient le point. Refusée seulement si
/// cette cellule-là est sans donnée.
pub(crate) fn endpoint_elevation(dem: &Dem, p: &Point, name: &str) -> Result<f64, CommandFailure> {
    if let Some(z) = bilinear(dem, p.x, p.y) {
        return Ok(z);
    }
    // Le bord droit et le bord bas de l'emprise appartiennent à la dernière cellule.
    let col = ((p.x - dem.origin_x) / dem.cell_size_x)
        .floor()
        .clamp(0.0, f64::from(dem.width - 1)) as u32;
    let row = ((dem.origin_y - p.y) / dem.cell_size_y)
        .floor()
        .clamp(0.0, f64::from(dem.height - 1)) as u32;
    dem.value(col, row).map(f64::from).ok_or_else(|| {
        refusal(
            "terrain.no_elevation",
            format!(
                "Le point {name} ({}, {}) tombe sur une zone sans donnée du MNT : son altitude est inconnue.",
                p.x, p.y
            ),
        )
    })
}

/// Segment échantillonné de l'observateur à la cible : au plus la plus petite taille de cellule
/// entre deux échantillons.
pub(crate) struct Line {
    o: (f64, f64),
    dx: f64,
    dy: f64,
    /// Distance horizontale de l'observateur à la cible, en mètres.
    pub distance: f64,
    /// Nombre d'intervalles entre échantillons (0 si les points sont confondus).
    pub intervals: u64,
}

impl Line {
    pub(crate) fn new(dem: &Dem, o: &Point, t: &Point) -> Line {
        let (dx, dy) = (t.x - o.x, t.y - o.y);
        // Racine carrée IEEE, correctement arrondie partout : `hypot` dépend de la libm de la
        // plateforme, et un journal écrit sous Windows doit se rejouer à l'identique sous Linux.
        let distance = (dx * dx + dy * dy).sqrt();
        let step = dem.cell_size_x.min(dem.cell_size_y);
        let intervals = if distance > 0.0 {
            ((distance / step).ceil() as u64).max(1)
        } else {
            0
        };
        Line {
            o: (o.x, o.y),
            dx,
            dy,
            distance,
            intervals,
        }
    }

    /// Échantillon `i` sur `n` : distance à l'observateur, x, y.
    pub(crate) fn at(&self, i: u64, n: u64) -> (f64, f64, f64) {
        if n == 0 {
            return (0.0, self.o.0, self.o.1);
        }
        // Multiplier avant de diviser : un échantillon tombe pile sur un centre quand il le doit.
        let (i, n) = (i as f64, n as f64);
        (
            self.distance * i / n,
            self.o.0 + self.dx * i / n,
            self.o.1 + self.dy * i / n,
        )
    }
}

/// Verdict du calcul partagé par `terrain.line_of_sight` et `terrain.viewshed`.
pub(crate) struct Sight {
    /// Premier obstacle : distance, x, y. Absent si la cible est visible.
    pub obstruction: Option<(f64, f64, f64)>,
    /// Échantillons sans altitude parcourus (tous, sauf arrêt au premier obstacle).
    pub nodata_samples: u64,
}

/// Le calcul de visibilité, unique pour les deux commandes : c'est ce qui garantit qu'une
/// cellule du champ de vision a le verdict de la ligne de vue vers son centre. `eye` est
/// l'altitude de l'œil (sol + hauteur), `z_tgt` celle du sol de la cible. Les échantillons
/// intermédiaires sont interpolés de façon bilinéaire et abaissés de la chute due à la
/// courbure corrigée de la réfraction ; un échantillon sans altitude ne masque rien. Avec
/// `stop_at_first`, le parcours s'arrête au premier obstacle, sans changer le verdict.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sight(
    dem: &Dem,
    line: &Line,
    eye: f64,
    z_tgt: f64,
    target_height: f64,
    curvature: bool,
    refraction: f64,
    stop_at_first: bool,
) -> Sight {
    let (distance, intervals) = (line.distance, line.intervals);
    let drop = |d: f64| {
        if curvature {
            (1.0 - refraction) * d * d / (2.0 * EARTH_RADIUS)
        } else {
            0.0
        }
    };
    let aim = z_tgt - drop(distance) + target_height;
    let mut obstruction = None;
    let mut nodata_samples = 0u64;
    // Les extrémités ont leur altitude : seuls les échantillons intermédiaires sont lus.
    for i in 1..intervals {
        let (d, x, y) = line.at(i, intervals);
        let Some(z) = bilinear(dem, x, y) else {
            nodata_samples += 1;
            continue;
        };
        if obstruction.is_none() {
            let ground = z - drop(d);
            let sightline = eye + (aim - eye) * d / distance;
            if ground > sightline {
                obstruction = Some((d, x, y));
                if stop_at_first {
                    break;
                }
            }
        }
    }
    Sight {
        obstruction,
        nodata_samples,
    }
}

/// Ligne de vue entre deux points sur un MNT déjà ouvert. Fonction pure et déterministe :
/// calcul en f64, même entrée, même résultat.
pub fn line_of_sight(
    dem: &Dem,
    params: &TerrainLineOfSightParams,
) -> Result<TerrainLineOfSightOutput, CommandFailure> {
    let (o, t) = (&params.observer, &params.target);
    let ends = [(o, "observateur"), (t, "cible")];
    // Chaque contrôle passe sur les deux points avant le suivant : système, emprise, altitude.
    for (p, name) in ends {
        check_crs(dem, p, name)?;
    }
    for (p, name) in ends {
        check_extent(dem, p, name)?;
    }
    let z_obs = endpoint_elevation(dem, o, "observateur")?;
    let z_tgt = endpoint_elevation(dem, t, "cible")?;
    // Les hauteurs sont converties en mètres dès l'entrée.
    let observer_height = params.observer_height.metres();
    let target_height = params.target_height.metres();

    let line = Line::new(dem, o, t);
    let seen = sight(
        dem,
        &line,
        z_obs + observer_height,
        z_tgt,
        target_height,
        params.curvature,
        params.refraction,
        false,
    );
    let (distance, intervals) = (line.distance, line.intervals);
    let (obstruction, nodata_samples) = (seen.obstruction, seen.nodata_samples);

    // Profil : tous les échantillons s'ils tiennent dans la borne, sinon 256 échantillons
    // également espacés, premier et dernier compris, interpolés aux mêmes règles.
    let shown = if intervals as usize + 1 > MAX_PROFILE {
        MAX_PROFILE as u64 - 1
    } else {
        intervals
    };
    let profile = (0..=shown)
        .map(|i| {
            let (d, x, y) = line.at(i, shown);
            let elevation = if i == 0 {
                Some(z_obs)
            } else if i == shown {
                Some(z_tgt)
            } else {
                bilinear(dem, x, y)
            };
            ProfileSample {
                distance: d,
                elevation,
            }
        })
        .collect();
    let profile_step = if shown == 0 {
        0.0
    } else {
        distance / shown as f64
    };

    Ok(TerrainLineOfSightOutput {
        visible: obstruction.is_none(),
        unit: MetreUnit::M,
        distance,
        obstruction: obstruction.map(|(_, x, y)| Point {
            x,
            y,
            epsg: dem.epsg,
        }),
        obstruction_distance: obstruction.map(|(d, _, _)| d),
        profile,
        profile_step,
        nodata_samples,
    })
}

fn failure(e: DemError) -> CommandFailure {
    CommandFailure::new(e.code, e.message)
}

/// `terrain.line_of_sight` : commande du noyau.
pub struct TerrainLineOfSight;

impl Command for TerrainLineOfSight {
    type Params = TerrainLineOfSightParams;
    type Output = TerrainLineOfSightOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<TerrainLineOfSightParams, TerrainLineOfSightOutput>(
            "terrain.line_of_sight",
            "Ligne de vue entre deux points sur un MNT : dit si un observateur voit une cible sur le relief d'un modèle numérique d'élévation (fichier GeoTIFF désigné par `dem`, chemin de fichier MNT recopié de l'inventaire, pas un id de couche). Les deux points sont dans le système du MNT, avec la hauteur de l'œil et de la cible au-dessus du sol ; la courbure terrestre et la réfraction sont prises en compte par défaut. Rend le verdict, le premier obstacle et un profil d'altitudes. N'écrit aucun fichier et ne modifie pas le MNT.",
            "Line of sight between two points on a DEM: tells whether an observer sees a target over the relief of a digital elevation model (GeoTIFF file given by `dem`, a DEM file path copied from the inventory, not a layer id). Both points are in the DEM's coordinate system, with eye and target heights above the ground; Earth curvature and refraction are taken into account by default. Returns the verdict, the first obstruction and an elevation profile. Writes no file and does not modify the DEM.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["dem"])
        .example(serde_json::json!({
            "dem": "mnt-colline-2154.tif",
            "observer": {"x": 652030, "y": 6850250, "epsg": 2154},
            "target": {"x": 652470, "y": 6850250, "epsg": 2154},
            "observer_height": {"value": 60, "unit": "m"},
            "target_height": {"value": 60, "unit": "m"}
        }))
        .example(serde_json::json!({
            "dem": "mnt-colline-2154.tif",
            "observer": {"x": 652030, "y": 6850250, "epsg": 2154},
            "target": {"x": 652130, "y": 6850250, "epsg": 2154},
            "observer_height": {"value": 2, "unit": "m"},
            "target_height": {"value": 2, "unit": "m"}
        }))
        .tests(&[
            "the_hill_hides_the_other_side",
            "the_same_slope_is_visible",
            "curvature_hides_a_far_target_on_a_flat_dem",
            "a_point_in_another_system_is_refused",
            "a_point_outside_the_dem_is_refused",
            "a_point_without_elevation_is_refused",
            "a_hole_on_the_line_hides_nothing_and_is_counted",
            "coincident_points_are_visible_with_one_sample",
            "a_long_line_gives_a_bounded_profile_and_a_verdict_on_every_sample",
            "defaults_and_invalid_params",
            "two_runs_give_identical_results",
            "terrain_line_of_sight_runs_in_the_core_and_the_contract_accepts_it",
        ])
    }

    fn run(
        &self,
        params: TerrainLineOfSightParams,
    ) -> Result<TerrainLineOfSightOutput, CommandFailure> {
        let dem = Dem::open(Path::new(&params.dem)).map_err(failure)?;
        line_of_sight(&dem, &params)
    }
}

inventory::submit! { Registration(|r| r.register(TerrainLineOfSight)) }

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

    fn m(value: f64) -> Height {
        Height {
            value,
            unit: DistanceUnit::M,
        }
    }

    fn pt(x: f64, y: f64) -> Point {
        Point { x, y, epsg: 2154 }
    }

    fn params(observer: Point, target: Point) -> TerrainLineOfSightParams {
        TerrainLineOfSightParams {
            dem: String::new(),
            observer,
            target,
            observer_height: m(1.6),
            target_height: m(0.0),
            curvature: true,
            refraction: 0.13,
        }
    }

    fn hill() -> Dem {
        Dem::open(&fixture("mnt-colline-2154.tif")).unwrap()
    }

    /// MNT plat à 100 m, de `width` × 3 cellules de `cell` mètres, coin haut-gauche (0, 300).
    fn flat(width: u32, cell: f64) -> Dem {
        Dem {
            width,
            height: 3,
            origin_x: 0.0,
            origin_y: 3.0 * cell,
            cell_size_x: cell,
            cell_size_y: cell,
            epsg: 2154,
            nodata: Some(-9999.0),
            values: vec![100.0; width as usize * 3],
        }
    }

    fn call(p: Value) -> Result<Value, crate::registry::RegistryError> {
        Registry::standard().call("terrain.line_of_sight", p)
    }

    #[test]
    fn the_hill_hides_the_other_side() {
        let mut p = params(pt(652030.0, 6850250.0), pt(652470.0, 6850250.0));
        p.observer_height = m(60.0);
        p.target_height = m(60.0);
        let r = line_of_sight(&hill(), &p).unwrap();
        assert!(!r.visible);
        assert_eq!(r.unit, MetreUnit::M);
        assert_eq!(r.distance, 440.0);
        let o = r.obstruction.unwrap();
        assert_eq!((o.y, o.epsg), (6850250.0, 2154));
        // Côté observateur, à moins de 50 m du sommet (652250).
        assert!(o.x < 652250.0 && 652250.0 - o.x < 50.0, "{o:?}");
        assert!((r.obstruction_distance.unwrap() - (o.x - 652030.0)).abs() < 1e-9);
        assert_eq!(r.nodata_samples, 0);
        assert_eq!(r.profile.len(), 45);
        assert_eq!(r.profile_step, 10.0);
        // Le sommet du profil est au milieu, vers 150 m.
        let mid = r.profile[22].elevation.unwrap();
        assert!(mid > 140.0 && mid <= 150.0, "{mid}");
        // La même hauteur en kilomètres donne le même résultat, à l'octet.
        let mut km = p.clone();
        km.observer_height = Height {
            value: 0.06,
            unit: DistanceUnit::Km,
        };
        let rk = line_of_sight(&hill(), &km).unwrap();
        assert_eq!(rk.visible, r.visible);
        assert_eq!(rk.obstruction, r.obstruction);
        assert_eq!(serde_json::to_value(&r).unwrap()["unit"], "m");
    }

    #[test]
    fn the_same_slope_is_visible() {
        let mut p = params(pt(652030.0, 6850250.0), pt(652130.0, 6850250.0));
        p.observer_height = m(2.0);
        p.target_height = m(2.0);
        let r = line_of_sight(&hill(), &p).unwrap();
        assert!(r.visible);
        assert!(r.obstruction.is_none() && r.obstruction_distance.is_none());
        let v = serde_json::to_value(&r).unwrap();
        assert!(v.get("obstruction").is_none() && v.get("obstruction_distance").is_none());
    }

    #[test]
    fn curvature_hides_a_far_target_on_a_flat_dem() {
        // 20,1 km de plat, cellules de 100 m.
        let dem = flat(201, 100.0);
        let mut p = params(pt(50.0, 150.0), pt(20050.0, 150.0));
        p.observer_height = m(2.0);
        p.curvature = false;
        let r = line_of_sight(&dem, &p).unwrap();
        assert!(r.visible, "{r:?}");
        p.curvature = true;
        let r = line_of_sight(&dem, &p).unwrap();
        assert!(!r.visible, "{r:?}");
        assert_eq!(r.distance, 20000.0);
    }

    #[test]
    fn a_point_in_another_system_is_refused() {
        let mut p = params(pt(652030.0, 6850250.0), pt(652130.0, 6850250.0));
        p.target.epsg = 4326;
        let err = line_of_sight(&hill(), &p).unwrap_err();
        assert_eq!(err.code, "terrain.crs_mismatch");
        // Même refus si l'autre point est en plus hors du MNT : le système passe d'abord.
        p.observer = pt(0.0, 0.0);
        let err = line_of_sight(&hill(), &p).unwrap_err();
        assert_eq!(err.code, "terrain.crs_mismatch");
        let err = call(json!({
            "dem": fixture("mnt-colline-2154.tif").to_string_lossy(),
            "observer": {"x": 2.3, "y": 48.7, "epsg": 4326},
            "target": {"x": 652130, "y": 6850250, "epsg": 2154},
        }))
        .unwrap_err();
        assert_eq!(err.code(), "terrain.crs_mismatch");
    }

    #[test]
    fn a_point_outside_the_dem_is_refused() {
        for (o, t) in [
            (pt(651990.0, 6850250.0), pt(652130.0, 6850250.0)),
            (pt(652030.0, 6850250.0), pt(652130.0, 6850501.0)),
        ] {
            let err = line_of_sight(&hill(), &params(o, t)).unwrap_err();
            assert_eq!(err.code, "terrain.outside_dem");
        }
        // L'emprise des deux points passe avant l'altitude : observateur sur un trou et cible
        // hors du MNT donnent `outside_dem`.
        let err = line_of_sight(&holed(), &params(pt(35.0, 35.0), pt(500.0, 35.0))).unwrap_err();
        assert_eq!(err.code, "terrain.outside_dem");
        // Le bord de l'emprise est dedans.
        let r = line_of_sight(&hill(), &params(pt(652000.0, 6850500.0), pt(652500.0, 6850000.0)));
        assert!(r.is_ok());
    }

    /// MNT plat 7 × 7 de cellules de 10 m, avec une sans-donnée en (3, 3).
    fn holed() -> Dem {
        let mut dem = flat(7, 10.0);
        dem.height = 7;
        dem.origin_y = 70.0;
        dem.values = vec![100.0; 49];
        dem.values[3 * 7 + 3] = -9999.0;
        dem
    }

    #[test]
    fn a_point_without_elevation_is_refused() {
        let err = line_of_sight(&holed(), &params(pt(35.0, 35.0), pt(5.0, 5.0))).unwrap_err();
        assert_eq!(err.code, "terrain.no_elevation");
        let err = line_of_sight(&holed(), &params(pt(5.0, 5.0), pt(35.0, 35.0))).unwrap_err();
        assert_eq!(err.code, "terrain.no_elevation");
        // Une extrémité sur une cellule valide voisine du trou garde l'altitude de sa cellule.
        let r = line_of_sight(&holed(), &params(pt(28.0, 35.0), pt(5.0, 5.0))).unwrap();
        assert_eq!(r.profile[0].elevation, Some(100.0));
        let r = line_of_sight(&holed(), &params(pt(5.0, 5.0), pt(42.0, 38.0))).unwrap();
        assert_eq!(r.profile.last().unwrap().elevation, Some(100.0));
    }

    #[test]
    fn a_hole_on_the_line_hides_nothing_and_is_counted() {
        // Ligne ouest-est par les centres de la ligne 3, à travers le trou.
        let r = line_of_sight(&holed(), &params(pt(5.0, 35.0), pt(65.0, 35.0))).unwrap();
        assert!(r.visible);
        // Échantillons sur les centres, tous les 10 m de x = 5 à 65 : seul x = 35 est le trou.
        assert_eq!(r.nodata_samples, 1);
        // Hors des centres, les quatre voisines comptent : x = 30 lit le trou.
        let r = line_of_sight(&holed(), &params(pt(10.0, 35.0), pt(60.0, 35.0))).unwrap();
        assert_eq!(r.nodata_samples, 2);
        assert_eq!(r.profile.len(), 6);
        assert!(r.profile[3].elevation.is_none());
        assert_eq!(r.profile[0].elevation, Some(100.0));
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["profile"][3]["elevation"], Value::Null);
    }

    #[test]
    fn coincident_points_are_visible_with_one_sample() {
        let p = params(pt(652130.0, 6850250.0), pt(652130.0, 6850250.0));
        let r = line_of_sight(&hill(), &p).unwrap();
        assert!(r.visible);
        assert_eq!(r.distance, 0.0);
        assert_eq!(r.profile.len(), 1);
        assert_eq!(r.profile_step, 0.0);
        assert_eq!(r.nodata_samples, 0);
    }

    #[test]
    fn a_long_line_gives_a_bounded_profile_and_a_verdict_on_every_sample() {
        // 4 km de plat, cellules de 10 m : 400 échantillons.
        let mut dem = flat(401, 10.0);
        // Un mur d'une cellule à x = 1005, et un trou à x = 2005, sur la ligne du milieu.
        dem.values[401 + 100] = 200.0;
        dem.values[401 + 200] = -9999.0;
        let p = params(pt(5.0, 15.0), pt(4005.0, 15.0));
        let r = line_of_sight(&dem, &p).unwrap();
        assert_eq!(r.distance, 4000.0);
        assert_eq!(r.profile.len(), MAX_PROFILE);
        assert_eq!(r.profile[0].distance, 0.0);
        assert_eq!(r.profile[MAX_PROFILE - 1].distance, 4000.0);
        assert_eq!(r.profile_step, 4000.0 / 255.0);
        assert!(!r.visible);
        let o = r.obstruction.unwrap();
        assert!((o.x - 1005.0).abs() < 10.0, "{o:?}");
        // Au pas de 10 m, sur les centres, un seul échantillon tombe sur le trou.
        assert_eq!(r.nodata_samples, 1);
    }

    #[test]
    fn defaults_and_invalid_params() {
        let base = json!({
            "dem": "mnt.tif",
            "observer": {"x": 1, "y": 2, "epsg": 2154},
            "target": {"x": 3, "y": 4, "epsg": 2154},
        });
        let p: TerrainLineOfSightParams = serde_json::from_value(base.clone()).unwrap();
        assert_eq!(
            (p.observer_height, p.target_height, p.curvature, p.refraction),
            (m(1.6), m(0.0), true, 0.13)
        );
        let mut v = base.clone();
        v["observer_height"] = json!({"value": 0.06, "unit": "km"});
        let p: TerrainLineOfSightParams = serde_json::from_value(v).unwrap();
        assert_eq!(p.observer_height.metres(), 60.0);
        for (key, bad) in [
            ("observer_height", json!(60)),
            ("observer_height", json!({"value": 60})),
            ("observer_height", json!({"value": 60, "unit": "mi"})),
            ("observer_height", json!({"value": -1, "unit": "m"})),
            ("target_height", json!({"value": -0.5, "unit": "m"})),
            ("target_height", json!(0)),
            ("refraction", json!(1.0)),
            ("refraction", json!(-0.1)),
            ("curvature", json!("oui")),
        ] {
            let mut v = base.clone();
            v[key] = bad;
            let err = call(v).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{key}");
        }
        let mut v = base.clone();
        v["out"] = json!("x.tif");
        assert_eq!(call(v).unwrap_err().code(), "registry.invalid_params");
    }

    #[test]
    fn two_runs_give_identical_results() {
        let p = json!({
            "dem": fixture("mnt-colline-2154.tif").to_string_lossy(),
            "observer": {"x": 652030, "y": 6850250, "epsg": 2154},
            "target": {"x": 652470, "y": 6850260, "epsg": 2154},
            "observer_height": {"value": 60, "unit": "m"},
            "target_height": {"value": 0.06, "unit": "km"},
        });
        let a = call(p.clone()).unwrap();
        let b = call(p).unwrap();
        assert_eq!(a.to_string(), b.to_string());
        assert_eq!(a["visible"], false);
    }

    #[test]
    fn terrain_line_of_sight_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("terrain.line_of_sight").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert_eq!(spec.path_params, vec!["dem".to_string()]);
        assert!(spec.output_params.is_empty());
        let violations = registry.check_contract();
        assert!(
            violations.iter().all(|v| v.id != "terrain.line_of_sight"),
            "{violations:?}"
        );
        let schema = &spec.output_schema;
        assert_eq!(
            schema["properties"]["profile"]["maxItems"],
            json!(MAX_PROFILE)
        );
        let obstruction = &schema["properties"]["obstruction"];
        let candidates: Vec<&Value> = match obstruction.get("anyOf").and_then(Value::as_array) {
            Some(alts) => alts.iter().collect(),
            None => vec![obstruction],
        };
        assert!(
            candidates
                .iter()
                .any(|c| crate::point::is_point(c, schema)),
            "{obstruction}"
        );
        assert!(schema.to_string().contains("\"x-cassini\":\"point\""));
        let params = &spec.params_schema;
        assert!(crate::point::is_point(
            &params["properties"]["observer"],
            params
        ));
        assert_eq!(
            params["properties"]["refraction"]["exclusiveMaximum"],
            json!(1.0)
        );
        assert_eq!(
            params["properties"]["observer_height"]["default"],
            json!({"value": 1.6, "unit": "m"})
        );
        assert_eq!(
            params["properties"]["target_height"]["default"],
            json!({"value": 0.0, "unit": "m"})
        );
        // `unit` du résultat ne peut valoir que `m`, en direct ou par référence.
        let mut unit = &schema["properties"]["unit"];
        if let Some(r) = unit.get("$ref").and_then(Value::as_str) {
            let name = r.rsplit('/').next().unwrap();
            unit = schema
                .get("$defs")
                .or_else(|| schema.get("definitions"))
                .map(|d| &d[name])
                .unwrap();
        }
        assert!(unit.to_string().contains("\"m\""), "{unit}");
        assert!(!unit.to_string().contains("\"km\""), "{unit}");
    }
}
