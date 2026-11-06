//! Lecture et rendu d'un point pour `coord.convert` et les commandes `geo.*` (AD-19 amendé,
//! AD-31, CAP-15) : un [`Point`] en WGS84 (EPSG:4326) ou en UTM sur WGS84 (EPSG:32601 à
//! 32660, 32701 à 32760) devient une latitude et une longitude WGS84 ; tout autre système est
//! refusé par `geo.unsupported_crs`, qui nomme l'étape à ajouter. Aucune commande ici.

use crate::commands::analysis_buffer::{Distance, DistanceUnit};
use crate::geodesy::{self, Utm};
use crate::point::Point;
use crate::registry::CommandFailure;

/// Code EPSG du WGS84 géographique.
pub const WGS84: u32 = 4326;

/// Échec `coord.out_of_range`.
pub fn out_of_range(message: String) -> CommandFailure {
    CommandFailure::new("coord.out_of_range", message)
}

/// Arrondi à `1 / scale` : le millimètre pour l'UTM et les distances (1e3), le milliardième de
/// degré pour le WGS84 (1e9). L'entier arrondi est divisé, non multiplié par le pas : le flottant
/// rendu est le plus proche du décimal court, que le JSON écrit et relit à l'identique (le rejeu
/// compare le résultat relu du journal à celui qu'il recalcule).
pub fn rounded(value: f64, scale: f64) -> f64 {
    libm::round(value * scale) / scale
}

/// Lit un point d'entrée de `command` en WGS84 : (latitude, longitude) en degrés.
pub fn read_point(point: &Point, command: &str) -> Result<(f64, f64), CommandFailure> {
    if !(point.x.is_finite() && point.y.is_finite()) {
        return Err(out_of_range(
            "Les coordonnées du point ne sont pas des nombres finis.".into(),
        ));
    }
    if point.epsg == WGS84 {
        if !(-90.0..=90.0).contains(&point.y) || !(-180.0..=180.0).contains(&point.x) {
            return Err(out_of_range(format!(
                "Le point ({}, {}) n'est pas un point WGS84 : la longitude (x) va de -180 à 180, la latitude (y) de -90 à 90.",
                point.x, point.y
            )));
        }
        return Ok((point.y, point.x));
    }
    let Some((zone, north)) = Utm::zone_of_epsg(point.epsg) else {
        // `coord.convert` lit lui-même les références MGRS : seule une commande `geo.*` renvoie
        // vers lui.
        let mgrs = if command == "coord.convert" {
            ""
        } else {
            " ; une référence MGRS passe, elle, par une étape coord.convert"
        };
        return Err(CommandFailure::new(
            "geo.unsupported_crs",
            format!(
                "Le système EPSG:{} n'est pas pris en charge par {command}, qui ne connaît que le WGS84 (EPSG:4326) et l'UTM sur WGS84 (EPSG:32601 à 32660, 32701 à 32760) : ajoutez d'abord une étape data.transform_point vers EPSG:4326{mgrs}.",
                point.epsg
            ),
        ));
    };
    let (lat, lon) = geodesy::from_utm(&Utm {
        zone,
        north,
        easting: point.x,
        northing: point.y,
    });
    // Latitude dans la grille, hémisphère conforme au code EPSG, et longitude à au plus 9° du
    // méridien central (une zone voisine au plus) : sinon les nombres ne sont pas un point de
    // cette zone.
    let inside = lat.is_finite()
        && lon.is_finite()
        && (geodesy::MIN_LATITUDE..=geodesy::MAX_LATITUDE).contains(&lat)
        && north == (lat >= 0.0)
        && geodesy::wrap_longitude(lon - geodesy::central_meridian(zone)).abs() <= 9.0;
    if !inside {
        return Err(out_of_range(format!(
            "Le point ({}, {}) est hors de la zone UTM EPSG:{}.",
            point.x, point.y, point.epsg
        )));
    }
    Ok((lat, lon))
}

/// La latitude doit être couverte par l'UTM et le MGRS.
pub fn check_grid_latitude(lat: f64) -> Result<(), CommandFailure> {
    if (geodesy::MIN_LATITUDE..=geodesy::MAX_LATITUDE).contains(&lat) {
        return Ok(());
    }
    Err(out_of_range(format!(
        "La latitude {lat} est hors de la grille UTM et MGRS, qui va de 80° S à 84° N : les zones polaires ne sont pas prises en charge."
    )))
}

/// Un point WGS84 au milliardième de degré.
pub fn wgs84_point(lat: f64, lon: f64) -> Point {
    Point {
        x: rounded(lon, 1e9),
        y: rounded(lat, 1e9),
        epsg: WGS84,
    }
}

/// Un point UTM de la zone `zone`, au millimètre ; l'hémisphère suit la latitude.
pub fn utm_point(lat: f64, lon: f64, zone: u8) -> Point {
    let utm = geodesy::to_utm(lat, lon, zone);
    Point {
        x: rounded(utm.easting, 1e3),
        y: rounded(utm.northing, 1e3),
        epsg: utm.epsg(),
    }
}

/// Rend un point calculé dans le système `epsg` d'un point d'entrée déjà lu par
/// [`read_point`] : WGS84, ou la même zone UTM (l'hémisphère suit la latitude). Un point à plus
/// d'une zone de celle d'entrée, ou hors de la grille, est refusé (`coord.out_of_range`).
pub fn render_point(lat: f64, lon: f64, epsg: u32) -> Result<Point, CommandFailure> {
    let Some((zone, _)) = Utm::zone_of_epsg(epsg) else {
        return Ok(wgs84_point(lat, lon));
    };
    check_grid_latitude(lat)?;
    let normal = geodesy::utm_zone(lat, lon);
    if geodesy::zone_distance(zone, normal) > 1 {
        return Err(out_of_range(format!(
            "Le point calculé (longitude {}, latitude {}) tombe en zone UTM {normal}, à plus d'une zone de la zone {zone} du point d'entrée (EPSG:{epsg}) : donnez ce point en WGS84 (EPSG:4326), ou passez par une étape coord.convert.",
            rounded(lon, 1e6),
            rounded(lat, 1e6)
        )));
    }
    Ok(utm_point(lat, lon, zone))
}

/// Une distance du noyau, en mètres, au millimètre.
pub fn metres(value: f64) -> Distance {
    Distance {
        value: rounded(value, 1e3),
        unit: DistanceUnit::M,
    }
}
