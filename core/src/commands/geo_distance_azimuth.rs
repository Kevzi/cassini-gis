//! Commande `geo.distance_azimuth` : distance géodésique et azimuts entre deux points, sur
//! l'ellipsoïde WGS84 (problème inverse de Karney, `geodesic.rs`), AD-31, CAP-15.
//!
//! Points en WGS84 ou en UTM sur WGS84, éventuellement dans deux systèmes différents de la
//! liste ; tout autre système est refusé (`geo.unsupported_crs`), jamais converti en silence.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::analysis_buffer::Distance;
use super::geo_point::{metres, read_point};
use crate::angle::{Angle, AngleUnit};
use crate::geodesic;
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
}

/// Résultat de `geo.distance_azimuth`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GeoDistanceAzimuthOutput {
    /// Distance géodésique, en mètres.
    pub distance: Distance,
    /// Azimut au départ, de `from` vers `to`, nord vrai, sens horaire.
    pub azimuth: Angle,
    /// Azimut à l'arrivée, de `to` vers `from`.
    pub back_azimuth: Angle,
}

/// `geo.distance_azimuth` : commande du noyau.
pub struct GeoDistanceAzimuth;

impl Command for GeoDistanceAzimuth {
    type Params = GeoDistanceAzimuthParams;
    type Output = GeoDistanceAzimuthOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<GeoDistanceAzimuthParams, GeoDistanceAzimuthOutput>(
            ID,
            "Distance géodésique et azimuts (nord vrai, sens horaire) entre deux points en WGS84 (EPSG:4326, x longitude, y latitude) ou UTM (EPSG:326NN, 327NN), sur l'ellipsoïde WGS84. Un point dans un autre système passe d'abord par data.transform_point, une référence MGRS par coord.convert.",
            "Geodesic distance and azimuths (true north, clockwise) between two points in WGS84 (EPSG:4326, x longitude, y latitude) or UTM on WGS84 (EPSG:326NN, 327NN), on the WGS84 ellipsoid (Karney). The two points may be in different systems of that list. A point in another system goes through data.transform_point first, an MGRS reference through coord.convert.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .example(serde_json::json!({"from": {"x": 2.3522, "y": 48.8566, "epsg": 4326},
            "to": {"x": 5.3698, "y": 43.2965, "epsg": 4326}}))
        .example(serde_json::json!({"from": {"x": 448251.0, "y": 5411750.0, "epsg": 32631},
            "to": {"x": 2.3522, "y": 48.8566, "epsg": 4326}, "unit": "gon"}))
        .tests(&[
            "geographiclib_inverse_references_are_found",
            "near_antipodal_references_are_found",
            "points_in_mixed_systems_give_the_same_result",
            "azimuths_are_rendered_in_grads_on_request",
            "coincident_points_are_refused",
            "lambert_93_names_data_transform_point",
            "geo_distance_azimuth_runs_in_the_core_and_takes_step_points",
        ])
    }

    fn run(&self, params: GeoDistanceAzimuthParams) -> Result<GeoDistanceAzimuthOutput, CommandFailure> {
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
        Ok(GeoDistanceAzimuthOutput {
            distance,
            azimuth: Angle::azimuth(inverse.azi1, unit),
            back_azimuth: Angle::azimuth(inverse.azi2 + 180.0, unit),
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
}
