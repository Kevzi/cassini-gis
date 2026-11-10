//! Modèle magnétique mondial WMM2025 (NOAA et British Geological Survey, domaine public) : le
//! champ géomagnétique principal en un point, et sa déclinaison, pour l'azimut magnétique de
//! `geo.distance_azimuth` et `geo.destination_point` (`north: "magnetic"`, AD-31 amendée).
//!
//! Écrit en salle blanche d'après le rapport technique — NOAA et BGS, *The US/UK World
//! Magnetic Model for 2025-2030: Technical Report*, doi:10.25923/prbc-s316, §1.2 (équations 7 à
//! 19) et §1.4 (prolongement au pôle) — jamais d'après le logiciel de la NOAA. Coefficients
//! publiés (`core/data/wmm2025/WMM2025.COF`, 17 U.S.C. 403) compilés dans le binaire : aucun
//! fichier n'est lu à l'exécution, rien n'est téléchargé (D6). Trigonométrie par `libm`, mêmes
//! octets sous Windows et sous Linux (rejeu, AD-11).

use std::sync::OnceLock;

/// Coefficients du WMM2025, recopiés tels que publiés (voir `core/data/wmm2025/LISEZMOI.md`).
const COF: &str = include_str!("../data/wmm2025/WMM2025.COF");

/// Degré maximal du développement en harmoniques sphériques.
const N: usize = 12;

/// Demi-grand axe de l'ellipsoïde WGS84, en mètres.
const A: f64 = 6_378_137.0;
/// Aplatissement inverse du WGS84.
const INVERSE_FLATTENING: f64 = 298.257_223_563;
/// Rayon de référence géomagnétique, en mètres.
const REFERENCE_RADIUS: f64 = 6_371_200.0;

/// Première date de la période de validité du WMM2025.
pub const VALID_FROM: &str = "2025-01-01";
/// Dernière date de la période de validité du WMM2025.
pub const VALID_TO: &str = "2029-12-31";

/// Seuil de la zone d'ombre autour d'un pôle magnétique (rapport, §1.8), en nT.
pub const BLACKOUT_NT: f64 = 2000.0;
/// Seuil de la zone de prudence qui l'entoure (rapport, §1.8), en nT.
pub const CAUTION_NT: f64 = 6000.0;

/// Coefficients de Gauss semi-normalisés de Schmidt, à l'époque du modèle, et leur variation
/// séculaire annuelle : `g[n][m]`, en nT et nT/an.
struct Model {
    epoch: f64,
    g: [[f64; N + 1]; N + 1],
    h: [[f64; N + 1]; N + 1],
    g_dot: [[f64; N + 1]; N + 1],
    h_dot: [[f64; N + 1]; N + 1],
}

/// Lit le fichier de coefficients : un en-tête (époque, nom, date), puis une ligne
/// « n m g h ġ ḣ » par coefficient, jusqu'à la ligne de 9.
fn parse(text: &str) -> Model {
    let mut lines = text.lines();
    let epoch = lines
        .next()
        .and_then(|l| l.split_whitespace().next())
        .and_then(|e| e.parse().ok())
        .expect("en-tête du WMM2025.COF");
    let zero = [[0.0; N + 1]; N + 1];
    let mut model = Model {
        epoch,
        g: zero,
        h: zero,
        g_dot: zero,
        h_dot: zero,
    };
    for line in lines {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 6 {
            break;
        }
        let n: usize = fields[0].parse().expect("degré n");
        let m: usize = fields[1].parse().expect("ordre m");
        let v: Vec<f64> = fields[2..6]
            .iter()
            .map(|f| f.parse().expect("coefficient"))
            .collect();
        model.g[n][m] = v[0];
        model.h[n][m] = v[1];
        model.g_dot[n][m] = v[2];
        model.h_dot[n][m] = v[3];
    }
    model
}

fn model() -> &'static Model {
    static MODEL: OnceLock<Model> = OnceLock::new();
    MODEL.get_or_init(|| parse(COF))
}

/// Champ géomagnétique principal en un point : composantes nord (X), est (Y) et vers le bas (Z),
/// intensités horizontale (H) et totale (F), en nT ; déclinaison (D, positive à l'est) et
/// inclinaison (I, positive vers le bas), en degrés.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub h: f64,
    pub f: f64,
    pub d: f64,
    pub i: f64,
}

