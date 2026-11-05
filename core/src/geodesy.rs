//! Géodésie du noyau : WGS84, Mercator transverse, zones UTM et grille MGRS (AD-19 amendé,
//! AD-31, CAP-15).
//!
//! Provenance, écrite d'après les seules publications, sans reprendre de code :
//! - Mercator transverse : C. F. F. Karney, « Transverse Mercator with an accuracy of a few
//!   nanometers », *Journal of Geodesy*, 85(8), 2011, p. 475-485 (séries de Krüger à l'ordre
//!   6 en n, éq. 35 et 36 ; latitude conforme, éq. 7 à 9 ; inversion de Newton, éq. 19 à 21).
//! - Zones UTM, bandes de latitude, carrés de 100 km (schéma AA du WGS84) et exceptions 32V et
//!   31X à 37X : NGA, *The Universal Grids and the Transverse Mercator and Polar Stereographic
//!   Map Projections*, NGA.SIG.0012_2.0.0_UTMUPS, 2014.
//!
//! Toute trigonométrie passe par `libm`, version figée dans `core/Cargo.toml` : les mêmes octets
//! sous Windows et sous Linux, condition du rejeu (AD-11). Les zones polaires (UPS, bandes A, B,
//! Y, Z) sont hors du module. Aucune conversion d'une couche ni d'un autre système n'est faite
//! ici : le noyau ne convertit que des points, entre WGS84, UTM et MGRS.

/// Demi-grand axe de l'ellipsoïde WGS84, en mètres.
pub const WGS84_A: f64 = 6_378_137.0;
/// Aplatissement de l'ellipsoïde WGS84.
pub const WGS84_F: f64 = 1.0 / 298.257_223_563;
/// Facteur d'échelle de l'UTM sur le méridien central.
pub const UTM_K0: f64 = 0.9996;
/// Fausse abscisse de l'UTM, en mètres.
pub const FALSE_EASTING: f64 = 500_000.0;
/// Fausse ordonnée de l'UTM dans l'hémisphère sud, en mètres.
pub const FALSE_NORTHING_SOUTH: f64 = 10_000_000.0;
/// Latitude la plus au sud couverte par l'UTM et le MGRS de ce module.
pub const MIN_LATITUDE: f64 = -80.0;
/// Latitude la plus au nord couverte par l'UTM et le MGRS de ce module.
pub const MAX_LATITUDE: f64 = 84.0;

/// Marge de la troncature au mètre, en mètres (voir [`Mgrs::from_lat_lon`]).
const TRUNCATION_SLACK: f64 = 1e-6;
/// Bandes de latitude, de 8° chacune depuis 80° S (X : 12°, de 72° N à 84° N).
const BANDS: &[u8; 20] = b"CDEFGHJKLMNPQRSTUVWX";
/// Lettres de colonne des carrés de 100 km : trois jeux, selon la zone modulo 3.
const COLUMNS: [&[u8; 8]; 3] = [b"ABCDEFGH", b"JKLMNPQR", b"STUVWXYZ"];
/// Lettres de rang des carrés de 100 km : cycle de 20, décalé de 5 dans les zones paires.
const ROWS: &[u8; 20] = b"ABCDEFGHJKLMNPQRSTUV";

// -- Mercator transverse de Karney ----------------------------------------------------------

/// Coefficients de la projection pour le WGS84.
struct Series {
    /// Rayon rectifiant A (éq. 14).
    rectifying: f64,
    /// Coefficients α de la projection directe (éq. 35).
    alpha: [f64; 6],
    /// Coefficients β de la projection inverse (éq. 36).
    beta: [f64; 6],
    /// Excentricité.
    e: f64,
    /// Carré de l'excentricité.
    e2: f64,
}

