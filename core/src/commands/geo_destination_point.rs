//! Commande `geo.destination_point` : point atteint depuis un départ, à un azimut et une
//! distance géodésiques (problème direct de Karney, `geodesic.rs`), AD-31, CAP-15.
//!
//! Le point d'arrivée est rendu dans le système du départ : WGS84, ou la même zone UTM
//! (l'hémisphère suit la latitude) ; à plus d'une zone de celle du départ, il est refusé
//! (`coord.out_of_range`) au lieu d'être rendu dans une zone que la demande n'a pas choisie.
//!
//! Avec `north: "grid"` (AD-31 amendée), l'azimut d'entrée est un gisement : l'azimut vrai de
//! départ est ce gisement augmenté de la convergence des méridiens en `from`, dans le
//! quadrillage UTM choisi (`geodesy::grid_convergence`, Karney 2011).
//!
//! Avec `north: "magnetic"` (entrée 29), l'azimut d'entrée est magnétique : l'azimut vrai de
//! départ est cet azimut augmenté de la déclinaison du WMM2025 en `from`, à l'altitude 0 m, à la
//! date `date` (absente : la date du jour, rendue au résultat et repassée au rejeu).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::analysis_buffer::Distance;
use super::geo_point::{
    check_date_used, check_zone_used, grid_zone, magnetic_date, magnetic_field, read_point,
    render_point, utm_zone_param,
};
use crate::angle::{Angle, North};
use crate::geodesic;
use crate::geodesy;
use crate::point::Point;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};
use crate::wmm;

const ID: &str = "geo.destination_point";

/// Paramètres de `geo.destination_point`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeoDestinationPointParams {
    /// Point de départ, écrit dans la demande (EPSG 4326, 326NN ou 327NN).
    pub from: Point,
    /// Azimut de départ, depuis le nord choisi (nord vrai par défaut), sens horaire.
    pub azimuth: Angle,
    /// Distance géodésique à parcourir.
    pub distance: Distance,
    /// Nord de l'azimut : true (nord vrai), grid (gisement, nord du quadrillage UTM) ou magnetic (azimut magnétique, boussole), seulement si la demande le nomme.
    #[serde(default)]
    pub north: Option<North>,
    /// Zone UTM du quadrillage, avec north grid et seulement si la demande la nomme.
    #[serde(default, deserialize_with = "utm_zone_param")]
    #[schemars(range(min = 1, max = 60))]
    pub utm_zone: Option<u8>,
    /// Date de la déclinaison, AAAA-MM-JJ, avec north magnetic et seulement si la demande l'écrit.
    #[serde(default)]
    pub date: Option<String>,
}

/// Résultat de `geo.destination_point`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GeoDestinationPointOutput {
    /// Point d'arrivée, dans la zone UTM de `from` (l'hémisphère suit la latitude), ou en
    /// WGS84 si `from` l'est.
    pub point: Point,
    /// Nord employé, présent si la demande l'a nommé.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub north: Option<North>,
    /// Convergence des méridiens au départ, dans l'unité de l'azimut, présente pour north grid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub convergence: Option<Angle>,
    /// Déclinaison magnétique au départ, positive à l'est, dans l'unité de l'azimut, présente
    /// pour north magnetic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declination: Option<Angle>,
    /// Date de la déclinaison, AAAA-MM-JJ, présente pour north magnetic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Avertissement : date hors de la validité du WMM2025, ou départ près d'un pôle magnétique.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// `geo.destination_point` : commande du noyau.
pub struct GeoDestinationPoint;