/// Étapes intermédiaires du calcul (rapport, table 3b) : latitude géocentrique φ′ (rad), rayon
/// r (m), composantes en sphérique géocentrique X′, Y′, Z′ (nT).
#[derive(Debug, Clone, Copy)]
// `phi_prime` et `r` ne sont lus que par le test de la table 3b.
#[cfg_attr(not(test), allow(dead_code))]
struct Spherical {
    phi_prime: f64,
    r: f64,
    x: f64,
    y: f64,
    z: f64,
}

/// Le champ à l'année décimale `decimal_year`, à la hauteur `height_m` au-dessus de
/// l'ellipsoïde WGS84 et au point géodésique (`lat`, `lon`) en degrés.
pub fn field(decimal_year: f64, height_m: f64, lat: f64, lon: f64) -> Field {
    field_of(&compute(decimal_year, height_m, lat, lon), lat)
}

/// Rotation des composantes sphériques vers le repère géodésique (rapport, éq. 17 à 19).
fn field_of(s: &Spherical, lat: f64) -> Field {
    let phi = lat.to_radians();
    let (sd, cd) = (libm::sin(s.phi_prime - phi), libm::cos(s.phi_prime - phi));
    let x = s.x * cd - s.z * sd;
    let y = s.y;
    let z = s.x * sd + s.z * cd;
    let h = libm::sqrt(x * x + y * y);
    let f = libm::sqrt(h * h + z * z);
    Field {
        x,
        y,
        z,
        h,
        f,
        d: libm::atan2(y, x).to_degrees(),
        i: libm::atan2(z, h).to_degrees(),
    }
}

fn compute(decimal_year: f64, height_m: f64, lat: f64, lon: f64) -> Spherical {
    let model = model();
    // Géodésique → sphérique géocentrique (rapport, éq. 7 et 8).
    let phi = lat.to_radians();
    let lambda = lon.to_radians();
    let flattening = 1.0 / INVERSE_FLATTENING;
    let e2 = flattening * (2.0 - flattening);
    let sin_phi = libm::sin(phi);
    let rc = A / libm::sqrt(1.0 - e2 * sin_phi * sin_phi);
    let p = (rc + height_m) * libm::cos(phi);
    let zc = (rc * (1.0 - e2) + height_m) * sin_phi;
    let r = libm::sqrt(p * p + zc * zc);
    let phi_prime = libm::asin(zc / r);

    // Fonctions de Legendre associées semi-normalisées de Schmidt P(n, m)(sin φ′), avec
    // cos θ = sin φ′ et sin θ = cos φ′, leurs dérivées en θ, et Q = P / sin θ pour m ≥ 1 : Y′ se
    // calcule sans diviser par cos φ′, et reste fini au pôle (prolongement, rapport §1.4).
    let cos_t = libm::sin(phi_prime);
    let sin_t = libm::cos(phi_prime);
    let mut pp = [[0.0; N + 1]; N + 1];
    let mut dp = [[0.0; N + 1]; N + 1];
    let mut q = [[0.0; N + 1]; N + 1];
    pp[0][0] = 1.0;
    for n in 1..=N {
        let nf = n as f64;
        for m in 0..=n {
            let mf = m as f64;
            if m == n {
                if n == 1 {
                    pp[1][1] = sin_t;
                    dp[1][1] = cos_t;
                    q[1][1] = 1.0;
                } else {
                    let k = libm::sqrt((2.0 * nf - 1.0) / (2.0 * nf));
                    pp[n][n] = k * sin_t * pp[n - 1][n - 1];
                    dp[n][n] = k * (cos_t * pp[n - 1][n - 1] + sin_t * dp[n - 1][n - 1]);
                    q[n][n] = k * sin_t * q[n - 1][n - 1];
                }
            } else {
                let s = libm::sqrt(nf * nf - mf * mf);
                // P(n − 2, m) n'existe pas pour n − 2 < m : son facteur est alors nul.
                let (k2, p2, dp2, q2) = if n >= m + 2 {
                    (
                        libm::sqrt((nf - 1.0) * (nf - 1.0) - mf * mf),
                        pp[n - 2][m],
                        dp[n - 2][m],
                        q[n - 2][m],
                    )
                } else {
                    (0.0, 0.0, 0.0, 0.0)
                };
                let a = 2.0 * nf - 1.0;
                pp[n][m] = (a * cos_t * pp[n - 1][m] - k2 * p2) / s;
                dp[n][m] = (a * (cos_t * dp[n - 1][m] - sin_t * pp[n - 1][m]) - k2 * dp2) / s;
                if m >= 1 {
                    q[n][m] = (a * cos_t * q[n - 1][m] - k2 * q2) / s;
                }
            }
        }
    }

    // Coefficients à la date (rapport, éq. 9), puis sommes du champ (éq. 10 à 12) ; d/dφ′ =
    // −d/dθ.
    let dt = decimal_year - model.epoch;
    let ratio = REFERENCE_RADIUS / r;
    let mut power = ratio * ratio;
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    for n in 1..=N {
        power *= ratio;
        let nf = n as f64;
        for m in 0..=n {
            let g = model.g[n][m] + dt * model.g_dot[n][m];
            let h = model.h[n][m] + dt * model.h_dot[n][m];
            let ml = m as f64 * lambda;
            let (s, c) = (libm::sin(ml), libm::cos(ml));
            let gc_hs = g * c + h * s;
            x += power * gc_hs * dp[n][m];
            if m >= 1 {
                y += power * m as f64 * (g * s - h * c) * q[n][m];
            }
            z -= (nf + 1.0) * power * gc_hs * pp[n][m];
        }
    }
    Spherical {
        phi_prime,
        r,
        x,
        y,
        z,
    }
}

