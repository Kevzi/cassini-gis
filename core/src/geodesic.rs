//! Géodésiques sur l'ellipsoïde WGS84 : problèmes direct et inverse, et recoupement de deux
//! visées (AD-31, CAP-15).
//!
//! Provenance, écrite d'après les seules publications :
//! - C. F. F. Karney, « Algorithms for geodesics », *Journal of Geodesy*, 87(1), 2013,
//!   p. 43-55 (doi:10.1007/s00190-012-0578-z) : séries d'ordre 6 en ε et n (éq. 15 à 25 ; A₁,
//!   C₁, C′₁, A₂, C₂, A₃, C₃), sommation de Clenshaw, problème direct (§ 4), problème inverse
//!   (§ 5) : Newton sur α₁ (éq. 54 à 57), départ par l'astroïde pour les points quasi
//!   antipodaux (§ 6, éq. 55 à 59), cas des méridiens et de l'équateur.
//! - Coefficients et tolérances confrontés à GeographicLib 2.0 (C. F. F. Karney, licence MIT),
//!   dont le jeu d'essai sert aux tests.
//!
//! Toute trigonométrie passe par `libm`, version figée dans `core/Cargo.toml` : les mêmes octets
//! sous Windows et sous Linux, condition du rejeu (AD-11). Aucune distance plane n'est calculée
//! ici. Le recoupement est bâti sur les deux problèmes : départ sphérique, puis Newton sur les
//! deux longueurs.

use std::f64::consts::PI;
use std::sync::OnceLock;

use crate::geodesy::{WGS84_A, WGS84_F};

/// Ordre des séries.
const ORDER: usize = 6;
/// Itérations de Newton sur α₁ avant de passer à la bissection.
const MAXIT1: u32 = 20;
/// Itérations au total (Newton puis bissection), 53 bits de mantisse compris.
const MAXIT2: u32 = MAXIT1 + f64::MANTISSA_DIGITS + 10;

/// L'ellipsoïde et ses coefficients, calculés une fois.
struct Ellipsoid {
    a: f64,
    f: f64,
    f1: f64,
    ep2: f64,
    n: f64,
    b: f64,
    a3x: [f64; ORDER],
    c3x: [f64; 15],
    tiny: f64,
    tol0: f64,
    tol1: f64,
    tolb: f64,
    xthresh: f64,
    etol2: f64,
}

fn wgs84() -> &'static Ellipsoid {
    static E: OnceLock<Ellipsoid> = OnceLock::new();
    E.get_or_init(|| Ellipsoid::new(WGS84_A, WGS84_F))
}

/// Polynôme de degré `n`, coefficients du plus haut degré au plus bas, par Horner.
fn polyval(n: usize, p: &[f64], x: f64) -> f64 {
    let mut y = p[0];
    for &c in &p[1..=n] {
        y = y * x + c;
    }
    y
}

/// Coefficients de A₃ (Karney 2013, éq. 24), par puissance de ε décroissante, en n.
const COEFF_A3: [f64; 18] = [
    -3.0, 128.0, // ε⁵
    -2.0, -3.0, 64.0, // ε⁴
    -1.0, -3.0, -1.0, 16.0, // ε³
    3.0, -1.0, -2.0, 8.0, // ε²
    1.0, -1.0, 2.0, // ε¹
    1.0, 1.0, // ε⁰
];

/// Coefficients de C₃ₗ (Karney 2013, éq. 25), l = 1 à 5, par puissance de ε décroissante, en n.
const COEFF_C3: [f64; 45] = [
    3.0, 128.0, 2.0, 5.0, 128.0, -1.0, 3.0, 3.0, 64.0, -1.0, 0.0, 1.0, 8.0, -1.0, 1.0,
    4.0, // C₃₁
    5.0, 256.0, 1.0, 3.0, 128.0, -3.0, -2.0, 3.0, 64.0, 1.0, -3.0, 2.0, 32.0, // C₃₂
    7.0, 512.0, -10.0, 9.0, 384.0, 5.0, -9.0, 5.0, 192.0, // C₃₃
    7.0, 512.0, -14.0, 7.0, 512.0, // C₃₄
    21.0, 2560.0, // C₃₅
];

/// Coefficients de C₁ₗ (éq. 18), en ε², chaque bloc suivi de son dénominateur.
const COEFF_C1: [f64; 18] = [
    -1.0, 6.0, -16.0, 32.0, -9.0, 64.0, -128.0, 2048.0, 9.0, -16.0, 768.0, 3.0, -5.0, 512.0, -7.0,
    1280.0, -7.0, 2048.0,
];

/// Coefficients de C′₁ₗ (éq. 21).
const COEFF_C1P: [f64; 18] = [
    205.0, -432.0, 768.0, 1536.0, 4005.0, -4736.0, 3840.0, 12288.0, -225.0, 116.0, 384.0, -7173.0,
    2695.0, 7680.0, 3467.0, 7680.0, 38081.0, 61440.0,
];

/// Coefficients de C₂ₗ (éq. 43).
const COEFF_C2: [f64; 18] = [
    1.0, 2.0, 16.0, 32.0, 35.0, 64.0, 384.0, 2048.0, 15.0, 80.0, 768.0, 7.0, 35.0, 512.0, 63.0,
    1280.0, 77.0, 2048.0,
];

impl Ellipsoid {
    fn new(a: f64, f: f64) -> Self {
        let f1 = 1.0 - f;
        let e2 = f * (2.0 - f);
        let ep2 = e2 / (f1 * f1);
        let n = f / (2.0 - f);
        let mut a3x = [0.0; ORDER];
        let mut o = 0;
        for (k, j) in (0..ORDER).rev().enumerate() {
            let m = (ORDER - j - 1).min(j);
            a3x[k] = polyval(m, &COEFF_A3[o..], n) / COEFF_A3[o + m + 1];
            o += m + 2;
        }
        let mut c3x = [0.0; 15];
        let (mut o, mut k) = (0, 0);
        for l in 1..ORDER {
            for j in (l..ORDER).rev() {
                let m = (ORDER - j - 1).min(j);
                c3x[k] = polyval(m, &COEFF_C3[o..], n) / COEFF_C3[o + m + 1];
                k += 1;
                o += m + 2;
            }
        }
        let tol0 = f64::EPSILON;
        let tol2 = libm::sqrt(tol0);
        Ellipsoid {
            a,
            f,
            f1,
            ep2,
            n,
            b: a * f1,
            a3x,
            c3x,
            tiny: libm::sqrt(f64::MIN_POSITIVE),
            tol0,
            tol1: 200.0 * tol0,
            tolb: tol0 * tol2,
            xthresh: 1000.0 * tol2,
            etol2: 0.1 * tol2 / libm::sqrt(f.abs().max(0.001) * (1.0 - f / 2.0).min(1.0) / 2.0),
        }
    }

