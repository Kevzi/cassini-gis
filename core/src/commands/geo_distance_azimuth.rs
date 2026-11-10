//! Commande `geo.distance_azimuth` : distance géodésique et azimuts entre deux points, sur
//! l'ellipsoïde WGS84 (problème inverse de Karney, `geodesic.rs`), AD-31, CAP-15.
//!
//! Points en WGS84 ou en UTM sur WGS84, éventuellement dans deux systèmes différents de la
//! liste ; tout autre système est refusé (`geo.unsupported_crs`), jamais converti en silence.
//!
//! Avec `north: "grid"` (AD-31 amendée), les azimuts sont des gisements : chacun est l'azimut de
//! la géodésique au point où il est mesuré, diminué de la convergence des méridiens en ce point
//! dans le quadrillage UTM choisi (`geodesy::grid_convergence`, Karney 2011).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::analysis_buffer::Distance;
use super::geo_point::{
    check_in_grid, check_zone_used, grid_zone, metres, read_point, utm_zone_param,
};
use crate::angle::{Angle, AngleUnit, North};
use crate::geodesic;
use crate::geodesy;
use crate::point::Point;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

const ID: &str = "geo.distance_azimuth";

/// Paramètres de `geo.distance_azimuth`. Les commentaires de documentation des champs
/// deviennent les descriptions du schéma, lues par le modèle : ils restent courts.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeoDistanceAzimuthParams {
    /// Point de départ, écrit dans la demande (EPSG 4326, 326NN ou 327NN).
    pub from: Point,
    /// Point d'arrivée, même liste de systèmes.
    pub to: Point,
    /// Unité des angles rendus, seulement si la demande la nomme.
    #[serde(default)]
    pub unit: Option<AngleUnit>,
    /// Nord des azimuts : true (nord vrai) ou grid (gisement, nord du quadrillage UTM), seulement si la demande le nomme.
    #[serde(default)]
    pub north: Option<North>,
    /// Zone UTM du quadrillage, avec north grid et seulement si la demande la nomme.
    #[serde(default, deserialize_with = "utm_zone_param")]
    #[schemars(range(min = 1, max = 60))]
    pub utm_zone: Option<u8>,
}

/// Résultat de `geo.distance_azimuth`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GeoDistanceAzimuthOutput {
    /// Distance géodésique, en mètres.
    pub distance: Distance,
    /// Azimut au départ, de `from` vers `to`, depuis le nord employé, sens horaire.
    pub azimuth: Angle,
    /// Azimut à l'arrivée, de `to` vers `from`.
    pub back_azimuth: Angle,
    /// Nord employé, présent si la demande l'a nommé.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub north: Option<North>,
    /// Convergence des méridiens au départ, présente pour north grid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub convergence: Option<Angle>,
}

/// `geo.distance_azimuth` : commande du noyau.
pub struct GeoDistanceAzimuth;

