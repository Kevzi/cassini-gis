//! Commande `geo.destination_point` : point atteint depuis un départ, à un azimut et une
//! distance géodésiques (problème direct de Karney, `geodesic.rs`), AD-31, CAP-15.
//!
//! Le point d'arrivée est rendu dans le système du départ : WGS84, ou la même zone UTM
//! (l'hémisphère suit la latitude) ; à plus d'une zone de celle du départ, il est refusé
//! (`coord.out_of_range`) au lieu d'être rendu dans une zone que la demande n'a pas choisie.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::analysis_buffer::Distance;
use super::geo_point::{read_point, render_point};
use crate::angle::Angle;
use crate::geodesic;
use crate::point::Point;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

const ID: &str = "geo.destination_point";

/// Paramètres de `geo.destination_point`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeoDestinationPointParams {
    /// Point de départ, écrit dans la demande (EPSG 4326, 326NN ou 327NN).
    pub from: Point,
    /// Azimut de départ, nord vrai, sens horaire.
    pub azimuth: Angle,
    /// Distance géodésique à parcourir.
    pub distance: Distance,
}

/// Résultat de `geo.destination_point`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GeoDestinationPointOutput {
    /// Point d'arrivée, dans la zone UTM de `from` (l'hémisphère suit la latitude), ou en
    /// WGS84 si `from` l'est.
    pub point: Point,
}

/// `geo.destination_point` : commande du noyau.
pub struct GeoDestinationPoint;

impl Command for GeoDestinationPoint {
    type Params = GeoDestinationPointParams;
    type Output = GeoDestinationPointOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<GeoDestinationPointParams, GeoDestinationPointOutput>(
            ID,
            "Point atteint depuis un point de départ en WGS84 (EPSG:4326, x longitude, y latitude) ou UTM (EPSG:326NN, 327NN), à un azimut (nord vrai, sens horaire) et une distance géodésiques, sur l'ellipsoïde WGS84 ; rendu dans la zone UTM du départ (l'hémisphère suit la latitude) ou en WGS84 si le départ l'est.",
            "Point reached from a start point in WGS84 (EPSG:4326, x longitude, y latitude) or UTM on WGS84 (EPSG:326NN, 327NN), along a geodesic at an azimuth (true north, clockwise) and a distance, on the WGS84 ellipsoid (Karney); returned in the UTM zone of the start point (hemisphere following the latitude), or in WGS84 if the start point is. A point in another system goes through data.transform_point first, an MGRS reference through coord.convert.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .example(serde_json::json!({"from": {"x": 2.3522, "y": 48.8566, "epsg": 4326},
            "azimuth": {"value": 45}, "distance": {"value": 10, "unit": "km"}}))
        .example(serde_json::json!({"from": {"x": 448251.0, "y": 5411750.0, "epsg": 32631},
            "azimuth": {"value": 100, "unit": "gon"}, "distance": {"value": 1500, "unit": "m"}}))
        .tests(&[
            "geographiclib_direct_references_are_found",
            "a_utm_start_gives_a_point_in_its_zone",
            "a_point_beyond_the_next_zone_is_out_of_range",
            "a_direct_and_its_inverse_agree",
            "invalid_angles_and_distances_are_refused",
            "lambert_93_start_names_data_transform_point",
            "geo_destination_point_runs_in_the_core_and_its_point_is_reusable",
        ])
    }

    fn run(&self, params: GeoDestinationPointParams) -> Result<GeoDestinationPointOutput, CommandFailure> {
        let (lat, lon) = read_point(&params.from, ID)?;
        let end = geodesic::direct(lat, lon, params.azimuth.degrees(), params.distance.metres());
        Ok(GeoDestinationPointOutput {
            point: render_point(end.lat, end.lon, params.from.epsg)?,
        })
    }
}