    fn a3f(&self, eps: f64) -> f64 {
        polyval(ORDER - 1, &self.a3x, eps)
    }

    fn c3f(&self, eps: f64) -> [f64; ORDER] {
        let mut c = [0.0; ORDER];
        let (mut mult, mut o) = (1.0, 0);
        for (l, cl) in c.iter_mut().enumerate().skip(1) {
            let m = ORDER - l - 1;
            mult *= eps;
            *cl = mult * polyval(m, &self.c3x[o..], eps);
            o += m + 1;
        }
        c
    }

    /// Longueur réduite et distance, rapportées à b (éq. 38 et 41).
    #[allow(clippy::too_many_arguments)]
    fn lengths(
        &self,
        eps: f64,
        sig12: f64,
        (ssig1, csig1, dn1): (f64, f64, f64),
        (ssig2, csig2, dn2): (f64, f64, f64),
    ) -> (f64, f64) {
        let c1a = c_series(&COEFF_C1, eps);
        let c2a = c_series(&COEFF_C2, eps);
        let a1 = a1m1f(eps);
        let a2 = a2m1f(eps);
        let m0x = a1 - a2;
        let (a1, a2) = (1.0 + a1, 1.0 + a2);
        let b1 = sin_cos_series(ssig2, csig2, &c1a) - sin_cos_series(ssig1, csig1, &c1a);
        let b2 = sin_cos_series(ssig2, csig2, &c2a) - sin_cos_series(ssig1, csig1, &c2a);
        let s12b = a1 * (sig12 + b1);
        let j12 = m0x * sig12 + (a1 * b1 - a2 * b2);
        let m12b = dn2 * (csig1 * ssig2) - dn1 * (ssig1 * csig2) - csig1 * csig2 * j12;
        (s12b, m12b)
    }
}

/// A₁ − 1 (éq. 17).
fn a1m1f(eps: f64) -> f64 {
    const COEFF: [f64; 5] = [1.0, 4.0, 64.0, 0.0, 256.0];
    let t = polyval(3, &COEFF, eps * eps) / COEFF[4];
    (t + eps) / (1.0 - eps)
}

/// A₂ − 1 (éq. 42).
fn a2m1f(eps: f64) -> f64 {
    const COEFF: [f64; 5] = [-11.0, -28.0, -192.0, 0.0, 256.0];
    let t = polyval(3, &COEFF, eps * eps) / COEFF[4];
    (t - eps) / (1.0 + eps)
}

/// Coefficients C₁, C′₁ ou C₂ d'ordre 1 à 6 (indice 0 inutilisé).
fn c_series(coeff: &[f64; 18], eps: f64) -> [f64; ORDER + 1] {
    let mut c = [0.0; ORDER + 1];
    let eps2 = eps * eps;
    let (mut d, mut o) = (eps, 0);
    for (l, cl) in c.iter_mut().enumerate().skip(1) {
        let m = (ORDER - l) / 2;
        *cl = d * polyval(m, &coeff[o..], eps2) / coeff[o + m + 1];
        o += m + 2;
        d *= eps;
    }
    c
}

/// Σ cₗ sin 2lσ, par la sommation de Clenshaw (c[0] inutilisé).
fn sin_cos_series(sinx: f64, cosx: f64, c: &[f64]) -> f64 {
    let mut k = c.len();
    let mut n = k - 1;
    let ar = 2.0 * (cosx - sinx) * (cosx + sinx);
    let mut y0 = if n & 1 == 1 {
        k -= 1;
        c[k]
    } else {
        0.0
    };
    let mut y1 = 0.0;
    n /= 2;
    while n > 0 {
        n -= 1;
        k -= 1;
        y1 = ar * y0 - y1 + c[k];
        k -= 1;
        y0 = ar * y1 - y0 + c[k];
    }
    2.0 * sinx * cosx * y0
}

// -- Angles en degrés, sans perte aux multiples de 90° --------------------------------------

fn norm(x: f64, y: f64) -> (f64, f64) {
    let r = libm::hypot(x, y);
    (x / r, y / r)
}

/// Somme exacte : (s, t) avec s + t = u + v.
fn sum(u: f64, v: f64) -> (f64, f64) {
    let s = u + v;
    let up = s - v;
    let vpp = s - up;
    let up = up - u;
    let vpp = vpp - v;
    let t = if s != 0.0 { 0.0 - (up + vpp) } else { s };
    (s, t)
}

/// Ramène un angle à [-180, 180].
pub fn ang_normalize(x: f64) -> f64 {
    let y = libm::remainder(x, 360.0);
    if y.abs() == 180.0 {
        180.0_f64.copysign(x)
    } else {
        y
    }
}

/// y − x, ramené à [-180, 180], et l'erreur d'arrondi.
fn ang_diff(x: f64, y: f64) -> (f64, f64) {
    let (d, t) = sum(libm::remainder(-x, 360.0), libm::remainder(y, 360.0));
    let (mut d, t) = sum(libm::remainder(d, 360.0), t);
    if d == 0.0 || d.abs() == 180.0 {
        d = d.copysign(if t == 0.0 { y - x } else { -t });
    }
    (d, t)
}

/// Arrondit les très petits angles au multiple de 2⁻⁵⁷ le plus proche (Karney 2013, § 5).
fn ang_round(x: f64) -> f64 {
    const Z: f64 = 1.0 / 16.0;
    let mut y = x.abs();
    let w = Z - y;
    if w > 0.0 {
        y = Z - w;
    }
    y.copysign(x)
}