impl Command for GeoDistanceAzimuth {
    type Params = GeoDistanceAzimuthParams;
    type Output = GeoDistanceAzimuthOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<GeoDistanceAzimuthParams, GeoDistanceAzimuthOutput>(
            ID,
            "Distance géodésique et azimuts (nord vrai, sens horaire) entre deux points en WGS84 (EPSG:4326, x longitude, y latitude) ou UTM (EPSG:326NN, 327NN), sur l'ellipsoïde WGS84. Avec north grid, les azimuts sont des gisements, comptés depuis le nord du quadrillage UTM de from (ou de utm_zone). Un point dans un autre système passe d'abord par data.transform_point, une référence MGRS par coord.convert.",
            "Geodesic distance and azimuths (true north, clockwise) between two points in WGS84 (EPSG:4326, x longitude, y latitude) or UTM on WGS84 (EPSG:326NN, 327NN), on the WGS84 ellipsoid (Karney). The two points may be in different systems of that list. With north grid, the azimuths are grid bearings, from the grid north of the UTM zone of from (or of utm_zone). A point in another system goes through data.transform_point first, an MGRS reference through coord.convert.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .example(serde_json::json!({"from": {"x": 2.3522, "y": 48.8566, "epsg": 4326},
            "to": {"x": 5.3698, "y": 43.2965, "epsg": 4326}}))
        .example(serde_json::json!({"from": {"x": 448251.0, "y": 5411750.0, "epsg": 32631},
            "to": {"x": 2.3522, "y": 48.8566, "epsg": 4326}, "unit": "gon"}))
        .example(serde_json::json!({"from": {"x": 448251.0, "y": 5411750.0, "epsg": 32631},
            "to": {"x": 452300.0, "y": 5414900.0, "epsg": 32631}, "unit": "gon", "north": "grid"}))
        .tests(&[
            "geographiclib_inverse_references_are_found",
            "near_antipodal_references_are_found",
            "points_in_mixed_systems_give_the_same_result",
            "azimuths_are_rendered_in_grads_on_request",
            "coincident_points_are_refused",
            "lambert_93_names_data_transform_point",
            "geo_distance_azimuth_runs_in_the_core_and_takes_step_points",
            "an_absent_or_true_north_keeps_the_true_azimuths",
            "a_grid_bearing_is_the_true_azimuth_less_the_convergence",
            "an_imposed_grid_zone_is_a_neighbour_of_the_start",
            "a_far_end_point_has_no_grid_bearing",
            "a_zone_without_grid_north_is_refused",
            "a_short_grid_bearing_is_within_one_second_of_the_chord",
        ])
    }

    fn run(&self, params: GeoDistanceAzimuthParams) -> Result<GeoDistanceAzimuthOutput, CommandFailure> {
        check_zone_used(params.north, params.utm_zone)?;
        let (lat1, lon1) = read_point(&params.from, ID)?;
        let (lat2, lon2) = read_point(&params.to, ID)?;
        let inverse = geodesic::inverse(lat1, lon1, lat2, lon2);
        let distance = metres(inverse.distance);
        if distance.value == 0.0 {
            return Err(CommandFailure::new(
                "geo.coincident_points",
                "Les deux points sont confondus (à moins d'un demi-millimètre) : la distance est nulle et l'azimut n'est pas défini.",
            ));
        }
        let unit = params.unit.unwrap_or_default();
        // Chaque azimut est corrigé de la convergence au point où il est mesuré : `from` pour
        // l'azimut, `to` pour le contre-azimut.
        let (gamma1, gamma2) = match params.north {
            Some(North::Grid) => {
                let zone = grid_zone(&params.from, lat1, lon1, params.utm_zone)?;
                check_in_grid(lat2, lon2, zone)?;
                (
                    geodesy::grid_convergence(lat1, lon1, zone),
                    geodesy::grid_convergence(lat2, lon2, zone),
                )
            }
            _ => (0.0, 0.0),
        };
        Ok(GeoDistanceAzimuthOutput {
            distance,
            azimuth: Angle::azimuth(inverse.azi1 - gamma1, unit),
            back_azimuth: Angle::azimuth(inverse.azi2 + 180.0 - gamma2, unit),
            north: params.north,
            convergence: (params.north == Some(North::Grid)).then(|| Angle::signed(gamma1, unit)),
        })
    }
}