/// Une date civile du calendrier grégorien.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

fn leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

impl Date {
    /// Lit une date AAAA-MM-JJ, quatre chiffres, deux et deux, séparés par des tirets ; une date
    /// qui n'existe pas au calendrier grégorien (2026-02-30) est refusée.
    pub fn parse(text: &str) -> Option<Date> {
        let b = text.as_bytes();
        let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        if !(digits(0..4) && digits(5..7) && digits(8..10)) {
            return None;
        }
        let date = Date {
            year: text[0..4].parse().ok()?,
            month: text[5..7].parse().ok()?,
            day: text[8..10].parse().ok()?,
        };
        (1..=days_in_month(date.year, date.month))
            .contains(&date.day)
            .then_some(date)
    }

    /// Année décimale de la date, à 0 h UTC : année + (jour de l'année − 1) / nombre de jours de
    /// l'année (décision M1).
    pub fn decimal_year(&self) -> f64 {
        let before: u32 = (1..self.month).map(|m| days_in_month(self.year, m)).sum();
        let length = if leap(self.year) { 366.0 } else { 365.0 };
        f64::from(self.year) + f64::from(before + self.day - 1) / length
    }

    /// Vrai si la date est dans la période de validité du WMM2025 (2025-01-01 au 2029-12-31).
    pub fn is_valid_for_model(&self) -> bool {
        (2025..=2029).contains(&self.year)
    }

    /// La date du jour `days` comptés depuis le 1970-01-01 (jour 0).
    pub fn from_days(days: i64) -> Date {
        // Conversion des jours en date civile par ères de 400 ans (146 097 jours), l'année
        // commençant au 1er mars pour que le jour bissextile soit le dernier.
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
        Date { year, month, day }
    }