fn sincosd(x: f64) -> (f64, f64) {
    let mut r = libm::fmod(x, 360.0);
    let q = libm::round(r / 90.0);
    r -= 90.0 * q;
    let r = r.to_radians();
    let (s, c) = (libm::sin(r), libm::cos(r));
    let (sinx, cosx) = match (q as i64).rem_euclid(4) {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    };
    let sinx = if sinx == 0.0 { sinx.copysign(x) } else { sinx };
    (sinx, cosx + 0.0)
}

fn atan2d(y: f64, x: f64) -> f64 {
    let (mut x, mut y) = (x, y);
    let mut q = 0;
    if y.abs() > x.abs() {
        std::mem::swap(&mut x, &mut y);
        q = 2;
    }
    if x < 0.0 {
        q += 1;
        x = -x;
    }
    let ang = libm::atan2(y, x).to_degrees();
    match q {
        1 => 180.0_f64.copysign(y) - ang,
        2 => 90.0 - ang,
        3 => -90.0 + ang,
        _ => ang,
    }
}

/// Racine positive k de l'équation de l'astroïde (Karney 2013, éq. 55).
fn astroid(x: f64, y: f64) -> f64 {
    let p = x * x;
    let q = y * y;
    let r = (p + q - 1.0) / 6.0;
    if q == 0.0 && r <= 0.0 {
        return 0.0;
    }
    let s = p * q / 4.0;
    let r2 = r * r;
    let r3 = r * r2;
    let disc = s * (s + 2.0 * r3);
    let mut u = r;
    if disc >= 0.0 {
        let mut t3 = s + r3;
        t3 += if t3 < 0.0 {
            -libm::sqrt(disc)
        } else {
            libm::sqrt(disc)
        };
        let t = libm::cbrt(t3);
        u += t + if t != 0.0 { r2 / t } else { 0.0 };
    } else {
        let ang = libm::atan2(libm::sqrt(-disc), -(s + r3));
        u += 2.0 * r * libm::cos(ang / 3.0);
    }
    let v = libm::sqrt(u * u + q);
    let uv = if u < 0.0 { q / (v - u) } else { u + v };
    let w = (uv - q) / (2.0 * v);
    uv / (libm::sqrt(uv + w * w) + w)
}

// -- Problème inverse --------------------------------------------------------------------------

/// Résultat du problème inverse : distance en mètres, azimuts en degrés dans [-180, 180].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inverse {
    /// Distance géodésique, en mètres.
    pub distance: f64,
    /// Azimut au départ, vers l'arrivée.
    pub azi1: f64,
    /// Azimut de marche à l'arrivée (dans le prolongement de la géodésique).
    pub azi2: f64,
}

/// Une latitude réduite : sinus, cosinus, et √(1 + e′² sin²β).
#[derive(Clone, Copy)]
struct Beta {
    s: f64,
    c: f64,
    dn: f64,
}

struct Lambda {
    v: f64,
    salp2: f64,
    calp2: f64,
    sig12: f64,
    sig1: (f64, f64),
    sig2: (f64, f64),
    eps: f64,
    dv: f64,
}

impl Ellipsoid {
    fn beta(&self, lat: f64) -> Beta {
        let (s, c) = sincosd(lat);
        let (s, c) = norm(s * self.f1, c);
        let c = c.max(self.tiny);
        Beta {
            s,
            c,
            dn: libm::sqrt(1.0 + self.ep2 * s * s),
        }
    }

    /// Départ de Newton (Karney 2013, § 5 et 6) : renvoie σ₁₂ ≥ 0 quand la solution courte est
    /// déjà la bonne, sinon −1, et α₁ de départ.
    #[allow(clippy::type_complexity)]
    fn inverse_start(
        &self,
        b1: Beta,
        b2: Beta,
        lam12: f64,
        slam12: f64,
        clam12: f64,
    ) -> (f64, f64, f64, f64, f64, f64) {
        let (sbet1, cbet1, sbet2, cbet2) = (b1.s, b1.c, b2.s, b2.c);
        let mut sig12 = -1.0;
        let (mut salp2, mut calp2, mut dnm) = (f64::NAN, f64::NAN, f64::NAN);
        let sbet12 = sbet2 * cbet1 - cbet2 * sbet1;
        let cbet12 = cbet2 * cbet1 + sbet2 * sbet1;
        let sbet12a = sbet2 * cbet1 + cbet2 * sbet1;
        let shortline = cbet12 >= 0.0 && sbet12 < 0.5 && cbet2 * lam12 < 0.5;
        let (mut somg12, mut comg12);
        if shortline {
            let mut sbetm2 = (sbet1 + sbet2) * (sbet1 + sbet2);
            sbetm2 /= sbetm2 + (cbet1 + cbet2) * (cbet1 + cbet2);
            dnm = libm::sqrt(1.0 + self.ep2 * sbetm2);
            let omg12 = lam12 / (self.f1 * dnm);
            somg12 = libm::sin(omg12);
            comg12 = libm::cos(omg12);
        } else {
            somg12 = slam12;
            comg12 = clam12;
        }
        let mut salp1 = cbet2 * somg12;
        let mut calp1 = if comg12 >= 0.0 {
            sbet12 + cbet2 * sbet1 * somg12 * somg12 / (1.0 + comg12)
        } else {
            sbet12a - cbet2 * sbet1 * somg12 * somg12 / (1.0 - comg12)
        };
        let ssig12 = libm::hypot(salp1, calp1);
        let csig12 = sbet1 * sbet2 + cbet1 * cbet2 * comg12;
        if shortline && ssig12 < self.etol2 {
            salp2 = cbet1 * somg12;
            calp2 = sbet12
                - cbet1
                    * sbet2
                    * if comg12 >= 0.0 {
                        somg12 * somg12 / (1.0 + comg12)
                    } else {
                        1.0 - comg12
                    };
            (salp2, calp2) = norm(salp2, calp2);
            sig12 = libm::atan2(ssig12, csig12);
        } else if self.n.abs() > 0.1
            || csig12 >= 0.0
            || ssig12 >= 6.0 * self.n.abs() * PI * cbet1 * cbet1
        {
            // Rien : la formule ci-dessus est un bon départ.
        } else {
            // Points quasi antipodaux : départ par l'astroïde (ellipsoïde aplati, f > 0).
            let lam12x = libm::atan2(-slam12, -clam12);
            let k2 = sbet1 * sbet1 * self.ep2;
            let eps = k2 / (2.0 * (1.0 + libm::sqrt(1.0 + k2)) + k2);
            let lamscale = self.f * cbet1 * self.a3f(eps) * PI;
            let betscale = lamscale * cbet1;
            let x = lam12x / lamscale;
            let y = sbet12a / betscale;
            if y > -self.tol1 && x > -1.0 - self.xthresh {
                salp1 = (-x).min(1.0);
                calp1 = -libm::sqrt(1.0 - salp1 * salp1);
            } else {
                let k = astroid(x, y);
                let omg12a = lamscale * (-x * k / (1.0 + k));
                somg12 = libm::sin(omg12a);
                comg12 = -libm::cos(omg12a);
                salp1 = cbet2 * somg12;
                calp1 = sbet12a - cbet2 * sbet1 * somg12 * somg12 / (1.0 - comg12);
            }
        }
        if salp1 > 0.0 {
            (salp1, calp1) = norm(salp1, calp1);
        } else {
            salp1 = 1.0;
            calp1 = 0.0;
        }
        (sig12, salp1, calp1, salp2, calp2, dnm)
    }