fn series() -> Series {
    let f = WGS84_F;
    let n = f / (2.0 - f);
    let n2 = n * n;
    let n3 = n2 * n;
    let n4 = n3 * n;
    let n5 = n4 * n;
    let n6 = n5 * n;
    let rectifying = WGS84_A / (1.0 + n) * (1.0 + n2 / 4.0 + n4 / 64.0 + n6 / 256.0);
    let alpha = [
        n / 2.0 - 2.0 * n2 / 3.0 + 5.0 * n3 / 16.0 + 41.0 * n4 / 180.0 - 127.0 * n5 / 288.0
            + 7891.0 * n6 / 37800.0,
        13.0 * n2 / 48.0 - 3.0 * n3 / 5.0 + 557.0 * n4 / 1440.0 + 281.0 * n5 / 630.0
            - 1_983_433.0 * n6 / 1_935_360.0,
        61.0 * n3 / 240.0 - 103.0 * n4 / 140.0
            + 15061.0 * n5 / 26880.0
            + 167_603.0 * n6 / 181_440.0,
        49561.0 * n4 / 161_280.0 - 179.0 * n5 / 168.0 + 6_601_661.0 * n6 / 7_257_600.0,
        34729.0 * n5 / 80640.0 - 3_418_889.0 * n6 / 1_995_840.0,
        212_378_941.0 * n6 / 319_334_400.0,
    ];
    let beta = [
        n / 2.0 - 2.0 * n2 / 3.0 + 37.0 * n3 / 96.0 - n4 / 360.0 - 81.0 * n5 / 512.0
            + 96199.0 * n6 / 604_800.0,
        n2 / 48.0 + n3 / 15.0 - 437.0 * n4 / 1440.0 + 46.0 * n5 / 105.0
            - 1_118_711.0 * n6 / 3_870_720.0,
        17.0 * n3 / 480.0 - 37.0 * n4 / 840.0 - 209.0 * n5 / 4480.0 + 5569.0 * n6 / 90720.0,
        4397.0 * n4 / 161_280.0 - 11.0 * n5 / 504.0 - 830_251.0 * n6 / 7_257_600.0,
        4583.0 * n5 / 161_280.0 - 108_847.0 * n6 / 3_991_680.0,
        20_648_693.0 * n6 / 638_668_800.0,
    ];
    let e2 = f * (2.0 - f);
    Series {
        rectifying,
        alpha,
        beta,
        e: libm::sqrt(e2),
        e2,
    }
}

/// Tangente de la latitude conforme τ′ depuis celle de la latitude géographique τ (éq. 7 à 9).
fn conformal_tan(tau: f64, e: f64) -> f64 {
    let tau1 = libm::sqrt(1.0 + tau * tau);
    let sigma = libm::sinh(e * libm::atanh(e * tau / tau1));
    tau * libm::sqrt(1.0 + sigma * sigma) - sigma * tau1
}

/// Inverse de [`conformal_tan`], par la méthode de Newton (éq. 19 à 21), en un nombre borné
/// d'itérations : le même calcul, donc les mêmes octets, à chaque appel.
fn geographic_tan(taup: f64, s: &Series) -> f64 {
    let mut tau = taup;
    for _ in 0..12 {
        let tp = conformal_tan(tau, s.e);
        let dtau = (taup - tp) / libm::sqrt(1.0 + tp * tp) * (1.0 + (1.0 - s.e2) * tau * tau)
            / ((1.0 - s.e2) * libm::sqrt(1.0 + tau * tau));
        tau += dtau;
        if dtau.abs() <= 1e-14 * tau.abs().max(1.0) {
            break;
        }
    }
    tau
}

