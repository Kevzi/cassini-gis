//! Commande `geo.bearing_intersection` : recoupement de deux visées géodésiques, chacune
//! donnée par une station et un azimut, sur l'ellipsoïde WGS84 (`geodesic.rs`), AD-31, CAP-15.
//!
//! Seule compte l'intersection en avant des deux stations, la plus proche. Deux visées qui se
//! coupent sous moins de 1°, ou portées par la même géodésique, sont tenues pour parallèles
//! (décision du propriétaire, 2026-11-05) : `geo.no_intersection`, qui donne l'angle. Le point
//! est rendu dans la zone UTM de `first` (l'hémisphère suit la latitude), ou en WGS84 si
//! `first` l'est ; en UTM, à plus d'une zone de celle de `first`, il est
//! refusé (`coord.out_of_range`), comme pour `geo.destination_point`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::analysis_buffer::Distance;
use super::geo_point::{metres, read_point, render_point};
use crate::angle::Angle;
use crate::geodesic::{self, Intersection, PARALLEL_LIMIT_DEG};
use crate::point::Point;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

const ID: &str = "geo.bearing_intersection";

/// Paramètres de `geo.bearing_intersection`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeoBearingIntersectionParams {
    /// Première station, écrite dans la demande (EPSG 4326, 326NN ou 327NN).
    pub first: Point,
    /// Visée depuis la première station, nord vrai, sens horaire.
    pub first_azimuth: Angle,
    /// Seconde station, même liste de systèmes.
    pub second: Point,
    /// Visée depuis la seconde station.
    pub second_azimuth: Angle,
}

/// Résultat de `geo.bearing_intersection`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GeoBearingIntersectionOutput {
    /// Intersection, dans la zone UTM de `first` (l'hémisphère suit la latitude), ou en
    /// WGS84 si `first` l'est.
    pub point: Point,
    /// Distance géodésique de `first` à l'intersection, en mètres.
    pub distance_from_first: Distance,
    /// Distance géodésique de `second` à l'intersection, en mètres.
    pub distance_from_second: Distance,
}

/// Échec `geo.no_intersection`.
fn no_intersection(message: String) -> CommandFailure {
    CommandFailure::new("geo.no_intersection", message)
}

/// Angle en degrés, à la française, tronqué au millième : un angle sous la limite ne s'affiche
/// jamais à la limite (0,9996° donne « 0,999° », pas « 1,000° »). Un milliardième de degré
/// absorbe le bruit du calcul (0,5° calculé 0,49999999999° s'affiche « 0,500° »).
fn french_degrees(angle: f64) -> String {
    let truncated = libm::trunc(angle * 1e3 + 1e-6) / 1e3;
    format!("{truncated:.3}").replace('.', ",")
}

/// `geo.bearing_intersection` : commande du noyau.
pub struct GeoBearingIntersection;