    /// λ₁₂(α₁) − λ₁₂ visé, et sa dérivée (Karney 2013, éq. 49 et 57).
    #[allow(clippy::too_many_arguments)]
    fn lambda12(
        &self,
        b1: Beta,
        b2: Beta,
        salp1: f64,
        mut calp1: f64,
        slam120: f64,
        clam120: f64,
        diffp: bool,
    ) -> Lambda {
        let (sbet1, cbet1, sbet2, cbet2) = (b1.s, b1.c, b2.s, b2.c);
        if sbet1 == 0.0 && calp1 == 0.0 {
            calp1 = -self.tiny;
        }
        let salp0 = salp1 * cbet1;
        let calp0 = libm::hypot(calp1, salp1 * sbet1);
        let (ssig1, csig1) = norm(sbet1, calp1 * cbet1);
        let (somg1, comg1) = (salp0 * sbet1, calp1 * cbet1);
        let salp2 = if cbet2 != cbet1 { salp0 / cbet2 } else { salp1 };
        let calp2 = if cbet2 != cbet1 || sbet2.abs() != -sbet1 {
            libm::sqrt(
                (calp1 * cbet1) * (calp1 * cbet1)
                    + if cbet1 < -sbet1 {
                        (cbet2 - cbet1) * (cbet1 + cbet2)
                    } else {
                        (sbet1 - sbet2) * (sbet1 + sbet2)
                    },
            ) / cbet2
        } else {
            calp1.abs()
        };
        let (ssig2, csig2) = norm(sbet2, calp2 * cbet2);
        let (somg2, comg2) = (salp0 * sbet2, calp2 * cbet2);
        let sig12 = libm::atan2(
            (csig1 * ssig2 - ssig1 * csig2).max(0.0),
            csig1 * csig2 + ssig1 * ssig2,
        );
        let somg12 = (comg1 * somg2 - somg1 * comg2).max(0.0);
        let comg12 = comg1 * comg2 + somg1 * somg2;
        let eta = libm::atan2(
            somg12 * clam120 - comg12 * slam120,
            comg12 * clam120 + somg12 * slam120,
        );
        let k2 = calp0 * calp0 * self.ep2;
        let eps = k2 / (2.0 * (1.0 + libm::sqrt(1.0 + k2)) + k2);
        let c3a = self.c3f(eps);
        let b312 = sin_cos_series(ssig2, csig2, &c3a) - sin_cos_series(ssig1, csig1, &c3a);
        let domg12 = -self.f * self.a3f(eps) * salp0 * (sig12 + b312);
        let v = eta + domg12;
        let dv = if !diffp {
            f64::NAN
        } else if calp2 == 0.0 {
            -2.0 * self.f1 * b1.dn / sbet1
        } else {
            let (_, m12b) = self.lengths(eps, sig12, (ssig1, csig1, b1.dn), (ssig2, csig2, b2.dn));
            m12b * self.f1 / (calp2 * cbet2)
        };
        Lambda {
            v,
            salp2,
            calp2,
            sig12,
            sig1: (ssig1, csig1),
            sig2: (ssig2, csig2),
            eps,
            dv,
        }
    }

    // La comparaison niée de la boucle de Newton est voulue : un NaN y sort de la boucle.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    fn inverse(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> Inverse {
        let (lon12, lon12s) = ang_diff(lon1, lon2);
        let mut lonsign: f64 = if lon12.is_sign_negative() { -1.0 } else { 1.0 };
        let lon12 = lonsign * ang_round(lon12);
        let lon12s = ang_round((180.0 - lon12) - lonsign * lon12s);
        let lam12 = lon12.to_radians();
        let (slam12, clam12) = if lon12 > 90.0 {
            let (s, c) = sincosd(lon12s);
            (s, -c)
        } else {
            sincosd(lon12)
        };
        let (mut lat1, mut lat2) = (ang_round(lat1), ang_round(lat2));
        let swapp: f64 = if lat1.abs() < lat2.abs() { -1.0 } else { 1.0 };
        if swapp < 0.0 {
            lonsign = -lonsign;
            std::mem::swap(&mut lat1, &mut lat2);
        }
        let latsign: f64 = if lat1.is_sign_negative() { 1.0 } else { -1.0 };
        lat1 *= latsign;
        lat2 *= latsign;
        let b1 = self.beta(lat1);
        let mut b2 = self.beta(lat2);
        if b1.c < -b1.s {
            if b2.c == b1.c {
                b2.s = b1.s.copysign(b2.s);
            }
        } else if b2.s.abs() == -b1.s {
            b2.c = b1.c;
        }