/// Projection directe, facteur d'échelle `UTM_K0` compris, sans fausse abscisse ni fausse
/// ordonnée : (x, y) en mètres depuis une latitude et un écart de longitude au méridien
/// central, en degrés.
pub fn tm_forward(lat: f64, dlon: f64) -> (f64, f64) {
    let s = series();
    let phi = lat.to_radians();
    let lam = dlon.to_radians();
    let taup = conformal_tan(libm::tan(phi), s.e);
    let (sin_lam, cos_lam) = (libm::sin(lam), libm::cos(lam));
    let xip = libm::atan2(taup, cos_lam);
    let etap = libm::asinh(sin_lam / libm::sqrt(taup * taup + cos_lam * cos_lam));
    let (mut xi, mut eta) = (xip, etap);
    for (j, a) in s.alpha.iter().enumerate() {
        let k = 2.0 * (j + 1) as f64;
        xi += a * libm::sin(k * xip) * libm::cosh(k * etap);
        eta += a * libm::cos(k * xip) * libm::sinh(k * etap);
    }
    (UTM_K0 * s.rectifying * eta, UTM_K0 * s.rectifying * xi)
}

/// Projection inverse de [`tm_forward`] : (latitude, écart de longitude), en degrés.
pub fn tm_reverse(x: f64, y: f64) -> (f64, f64) {
    let s = series();
    let xi = y / (UTM_K0 * s.rectifying);
    let eta = x / (UTM_K0 * s.rectifying);
    let (mut xip, mut etap) = (xi, eta);
    for (j, b) in s.beta.iter().enumerate() {
        let k = 2.0 * (j + 1) as f64;
        xip -= b * libm::sin(k * xi) * libm::cosh(k * eta);
        etap -= b * libm::cos(k * xi) * libm::sinh(k * eta);
    }
    let sinh_etap = libm::sinh(etap);
    let (sin_xip, cos_xip) = (libm::sin(xip), libm::cos(xip));
    let taup = sin_xip / libm::sqrt(sinh_etap * sinh_etap + cos_xip * cos_xip);
    let lam = libm::atan2(sinh_etap, cos_xip);
    let tau = geographic_tan(taup, &s);
    (libm::atan(tau).to_degrees(), lam.to_degrees())
}

// -- UTM --------------------------------------------------------------------------------------

/// Ramène une longitude dans [-180, 180).
pub fn wrap_longitude(lon: f64) -> f64 {
    let mut l = lon;
    while l >= 180.0 {
        l -= 360.0;
    }
    while l < -180.0 {
        l += 360.0;
    }
    l
}

/// Zone UTM normale d'un point (1 à 60), exceptions de la NGA comprises : 32V (56° N à 64° N,
/// 3° E à 12° E) et, au Svalbard (72° N à 84° N, 0° à 42° E), les zones 31X, 33X, 35X et 37X.
pub fn utm_zone(lat: f64, lon: f64) -> u8 {
    let lon = wrap_longitude(lon);
    let mut zone = (((lon + 180.0) / 6.0).floor() as i32 + 1).clamp(1, 60) as u8;
    if (56.0..64.0).contains(&lat) && (3.0..12.0).contains(&lon) {
        zone = 32;
    }
    if (72.0..=MAX_LATITUDE).contains(&lat) && (0.0..42.0).contains(&lon) {
        zone = if lon < 9.0 {
            31
        } else if lon < 21.0 {
            33
        } else if lon < 33.0 {
            35
        } else {
            37
        };
    }
    zone
}

/// Nombre de zones entre deux zones UTM, en tenant compte du passage de 60 à 1.
pub fn zone_distance(a: u8, b: u8) -> u8 {
    let d = a.abs_diff(b);
    d.min(60 - d)
}

/// Longitude du méridien central d'une zone, en degrés.
pub fn central_meridian(zone: u8) -> f64 {
    f64::from(zone) * 6.0 - 183.0
}

/// Coordonnées UTM d'un point : zone, hémisphère, abscisse et ordonnée en mètres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Utm {
    pub zone: u8,
    pub north: bool,
    pub easting: f64,
    pub northing: f64,
}

impl Utm {
    /// Code EPSG de la zone sur WGS84 : 326NN au nord, 327NN au sud.
    pub fn epsg(&self) -> u32 {
        (if self.north { 32600 } else { 32700 }) + u32::from(self.zone)
    }