impl Command for GeoBearingIntersection {
    type Params = GeoBearingIntersectionParams;
    type Output = GeoBearingIntersectionOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<GeoBearingIntersectionParams, GeoBearingIntersectionOutput>(
            ID,
            "Recoupement : intersection de deux visées géodésiques, chacune donnée par une station en WGS84 (EPSG:4326, x longitude, y latitude) ou UTM (EPSG:326NN, 327NN) et un azimut (nord vrai, sens horaire), sur l'ellipsoïde WGS84 ; seule compte l'intersection en avant des deux stations, rendue dans la zone UTM de `first` (l'hémisphère suit la latitude) ou en WGS84 si `first` l'est.",
            "Resection by two bearings: intersection of two geodesic sight lines, each given by a station in WGS84 (EPSG:4326, x longitude, y latitude) or UTM on WGS84 (EPSG:326NN, 327NN) and an azimuth (true north, clockwise), on the WGS84 ellipsoid. Only the nearest intersection ahead of both stations counts; it is returned in the UTM zone of `first` (hemisphere following the latitude), or in WGS84 if `first` is. Sight lines crossing at less than 1 degree are treated as parallel.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .example(serde_json::json!({
            "first": {"x": 2.0, "y": 48.0, "epsg": 4326}, "first_azimuth": {"value": 45},
            "second": {"x": 2.2, "y": 48.0, "epsg": 4326}, "second_azimuth": {"value": 315}}))
        .example(serde_json::json!({
            "first": {"x": 448251.0, "y": 5411750.0, "epsg": 32631},
            "first_azimuth": {"value": 50, "unit": "gon"},
            "second": {"x": 458251.0, "y": 5411750.0, "epsg": 32631},
            "second_azimuth": {"value": 350, "unit": "gon"}}))
        .tests(&[
            "geographiclib_references_are_recovered_by_intersection",
            "utm_stations_give_a_point_in_the_zone_of_the_first",
            "sight_lines_crossing_only_behind_do_not_intersect",
            "parallel_or_shared_sight_lines_do_not_intersect",
            "an_intersection_beyond_the_next_zone_is_out_of_range",
            "lambert_93_station_names_data_transform_point",
            "geo_bearing_intersection_runs_in_the_core_and_its_point_is_reusable",
        ])
    }

    fn run(&self, params: GeoBearingIntersectionParams) -> Result<GeoBearingIntersectionOutput, CommandFailure> {
        let (lat1, lon1) = read_point(&params.first, ID)?;
        let (lat2, lon2) = read_point(&params.second, ID)?;
        let found = geodesic::intersection(
            lat1,
            lon1,
            params.first_azimuth.degrees(),
            lat2,
            lon2,
            params.second_azimuth.degrees(),
        );
        let (lat, lon, s1, s2) = match found {
            Intersection::Found { lat, lon, s1, s2 } => (lat, lon, s1, s2),
            Intersection::Parallel { angle } => {
                return Err(no_intersection(format!(
                    "Les deux visées se coupent sous un angle de {}°, inférieur à {}° : elles sont tenues pour parallèles, et leur recoupement n'est pas défini.",
                    french_degrees(angle),
                    PARALLEL_LIMIT_DEG
                )))
            }
            Intersection::Behind => {
                return Err(no_intersection(
                    "Les deux visées ne se coupent pas en avant des deux stations (à moins d'un demi-tour de la Terre) : vérifiez les azimuts.".into(),
                ))
            }
            Intersection::NoConvergence => {
                return Err(no_intersection(
                    "Le recoupement des deux visées n'a pas pu être calculé : vérifiez les stations et les azimuts.".into(),
                ))
            }
        };
        Ok(GeoBearingIntersectionOutput {
            point: render_point(lat, lon, params.first.epsg)?,
            distance_from_first: metres(s1),
            distance_from_second: metres(s2),
        })
    }
}