        let (mut salp1, mut calp1, mut salp2, mut calp2) = (f64::NAN, f64::NAN, f64::NAN, f64::NAN);
        let mut s12x = f64::NAN;
        let mut meridian = lat1 == -90.0 || slam12 == 0.0;
        if meridian {
            // Le long d'un méridien : α₁ = λ₁₂, α₂ = 0.
            calp1 = clam12;
            salp1 = slam12;
            calp2 = 1.0;
            salp2 = 0.0;
            let (ssig1, csig1) = (b1.s, calp1 * b1.c);
            let (ssig2, csig2) = (b2.s, calp2 * b2.c);
            let sig12 = libm::atan2(
                (csig1 * ssig2 - ssig1 * csig2).max(0.0),
                csig1 * csig2 + ssig1 * ssig2,
            );
            let (s12b, m12b) =
                self.lengths(self.n, sig12, (ssig1, csig1, b1.dn), (ssig2, csig2, b2.dn));
            if sig12 < 1.0 || m12b >= 0.0 {
                let null =
                    sig12 < 3.0 * self.tiny || (sig12 < self.tol0 && (s12b < 0.0 || m12b < 0.0));
                s12x = if null { 0.0 } else { s12b * self.b };
            } else {
                meridian = false;
            }
        }
        if !meridian && b1.s == 0.0 && (self.f <= 0.0 || lon12s >= self.f * 180.0) {
            // Le long de l'équateur.
            calp1 = 0.0;
            calp2 = 0.0;
            salp1 = 1.0;
            salp2 = 1.0;
            s12x = self.a * lam12;
        } else if !meridian {
            let (sig12, s1, c1, s2, c2, dnm) = self.inverse_start(b1, b2, lam12, slam12, clam12);
            salp1 = s1;
            calp1 = c1;
            if sig12 >= 0.0 {
                // Géodésique courte : la solution sphérique suffit.
                salp2 = s2;
                calp2 = c2;
                s12x = sig12 * self.b * dnm;
            } else {
                let mut numit = 0;
                let (mut tripn, mut tripb) = (false, false);
                let (mut salp1a, mut calp1a) = (self.tiny, 1.0);
                let (mut salp1b, mut calp1b) = (self.tiny, -1.0);
                let lambda = loop {
                    let l = self.lambda12(b1, b2, salp1, calp1, slam12, clam12, numit < MAXIT1);
                    let v = l.v;
                    if tripb
                        || !(v.abs() >= if tripn { 8.0 } else { 1.0 } * self.tol0)
                        || numit >= MAXIT2
                    {
                        break l;
                    }
                    if v > 0.0 && (numit > MAXIT1 || calp1 / salp1 > calp1b / salp1b) {
                        salp1b = salp1;
                        calp1b = calp1;
                    } else if v < 0.0 && (numit > MAXIT1 || calp1 / salp1 < calp1a / salp1a) {
                        salp1a = salp1;
                        calp1a = calp1;
                    }
                    if numit < MAXIT1 && l.dv > 0.0 {
                        let dalp1 = -v / l.dv;
                        let (sdalp1, cdalp1) = (libm::sin(dalp1), libm::cos(dalp1));
                        let nsalp1 = salp1 * cdalp1 + calp1 * sdalp1;
                        if nsalp1 > 0.0 && dalp1.abs() < PI {
                            calp1 = calp1 * cdalp1 - salp1 * sdalp1;
                            salp1 = nsalp1;
                            (salp1, calp1) = norm(salp1, calp1);
                            tripn = v.abs() <= 16.0 * self.tol0;
                            // Pas de la boucle, après les tests (comme GeographicLib).
                            numit += 1;
                            continue;
                        }
                    }
                    // Newton s'égare : bissection dans l'intervalle encadrant.
                    (salp1, calp1) = norm((salp1a + salp1b) / 2.0, (calp1a + calp1b) / 2.0);
                    tripn = false;
                    tripb = (salp1a - salp1).abs() + (calp1a - calp1) < self.tolb
                        || (salp1 - salp1b).abs() + (calp1 - calp1b) < self.tolb;
                    numit += 1;
                };
                salp2 = lambda.salp2;
                calp2 = lambda.calp2;
                let (s12b, _) = self.lengths(
                    lambda.eps,
                    lambda.sig12,
                    (lambda.sig1.0, lambda.sig1.1, b1.dn),
                    (lambda.sig2.0, lambda.sig2.1, b2.dn),
                );
                s12x = s12b * self.b;
            }
        } else {
            // Méridien : α déjà posés.
        }
        if swapp < 0.0 {
            std::mem::swap(&mut salp1, &mut salp2);
            std::mem::swap(&mut calp1, &mut calp2);
        }
        salp1 *= swapp * lonsign;
        calp1 *= swapp * latsign;
        salp2 *= swapp * lonsign;
        calp2 *= swapp * latsign;
        Inverse {
            distance: 0.0 + s12x,
            azi1: atan2d(salp1, calp1),
            azi2: atan2d(salp2, calp2),
        }
    }

    // -- Problème direct ----------------------------------------------------------------------

    fn direct(&self, lat1: f64, lon1: f64, azi1: f64, s12: f64) -> Direct {
        let azi1 = ang_normalize(azi1);
        let (salp1, calp1) = sincosd(ang_round(azi1));
        let b1 = self.beta(ang_round(lat1));
        let (sbet1, cbet1) = (b1.s, b1.c);
        let salp0 = salp1 * cbet1;
        let calp0 = libm::hypot(calp1, salp1 * sbet1);
        let csig1_raw = if sbet1 != 0.0 || calp1 != 0.0 {
            cbet1 * calp1
        } else {
            1.0
        };
        let (ssig1, csig1) = norm(sbet1, csig1_raw);
        let (somg1, comg1) = (salp0 * sbet1, csig1_raw);
        let k2 = calp0 * calp0 * self.ep2;
        let eps = k2 / (2.0 * (1.0 + libm::sqrt(1.0 + k2)) + k2);
        let a1m1 = a1m1f(eps);
        let c1a = c_series(&COEFF_C1, eps);
        let b11 = sin_cos_series(ssig1, csig1, &c1a);
        let (s, c) = (libm::sin(b11), libm::cos(b11));
        let stau1 = ssig1 * c + csig1 * s;
        let ctau1 = csig1 * c - ssig1 * s;
        let c1pa = c_series(&COEFF_C1P, eps);
        let a3c = -self.f * salp0 * self.a3f(eps);
        let c3a = self.c3f(eps);
        let b31 = sin_cos_series(ssig1, csig1, &c3a);

        let tau12 = s12 / (self.b * (1.0 + a1m1));
        let (s, c) = (libm::sin(tau12), libm::cos(tau12));
        let b12 = -sin_cos_series(stau1 * c + ctau1 * s, ctau1 * c - stau1 * s, &c1pa);
        let sig12 = tau12 - (b12 - b11);
        let (ssig12, csig12) = (libm::sin(sig12), libm::cos(sig12));
        let ssig2 = ssig1 * csig12 + csig1 * ssig12;
        let mut csig2 = csig1 * csig12 - ssig1 * ssig12;
        let sbet2 = calp0 * ssig2;
        let mut cbet2 = libm::hypot(salp0, calp0 * csig2);
        if cbet2 == 0.0 {
            cbet2 = self.tiny;
            csig2 = self.tiny;
        }
        let salp2 = salp0;
        let calp2 = calp0 * csig2;
        let (somg2, comg2) = (salp0 * ssig2, csig2);
        let omg12 = libm::atan2(somg2 * comg1 - comg2 * somg1, comg2 * comg1 + somg2 * somg1);
        let lam12 = omg12 + a3c * (sig12 + (sin_cos_series(ssig2, csig2, &c3a) - b31));
        let lon12 = lam12.to_degrees();
        Direct {
            lat: atan2d(sbet2, self.f1 * cbet2),
            lon: ang_normalize(ang_normalize(lon1) + ang_normalize(lon12)),
            azi: atan2d(salp2, calp2),
        }
    }
}