    /// La date du jour, en temps universel.
    pub fn today_utc() -> Date {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Date::from_days((seconds / 86_400) as i64)
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Avertissement d'un azimut magnétique, en français, ou `None` : date hors de la validité du
/// WMM2025 (décision M2), point où la composante horizontale est faible, près d'un pôle
/// magnétique (zone d'ombre sous 2 000 nT, de prudence sous 6 000 nT, rapport §1.8, décision
/// M3). `points` : chaque point où un azimut est mesuré, nommé (« Le point de départ »), avec son
/// champ. Plusieurs avertissements forment un seul texte.
pub fn warning(date: &Date, points: &[(&str, &Field)]) -> Option<String> {
    let mut parts = Vec::new();
    if !date.is_valid_for_model() {
        parts.push(format!(
            "La date du {date} est hors de la période de validité du modèle magnétique mondial WMM2025 (du {VALID_FROM} au {VALID_TO}) : la déclinaison est extrapolée, sa précision n'est pas garantie."
        ));
    }
    for (name, field) in points {
        if field.h < BLACKOUT_NT {
            parts.push(format!(
                "{name} est dans la zone d'ombre d'un pôle magnétique définie par le WMM (composante horizontale du champ sous 2 000 nT) : la boussole n'y est pas fiable, l'erreur de déclinaison peut atteindre 180°."
            ));
        } else if field.h < CAUTION_NT {
            parts.push(format!(
                "{name} approche de la zone d'ombre d'un pôle magnétique définie par le WMM (composante horizontale du champ sous 6 000 nT) : la précision de la boussole peut y être dégradée."
            ));
        }
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Valeurs de test publiées avec le modèle, lues par les tests seulement.
    const TEST_VALUES: &str = include_str!("../data/wmm2025/WMM2025_TestValues.txt");

    // Rapport technique, tables 3a et 3b (WMM2025) : 2027,5, 100 km au-dessus de l'ellipsoïde,
    // 80° S, 240° E ; tables recopiées à toutes leurs décimales.
    #[test]
    fn the_report_numerical_example_is_found() {
        let (t, height, lat, lon) = (2027.5, 100_000.0, -80.0, 240.0);
        let s = compute(t, height, lat, lon);
        let f = field_of(&s, lat);
        let close = |got: f64, want: f64, tol: f64, what: &str| {
            assert!(
                (got - want).abs() <= tol,
                "{what} : {got} au lieu de {want}"
            );
        };
        close(s.phi_prime, -1.395_128_958_9, 1e-10, "φ′");
        close(s.r, 6_457_402.348_447_37, 1e-6, "r");
        close(s.x, 5_928.024_139_258_8, 1e-6, "X′");
        close(s.y, 14_760.135_975_786_8, 1e-6, "Y′");
        close(s.z, -49_324.427_357_028_4, 1e-6, "Z′");
        close(f.x, 5_983.976_049_651_8, 1e-6, "X");
        close(f.y, 14_760.135_975_786_8, 1e-6, "Y");
        close(f.z, -49_317.670_615_425_5, 1e-6, "Z");
        close(f.f, 51_825.690_717_231_4, 1e-6, "F");
        close(f.h, 15_927.007_986_013_0, 1e-6, "H");
        close(f.d.to_radians(), 1.185_630_840_7, 1e-10, "D");
        close(f.i.to_radians(), -1.258_422_154_1, 1e-10, "I");
    }

    // Rapport, table 3d (« Grivation calculations for WMM2025 ») : même point, « TrueN-to-MagN »
    // = 67,9316432316° ; « GridN-to-MagN » en zone UTM 10 = D − γ = 70,8861479117°, γ étant la
    // convergence de l'entrée 28 (−2,9545046801°).
    #[test]
    fn the_report_grivation_table_is_found() {
        let d = field(2027.5, 100_000.0, -80.0, 240.0).d;
        assert!((d - 67.931_643_231_6).abs() <= 1e-9, "{d}");
        let gamma = crate::geodesy::grid_convergence(-80.0, 240.0, 10);
        assert!(
            (d - gamma - 70.886_147_911_7).abs() <= 1e-9,
            "{}",
            d - gamma
        );
    }

    // Les 100 valeurs de test publiées (WMM2025_TestValues.txt), chacune à sa date, son altitude
    // et sa position : Y et Z à 1e-5 nT, X, H et F à 1e-3 nT (le fichier publié s'écarte de
    // quelques 1e-4 nT sur X, la table 3b est retrouvée au dernier chiffre), la déclinaison à
    // 1e-5° de atan2(Y, X) publiés et à 0,005° de D publiée (arrondie au centième).
    #[test]
    fn the_published_test_values_are_found() {
        let mut count = 0;
        for line in TEST_VALUES.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let v: Vec<f64> = line
                .split_whitespace()
                .map(|f| f.parse().unwrap())
                .collect();
            assert_eq!(v.len(), 18, "{line}");
            let f = field(v[0], v[1] * 1000.0, v[2], v[3]);
            let at = format!("{} {} km ({}, {})", v[0], v[1], v[2], v[3]);
            assert!((f.y - v[8]).abs() <= 1e-5, "Y {at} : {} / {}", f.y, v[8]);
            assert!((f.z - v[9]).abs() <= 1e-5, "Z {at} : {} / {}", f.z, v[9]);
            assert!((f.x - v[7]).abs() <= 1e-3, "X {at} : {} / {}", f.x, v[7]);
            assert!((f.h - v[6]).abs() <= 1e-3, "H {at} : {} / {}", f.h, v[6]);
            assert!((f.f - v[10]).abs() <= 1e-3, "F {at} : {} / {}", f.f, v[10]);
            let d = libm::atan2(v[8], v[7]).to_degrees();
            assert!((f.d - d).abs() <= 1e-5, "D {at} : {} / {d}", f.d);
            assert!(
                (f.d - v[4]).abs() <= 0.005 + 1e-9,
                "D {at} : {} / {}",
                f.d,
                v[4]
            );
            count += 1;
        }
        assert_eq!(count, 100);
    }

    #[test]
    fn the_coefficients_are_the_published_ones() {
        let m = model();
        assert_eq!(m.epoch, 2025.0);
        assert!(COF.starts_with("    2025.0            WMM-2025     11/13/2024"));
        assert_eq!(m.g[1][0], -29351.8);
        assert_eq!(m.h[1][1], 4545.4);
        assert_eq!(m.g_dot[1][1], 9.7);
        assert_eq!(m.g[12][12], -0.7);
        assert_eq!(m.h_dot[12][12], -0.1);
    }

    #[test]
    fn the_field_is_finite_at_the_geographic_poles() {
        for lat in [90.0, -90.0] {
            let f = field(2026.0, 0.0, lat, 0.0);
            for v in [f.x, f.y, f.z, f.h, f.f, f.d, f.i] {
                assert!(v.is_finite(), "{lat} : {f:?}");
            }
        }
    }

    #[test]
    fn dates_are_read_strictly_and_give_a_decimal_year() {
        assert_eq!(
            Date::parse("2026-05-01"),
            Some(Date {
                year: 2026,
                month: 5,
                day: 1
            })
        );
        for bad in [
            "2026-13-01",
            "2026-02-30",
            "2025-02-29",
            "10/05/2026",
            "2026-5-1",
            "2026-05-01T00:00",
            " 2026-05-01",
            "2026-00-10",
            "2026-04-31",
            "+026-05-01",
            "",
        ] {
            assert_eq!(Date::parse(bad), None, "{bad}");
        }
        assert!(Date::parse("2028-02-29").is_some());
        assert!(Date::parse("2000-02-29").is_some());
        assert!(Date::parse("1900-02-29").is_none());
        // M1 : année + (jour de l'année − 1) / nombre de jours de l'année.
        assert_eq!(Date::parse("2025-01-01").unwrap().decimal_year(), 2025.0);
        assert_eq!(
            Date::parse("2026-05-01").unwrap().decimal_year(),
            2026.0 + 120.0 / 365.0
        );
        assert_eq!(
            Date::parse("2028-12-31").unwrap().decimal_year(),
            2028.0 + 365.0 / 366.0
        );
        assert_eq!(
            Date::parse("2027-07-02").unwrap().decimal_year(),
            2027.0 + 182.0 / 365.0
        );
        assert_eq!(Date::parse("2026-05-01").unwrap().to_string(), "2026-05-01");
        assert!(Date::parse("2029-12-31").unwrap().is_valid_for_model());
        assert!(!Date::parse("2030-01-01").unwrap().is_valid_for_model());
        assert!(!Date::parse("2024-12-31").unwrap().is_valid_for_model());
    }

    #[test]
    fn days_since_1970_give_the_civil_date() {
        for (days, text) in [
            (0, "1970-01-01"),
            (-1, "1969-12-31"),
            (11_017, "2000-03-01"),
            (19_782, "2024-02-29"),
            (20_767, "2026-11-10"),
        ] {
            assert_eq!(Date::from_days(days).to_string(), text, "{days}");
        }
        let today = Date::today_utc();
        assert!(today.year >= 2026, "{today}");
        assert_eq!(Date::parse(&today.to_string()), Some(today));
    }

    #[test]
    fn a_weak_horizontal_field_or_an_old_date_warns() {
        let date = Date::parse("2026-05-01").unwrap();
        let paris = field(date.decimal_year(), 0.0, 48.85, 2.35);
        assert!(paris.h > CAUTION_NT);
        assert_eq!(warning(&date, &[("Le point de départ", &paris)]), None);
        let late = Date::parse("2031-03-01").unwrap();
        let text = warning(&late, &[("Le point de départ", &paris)]).unwrap();
        assert!(
            text.contains("2031-03-01") && text.contains("2029-12-31"),
            "{text}"
        );
        // Près du pôle nord magnétique (vers 85° N, 140° E en 2026), H est sous 2 000 nT.
        let pole = field(date.decimal_year(), 0.0, 85.5, 139.0);
        assert!(pole.h < BLACKOUT_NT, "{pole:?}");
        let text = warning(&date, &[("Le point d'arrivée", &pole)]).unwrap();
        assert!(
            text.starts_with("Le point d'arrivée est dans la zone d'ombre"),
            "{text}"
        );
        let text = warning(
            &late,
            &[
                ("Le point de départ", &paris),
                ("Le point d'arrivée", &pole),
            ],
        )
        .unwrap();
        assert!(
            text.contains("validité") && text.contains("zone d'ombre"),
            "{text}"
        );
    }
}