impl Command for GeoDestinationPoint {
    type Params = GeoDestinationPointParams;
    type Output = GeoDestinationPointOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<GeoDestinationPointParams, GeoDestinationPointOutput>(
            ID,
            "Point atteint depuis un point de départ en WGS84 (EPSG:4326, x longitude, y latitude) ou UTM (EPSG:326NN, 327NN), à un azimut (nord vrai, sens horaire) et une distance géodésiques, sur l'ellipsoïde WGS84 ; rendu dans la zone UTM du départ (l'hémisphère suit la latitude) ou en WGS84 si le départ l'est. Avec north grid, l'azimut est un gisement, compté depuis le nord du quadrillage UTM de from (ou de utm_zone) ; avec north magnetic, un azimut magnétique (boussole), par la déclinaison du modèle magnétique mondial WMM2025 à la date date (absente : aujourd'hui).",
            "Point reached from a start point in WGS84 (EPSG:4326, x longitude, y latitude) or UTM on WGS84 (EPSG:326NN, 327NN), along a geodesic at an azimuth (true north, clockwise) and a distance, on the WGS84 ellipsoid (Karney); returned in the UTM zone of the start point (hemisphere following the latitude), or in WGS84 if the start point is. With north grid, the azimuth is a grid bearing, from the grid north of the UTM zone of from (or of utm_zone); with north magnetic, a magnetic (compass) azimuth, through the declination of the World Magnetic Model WMM2025 at date (absent: today). A point in another system goes through data.transform_point first, an MGRS reference through coord.convert.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .example(serde_json::json!({"from": {"x": 2.3522, "y": 48.8566, "epsg": 4326},
            "azimuth": {"value": 45}, "distance": {"value": 10, "unit": "km"}}))
        .example(serde_json::json!({"from": {"x": 448251.0, "y": 5411750.0, "epsg": 32631},
            "azimuth": {"value": 100, "unit": "gon"}, "distance": {"value": 1500, "unit": "m"}}))
        .example(serde_json::json!({"from": {"x": 448251.0, "y": 5411750.0, "epsg": 32631},
            "azimuth": {"value": 250, "unit": "gon"}, "distance": {"value": 800, "unit": "m"},
            "north": "grid"}))
        .example(serde_json::json!({"from": {"x": 6.1294, "y": 45.8992, "epsg": 4326},
            "azimuth": {"value": 120}, "distance": {"value": 2, "unit": "km"},
            "north": "magnetic", "date": "2026-07-14"}))
        .filled_param("date", "date")
        .tests(&[
            "geographiclib_direct_references_are_found",
            "a_utm_start_gives_a_point_in_its_zone",
            "a_point_beyond_the_next_zone_is_out_of_range",
            "a_direct_and_its_inverse_agree",
            "invalid_angles_and_distances_are_refused",
            "lambert_93_start_names_data_transform_point",
            "geo_destination_point_runs_in_the_core_and_its_point_is_reusable",
            "a_grid_bearing_is_turned_into_a_true_azimuth_at_the_start",
            "a_grid_bearing_and_its_inverse_agree",
            "the_grid_zone_rules_hold_for_a_destination",
            "a_magnetic_azimuth_is_turned_into_a_true_azimuth_at_the_start",
            "a_magnetic_destination_and_its_inverse_agree",
            "the_date_rules_hold_for_a_destination",
        ])
    }

    fn run(&self, params: GeoDestinationPointParams) -> Result<GeoDestinationPointOutput, CommandFailure> {
        check_zone_used(params.north, params.utm_zone)?;
        check_date_used(params.north, params.date.as_deref())?;
        let date = match params.north {
            Some(North::Magnetic) => Some(magnetic_date(params.date.as_deref())?),
            _ => None,
        };
        let (lat, lon) = read_point(&params.from, ID)?;
        let gamma = match params.north {
            Some(North::Grid) => {
                let zone = grid_zone(&params.from, lat, lon, params.utm_zone)?;
                Some(geodesy::grid_convergence(lat, lon, zone))
            }
            _ => None,
        };
        // L'azimut magnétique se mesure au départ seulement : la déclinaison et l'avertissement
        // sont ceux de `from`.
        let magnetic = date.as_ref().map(|date| {
            let field = magnetic_field(lat, lon, date);
            (field.d, wmm::warning(date, &[("Le point de départ", &field)]))
        });
        let correction = gamma.or(magnetic.as_ref().map(|(d, _)| *d));
        let azimuth = params.azimuth.degrees() + correction.unwrap_or(0.0);
        let end = geodesic::direct(lat, lon, azimuth, params.distance.metres());
        let unit = params.azimuth.unit;
        let (declination, warning) = match magnetic {
            Some((d, warning)) => (Some(Angle::signed(d, unit)), warning),
            None => (None, None),
        };
        Ok(GeoDestinationPointOutput {
            point: render_point(end.lat, end.lon, params.from.epsg)?,
            north: params.north,
            convergence: gamma.map(|g| Angle::signed(g, unit)),
            declination,
            date: date.map(|d| d.to_string()),
            warning,
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

    // -- gisement (epic 10, entrée 28) ---------------------------------------------------------

    #[test]
    fn a_grid_bearing_is_turned_into_a_true_azimuth_at_the_start() {
        let start = json!({"x": 650000.0, "y": 5400000.0, "epsg": 32631});
        let (lat, lon) = crate::geodesy::from_utm(&crate::geodesy::Utm { zone: 31,
            north: true, easting: 650000.0, northing: 5400000.0 });
        let gamma = crate::geodesy::grid_convergence(lat, lon, 31);
        // Absent : le point seul, comme aujourd'hui ; `true` écrit : même point, plus `north`.
        let plain = reach(json!({"from": start, "azimuth": {"value": 40},
            "distance": {"value": 2, "unit": "km"}})).unwrap();
        let keys: Vec<&String> = plain.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["point"]);
        let written = reach(json!({"from": start, "azimuth": {"value": 40},
            "distance": {"value": 2, "unit": "km"}, "north": "true"})).unwrap();
        assert_eq!(written["point"], plain["point"]);
        assert_eq!(written["north"], "true");
        assert!(written.get("convergence").is_none());
        // Gisement 40° : azimut vrai 40° + γ ; en grades, la convergence est rendue en grades.
        for (azimuth, factor) in [(json!({"value": 40}), 1.0),
            (json!({"value": 40.0 / 0.9, "unit": "gon"}), 10.0 / 9.0)] {
            let grid = reach(json!({"from": start, "azimuth": azimuth,
                "distance": {"value": 2, "unit": "km"}, "north": "grid"})).unwrap();
            let truth = reach(json!({"from": start, "azimuth": {"value": 40.0 + gamma},
                "distance": {"value": 2, "unit": "km"}})).unwrap();
            assert_eq!(grid["north"], "grid");
            let c = grid["convergence"]["value"].as_f64().unwrap();
            assert!((c - gamma * factor).abs() <= 1e-9, "{grid}");
            let (x, y) = (grid["point"]["x"].as_f64().unwrap(), grid["point"]["y"].as_f64().unwrap());
            let (tx, ty) = (truth["point"]["x"].as_f64().unwrap(),
                truth["point"]["y"].as_f64().unwrap());
            assert!((x - tx).abs() <= 1e-3 && (y - ty).abs() <= 1e-3, "{grid} / {truth}");
        }
        // Le gisement de 0 suit le quadrillage : le point garde son abscisse à quelques
        // centimètres près, alors que le nord vrai la déplace.
        let north = reach(json!({"from": start, "azimuth": {"value": 0},
            "distance": {"value": 2, "unit": "km"}, "north": "grid"})).unwrap();
        assert!((north["point"]["x"].as_f64().unwrap() - 650000.0).abs() < 0.1, "{north}");
        let truth = reach(json!({"from": start, "azimuth": {"value": 0},
            "distance": {"value": 2, "unit": "km"}})).unwrap();
        assert!((truth["point"]["x"].as_f64().unwrap() - 650000.0).abs() > 10.0, "{truth}");
    }

    #[test]
    fn a_grid_bearing_and_its_inverse_agree() {
        let from = wgs84(6.9, 48.3);
        let out = reach(json!({"from": from, "azimuth": {"value": 130, "unit": "gon"},
            "distance": {"value": 7.5, "unit": "km"}, "north": "grid"})).unwrap();
        let back = Registry::standard()
            .call("geo.distance_azimuth", json!({"from": from, "to": out["point"],
                "north": "grid", "unit": "gon"}))
            .unwrap();
        let azimuth = back["azimuth"]["value"].as_f64().unwrap();
        assert!((azimuth - 130.0).abs() <= 1e-6, "{back}");
        assert_eq!(back["convergence"], out["convergence"]);
        assert_eq!(serde_json::from_str::<Value>(&out.to_string()).unwrap(), out);
    }

    #[test]
    fn the_grid_zone_rules_hold_for_a_destination() {
        let from = wgs84(6.5, 48.0);
        let base = json!({"from": from, "azimuth": {"value": 10},
            "distance": {"value": 1, "unit": "km"}});
        let with = |extra: Value| {
            let mut p = base.clone();
            p.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            reach(p)
        };
        let normal = with(json!({"north": "grid"})).unwrap();
        let imposed = with(json!({"north": "grid", "utm_zone": 31})).unwrap();
        assert_ne!(normal["convergence"], imposed["convergence"]);
        assert_eq!(with(json!({"north": "grid", "utm_zone": 34})).unwrap_err().code(),
            "coord.out_of_range");
        assert_eq!(with(json!({"utm_zone": 32})).unwrap_err().code(), "geo.zone_unused");
        assert_eq!(with(json!({"north": "magnetic", "utm_zone": 32})).unwrap_err().code(),
            "geo.zone_unused");
    }

    // -- azimut magnétique (epic 10, entrée 29) ------------------------------------------------

    #[test]
    fn a_magnetic_azimuth_is_turned_into_a_true_azimuth_at_the_start() {
        let start = json!({"x": 650000.0, "y": 5400000.0, "epsg": 32631});
        let (lat, lon) = crate::geodesy::from_utm(&crate::geodesy::Utm { zone: 31,
            north: true, easting: 650000.0, northing: 5400000.0 });
        let date = crate::wmm::Date::parse("2026-05-01").unwrap();
        let d = crate::wmm::field(date.decimal_year(), 0.0, lat, lon).d;
        assert!(d > 0.0, "déclinaison est en France en 2026 : {d}");
        // Azimut magnétique 40° : azimut vrai 40° + D ; en grades, la déclinaison en grades.
        for (azimuth, factor) in [(json!({"value": 40}), 1.0),
            (json!({"value": 40.0 / 0.9, "unit": "gon"}), 10.0 / 9.0)] {
            let out = reach(json!({"from": start, "azimuth": azimuth,
                "distance": {"value": 2, "unit": "km"}, "north": "magnetic",
                "date": "2026-05-01"})).unwrap();
            let truth = reach(json!({"from": start, "azimuth": {"value": 40.0 + d},
                "distance": {"value": 2, "unit": "km"}})).unwrap();
            assert_eq!(out["north"], "magnetic");
            assert_eq!(out["date"], "2026-05-01");
            assert!(out.get("warning").is_none() && out.get("convergence").is_none(), "{out}");
            let c = out["declination"]["value"].as_f64().unwrap();
            assert!((c - d * factor).abs() <= 1e-9, "{out}");
            let (x, y) = (out["point"]["x"].as_f64().unwrap(), out["point"]["y"].as_f64().unwrap());
            let (tx, ty) = (truth["point"]["x"].as_f64().unwrap(),
                truth["point"]["y"].as_f64().unwrap());
            assert!((x - tx).abs() <= 1e-3 && (y - ty).abs() <= 1e-3, "{out} / {truth}");
        }
        // Date absente : celle du jour, rendue ; hors validité : avertissement, point rendu.
        // La date du jour lue avant et après l'appel : l'exécution peut franchir minuit UTC.
        let before = crate::wmm::Date::today_utc().to_string();
        let out = reach(json!({"from": start, "azimuth": {"value": 40},
            "distance": {"value": 2, "unit": "km"}, "north": "magnetic"})).unwrap();
        let after = crate::wmm::Date::today_utc().to_string();
        assert!(out["date"] == before.as_str() || out["date"] == after.as_str(), "{out}");
        let out = reach(json!({"from": start, "azimuth": {"value": 40},
            "distance": {"value": 2, "unit": "km"}, "north": "magnetic",
            "date": "2031-03-01"})).unwrap();
        assert!(out["warning"].as_str().unwrap().contains("2031-03-01"), "{out}");
        assert_eq!(out["point"]["epsg"], 32631);
        // Départ dans la zone d'ombre du pôle nord magnétique : avertissement, point rendu.
        let out = reach(json!({"from": wgs84(139.0, 85.5), "azimuth": {"value": 40},
            "distance": {"value": 2, "unit": "km"}, "north": "magnetic",
            "date": "2026-05-01"})).unwrap();
        let warning = out["warning"].as_str().unwrap();
        assert!(warning.starts_with("Le point de départ est dans la zone d'ombre"), "{warning}");
    }

    #[test]
    fn a_magnetic_destination_and_its_inverse_agree() {
        let from = wgs84(6.1294, 45.8992);
        let out = reach(json!({"from": from, "azimuth": {"value": 130, "unit": "gon"},
            "distance": {"value": 7.5, "unit": "km"}, "north": "magnetic",
            "date": "2027-03-20"})).unwrap();
        let back = Registry::standard()
            .call("geo.distance_azimuth", json!({"from": from, "to": out["point"],
                "north": "magnetic", "unit": "gon", "date": "2027-03-20"}))
            .unwrap();
        let azimuth = back["azimuth"]["value"].as_f64().unwrap();
        assert!((azimuth - 130.0).abs() <= 1e-6, "{back}");
        assert_eq!(back["declination"], out["declination"]);
        assert_eq!(serde_json::from_str::<Value>(&out.to_string()).unwrap(), out);
    }

    #[test]
    fn the_date_rules_hold_for_a_destination() {
        let base = json!({"from": wgs84(6.5, 48.0), "azimuth": {"value": 10},
            "distance": {"value": 1, "unit": "km"}});
        let with = |extra: Value| {
            let mut p = base.clone();
            p.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            reach(p)
        };
        for bad in ["2026-13-01", "2026-02-30", "10/05/2026"] {
            assert_eq!(with(json!({"north": "magnetic", "date": bad})).unwrap_err().code(),
                "geo.invalid_date", "{bad}");
        }
        for north in [json!(null), json!("true"), json!("grid")] {
            let mut extra = json!({"date": "2026-05-01"});
            if !north.is_null() {
                extra["north"] = north.clone();
            }
            assert_eq!(with(extra).unwrap_err().code(), "geo.date_unused", "{north}");
        }
    }
}