/// Résultat du problème direct : point d'arrivée et azimut de marche, en degrés.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Direct {
    pub lat: f64,
    pub lon: f64,
    /// Azimut de marche à l'arrivée, dans [-180, 180].
    pub azi: f64,
}

/// Problème inverse sur le WGS84 : distance et azimuts entre deux points (degrés).
pub fn inverse(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> Inverse {
    wgs84().inverse(lat1, lon1, lat2, lon2)
}

/// Problème direct sur le WGS84 : point atteint depuis (lat1, lon1) à l'azimut `azi1` (degrés)
/// après `s12` mètres.
pub fn direct(lat1: f64, lon1: f64, azi1: f64, s12: f64) -> Direct {
    wgs84().direct(lat1, lon1, azi1, s12)
}

// -- Recoupement ----------------------------------------------------------------------------

/// Angle de recoupement au-dessous duquel deux visées sont tenues pour parallèles, en degrés
/// (décision du propriétaire, 2026-11-05).
pub const PARALLEL_LIMIT_DEG: f64 = 1.0;
/// Écart entre les deux extrémités au-dessous duquel le recoupement est atteint, en mètres.
const MEET_TOLERANCE: f64 = 1e-6;
/// Écart toléré entre la longueur d'une visée et la distance la plus courte au point, en mètres.
const SHORTEST_TOLERANCE: f64 = 1e-3;
/// Itérations de Newton du recoupement.
const MEET_ITERATIONS: usize = 40;
/// Rayon moyen de la Terre, pour le départ sphérique seulement, en mètres.
const MEAN_RADIUS: f64 = 6_371_008.8;

/// Issue d'un recoupement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Intersection {
    /// Les visées se coupent en avant des deux stations.
    Found {
        lat: f64,
        lon: f64,
        /// Distance géodésique de la première station au point, en mètres.
        s1: f64,
        /// Distance géodésique de la seconde station au point, en mètres.
        s2: f64,
    },
    /// Visées sous un angle inférieur à [`PARALLEL_LIMIT_DEG`], ou portées par la même
    /// géodésique : l'angle de recoupement, en degrés.
    Parallel { angle: f64 },
    /// Les visées ne se coupent qu'en arrière d'une des stations au moins, ou au-delà de
    /// l'antipode d'une station (la visée n'y est plus la plus courte).
    Behind,
    /// Le calcul ne converge pas.
    NoConvergence,
}

type V3 = [f64; 3];

