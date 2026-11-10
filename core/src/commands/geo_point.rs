//! Lecture et rendu d'un point pour `coord.convert` et les commandes `geo.*` (AD-19 amendé,
//! AD-31, CAP-15) : un [`Point`] en WGS84 (EPSG:4326) ou en UTM sur WGS84 (EPSG:32601 à
//! 32660, 32701 à 32760) devient une latitude et une longitude WGS84 ; tout autre système est
//! refusé par `geo.unsupported_crs`, qui nomme l'étape à ajouter. Aussi le choix du quadrillage
//! UTM d'un gisement (`north: "grid"`, AD-31 amendée). Aucune commande ici.

use serde::{Deserialize, Deserializer};

use crate::angle::North;
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

/// Lit `utm_zone` : de 1 à 60, ou absent.
pub fn utm_zone_param<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u8>, D::Error> {
    use serde::de::Error;
    let zone = Option::<u8>::deserialize(d)?;
    if zone.is_some_and(|z| !(1..=60).contains(&z)) {
        return Err(D::Error::custom("`utm_zone` va de 1 à 60"));
    }
    Ok(zone)
}

/// `utm_zone` ne sert qu'au gisement : sans `north: "grid"`, il est refusé (`geo.zone_unused`)
/// plutôt qu'ignoré.
pub fn check_zone_used(north: Option<North>, utm_zone: Option<u8>) -> Result<(), CommandFailure> {
    match (north, utm_zone) {
        (Some(North::Grid), _) | (_, None) => Ok(()),
        (_, Some(zone)) => Err(CommandFailure::new(
            "geo.zone_unused",
            format!(
                "La zone UTM {zone} (`utm_zone`) ne sert qu'à un gisement, compté depuis le nord du quadrillage : ajoutez `north: \"grid\"`, ou retirez `utm_zone` pour un azimut depuis le nord vrai."
            ),
        )),
    }
}

/// Zone du quadrillage d'un gisement mesuré en `from` (déjà lu par [`read_point`] en `lat`,
/// `lon`) : `utm_zone` si elle est donnée, à condition d'être la zone normale du point ou une
/// voisine (règle de `coord.convert`) ; sinon la zone de `from` s'il est en UTM, sinon sa zone
/// normale. Hors de la grille UTM (80° S à 84° N), `coord.out_of_range`.
pub fn grid_zone(
    from: &Point,
    lat: f64,
    lon: f64,
    utm_zone: Option<u8>,
) -> Result<u8, CommandFailure> {
    check_grid_latitude(lat)?;
    let normal = geodesy::utm_zone(lat, lon);
    match utm_zone {
        Some(zone) if geodesy::zone_distance(zone, normal) > 1 => Err(out_of_range(format!(
            "La zone UTM {zone} est à plus d'une zone de celle du point de départ ({normal}) : une zone imposée ne peut être que la sienne ou une voisine."
        ))),
        Some(zone) => Ok(zone),
        None => Ok(Utm::zone_of_epsg(from.epsg).map_or(normal, |(zone, _)| zone)),
    }
}

/// Un point où se mesure un azimut dans le quadrillage de la zone `zone` : dans la grille UTM et
/// à au plus une zone de `zone`, sinon `coord.out_of_range` (un gisement dans une zone
/// lointaine serait un nombre faux d'apparence juste).
pub fn check_in_grid(lat: f64, lon: f64, zone: u8) -> Result<(), CommandFailure> {
    // Zone normale (exceptions 32V et 31X à 37X de la NGA) ou zone numérique simple, sans
    // exception : de part et d'autre d'une limite d'exception, deux points voisins ont des zones
    // normales à deux numéros d'écart (75° N : 20,9° E en 33, 21,1° E en 35).
    let normal = geodesy::utm_zone(lat, lon);
    let plain = ((geodesy::wrap_longitude(lon) + 180.0) / 6.0).floor() as i32 + 1;
    let plain = plain.clamp(1, 60) as u8;
    if (geodesy::MIN_LATITUDE..=geodesy::MAX_LATITUDE).contains(&lat)
        && (geodesy::zone_distance(zone, normal) <= 1 || geodesy::zone_distance(zone, plain) <= 1)
    {
        return Ok(());
    }
    Err(out_of_range(format!(
        "Le point d'arrivée (longitude {}, latitude {}) est hors du quadrillage de la zone UTM {zone} (à plus d'une zone, ou hors de 80° S à 84° N) : son gisement n'y a pas de sens ; demandez l'azimut depuis le nord vrai (sans `north`).",
        rounded(lon, 1e6),
        rounded(lat, 1e6)
    )))
}