    /// Zone et hémisphère d'un code EPSG 32601 à 32660 ou 32701 à 32760.
    pub fn zone_of_epsg(epsg: u32) -> Option<(u8, bool)> {
        match epsg {
            32601..=32660 => Some(((epsg - 32600) as u8, true)),
            32701..=32760 => Some(((epsg - 32700) as u8, false)),
            _ => None,
        }
    }
}

/// Projette un point WGS84 (degrés) dans la zone `zone` ; l'hémisphère suit la latitude.
pub fn to_utm(lat: f64, lon: f64, zone: u8) -> Utm {
    let dlon = wrap_longitude(lon - central_meridian(zone));
    let (x, y) = tm_forward(lat, dlon);
    let north = lat >= 0.0;
    Utm {
        zone,
        north,
        easting: x + FALSE_EASTING,
        northing: if north { y } else { y + FALSE_NORTHING_SOUTH },
    }
}

/// Point WGS84 (latitude, longitude en degrés) de coordonnées UTM.
pub fn from_utm(utm: &Utm) -> (f64, f64) {
    let y = if utm.north {
        utm.northing
    } else {
        utm.northing - FALSE_NORTHING_SOUTH
    };
    let (lat, dlon) = tm_reverse(utm.easting - FALSE_EASTING, y);
    (lat, wrap_longitude(central_meridian(utm.zone) + dlon))
}

/// Indice de la bande de latitude (0 pour C, 19 pour X), si la latitude est dans [-80, 84].
fn band_index(lat: f64) -> Option<usize> {
    if !(MIN_LATITUDE..=MAX_LATITUDE).contains(&lat) {
        return None;
    }
    Some((((lat - MIN_LATITUDE) / 8.0).floor() as usize).min(BANDS.len() - 1))
}

/// Bande de latitude d'un point (C à X, sans I ni O), si la latitude est dans [-80, 84].
pub fn latitude_band(lat: f64) -> Option<char> {
    band_index(lat).map(|i| char::from(BANDS[i]))
}

// -- MGRS -------------------------------------------------------------------------------------

/// Partie fautive d'une référence MGRS mal formée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MgrsPart {
    Zone,
    Band,
    Square,
    Digits,
}

impl MgrsPart {
    /// Nom de la partie, en français, tel que le message d'erreur le donne.
    pub fn label(self) -> &'static str {
        match self {
            MgrsPart::Zone => "zone",
            MgrsPart::Band => "bande",
            MgrsPart::Square => "carré de 100 km",
            MgrsPart::Digits => "chiffres",
        }
    }
}

/// Référence MGRS refusée : la partie fautive et la raison, en français.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MgrsError {
    pub part: MgrsPart,
    pub detail: String,
}

impl MgrsError {
    fn new(part: MgrsPart, detail: impl Into<String>) -> Self {
        MgrsError {
            part,
            detail: detail.into(),
        }
    }
}

/// Une référence MGRS lue : zone, bande, carré de 100 km, puis `precision` chiffres par axe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mgrs {
    pub zone: u8,
    pub band: char,
    pub column: char,
    pub row: char,
    /// Abscisse dans le carré, sur `precision` chiffres.
    pub easting: u32,
    /// Ordonnée dans le carré, sur `precision` chiffres.
    pub northing: u32,
    /// Chiffres par axe, de 1 (10 km) à 5 (1 m).
    pub precision: u8,
}

/// Côté de la case désignée à la précision `precision`, en mètres.
fn cell_size(precision: u8) -> u32 {
    10u32.pow(5 - u32::from(precision))
}

fn column_letters(zone: u8) -> &'static [u8; 8] {
    COLUMNS[usize::from((zone - 1) % 3)]
}

fn row_offset(zone: u8) -> usize {
    if zone.is_multiple_of(2) {
        5
    } else {
        0
    }
}