fn cross(u: V3, v: V3) -> V3 {
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

fn dot(u: V3, v: V3) -> f64 {
    u[0] * v[0] + u[1] * v[1] + u[2] * v[2]
}

/// Position et direction de visée d'une station, sur la sphère unité.
fn on_sphere(lat: f64, lon: f64, azi: f64) -> (V3, V3) {
    let (sphi, cphi) = sincosd(lat);
    let (slam, clam) = sincosd(lon);
    let (salp, calp) = sincosd(azi);
    let p = [cphi * clam, cphi * slam, sphi];
    let east = [-slam, clam, 0.0];
    let north = [-sphi * clam, -sphi * slam, cphi];
    let d = [
        salp * east[0] + calp * north[0],
        salp * east[1] + calp * north[1],
        salp * east[2] + calp * north[2],
    ];
    (p, d)
}

/// Angle aigu entre deux directions données par leurs azimuts, en degrés, dans [0, 90].
fn crossing_angle(azi1: f64, azi2: f64) -> f64 {
    let d = ang_diff(azi1, azi2).0.abs();
    d.min(180.0 - d)
}

/// Recoupement de deux visées géodésiques (Design Notes de l'entrée 3 de l'epic 11) : départ
/// sphérique par les plans des grands cercles, puis Newton sur les deux longueurs (s1, s2) ; à
/// chaque pas, problème direct depuis chaque station, problème inverse entre les deux
/// extrémités, système 2 × 2 dans le plan tangent à la première. Seule compte l'intersection
/// en avant des deux stations (s1 > 0 et s2 > 0), la plus proche.
pub fn intersection(
    lat1: f64,
    lon1: f64,
    azi1: f64,
    lat2: f64,
    lon2: f64,
    azi2: f64,
) -> Intersection {
    let (p1, d1) = on_sphere(lat1, lon1, azi1);
    let (p2, d2) = on_sphere(lat2, lon2, azi2);
    let n1 = cross(p1, d1);
    let n2 = cross(p2, d2);
    let c = cross(n1, n2);
    let sin_angle = libm::sqrt(dot(c, c));
    let angle = libm::atan2(sin_angle, dot(n1, n2).abs()).to_degrees();
    if angle < PARALLEL_LIMIT_DEG {
        return Intersection::Parallel { angle };
    }
    let mut x = [c[0] / sin_angle, c[1] / sin_angle, c[2] / sin_angle];
    let mut t1 = libm::atan2(dot(x, d1), dot(x, p1));
    if t1 < 0.0 {
        x = [-x[0], -x[1], -x[2]];
        t1 = libm::atan2(dot(x, d1), dot(x, p1));
    }
    let t2 = libm::atan2(dot(x, d2), dot(x, p2));
    if t1 <= 0.0 || t2 <= 0.0 {
        return Intersection::Behind;
    }
    let (mut s1, mut s2) = (t1 * MEAN_RADIUS, t2 * MEAN_RADIUS);
    for _ in 0..MEET_ITERATIONS {
        let e1 = direct(lat1, lon1, azi1, s1);
        let e2 = direct(lat2, lon2, azi2, s2);
        let gap = inverse(e1.lat, e1.lon, e2.lat, e2.lon);
        // Direction de la seconde visée ramenée au plan tangent de la première extrémité :
        // l'azimut de la géodésique qui les joint tourne de azi2 − azi1 entre les deux.
        let turn = ang_diff(gap.azi1, gap.azi2).0;
        let a2 = e2.azi - turn;
        if gap.distance <= MEET_TOLERANCE {
            let angle = crossing_angle(e1.azi, a2);
            if angle < PARALLEL_LIMIT_DEG {
                return Intersection::Parallel { angle };
            }
            // Au-delà de l'antipode d'une station, la visée n'est plus la géodésique la plus
            // courte vers le point : ce recoupement-là n'est pas « en avant », il ne compte pas.
            let shortest = |lat: f64, lon: f64, s: f64| {
                (inverse(lat, lon, e1.lat, e1.lon).distance - s).abs() <= SHORTEST_TOLERANCE
            };
            if !(shortest(lat1, lon1, s1) && shortest(lat2, lon2, s2)) {
                return Intersection::Behind;
            }
            return Intersection::Found {
                lat: e1.lat,
                lon: e1.lon,
                s1,
                s2,
            };
        }
        let (dsin, dcos) = sincosd(gap.azi1);
        let (dx, dy) = (gap.distance * dsin, gap.distance * dcos);
        let (u1x, u1y) = sincosd(e1.azi);
        let (u2x, u2y) = sincosd(a2);
        // ds1 · u1 − ds2 · u2 = D.
        let det = u2x * u1y - u1x * u2y;
        if det.abs() < 1e-12 {
            return Intersection::NoConvergence;
        }
        let ds1 = (u2x * dy - dx * u2y) / det;
        let ds2 = (u1x * dy - u1y * dx) / det;
        s1 += ds1;
        s2 += ds2;
        if !(s1 > 0.0 && s2 > 0.0) {
            return Intersection::Behind;
        }
    }
    Intersection::NoConvergence
}

/// Jeu de référence partagé par les tests des commandes `geo.*`.
#[cfg(test)]
pub(crate) mod reference {
    /// Une géodésique GeodTest : (lat1, lon1, azi1, lat2, lon2, azi2, s12).
    pub type Geodesic = (f64, f64, f64, f64, f64, f64, f64);
    /// Un cas GeodSolve : (lat1, lon1, lat2, lon2, azi1, azi2, s12, tolérance des azimuts en
    /// degrés, tolérance de la distance en mètres) ; un azimut NaN n'est pas publié.
    pub type Solve = (f64, f64, f64, f64, f64, f64, f64, f64, f64);

    /// GeographicLib 2.0, `geographiclib/test/test_geodesic.py` (C. F. F. Karney, licence MIT,
    /// https://pypi.org/project/geographiclib/2.0/) : les 20 géodésiques de `testcases`, tirées
    /// du jeu d'essai GeodTest de Karney ; azi2 est l'azimut de marche à l'arrivée.
    #[rustfmt::skip]
    pub const GEODTEST: [Geodesic; 20] = [
        (35.60777, -139.44815, 111.09874842956033, -11.17491, -69.95921, 129.28927088970877, 8935244.56048183),
        (55.52454, 106.05087, 22.0200598809828, 77.03196, 197.18234, 109.11204111067151, 4105086.171392441),
        (-21.97856, 142.59065, -32.44456876433189, 41.84138, 98.56635, -41.84359951440466, 8394328.894657671),
        (-66.99028, 112.2363, 173.73491240878403, -12.70631, 285.90344, 2.512956620913668, 11150344.231208025),
        (-17.42761, 173.34268, -159.03355766119293, -15.84784, 5.93557, -20.78748465153699, 16076603.163118068),
        (32.84994, 48.28919, 150.492927788122, -56.28556, 202.29132, 48.11344939981676, 16727068.943816446),
        (6.96833, 52.74123, 92.58158538631771, -7.39675, 206.17291, 90.72169216592391, 17102477.249695837),
        (-50.56724, -16.30485, -105.43967990759016, -33.56571, -94.97412, -47.34854783565033, 6455670.511866869),
        (-58.93002, -8.90775, 140.96539790250068, -8.91104, 133.13503, 19.255429433416598, 11756066.021986462),
        (-68.82867, -74.28391, 93.77434776311487, -50.63005, -8.36685, 34.65564085411343, 3956936.926063544),
        (-10.62672, -32.0898, -86.42671328674776, 5.883, -134.31681, -80.47378097103487, 11470869.386456301),
        (-21.76221, 166.90563, 29.31942120693643, 48.72884, 213.97627, 43.50867194641017, 9098627.398655491),
        (-19.79938, -174.47484, 71.16727578017154, -11.99349, -154.35109, 65.58909977519923, 2319004.860116939),
        (-11.95887, -116.94513, 92.71261983045255, 4.57352, 7.16501, 78.64960934409585, 13834722.580140138),
        (-87.85331, 85.66836, -65.12031304024275, 66.48646, 16.09921, -4.888658719272296, 17286615.314714465),
        (1.74708, 128.32011, -101.58484363117385, -11.16617, 11.87109, -86.32579329643748, 12942901.124134742),
        (-25.72959, -144.90758, -153.6474686931172, -57.70581, -269.17879, -48.34398315887649, 9413446.745245311),
        (-41.22777, 122.32875, 14.285113402275739, -7.57291, 130.37946, 10.805303085187369, 3812686.035106021),
        (11.01307, 138.25278, 79.43682622782374, 6.62726, 247.05981, 103.70809021552266, 11911190.819018409),
        (-29.47124, 95.14681, -163.77913044168838, -27.46601, -69.15955, -15.90933594555497, 13487015.838114548),
    ];

    /// Même source, tests `GeodSolve` : géodésiques quasi antipodales et cas limites, avec la
    /// tolérance publiée (coordonnées publiées ramenées au flottant le plus proche).
    #[rustfmt::skip]
    pub const GEODSOLVE: [Solve; 10] = [
        // GeodSolve0 : New York – Paris.
        (40.6, -73.8, 49.01666667, 2.55, 53.47022, 111.59367, 5853226.0, 0.5e-5, 0.5),
        // GeodSolve6, 9, 10, 11 : quasi antipodes, les cas où le départ par l'astroïde compte.
        (88.202499451857, 0.0, -88.202499451857, 179.98102203299285, f64::NAN, f64::NAN, 20003898.214, 0.0, 0.5e-3),
        (89.333123580033, 0.0, -89.333123580033, 179.99295812360148, f64::NAN, f64::NAN, 20003926.881, 0.0, 0.5e-3),
        (56.320923501171, 0.0, -56.320923501171, 179.6647476717729, f64::NAN, f64::NAN, 19993558.287, 0.0, 0.5e-3),
        (52.784459512564, 0.0, -52.78445951256399, 179.6344074649438, f64::NAN, f64::NAN, 19991596.095, 0.0, 0.5e-3),
        (48.522876735459, 0.0, -48.522876735458986, 179.59972045622308, f64::NAN, f64::NAN, 19989144.774, 0.0, 0.5e-3),
        // GeodSolve33 : sur l'équateur, presque à l'antipode.
        (0.0, 0.0, 0.0, 179.0, 90.0, 90.0, 19926189.0, 0.5e-5, 0.5),
        // GeodSolve74 : géodésique très courte.
        (54.1589, 15.3872, 54.1591, 15.3877, 55.723110355, 55.723515675, 39.527686385, 5e-9, 5e-9),
        // GeodSolve76 : Wellington – Salamanque, échec classique de Vincenty.
        (-(41.0 + 19.0 / 60.0), 174.0 + 49.0 / 60.0, 40.0 + 58.0 / 60.0, -(5.0 + 30.0 / 60.0), 160.39137649664, 19.50042925176, 19960543.857179, 0.5e-11, 0.5e-6),
        // GeodSolve78 : là où le calculateur du NGS ne converge pas.
        (27.2, 0.0, -27.1, 179.5, 45.82468716758, 134.2277653267, 19974354.765767, 0.5e-11, 0.5e-6),
    ];
}

#[cfg(test)]
mod tests {
    use super::reference::{GEODSOLVE, GEODTEST};
    use super::*;

    #[track_caller]
    fn assert_angle(got: f64, expected: f64, tolerance: f64) {
        let d = ang_diff(expected, got).0.abs();
        assert!(
            d <= tolerance,
            "attendu {expected}, trouvé {got} (écart {d})"
        );
    }

    #[test]
    fn the_inverse_finds_the_geodtest_references() {
        for (lat1, lon1, azi1, lat2, lon2, azi2, s12) in GEODTEST {
            let inv = inverse(lat1, lon1, lat2, lon2);
            assert!(
                (inv.distance - s12).abs() <= 1e-8,
                "{lat1} {lon1} : {} / {s12}",
                inv.distance
            );
            assert_angle(inv.azi1, azi1, 1e-12);
            assert_angle(inv.azi2, azi2, 1e-12);
        }
    }

    #[test]
    fn the_direct_finds_the_geodtest_references() {
        for (lat1, lon1, azi1, lat2, lon2, azi2, s12) in GEODTEST {
            let d = direct(lat1, lon1, azi1, s12);
            assert!(
                (d.lat - lat2).abs() <= 1e-12,
                "{lat1} {lon1} : {} / {lat2}",
                d.lat
            );
            assert_angle(d.lon, lon2, 1e-12);
            assert_angle(d.azi, azi2, 1e-12);
        }
    }

    #[test]
    fn the_geodsolve_cases_are_found() {
        for (lat1, lon1, lat2, lon2, azi1, azi2, s12, ta, ts) in GEODSOLVE {
            let inv = inverse(lat1, lon1, lat2, lon2);
            assert!(
                (inv.distance - s12).abs() <= ts,
                "{lat1} {lon1} : {} / {s12}",
                inv.distance
            );
            if !azi1.is_nan() {
                assert_angle(inv.azi1, azi1, ta);
                assert_angle(inv.azi2, azi2, ta);
            }
        }
        // GeodSolve1 : le direct de New York vers Paris.
        let d = direct(40.63972222, -73.77888889, 53.5, 5850e3);
        assert!((d.lat - 49.01467).abs() <= 0.5e-5 && (d.lon - 2.56106).abs() <= 0.5e-5);
        assert_angle(d.azi, 111.62947, 0.5e-5);
        // GeodSolve5 : arrivée au pôle.
        let d = direct(0.01777745589997, 30.0, 0.0, 10e6);
        assert!((d.lat - 90.0).abs() <= 0.5e-5, "{}", d.lat);
    }

    #[test]
    fn coincident_points_and_meridians_are_handled() {
        let inv = inverse(48.8, 2.3, 48.8, 2.3);
        assert_eq!(inv.distance, 0.0);
        // Le long d'un méridien, vers le nord puis vers le sud.
        let inv = inverse(10.0, 5.0, 20.0, 5.0);
        assert_angle(inv.azi1, 0.0, 1e-12);
        let inv = inverse(20.0, 5.0, 10.0, 5.0);
        assert_angle(inv.azi1, 180.0, 1e-12);
        // D'un pôle à l'autre : la moitié du périmètre méridien (20 003 931,4586 m).
        let inv = inverse(90.0, 0.0, -90.0, 0.0);
        assert!(
            (inv.distance - 20_003_931.458_6).abs() < 1e-3,
            "{}",
            inv.distance
        );
    }

    #[test]
    fn an_intersection_is_found_and_agrees_with_the_inverse() {
        // Deux stations à 10 km l'une de l'autre, visées à 45° et 315°.
        let Intersection::Found { lat, lon, s1, s2 } =
            intersection(48.0, 2.0, 45.0, 48.0, 2.2, 315.0)
        else {
            panic!("pas de recoupement");
        };
        let a = inverse(48.0, 2.0, lat, lon);
        let b = inverse(48.0, 2.2, lat, lon);
        assert!((a.distance - s1).abs() < 1e-5 && (b.distance - s2).abs() < 1e-5);
        assert_angle(a.azi1, 45.0, 1e-9);
        assert_angle(b.azi1, -45.0, 1e-9);
    }
}