inventory::submit! { Registration(|r| r.register(GeoDistanceAzimuth)) }

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::geodesic::reference::{GEODSOLVE, GEODTEST};
    use crate::geodesy::{self, wrap_longitude};
    use crate::registry::{Registry, RegistryError};

    fn measure(params: Value) -> Result<Value, RegistryError> {
        Registry::standard().call(ID, params)
    }

    fn wgs84(lon: f64, lat: f64) -> Value {
        json!({"x": wrap_longitude(lon), "y": lat, "epsg": 4326})
    }

    /// Écart entre deux azimuts en degrés, modulo un tour.
    fn gap(a: f64, b: f64) -> f64 {
        let d = (a - b).rem_euclid(360.0);
        d.min(360.0 - d)
    }

    #[track_caller]
    fn assert_result(out: &Value, s12: f64, azi1: f64, azi2: f64, ts: f64, ta: f64) {
        assert_eq!(out["distance"]["unit"], "m", "{out}");
        assert_eq!(out["azimuth"]["unit"], "deg", "{out}");
        let distance = out["distance"]["value"].as_f64().unwrap();
        assert!((distance - s12).abs() <= ts, "distance {distance}, attendu {s12}");
        let azimuth = out["azimuth"]["value"].as_f64().unwrap();
        let back = out["back_azimuth"]["value"].as_f64().unwrap();
        assert!((0.0..360.0).contains(&azimuth) && (0.0..360.0).contains(&back), "{out}");
        assert!(gap(azimuth, azi1) <= ta, "azimut {azimuth}, attendu {azi1}");
        assert!(gap(back, azi2 + 180.0) <= ta, "contre-azimut {back}, attendu {azi2} + 180");
    }

    // Jeu de référence — GeographicLib 2.0 (C. F. F. Karney, licence MIT), géodésiques de
    // `test_geodesic.py` tirées du jeu GeodTest (voir `geodesic::reference`). Distance rendue au
    // millimètre, azimuts au milliardième de degré : demi-pas d'arrondi plus 1e-12.
    #[test]
    fn geographiclib_inverse_references_are_found() {
        for (lat1, lon1, azi1, lat2, lon2, azi2, s12) in GEODTEST {
            let out = measure(json!({"from": wgs84(lon1, lat1), "to": wgs84(lon2, lat2)})).unwrap();
            assert_result(&out, s12, azi1, azi2, 0.5e-3 + 1e-8, 0.5e-9 + 1e-12);
        }
    }

    // Même source, tests GeodSolve 0, 6, 9, 10, 11, 33, 74, 76 et 78 : quasi-antipodes (départ
    // par l'astroïde), équateur, géodésique très courte, échecs connus de Vincenty et du NGS ;
    // tolérances publiées, plus le demi-pas d'arrondi.
    #[test]
    fn near_antipodal_references_are_found() {
        for (lat1, lon1, lat2, lon2, azi1, azi2, s12, ta, ts) in GEODSOLVE {
            let out = measure(json!({"from": wgs84(lon1, lat1), "to": wgs84(lon2, lat2)})).unwrap();
            let distance = out["distance"]["value"].as_f64().unwrap();
            assert!((distance - s12).abs() <= ts + 0.5e-3, "{distance} / {s12}");
            if !azi1.is_nan() {
                assert_result(&out, s12, azi1, azi2, ts + 0.5e-3, ta + 0.5e-9);
            }
        }
    }

    #[test]
    fn points_in_mixed_systems_give_the_same_result() {
        // Le même lieu en UTM 31N et en WGS84 (sa conversion exacte) : même résultat.
        let utm = json!({"x": 448251.0, "y": 5411750.0, "epsg": 32631});
        let (lat, lon) = geodesy::from_utm(&geodesy::Utm {
            zone: 31,
            north: true,
            easting: 448251.0,
            northing: 5411750.0,
        });
        let to = wgs84(5.3698, 43.2965);
        let mixed = measure(json!({"from": utm, "to": to})).unwrap();
        let plain = measure(json!({"from": {"x": lon, "y": lat, "epsg": 4326}, "to": to})).unwrap();
        assert_eq!(mixed, plain);
        // Et dans l'autre sens, l'azimut et le contre-azimut s'échangent.
        let back = measure(json!({"from": to, "to": utm})).unwrap();
        assert_eq!(back["distance"], plain["distance"]);
        assert_eq!(back["azimuth"], plain["back_azimuth"]);
        assert_eq!(back["back_azimuth"], plain["azimuth"]);
        // Deux zones UTM, au nord et au sud de l'équateur (GeodSolve0, en UTM).
        let ny = json!({"x": 600000.0, "y": 4500000.0, "epsg": 32618});
        let sud = json!({"x": 500000.0, "y": 9889470.0, "epsg": 32731});
        let out = measure(json!({"from": ny, "to": sud})).unwrap();
        let (la, lo) = geodesy::from_utm(&geodesy::Utm { zone: 18, north: true,
            easting: 600000.0, northing: 4500000.0 });
        let (lb, lob) = geodesy::from_utm(&geodesy::Utm { zone: 31, north: false,
            easting: 500000.0, northing: 9889470.0 });
        let plain = measure(json!({"from": {"x": lo, "y": la, "epsg": 4326},
            "to": {"x": lob, "y": lb, "epsg": 4326}}))
        .unwrap();
        assert_eq!(out, plain);
        // Le résultat écrit puis relu en JSON est le même : condition du rejeu.
        assert_eq!(serde_json::from_str::<Value>(&out.to_string()).unwrap(), out);
    }

    #[test]
    fn azimuths_are_rendered_in_grads_on_request() {
        // GeodSolve76 (Wellington – Salamanque) : 160,39137649664° et 19,50042925176°.
        let (from, to) = (wgs84(174.0 + 49.0 / 60.0, -(41.0 + 19.0 / 60.0)),
            wgs84(-(5.0 + 30.0 / 60.0), 40.0 + 58.0 / 60.0));
        let out = measure(json!({"from": from, "to": to, "unit": "gon"})).unwrap();
        assert_eq!(out["azimuth"]["unit"], "gon");
        assert_eq!(out["back_azimuth"]["unit"], "gon");
        let azimuth = out["azimuth"]["value"].as_f64().unwrap();
        let back = out["back_azimuth"]["value"].as_f64().unwrap();
        assert!((azimuth - 160.391_376_496_64 / 0.9).abs() <= 1e-9, "{azimuth}");
        assert!((back - (19.500_429_251_76 + 180.0) / 0.9).abs() <= 1e-9, "{back}");
        assert_eq!(out["distance"], json!({"value": 19960543.857, "unit": "m"}));
        // Vers l'ouest le long du parallèle : la géodésique part un peu au nord de l'ouest,
        // vers 300,04 gon.
        let out = measure(json!({"from": wgs84(2.0, 48.0), "to": wgs84(1.9, 48.0),
            "unit": "gon"}))
        .unwrap();
        let azimuth = out["azimuth"]["value"].as_f64().unwrap();
        assert!((300.0..301.0).contains(&azimuth), "{azimuth}");
        for unit in ["rad", "mil"] {
            let err = measure(json!({"from": wgs84(2.0, 48.0), "to": wgs84(1.9, 48.0),
                "unit": unit}))
            .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{unit}");
        }
    }

    #[test]
    fn coincident_points_are_refused() {
        let utm = json!({"x": 448251.0, "y": 5411750.0, "epsg": 32631});
        let (lat, lon) = geodesy::from_utm(&geodesy::Utm {
            zone: 31,
            north: true,
            easting: 448251.0,
            northing: 5411750.0,
        });
        for (from, to) in [
            (wgs84(2.3, 48.8), wgs84(2.3, 48.8)),
            (utm.clone(), utm.clone()),
            (utm, json!({"x": lon, "y": lat, "epsg": 4326})),
        ] {
            let err = measure(json!({"from": from, "to": to})).unwrap_err();
            assert_eq!(err.code(), "geo.coincident_points");
            assert!(err.to_string().contains("confondus"), "{err}");
        }
    }

    #[test]
    fn lambert_93_names_data_transform_point() {
        let lambert = json!({"x": 652000, "y": 6850000, "epsg": 2154});
        for (from, to) in [(lambert.clone(), wgs84(2.3, 48.8)), (wgs84(2.3, 48.8), lambert)] {
            let err = measure(json!({"from": from, "to": to})).unwrap_err();
            assert_eq!(err.code(), "geo.unsupported_crs");
            let message = err.to_string();
            assert!(message.contains("data.transform_point"), "{message}");
            assert!(message.contains("coord.convert"), "{message}");
            assert!(message.contains("EPSG:2154") && message.contains(ID), "{message}");
        }
    }

    #[test]
    fn geo_distance_azimuth_runs_in_the_core_and_takes_step_points() {
        let registry = Registry::standard();
        let spec = registry.spec(ID).unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert!(!spec.dictated_by_core);
        let violations = registry.check_contract();
        assert!(violations.iter().all(|v| v.id != ID), "{violations:?}");
        let params = &spec.params_schema;
        assert!(crate::point::is_point(&params["properties"]["from"], params));
        assert!(crate::point::is_point(&params["properties"]["to"], params));
        // Un point converti d'une référence MGRS par une étape coord.convert se reprend ici.
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "coord.convert", "params": {"mgrs": "31UDQ4825111750", "to": ["wgs84"]}},
            {"command": ID, "params": {"from": "step:1.wgs84",
                "to": {"x": 5.3698, "y": 43.2965, "epsg": 4326}}}
        ]}))
        .unwrap();
        let checked = registry.check_plan(&plan);
        assert!(checked.is_ok(), "{checked:?}");
    }

    // -- gisement (epic 10, entrée 28) ---------------------------------------------------------

    fn value(v: &Value) -> f64 {
        v["value"].as_f64().unwrap()
    }

    #[test]
    fn an_absent_or_true_north_keeps_the_true_azimuths() {
        let (from, to) = (wgs84(2.3522, 48.8566), wgs84(5.3698, 43.2965));
        let plain = measure(json!({"from": from, "to": to})).unwrap();
        // Absent : les trois champs d'aujourd'hui, sans autre champ (rejeu des journaux anciens).
        let keys: Vec<&String> = plain.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["azimuth", "back_azimuth", "distance"]);
        // Écrit `true` : même résultat, plus `north`, sans `convergence`.
        let written = measure(json!({"from": from, "to": to, "north": "true"})).unwrap();
        assert_eq!(written["north"], "true");
        assert!(written.get("convergence").is_none(), "{written}");
        let mut without = written.clone();
        without.as_object_mut().unwrap().remove("north");
        assert_eq!(without, plain);
        for bad in [json!("magnetic"), json!("vrai"), json!(true)] {
            let err = measure(json!({"from": from, "to": to, "north": bad})).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn a_grid_bearing_is_the_true_azimuth_less_the_convergence() {
        // Départ en UTM 31N à l'est du méridien central, arrivée en WGS84 : quadrillage de la
        // zone 31, la zone de `from`.
        let from = json!({"x": 650000.0, "y": 5400000.0, "epsg": 32631});
        let (lat1, lon1) = geodesy::from_utm(&geodesy::Utm { zone: 31, north: true,
            easting: 650000.0, northing: 5400000.0 });
        let to = wgs84(4.9, 48.95);
        for unit in ["deg", "gon"] {
            let factor = if unit == "gon" { 10.0 / 9.0 } else { 1.0 };
            let truth = measure(json!({"from": from, "to": to, "unit": unit})).unwrap();
            let grid = measure(json!({"from": from, "to": to, "unit": unit, "north": "grid"}))
                .unwrap();
            assert_eq!(grid["north"], "grid");
            assert_eq!(grid["distance"], truth["distance"]);
            assert_eq!(grid["convergence"]["unit"], unit);
            let g1 = geodesy::grid_convergence(lat1, lon1, 31);
            let g2 = geodesy::grid_convergence(48.95, 4.9, 31);
            assert!(g1 > 0.0 && g2 > 0.0, "à l'est du méridien central, au nord");
            assert!((value(&grid["convergence"]) - g1 * factor).abs() <= 1e-9, "{grid}");
            let full = 360.0 * factor;
            let d = (value(&truth["azimuth"]) - g1 * factor - value(&grid["azimuth"]))
                .rem_euclid(full);
            assert!(d.min(full - d) <= 2e-9, "{grid} / {truth}");
            let d = (value(&truth["back_azimuth"]) - g2 * factor - value(&grid["back_azimuth"]))
                .rem_euclid(full);
            assert!(d.min(full - d) <= 2e-9, "{grid} / {truth}");
        }
        // Départ en WGS84 : sa zone normale. À l'ouest du méridien central (zone 31 : 3° E), la
        // convergence est négative ; dans l'hémisphère sud, à l'est, aussi.
        let out = measure(json!({"from": wgs84(1.5, 45.0), "to": wgs84(1.6, 45.1),
            "north": "grid"})).unwrap();
        assert!(value(&out["convergence"]) < 0.0, "{out}");
        assert!((value(&out["convergence"]) - geodesy::grid_convergence(45.0, 1.5, 31)).abs()
            <= 1e-9);
        let out = measure(json!({"from": wgs84(4.5, -30.0), "to": wgs84(4.6, -30.1),
            "north": "grid"})).unwrap();
        assert!(value(&out["convergence"]) < 0.0, "{out}");
        // Même demande, mêmes octets, relus à l'identique : condition du rejeu.
        let again = measure(json!({"from": from, "to": to, "north": "grid"})).unwrap();
        assert_eq!(again, measure(json!({"from": from, "to": to, "north": "grid"})).unwrap());
        assert_eq!(serde_json::from_str::<Value>(&again.to_string()).unwrap(), again);
    }

    #[test]
    fn an_imposed_grid_zone_is_a_neighbour_of_the_start() {
        // 6,5° E, 48° N : zone normale 32 ; la zone 31 voisine est imposée (P3).
        let (from, to) = (wgs84(6.5, 48.0), wgs84(6.6, 48.1));
        let normal = measure(json!({"from": from, "to": to, "north": "grid"})).unwrap();
        let imposed = measure(json!({"from": from, "to": to, "north": "grid", "utm_zone": 31}))
            .unwrap();
        assert!((value(&normal["convergence"]) - geodesy::grid_convergence(48.0, 6.5, 32)).abs()
            <= 1e-9);
        assert!((value(&imposed["convergence"]) - geodesy::grid_convergence(48.0, 6.5, 31))
            .abs() <= 1e-9);
        assert_ne!(normal["azimuth"], imposed["azimuth"]);
        // `utm_zone` l'emporte aussi sur la zone d'un départ UTM.
        let utm = json!({"x": 448251.0, "y": 5411750.0, "epsg": 32631});
        let out = measure(json!({"from": utm, "to": wgs84(2.4, 48.9), "north": "grid",
            "utm_zone": 30})).unwrap();
        assert!(value(&out["convergence"]) > 0.0, "à l'est du méridien de la zone 30 : {out}");
        // Zone lointaine : refusée.
        let err = measure(json!({"from": from, "to": to, "north": "grid", "utm_zone": 34}))
            .unwrap_err();
        assert_eq!(err.code(), "coord.out_of_range");
        assert!(err.to_string().contains("voisine"), "{err}");
        for bad in [0, 61] {
            let err = measure(json!({"from": from, "to": to, "north": "grid", "utm_zone": bad}))
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
        // Un départ polaire n'a pas de quadrillage UTM.
        let err = measure(json!({"from": wgs84(10.0, 85.0), "to": wgs84(10.0, 83.0),
            "north": "grid"})).unwrap_err();
        assert_eq!(err.code(), "coord.out_of_range");
    }

    #[test]
    fn a_far_end_point_has_no_grid_bearing() {
        // De Paris (zone 31) à Vienne (zone 33) : deux zones plus loin (P1).
        let err = measure(json!({"from": wgs84(2.35, 48.85), "to": wgs84(16.37, 48.21),
            "north": "grid"})).unwrap_err();
        assert_eq!(err.code(), "coord.out_of_range");
        assert!(err.to_string().contains("nord vrai"), "{err}");
        // Vers le pôle, au-delà de 84° N.
        let err = measure(json!({"from": wgs84(2.35, 83.0), "to": wgs84(2.35, 85.0),
            "north": "grid"})).unwrap_err();
        assert_eq!(err.code(), "coord.out_of_range");
        // Le même trajet en nord vrai aboutit, et vers une zone voisine le gisement aussi.
        assert!(measure(json!({"from": wgs84(2.35, 48.85), "to": wgs84(16.37, 48.21)})).is_ok());
        assert!(measure(json!({"from": wgs84(2.35, 48.85), "to": wgs84(7.75, 48.58),
            "north": "grid"})).is_ok());
        // De part et d'autre d'une limite d'exception de la NGA (bande X) : 20,9° E est en
        // zone 33, 21,1° E en zone 35, mais dans la zone numérique 34, voisine : accepté.
        assert!(measure(json!({"from": wgs84(20.9, 75.0), "to": wgs84(21.1, 75.0),
            "north": "grid"})).is_ok());
    }

    #[test]
    fn a_zone_without_grid_north_is_refused() {
        let (from, to) = (wgs84(2.35, 48.85), wgs84(2.4, 48.9));
        for params in [
            json!({"from": from, "to": to, "utm_zone": 31}),
            json!({"from": from, "to": to, "utm_zone": 31, "north": "true"}),
        ] {
            let err = measure(params).unwrap_err();
            assert_eq!(err.code(), "geo.zone_unused");
            assert!(err.to_string().contains("grid"), "{err}");
        }
    }

    // Correction de corde : à 300 km du méridien central, le gisement de la géodésique d'un
    // segment de 1 km diffère de celui de la corde (ΔE, ΔN) de moins d'une seconde d'arc
    // (≈ ΔN·(2E₁ + E₂) / (6 k0² R²), soit 0,76″ plein nord à 45° N, nulle plein est).
    #[test]
    fn a_short_grid_bearing_is_within_one_second_of_the_chord() {
        let (e1, n1) = (800_000.0, 5_000_000.0);
        let mut largest: f64 = 0.0;
        for k in 0..16 {
            let t = f64::from(k) * 22.5_f64.to_radians();
            let (de, dn) = (1000.0 * t.sin(), 1000.0 * t.cos());
            let out = measure(json!({"from": {"x": e1, "y": n1, "epsg": 32631},
                "to": {"x": e1 + de, "y": n1 + dn, "epsg": 32631}, "north": "grid"}))
            .unwrap();
            let chord = libm::atan2(de, dn).to_degrees().rem_euclid(360.0);
            let gap = gap(value(&out["azimuth"]), chord) * 3600.0;
            largest = largest.max(gap);
            assert!(gap < 1.0, "{k} : {gap}″");
        }
        assert!(largest > 0.5, "la correction de corde est visible : {largest}″");
    }
}