/// Étendue en longitude d'une zone de grille (zone et bande), en écart au méridien central de
/// la zone, en degrés : ±3° en général ; 31V de 0° à 3° E, 32V de 3° à 12° E, 31X de 0° à 9° E,
/// 33X de 9° à 21° E, 35X de 21° à 33° E, 37X de 33° à 42° E (NGA).
fn grid_zone_extent(zone: u8, band: char) -> (f64, f64) {
    let (west, east) = match (zone, band) {
        (31, 'V') => (0.0, 3.0),
        (32, 'V') => (3.0, 12.0),
        (31, 'X') => (0.0, 9.0),
        (33, 'X') => (9.0, 21.0),
        (35, 'X') => (21.0, 33.0),
        (37, 'X') => (33.0, 42.0),
        _ => return (-3.0, 3.0),
    };
    let cm = central_meridian(zone);
    (west - cm, east - cm)
}

impl Mgrs {
    /// Lit une référence : espaces et casse ignorés, zone de 1 à 60 (un ou deux chiffres),
    /// bande de C à X sans I ni O, carré de 100 km valable pour la zone, puis de 1 à 5 chiffres
    /// par axe, autant pour chaque axe. La cohérence de la bande avec l'ordonnée est vérifiée
    /// par [`Mgrs::south_west`].
    pub fn parse(text: &str) -> Result<Mgrs, MgrsError> {
        let chars: Vec<char> = text
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| c.to_ascii_uppercase())
            .collect();
        let zone_digits = chars.iter().take_while(|c| c.is_ascii_digit()).count();
        if zone_digits == 0 || zone_digits > 2 {
            return Err(MgrsError::new(
                MgrsPart::Zone,
                "la référence doit commencer par le numéro de la zone, de 1 à 60",
            ));
        }
        let zone = chars[..zone_digits]
            .iter()
            .fold(0u8, |n, c| n * 10 + c.to_digit(10).unwrap_or(0) as u8);
        if !(1..=60).contains(&zone) {
            return Err(MgrsError::new(
                MgrsPart::Zone,
                format!("la zone {zone} n'existe pas (de 1 à 60)"),
            ));
        }
        let band = match chars.get(zone_digits) {
            None => return Err(MgrsError::new(MgrsPart::Band, "la bande manque")),
            Some(c @ ('A' | 'B' | 'Y' | 'Z')) => {
                return Err(MgrsError::new(
                    MgrsPart::Band,
                    format!("la bande {c} est polaire (UPS), hors de la grille UTM"),
                ))
            }
            Some(c) if c.is_ascii() && BANDS.contains(&(*c as u8)) => *c,
            Some(c) => {
                return Err(MgrsError::new(
                    MgrsPart::Band,
                    format!("« {c} » n'est pas une bande (de C à X, sans I ni O)"),
                ))
            }
        };
        if band == 'X' && matches!(zone, 32 | 34 | 36) {
            return Err(MgrsError::new(
                MgrsPart::Zone,
                format!("la zone {zone}X n'existe pas : au Svalbard, seules 31X, 33X, 35X et 37X"),
            ));
        }
        let (Some(&column), Some(&row)) = (chars.get(zone_digits + 1), chars.get(zone_digits + 2))
        else {
            return Err(MgrsError::new(
                MgrsPart::Square,
                "le carré de 100 km (deux lettres) manque",
            ));
        };
        let letter = |c: char| c.is_ascii_uppercase().then_some(c as u8);
        if !letter(column).is_some_and(|c| column_letters(zone).contains(&c)) {
            return Err(MgrsError::new(
                MgrsPart::Square,
                format!("la colonne « {column} » n'existe pas dans la zone {zone}"),
            ));
        }
        if !letter(row).is_some_and(|c| ROWS.contains(&c)) {
            return Err(MgrsError::new(
                MgrsPart::Square,
                format!("le rang « {row} » n'existe pas (de A à V, sans I ni O)"),
            ));
        }
        let digits = &chars[zone_digits + 3..];
        if digits.is_empty() || !digits.iter().all(char::is_ascii_digit) {
            return Err(MgrsError::new(
                MgrsPart::Digits,
                "après le carré de 100 km ne viennent que des chiffres, au moins un par axe",
            ));
        }
        if !digits.len().is_multiple_of(2) || digits.len() > 10 {
            return Err(MgrsError::new(
                MgrsPart::Digits,
                format!(
                    "{} chiffres : il en faut autant pour chaque axe, de 1 à 5 par axe",
                    digits.len()
                ),
            ));
        }
        let half = digits.len() / 2;
        let number = |d: &[char]| {
            d.iter()
                .fold(0u32, |n, c| n * 10 + c.to_digit(10).unwrap_or(0))
        };
        Ok(Mgrs {
            zone,
            band,
            column,
            row,
            easting: number(&digits[..half]),
            northing: number(&digits[half..]),
            precision: half as u8,
        })
    }

    /// Forme canonique : majuscules, sans espace, zone sur deux chiffres (`31UDQ4825111750`).
    pub fn canonical(&self) -> String {
        let p = usize::from(self.precision);
        format!(
            "{:02}{}{}{}{:0p$}{:0p$}",
            self.zone, self.band, self.column, self.row, self.easting, self.northing
        )
    }

    /// Référence d'un point WGS84 dans sa zone normale, à `precision` chiffres par axe (1 à
    /// 5), **tronquée** (convention de la NGA). `None` hors de [-80, 84] de latitude.
    pub fn from_lat_lon(lat: f64, lon: f64, precision: u8) -> Option<Mgrs> {
        if !(1..=5).contains(&precision) || !lon.is_finite() {
            return None;
        }
        let band = latitude_band(lat)?;
        let zone = utm_zone(lat, lon);
        let utm = to_utm(lat, lon, zone);
        // Troncature au mètre, à un micromètre près : le coin d'une case relu puis reprojeté
        // retombe à quelques nanomètres de son mètre, parfois juste en dessous, et doit redonner
        // la même case.
        let easting = (utm.easting + TRUNCATION_SLACK).floor() as i64;
        let northing = (utm.northing + TRUNCATION_SLACK).floor() as i64;
        let column = easting.div_euclid(100_000);
        if !(1..=8).contains(&column) || northing < 0 {
            return None;
        }
        let row = (northing.div_euclid(100_000) as usize + row_offset(zone)) % ROWS.len();
        let size = i64::from(cell_size(precision));
        Some(Mgrs {
            zone,
            band,
            column: char::from(column_letters(zone)[column as usize - 1]),
            row: char::from(ROWS[row]),
            easting: (easting.rem_euclid(100_000) / size) as u32,
            northing: (northing.rem_euclid(100_000) / size) as u32,
            precision,
        })
    }

    /// Coin sud-ouest de la case désignée, en UTM dans la zone de la référence (convention de
    /// la NGA). Le cycle de 2 000 km des rangs est levé par la bande : la case doit toucher la
    /// bande annoncée, sinon la bande est incohérente (`MgrsPart::Band`).
    pub fn south_west(&self) -> Result<Utm, MgrsError> {
        let column = column_letters(self.zone)
            .iter()
            .position(|c| char::from(*c) == self.column)
            .ok_or_else(|| MgrsError::new(MgrsPart::Square, "colonne inconnue"))?;
        let row = ROWS
            .iter()
            .position(|c| char::from(*c) == self.row)
            .ok_or_else(|| MgrsError::new(MgrsPart::Square, "rang inconnu"))?;
        let band = BANDS
            .iter()
            .position(|c| char::from(*c) == self.band)
            .ok_or_else(|| MgrsError::new(MgrsPart::Band, "bande inconnue"))?;
        let size = f64::from(cell_size(self.precision));
        let easting = (column as f64 + 1.0) * 100_000.0 + f64::from(self.easting) * size;
        let base = ((row + ROWS.len() - row_offset(self.zone)) % ROWS.len()) as f64 * 100_000.0
            + f64::from(self.northing) * size;
        let south = MIN_LATITUDE + 8.0 * band as f64;
        let north_edge = if band == BANDS.len() - 1 {
            MAX_LATITUDE
        } else {
            south + 8.0
        };
        let north = band >= 10;
        for cycle in 0..5 {
            let northing = base + f64::from(cycle) * 2_000_000.0;
            let corners: Vec<(f64, f64)> = [(0.0, 0.0), (size, 0.0), (0.0, size), (size, size)]
                .iter()
                .map(|(de, dn)| {
                    from_utm(&Utm {
                        zone: self.zone,
                        north,
                        easting: easting + de,
                        northing: northing + dn,
                    })
                })
                .collect();
            let low = corners.iter().map(|c| c.0).fold(f64::INFINITY, f64::min);
            let high = corners
                .iter()
                .map(|c| c.0)
                .fold(f64::NEG_INFINITY, f64::max);
            if low < north_edge && high >= south {
                // La case doit aussi toucher l'étendue en longitude de sa zone de grille,
                // exceptions 31V, 32V et 31X à 37X comprises (écarts au méridien central).
                let cm = central_meridian(self.zone);
                let dlons: Vec<f64> = corners.iter().map(|c| wrap_longitude(c.1 - cm)).collect();
                let (west, east) = grid_zone_extent(self.zone, self.band);
                let min = dlons.iter().copied().fold(f64::INFINITY, f64::min);
                let max = dlons.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                if max < west || min > east {
                    return Err(MgrsError::new(
                        MgrsPart::Square,
                        format!(
                            "le carré {}{} est hors de la zone de grille {}{}",
                            self.column, self.row, self.zone, self.band
                        ),
                    ));
                }
                return Ok(Utm {
                    zone: self.zone,
                    north,
                    easting,
                    northing,
                });
            }
        }
        Err(MgrsError::new(
            MgrsPart::Band,
            format!(
                "le carré {}{} ne coupe pas la bande {} de la zone {}",
                self.column, self.row, self.band, self.zone
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_projection_round_trips_to_the_nanometre_scale() {
        for (lat, dlon) in [
            (0.0, 0.0),
            (48.7, -0.6),
            (-33.9, 2.9),
            (78.2, 7.5),
            (-79.9, 3.1),
        ] {
            let (x, y) = tm_forward(lat, dlon);
            let (lat2, dlon2) = tm_reverse(x, y);
            assert!((lat - lat2).abs() < 1e-11, "{lat} {lat2}");
            assert!((dlon - dlon2).abs() < 1e-11, "{dlon} {dlon2}");
        }
    }

    #[test]
    fn the_zones_follow_the_nga_exceptions() {
        assert_eq!(utm_zone(48.7, 2.4), 31);
        assert_eq!(utm_zone(60.4, 5.3), 32);
        assert_eq!(utm_zone(60.4, 2.9), 31);
        assert_eq!(utm_zone(78.0, 8.0), 31);
        assert_eq!(utm_zone(78.0, 20.0), 33);
        assert_eq!(utm_zone(78.0, 30.0), 35);
        assert_eq!(utm_zone(78.0, 40.0), 37);
        assert_eq!(utm_zone(0.0, 180.0), 1);
        assert_eq!(utm_zone(0.0, 179.9), 60);
        assert_eq!(zone_distance(60, 1), 1);
        assert_eq!(latitude_band(84.0), Some('X'));
        assert_eq!(latitude_band(-80.0), Some('C'));
        assert_eq!(latitude_band(84.1), None);
    }

    #[test]
    fn a_reference_is_read_leniently_and_written_canonically() {
        let m = Mgrs::parse(" 4qfj 1234 5678 ").unwrap();
        assert_eq!(m.canonical(), "04QFJ12345678");
        assert_eq!(m.precision, 4);
    }
}