inventory::submit! { Registration(|r| r.register(GeoBearingIntersection)) }

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::geodesic::reference::GEODTEST;
    use crate::geodesy::wrap_longitude;
    use crate::registry::{Registry, RegistryError};

    fn intersect(params: Value) -> Result<Value, RegistryError> {
        Registry::standard().call(ID, params)
    }

    fn wgs84(lon: f64, lat: f64) -> Value {
        json!({"x": wrap_longitude(lon), "y": lat, "epsg": 4326})
    }

    fn deg(value: f64) -> Value {
        json!({"value": value})
    }

    fn value(v: &Value) -> f64 {
        v.as_f64().unwrap()
    }

    // Jeu de référence — GeographicLib 2.0 (C. F. F. Karney, licence MIT), géodésiques GeodTest
    // de `test_geodesic.py` (voir `geodesic::reference`) : la première station et sa visée sont
    // le départ et l'azimut publiés ; la seconde station est prise à 500 km de la première,
    // perpendiculairement, et vise l'arrivée publiée. Le recoupement doit redonner l'arrivée et
    // la longueur publiées.
    #[test]
    fn geographiclib_references_are_recovered_by_intersection() {
        let mut tried = 0;
        for (lat1, lon1, azi1, lat2, lon2, _, s12) in GEODTEST {
            let side = geodesic::direct(lat1, lon1, azi1 + 90.0, 500_000.0);
            let sight = geodesic::inverse(side.lat, side.lon, lat2, lon2);
            // Seconde visée la plus courte, et assez franche (pas de visée presque rasante).
            if sight.distance > 19_000_000.0 {
                continue;
            }
            let out = intersect(json!({"first": wgs84(lon1, lat1), "first_azimuth": deg(azi1),
                "second": wgs84(side.lon, side.lat), "second_azimuth": deg(sight.azi1)}));
            let Ok(out) = out else {
                let err = out.unwrap_err();
                assert_eq!(err.code(), "geo.no_intersection", "{err}");
                assert!(err.to_string().contains("parallèles"), "{err}");
                continue;
            };
            tried += 1;
            let point = &out["point"];
            assert_eq!(point["epsg"], 4326);
            let dx = (value(&point["x"]) - wrap_longitude(lon2)).abs();
            assert!(dx.min(360.0 - dx) <= 1e-8, "{lat1} {lon1} : {point}");
            assert!((value(&point["y"]) - lat2).abs() <= 1e-8, "{lat1} {lon1} : {point}");
            let s1 = value(&out["distance_from_first"]["value"]);
            assert!((s1 - s12).abs() <= 2e-3, "{s1} / {s12}");
            let s2 = value(&out["distance_from_second"]["value"]);
            assert!((s2 - sight.distance).abs() <= 2e-3, "{s2} / {}", sight.distance);
            assert_eq!(out["distance_from_first"]["unit"], "m");
        }
        assert!(tried >= 12, "{tried} recoupements seulement");
    }

    #[test]
    fn utm_stations_give_a_point_in_the_zone_of_the_first() {
        // Deux stations à 10 km l'une de l'autre en 31N, visées à 50 gon et 350 gon : le point
        // est à mi-chemin, à environ 5 km au nord (à la convergence des méridiens près : les
        // azimuts sont comptés depuis le nord vrai, non depuis le nord du quadrillage), et ses
        // distances sont celles de l'inverse.
        let first = json!({"x": 448251.0, "y": 5411750.0, "epsg": 32631});
        let second = json!({"x": 458251.0, "y": 5411750.0, "epsg": 32631});
        let out = intersect(json!({"first": first, "first_azimuth": {"value": 50, "unit": "gon"},
            "second": second, "second_azimuth": {"value": 350, "unit": "gon"}}))
        .unwrap();
        let point = &out["point"];
        assert_eq!(point["epsg"], 32631);
        assert!((value(&point["x"]) - 453251.0).abs() < 200.0, "{point}");
        assert!((value(&point["y"]) - 5416750.0).abs() < 200.0, "{point}");
        for (station, field) in [(&first, "distance_from_first"), (&second, "distance_from_second")] {
            let check = Registry::standard()
                .call("geo.distance_azimuth", json!({"from": station, "to": point}))
                .unwrap();
            let d = value(&check["distance"]["value"]);
            assert!((d - value(&out[field]["value"])).abs() <= 2e-3, "{field} : {d} / {out}");
        }
        // Une seconde station en WGS84 : le point reste dans le système de la première.
        let out2 = intersect(json!({"first": first, "first_azimuth": {"value": 45},
            "second": wgs84(2.4, 48.9), "second_azimuth": {"value": 270}}))
        .unwrap();
        assert_eq!(out2["point"]["epsg"], 32631);
        assert_eq!(serde_json::from_str::<Value>(&out.to_string()).unwrap(), out);
    }

    #[test]
    fn sight_lines_crossing_only_behind_do_not_intersect() {
        // Visées divergentes : elles ne se coupent qu'en arrière (puis vers l'antipode).
        for (a1, a2) in [(315.0, 45.0), (225.0, 135.0), (90.0, 0.0)] {
            let err = intersect(json!({"first": wgs84(2.0, 48.0), "first_azimuth": deg(a1),
                "second": wgs84(2.2, 48.0), "second_azimuth": deg(a2)}))
            .unwrap_err();
            assert_eq!(err.code(), "geo.no_intersection", "{a1} {a2}");
            assert!(err.to_string().contains("en avant"), "{err}");
        }
    }

    #[test]
    fn parallel_or_shared_sight_lines_do_not_intersect() {
        // L'angle affiché est tronqué, jamais arrondi jusqu'à la limite.
        assert_eq!(french_degrees(0.9996), "0,999");
        assert_eq!(french_degrees(0.5), "0,500");
        assert_eq!(french_degrees(0.499_999_999_99), "0,500");
        // Deux visées nord voisines, à 0,5° de longitude : les méridiens se coupent au pôle sous
        // 0,5°.
        let err = intersect(json!({"first": wgs84(2.0, 48.0), "first_azimuth": deg(0.0),
            "second": wgs84(2.5, 48.0), "second_azimuth": deg(0.0)}))
        .unwrap_err();
        assert_eq!(err.code(), "geo.no_intersection");
        assert!(err.to_string().contains("0,500°"), "{err}");
        // Même géodésique : deux stations sur l'équateur, toutes deux vers l'est.
        let err = intersect(json!({"first": wgs84(2.0, 0.0), "first_azimuth": deg(90.0),
            "second": wgs84(3.0, 0.0), "second_azimuth": deg(90.0)}))
        .unwrap_err();
        assert_eq!(err.code(), "geo.no_intersection");
        assert!(err.to_string().contains("0,000°"), "{err}");
        // Sous 0,9° : parallèles ; sous 1,2° : un recoupement.
        let near = |a2: f64| intersect(json!({"first": wgs84(2.0, 48.0),
            "first_azimuth": deg(45.0), "second": wgs84(2.2, 48.0), "second_azimuth": deg(a2)}));
        assert_eq!(near(45.9).unwrap_err().code(), "geo.no_intersection");
        assert!(near(43.8).is_ok());
    }

    #[test]
    fn an_intersection_beyond_the_next_zone_is_out_of_range() {
        // Stations au méridien central de la zone 31 ; visées vers l'est-nord-est et l'est-sud-est
        // qui se rejoignent vers 16° E, en zone 33.
        let first = json!({"x": 500000.0, "y": 5_000_000.0, "epsg": 32631});
        let second = json!({"x": 500000.0, "y": 4_800_000.0, "epsg": 32631});
        let err = intersect(json!({"first": first, "first_azimuth": deg(95.0),
            "second": second, "second_azimuth": deg(80.0)}))
        .unwrap_err();
        assert_eq!(err.code(), "coord.out_of_range", "{err}");
        // Les mêmes stations en WGS84 : le point est rendu.
        let (a, b) = (
            Registry::standard()
                .call("coord.convert", json!({"point": first, "to": ["wgs84"]}))
                .unwrap(),
            Registry::standard()
                .call("coord.convert", json!({"point": second, "to": ["wgs84"]}))
                .unwrap(),
        );
        let out = intersect(json!({"first": a["wgs84"], "first_azimuth": deg(95.0),
            "second": b["wgs84"], "second_azimuth": deg(80.0)}))
        .unwrap();
        assert!(value(&out["point"]["x"]) > 12.0, "{out}");
    }

    #[test]
    fn lambert_93_station_names_data_transform_point() {
        let lambert = json!({"x": 652000, "y": 6850000, "epsg": 2154});
        for (first, second) in [(lambert.clone(), wgs84(2.3, 48.8)), (wgs84(2.3, 48.8), lambert)] {
            let err = intersect(json!({"first": first, "first_azimuth": deg(10.0),
                "second": second, "second_azimuth": deg(300.0)}))
            .unwrap_err();
            assert_eq!(err.code(), "geo.unsupported_crs");
            assert!(err.to_string().contains("data.transform_point"), "{err}");
        }
        let err = intersect(json!({"first": wgs84(2.0, 48.0), "first_azimuth": {"value": null},
            "second": wgs84(2.2, 48.0), "second_azimuth": deg(300.0)}))
        .unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
    }

    #[test]
    fn geo_bearing_intersection_runs_in_the_core_and_its_point_is_reusable() {
        let registry = Registry::standard();
        let spec = registry.spec(ID).unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        let violations = registry.check_contract();
        assert!(violations.iter().all(|v| v.id != ID), "{violations:?}");
        let output = &spec.output_schema;
        assert!(crate::point::is_point(&output["properties"]["point"], output));
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": ID, "params": {"first": {"x": 2.0, "y": 48.0, "epsg": 4326},
                "first_azimuth": {"value": 45},
                "second": {"x": 2.2, "y": 48.0, "epsg": 4326},
                "second_azimuth": {"value": 315}}},
            {"command": "coord.convert", "params": {"point": "step:1.point", "to": ["mgrs"]}}
        ]}))
        .unwrap();
        let checked = registry.check_plan(&plan);
        assert!(checked.is_ok(), "{checked:?}");
    }
}