inventory::submit! { Registration(|r| r.register(GeoDestinationPoint)) }

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::geodesic::reference::GEODTEST;
    use crate::geodesy::wrap_longitude;
    use crate::registry::{Registry, RegistryError};

    fn reach(params: Value) -> Result<Value, RegistryError> {
        Registry::standard().call(ID, params)
    }

    fn wgs84(lon: f64, lat: f64) -> Value {
        json!({"x": wrap_longitude(lon), "y": lat, "epsg": 4326})
    }

    #[track_caller]
    fn assert_wgs84(got: &Value, lon: f64, lat: f64, tolerance: f64) {
        assert_eq!(got["epsg"], 4326, "{got}");
        let (x, y) = (got["x"].as_f64().unwrap(), got["y"].as_f64().unwrap());
        let dx = (x - wrap_longitude(lon)).abs();
        assert!(
            dx.min(360.0 - dx) <= tolerance && (y - lat).abs() <= tolerance,
            "attendu ({lon}, {lat}), trouvé ({x}, {y})"
        );
    }

    // Jeu de référence — GeographicLib 2.0 (C. F. F. Karney, licence MIT), géodésiques de
    // `test_geodesic.py` tirées du jeu GeodTest (voir `geodesic::reference`), et GeodSolve1
    // (New York – Paris, 5 850 km, valeurs publiées à 0,5e-5°). Point rendu au milliardième de
    // degré : demi-pas d'arrondi plus 1e-12.
    #[test]
    fn geographiclib_direct_references_are_found() {
        for (lat1, lon1, azi1, lat2, lon2, _, s12) in GEODTEST {
            let out = reach(json!({"from": wgs84(lon1, lat1), "azimuth": {"value": azi1},
                "distance": {"value": s12, "unit": "m"}}))
            .unwrap();
            assert_wgs84(&out["point"], lon2, lat2, 0.5e-9 + 1e-12);
        }
        let out = reach(json!({"from": wgs84(-73.77888889, 40.63972222),
            "azimuth": {"value": 53.5, "unit": "deg"}, "distance": {"value": 5850, "unit": "km"}}))
        .unwrap();
        assert_wgs84(&out["point"], 2.56106, 49.01467, 0.5e-5);
    }

    #[test]
    fn a_utm_start_gives_a_point_in_its_zone() {
        // 100 gon = plein est : 10 km plus loin, toujours en 31N, ordonnée presque inchangée.
        let start = json!({"x": 448251.0, "y": 5411750.0, "epsg": 32631});
        let out = reach(json!({"from": start, "azimuth": {"value": 100, "unit": "gon"},
            "distance": {"value": 10, "unit": "km"}}))
        .unwrap();
        let point = &out["point"];
        assert_eq!(point["epsg"], 32631);
        let x = point["x"].as_f64().unwrap();
        assert!((x - 458251.0).abs() < 10.0, "{point}");
        // Rendu au millimètre.
        assert_eq!(x, (x * 1e3).round() / 1e3);
        // À 300 km plein est (zone 32 voisine), la zone 31 est gardée.
        let out = reach(json!({"from": start, "azimuth": {"value": 90},
            "distance": {"value": 300, "unit": "km"}}))
        .unwrap();
        assert_eq!(out["point"]["epsg"], 32631);
        assert!(out["point"]["x"].as_f64().unwrap() > 740_000.0, "{out}");
        // Vers le sud à travers l'équateur : même zone, hémisphère sud.
        let near_equator = json!({"x": 500000.0, "y": 50000.0, "epsg": 32631});
        let out = reach(json!({"from": near_equator, "azimuth": {"value": 180},
            "distance": {"value": 200, "unit": "km"}}))
        .unwrap();
        assert_eq!(out["point"]["epsg"], 32731, "{out}");
        let y = out["point"]["y"].as_f64().unwrap();
        assert!((y - (10_000_000.0 - 150_000.0)).abs() < 1_000.0, "{out}");
    }

    #[test]
    fn a_point_beyond_the_next_zone_is_out_of_range() {
        // Depuis le méridien central de la zone 31 (3° E), 1 000 km plein est : vers 16,5° E,
        // en zone 33, à deux zones de la 31.
        let start = json!({"x": 500000.0, "y": 5_000_000.0, "epsg": 32631});
        let err = reach(json!({"from": start, "azimuth": {"value": 90},
            "distance": {"value": 1000, "unit": "km"}}))
        .unwrap_err();
        assert_eq!(err.code(), "coord.out_of_range");
        assert!(err.to_string().contains("zone UTM 33"), "{err}");
        // Le même trajet depuis un départ WGS84 n'a pas de zone : il aboutit.
        assert!(reach(json!({"from": wgs84(3.0, 45.15), "azimuth": {"value": 90},
            "distance": {"value": 1000, "unit": "km"}}))
        .is_ok());
        // Vers le pôle, au-delà de 84° N : hors de la grille UTM.
        let err = reach(json!({"from": start, "azimuth": {"value": 0},
            "distance": {"value": 5000, "unit": "km"}}))
        .unwrap_err();
        assert_eq!(err.code(), "coord.out_of_range");
    }

    #[test]
    fn a_direct_and_its_inverse_agree() {
        let from = wgs84(2.3522, 48.8566);
        let out = reach(json!({"from": from, "azimuth": {"value": 157.5, "unit": "gon"},
            "distance": {"value": 660.5, "unit": "km"}}))
        .unwrap();
        let back = Registry::standard()
            .call("geo.distance_azimuth", json!({"from": from, "to": out["point"]}))
            .unwrap();
        let distance = back["distance"]["value"].as_f64().unwrap();
        assert!((distance - 660_500.0).abs() <= 2e-3, "{back}");
        let azimuth = back["azimuth"]["value"].as_f64().unwrap();
        assert!((azimuth - 157.5 * 0.9).abs() <= 1e-6, "{back}");
        // Même demande, même octets : condition du rejeu.
        let again = reach(json!({"from": from, "azimuth": {"value": 157.5, "unit": "gon"},
            "distance": {"value": 660.5, "unit": "km"}}))
        .unwrap();
        assert_eq!(out, again);
        assert_eq!(serde_json::from_str::<Value>(&out.to_string()).unwrap(), out);
    }

    #[test]
    fn invalid_angles_and_distances_are_refused() {
        let from = wgs84(2.0, 48.0);
        for (azimuth, distance) in [
            (json!({"value": null}), json!({"value": 1, "unit": "km"})),
            (json!({"value": "nord"}), json!({"value": 1, "unit": "km"})),
            (json!({"value": 10, "unit": "rad"}), json!({"value": 1, "unit": "km"})),
            (json!({"value": 10}), json!({"value": 0, "unit": "km"})),
            (json!({"value": 10}), json!({"value": -5, "unit": "m"})),
            (json!({"value": 10}), json!({"value": 5})),
        ] {
            let err = reach(json!({"from": from, "azimuth": azimuth, "distance": distance}))
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{azimuth} {distance}");
        }
        // Un azimut hors de [0, 360) reste un azimut : -90° vaut 270°.
        let west = reach(json!({"from": from, "azimuth": {"value": -90},
            "distance": {"value": 1, "unit": "km"}}))
        .unwrap();
        let also = reach(json!({"from": from, "azimuth": {"value": 270},
            "distance": {"value": 1, "unit": "km"}}))
        .unwrap();
        assert_eq!(west, also);
    }

    #[test]
    fn lambert_93_start_names_data_transform_point() {
        let err = reach(json!({"from": {"x": 652000, "y": 6850000, "epsg": 2154},
            "azimuth": {"value": 10}, "distance": {"value": 1, "unit": "km"}}))
        .unwrap_err();
        assert_eq!(err.code(), "geo.unsupported_crs");
        assert!(err.to_string().contains("data.transform_point"), "{err}");
        assert!(err.to_string().contains(ID), "{err}");
    }

    #[test]
    fn geo_destination_point_runs_in_the_core_and_its_point_is_reusable() {
        let registry = Registry::standard();
        let spec = registry.spec(ID).unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        let violations = registry.check_contract();
        assert!(violations.iter().all(|v| v.id != ID), "{violations:?}");
        let output = &spec.output_schema;
        assert!(crate::point::is_point(&output["properties"]["point"], output));
        // Le point atteint se reprend comme `Point` par une étape suivante.
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": ID, "params": {"from": {"x": 2.3522, "y": 48.8566, "epsg": 4326},
                "azimuth": {"value": 45}, "distance": {"value": 10, "unit": "km"}}},
            {"command": "coord.convert", "params": {"point": "step:1.point", "to": ["mgrs"]}}
        ]}))
        .unwrap();
        let checked = registry.check_plan(&plan);
        assert!(checked.is_ok(), "{checked:?}");
    }
}
