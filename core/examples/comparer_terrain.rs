//! Instrument de la comparaison des commandes de terrain du noyau à QGIS (et GRASS).
//!
//! Deux modes, sans QGIS :
//!
//! - `points <mnt>` imprime en JSON les 3 observateurs et les 30 cibles (10 par observateur, de
//!   1 à 10 km), tous au centre d'une cellule valide du MNT, choisis de façon déterministe ;
//! - `comparer <dossier>` lit dans le dossier de travail le MNT (`mnt.tif`), les points
//!   (`points.json`), les sorties du noyau et de QGIS déjà normalisées par le script
//!   `tests/comparaison-terrain/comparer.ps1`, et les temps (`temps.json`) ; il compare cellule
//!   par cellule la pente, l'exposition (écart sur le cercle) et l'ombrage, classe chaque
//!   désaccord de masque (bord, trou, cellule plate, ailleurs), calcule les lignes de vue du
//!   noyau par le registre, lit `r.viewshed` à la cellule de chaque cible, et imprime les
//!   tableaux d'écarts en Markdown.
//!
//! Il n'utilise que la porte raster du noyau (`Dem::open`), la fenêtre de Horn (`horn`) et le registre
//! (`Registry::standard().call`) : aucune dépendance de plus, ni GDAL, ni GRASS. Le mode
//! d'emploi est dans `tests/comparaison-terrain/LISEZMOI.md`.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use cassini_core::dem::Dem;
use cassini_core::horn::horn_gradient;
use cassini_core::registry::Registry;
use serde_json::{json, Value};

/// Hauteur de l'œil, en mètres : la même des deux côtés.
const OBSERVER_HEIGHT: f64 = 1.6;
/// Hauteur de la cible, en mètres.
const TARGET_HEIGHT: f64 = 0.0;
/// Coefficient de réfraction, passé aussi à GRASS par `refraction_coeff`.
const REFRACTION: f64 = 0.13;
/// Rayon des champs de vision, en mètres.
const RADIUS: f64 = 10_000.0;
/// Rayon de recherche du point haut et du point bas autour du centre, en mètres.
const SEARCH: f64 = 4_000.0;
/// Nombre de cibles par observateur, à 1, 2, … 10 km.
const TARGETS_PER_OBSERVER: u32 = 10;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = match args.as_slice() {
        [mode, mnt] if mode == "points" => points(Path::new(mnt)),
        [mode, dir] if mode == "comparer" => compare(Path::new(dir)),
        _ => Err("Usage : comparer_terrain points <mnt.tif> | comparer <dossier>".to_string()),
    };
    match outcome {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn open(path: &Path) -> Result<Dem, String> {
    Dem::open(path).map_err(|e| format!("{} : {}", path.display(), e.message))
}

// ---------------------------------------------------------------------------------------------
// Grilles

/// Centre de la cellule (`col`, `row`).
fn centre(dem: &Dem, col: u32, row: u32) -> (f64, f64) {
    (
        dem.origin_x + (f64::from(col) + 0.5) * dem.cell_size_x,
        dem.origin_y - (f64::from(row) + 0.5) * dem.cell_size_y,
    )
}

fn close(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

/// Refuse une grille qui n'est pas exactement celle du MNT (sortie de QGIS).
fn same_grid(mnt: &Dem, other: &Dem, name: &str) -> Result<(), String> {
    let tol = mnt.cell_size_x * 1e-6;
    let same = mnt.width == other.width
        && mnt.height == other.height
        && mnt.epsg == other.epsg
        && close(mnt.cell_size_x, other.cell_size_x, tol)
        && close(mnt.cell_size_y, other.cell_size_y, tol)
        && close(mnt.origin_x, other.origin_x, mnt.cell_size_x * 1e-3)
        && close(mnt.origin_y, other.origin_y, mnt.cell_size_y * 1e-3);
    if same {
        Ok(())
    } else {
        Err(format!(
            "La grille de {name} ({} × {}, origine {}, {}, cellule {} × {}, EPSG:{}) n'est pas celle du MNT ({} × {}, origine {}, {}, cellule {} × {}, EPSG:{}).",
            other.width, other.height, other.origin_x, other.origin_y, other.cell_size_x,
            other.cell_size_y, other.epsg, mnt.width, mnt.height, mnt.origin_x, mnt.origin_y,
            mnt.cell_size_x, mnt.cell_size_y, mnt.epsg
        ))
    }
}

/// Décalage en cellules d'une sous-grille alignée sur le MNT (champ de vision du noyau, rogné au
/// carré du cercle). Refuse une cellule différente ou une origine qui ne tombe pas sur un coin.
fn offset_in(mnt: &Dem, sub: &Dem, name: &str) -> Result<(i64, i64), String> {
    let tol = mnt.cell_size_x * 1e-6;
    if mnt.epsg != sub.epsg
        || !close(mnt.cell_size_x, sub.cell_size_x, tol)
        || !close(mnt.cell_size_y, sub.cell_size_y, tol)
    {
        return Err(format!(
            "La cellule ou le système de {name} diffère de ceux du MNT."
        ));
    }
    let fc = (sub.origin_x - mnt.origin_x) / mnt.cell_size_x;
    let fr = (mnt.origin_y - sub.origin_y) / mnt.cell_size_y;
    let (dc, dr) = (fc.round(), fr.round());
    if !close(fc, dc, 1e-3) || !close(fr, dr, 1e-3) {
        return Err(format!(
            "L'origine de {name} n'est pas alignée sur la grille du MNT (décalage {fc}, {fr} cellules)."
        ));
    }
    Ok((dc as i64, dr as i64))
}

/// Valeur d'une sous-grille décalée de (`dc`, `dr`), lue à la cellule (`col`, `row`) du MNT.
fn value_at(sub: &Dem, (dc, dr): (i64, i64), col: u32, row: u32) -> Option<f32> {
    let (c, r) = (i64::from(col) - dc, i64::from(row) - dr);
    if c < 0 || r < 0 || c >= i64::from(sub.width) || r >= i64::from(sub.height) {
        return None;
    }
    sub.value(c as u32, r as u32)
}

// ---------------------------------------------------------------------------------------------
// Points

#[derive(Debug, Clone, PartialEq)]
struct Site {
    name: String,
    col: u32,
    row: u32,
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct Target {
    observer: String,
    site: Site,
    distance: f64,
}

fn site(dem: &Dem, name: String, col: u32, row: u32) -> Site {
    let (x, y) = centre(dem, col, row);
    Site {
        name,
        col,
        row,
        x,
        y,
    }
}

/// Cellule valide la plus proche de (`col`, `row`), par anneaux croissants.
fn nearest_valid(dem: &Dem, col: u32, row: u32) -> Option<(u32, u32)> {
    let max = dem.width.max(dem.height) as i64;
    for ring in 0..max {
        for dr in -ring..=ring {
            for dc in -ring..=ring {
                if dr.abs() != ring && dc.abs() != ring {
                    continue;
                }
                let (c, r) = (i64::from(col) + dc, i64::from(row) + dr);
                if c >= 0 && r >= 0 && dem.value(c as u32, r as u32).is_some() {
                    return Some((c as u32, r as u32));
                }
            }
        }
    }
    None
}

/// O1 au centre, O2 au point le plus haut et O3 au point le plus bas à moins de 4 km du centre.
fn observers(dem: &Dem) -> Result<Vec<Site>, String> {
    let (cc, cr) = nearest_valid(dem, dem.width / 2, dem.height / 2)
        .ok_or("Le MNT n'a aucune cellule valide.")?;
    let (cx, cy) = centre(dem, cc, cr);
    // Altitude, colonne, ligne.
    let mut high: Option<(f32, u32, u32)> = None;
    let mut low: Option<(f32, u32, u32)> = None;
    for row in 0..dem.height {
        for col in 0..dem.width {
            let Some(z) = dem.value(col, row) else {
                continue;
            };
            let (x, y) = centre(dem, col, row);
            if ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() > SEARCH {
                continue;
            }
            if high.is_none_or(|(h, _, _)| z > h) {
                high = Some((z, col, row));
            }
            if low.is_none_or(|(l, _, _)| z < l) {
                low = Some((z, col, row));
            }
        }
    }
    let (high, low) = (high.expect("le centre est valide"), low.expect("idem"));
    let sites = vec![
        site(dem, "O1".into(), cc, cr),
        site(dem, "O2".into(), high.1, high.2),
        site(dem, "O3".into(), low.1, low.2),
    ];
    for (i, a) in sites.iter().enumerate() {
        if sites[..i].iter().any(|b| (b.col, b.row) == (a.col, a.row)) {
            return Err(format!("L'observateur {} tombe sur un autre.", a.name));
        }
    }
    Ok(sites)
}

/// Cible `k` (1 à 10) d'un observateur : une cellule valide dont le centre est à `k` km, à une
/// cellule près, et dans le rayon ; azimut de départ en angle d'or, puis par pas de 7,5°.
fn target(dem: &Dem, o: &Site, index: usize, k: u32) -> Result<Target, String> {
    let cell = dem.cell_size_x.max(dem.cell_size_y);
    // À 10 km, on vise une cellule en deçà : la cible reste dans le rayon du champ de vision.
    let wanted = (f64::from(k) * 1000.0).min(RADIUS - cell);
    let start = f64::from(k) * 137.507_764 + index as f64 * 40.0;
    for step in 0..48 {
        let azimuth = (start + f64::from(step) * 7.5).to_radians();
        let (x, y) = (o.x + wanted * azimuth.sin(), o.y + wanted * azimuth.cos());
        let fc = ((x - dem.origin_x) / dem.cell_size_x).floor();
        let fr = ((dem.origin_y - y) / dem.cell_size_y).floor();
        if fc < 0.0 || fr < 0.0 || fc >= f64::from(dem.width) || fr >= f64::from(dem.height) {
            continue;
        }
        let (col, row) = (fc as u32, fr as u32);
        if dem.value(col, row).is_none() {
            continue;
        }
        let (tx, ty) = centre(dem, col, row);
        let distance = ((tx - o.x).powi(2) + (ty - o.y).powi(2)).sqrt();
        if (distance - wanted).abs() <= cell && distance <= RADIUS - 1.0 {
            return Ok(Target {
                observer: o.name.clone(),
                site: site(dem, format!("{}-C{k:02}", o.name), col, row),
                distance,
            });
        }
    }
    Err(format!(
        "Aucune cible valide à {k} km de {} dans l'emprise du MNT.",
        o.name
    ))
}

fn targets(dem: &Dem, observers: &[Site]) -> Result<Vec<Target>, String> {
    let mut all = Vec::new();
    for (i, o) in observers.iter().enumerate() {
        for k in 1..=TARGETS_PER_OBSERVER {
            all.push(target(dem, o, i, k)?);
        }
    }
    Ok(all)
}

fn site_json(s: &Site, dem: &Dem) -> Value {
    json!({
        "nom": s.name, "col": s.col, "row": s.row, "x": s.x, "y": s.y,
        "altitude": dem.value(s.col, s.row),
    })
}

fn points(mnt: &Path) -> Result<String, String> {
    let dem = open(mnt)?;
    let obs = observers(&dem)?;
    let tgts = targets(&dem, &obs)?;
    let value = json!({
        "epsg": dem.epsg,
        "parametres": {
            "hauteur_observateur_m": OBSERVER_HEIGHT,
            "hauteur_cible_m": TARGET_HEIGHT,
            "refraction": REFRACTION,
            "rayon_m": RADIUS,
        },
        "observateurs": obs.iter().map(|s| site_json(s, &dem)).collect::<Vec<_>>(),
        "cibles": tgts.iter().map(|t| {
            let mut v = site_json(&t.site, &dem);
            v["observateur"] = json!(t.observer);
            v["distance"] = json!(t.distance);
            v
        }).collect::<Vec<_>>(),
    });
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&value).expect("JSON")
    ))
}

// ---------------------------------------------------------------------------------------------
// Pente

#[derive(Debug, Default, PartialEq)]
struct GridComparison {
    /// Cellules définies des deux côtés.
    both: u64,
    /// Définies par le seul noyau, par le seul QGIS, selon le lieu (`place`) : [bord de grille,
    /// voisine d'un trou, plate, ailleurs].
    core_only: [u64; 4],
    qgis_only: [u64; 4],
    neither: u64,
    sum_abs: f64,
    max_abs: f64,
    max_at: (u32, u32),
    /// Cellules dont l'écart dépasse 0,001 unité.
    above_milli: u64,
    /// Vrai si la commande laisse les cellules plates sans donnée (l'exposition seule) : un
    /// désaccord de masque sur une cellule plate y est expliqué ; ailleurs, il ne l'est pas.
    flat_explained: bool,
}

/// Noms des lieux de `place`, dans l'ordre des compteurs.
const PLACES: [&str; 4] = [
    "bord de grille",
    "voisine d'un trou",
    "cellule plate",
    "ailleurs",
];

/// Lieu d'une cellule : 0 bord de grille, 1 voisine (ou cellule) sans donnée du MNT, 2 cellule
/// plate (gradient de Horn nul), 3 ailleurs.
fn place(mnt: &Dem, col: u32, row: u32) -> usize {
    if col == 0 || row == 0 || col + 1 == mnt.width || row + 1 == mnt.height {
        return 0;
    }
    match horn_gradient(mnt, col, row) {
        None => 1,
        Some((dx, dy)) if dx == 0.0 && dy == 0.0 => 2,
        Some(_) => 3,
    }
}

/// Écart absolu, pour la pente et l'ombrage.
fn linear(a: f64, b: f64) -> f64 {
    (a - b).abs()
}

/// Écart entre deux azimuts en degrés, sur le cercle : 359° et 1° sont à 2°.
fn circular(a: f64, b: f64) -> f64 {
    let d = (a - b).abs().rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Une section de grilles du rapport : radical des fichiers (`noyau-<file>.tif`,
/// `qgis-<file>.tif`), nom dans les messages, titre, unité des écarts, fonction d'écart, et
/// vrai si les cellules plates sont sans donnée par construction (`flat_explained`).
type GridSection = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    fn(f64, f64) -> f64,
    bool,
);

/// Compare cellule par cellule deux grilles sur celle du MNT, avec la fonction d'écart `diff`.
fn compare_grids(
    mnt: &Dem,
    core: &Dem,
    qgis: &Dem,
    what: &str,
    diff: fn(f64, f64) -> f64,
    flat_explained: bool,
) -> Result<GridComparison, String> {
    same_grid(mnt, core, &format!("{what} du noyau"))?;
    same_grid(mnt, qgis, &format!("{what} de QGIS"))?;
    let mut s = GridComparison {
        flat_explained,
        ..GridComparison::default()
    };
    for row in 0..mnt.height {
        for col in 0..mnt.width {
            match (core.value(col, row), qgis.value(col, row)) {
                (Some(a), Some(b)) => {
                    let d = diff(f64::from(a), f64::from(b));
                    s.both += 1;
                    s.sum_abs += d;
                    if d > 1e-3 {
                        s.above_milli += 1;
                    }
                    if d > s.max_abs {
                        s.max_abs = d;
                        s.max_at = (col, row);
                    }
                }
                (Some(_), None) => s.core_only[place(mnt, col, row)] += 1,
                (None, Some(_)) => s.qgis_only[place(mnt, col, row)] += 1,
                (None, None) => s.neither += 1,
            }
        }
    }
    Ok(s)
}

impl GridComparison {
    fn mean_abs(&self) -> f64 {
        if self.both == 0 {
            0.0
        } else {
            self.sum_abs / self.both as f64
        }
    }

    /// Désaccords de masque hors des lieux expliqués (bord, trou, et cellule plate pour la seule
    /// exposition) : ils appellent un ticket de bogue.
    fn unexplained(&self) -> u64 {
        let flat = if self.flat_explained {
            0
        } else {
            self.core_only[2] + self.qgis_only[2]
        };
        self.core_only[3] + self.qgis_only[3] + flat
    }

    /// Le tableau Markdown de la comparaison, sous son titre ; `unit` suit chaque écart.
    fn report(&self, mnt: &Dem, title: &str, unit: &str) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "## {title}\n");
        let _ = writeln!(out, "| Mesure | Valeur |\n|---|---|");
        let _ = writeln!(
            out,
            "| Cellules du MNT | {} |",
            mnt.width as u64 * mnt.height as u64
        );
        let _ = writeln!(out, "| Définies des deux côtés | {} |", self.both);
        let _ = writeln!(out, "| Écart moyen | {}{unit} |", num(self.mean_abs(), 7));
        let _ = writeln!(
            out,
            "| Écart maximal | {}{unit} (colonne {}, ligne {}) |",
            num(self.max_abs, 7),
            self.max_at.0,
            self.max_at.1
        );
        let _ = writeln!(
            out,
            "| Cellules à plus de 0,001{unit} | {} |",
            self.above_milli
        );
        for (i, label) in PLACES.iter().enumerate() {
            let _ = writeln!(
                out,
                "| Masque différent, {label} : seul le noyau / seul QGIS | {} / {} |",
                self.core_only[i], self.qgis_only[i]
            );
        }
        let _ = writeln!(out, "| Sans donnée des deux côtés | {} |\n", self.neither);
        if self.unexplained() > 0 {
            let places = if self.flat_explained {
                "hors du bord, des trous et des cellules plates"
            } else {
                "hors du bord et des trous"
            };
            let _ = writeln!(
                out,
                "**{} désaccords de masque {places} : à expliquer, ou ticket de bogue.**\n",
                self.unexplained()
            );
        }
        out
    }
}

// ---------------------------------------------------------------------------------------------
// Champ de vision

/// Bandes de distance, en mètres, pour situer les désaccords.
const BANDS: [f64; 4] = [2_500.0, 5_000.0, 7_500.0, 10_000.0];

#[derive(Debug, Default, PartialEq)]
struct ViewshedComparison {
    /// Cellules définies des deux côtés, dont celles où les verdicts concordent.
    both: u64,
    agree: u64,
    /// Visible pour le noyau, masquée pour GRASS ; et l'inverse.
    core_visible_qgis_hidden: u64,
    core_hidden_qgis_visible: u64,
    /// Désaccords sur une lisière (une voisine a un autre verdict, dans l'un des deux outils).
    disagree_on_edge: u64,
    /// Cellules comparées qui sont sur une lisière.
    compared_on_edge: u64,
    core_only: u64,
    qgis_only: u64,
    /// Par bande de distance : cellules comparées, désaccords.
    bands: [(u64, u64); 4],
}

impl ViewshedComparison {
    fn disagree(&self) -> u64 {
        self.both - self.agree
    }
}

fn visible(v: Option<f32>) -> Option<bool> {
    v.map(|v| v > 0.5)
}

/// Vrai si une voisine définie de la cellule a un autre verdict.
fn on_edge(get: &dyn Fn(i64, i64) -> Option<bool>, col: i64, row: i64) -> bool {
    let Some(me) = get(col, row) else {
        return false;
    };
    (-1..=1).any(|dr| {
        (-1..=1).any(|dc| (dr, dc) != (0, 0) && get(col + dc, row + dr).is_some_and(|v| v != me))
    })
}

fn compare_viewshed(
    mnt: &Dem,
    observer: &Site,
    core: &Dem,
    qgis: &Dem,
) -> Result<ViewshedComparison, String> {
    let off = offset_in(mnt, core, &format!("la vue du noyau {}", observer.name))?;
    same_grid(mnt, qgis, &format!("la vue de GRASS {}", observer.name))?;
    let in_mnt = |c: i64, r: i64| c >= 0 && r >= 0 && c < mnt.width as i64 && r < mnt.height as i64;
    let core_at = |c: i64, r: i64| {
        if in_mnt(c, r) {
            visible(value_at(core, off, c as u32, r as u32))
        } else {
            None
        }
    };
    let qgis_at = |c: i64, r: i64| {
        if in_mnt(c, r) {
            visible(qgis.value(c as u32, r as u32))
        } else {
            None
        }
    };
    // Pour les lisières, GRASS n'est lu que là où le noyau définit la cellule : avec `-b`, il
    // écrit 0 hors du rayon et sur les trous, qui ne sont pas des cellules masquées.
    let qgis_in_disk = |c: i64, r: i64| core_at(c, r).and(qgis_at(c, r));
    let mut s = ViewshedComparison::default();
    for row in 0..mnt.height {
        for col in 0..mnt.width {
            let (c, r) = (i64::from(col), i64::from(row));
            match (core_at(c, r), qgis_at(c, r)) {
                (Some(a), Some(b)) => {
                    s.both += 1;
                    let edge = on_edge(&core_at, c, r) || on_edge(&qgis_in_disk, c, r);
                    if edge {
                        s.compared_on_edge += 1;
                    }
                    let (x, y) = centre(mnt, col, row);
                    let d = ((x - observer.x).powi(2) + (y - observer.y).powi(2)).sqrt();
                    let band = BANDS
                        .iter()
                        .position(|&b| d <= b)
                        .unwrap_or(BANDS.len() - 1);
                    s.bands[band].0 += 1;
                    if a == b {
                        s.agree += 1;
                        continue;
                    }
                    s.bands[band].1 += 1;
                    if edge {
                        s.disagree_on_edge += 1;
                    }
                    if a {
                        s.core_visible_qgis_hidden += 1;
                    } else {
                        s.core_hidden_qgis_visible += 1;
                    }
                }
                (Some(_), None) => s.core_only += 1,
                (None, Some(_)) => s.qgis_only += 1,
                (None, None) => {}
            }
        }
    }
    Ok(s)
}

// ---------------------------------------------------------------------------------------------
// Rapport

fn pct(part: u64, whole: u64) -> String {
    if whole == 0 {
        "—".to_string()
    } else {
        format!("{:.2} %", 100.0 * part as f64 / whole as f64).replace('.', ",")
    }
}

fn num(v: f64, decimals: usize) -> String {
    format!("{v:.decimals$}").replace('.', ",")
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{} : {e}", path.display()))?;
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("{} : {e}", path.display()))
}

fn sites(points: &Value, key: &str) -> Result<Vec<(Site, Option<String>, f64)>, String> {
    let list = points[key]
        .as_array()
        .ok_or(format!("points.json n'a pas de liste « {key} »."))?;
    list.iter()
        .map(|v| {
            let s = Site {
                name: v["nom"].as_str().ok_or("nom manquant")?.to_string(),
                col: v["col"].as_u64().ok_or("col manquante")? as u32,
                row: v["row"].as_u64().ok_or("row manquante")? as u32,
                x: v["x"].as_f64().ok_or("x manquant")?,
                y: v["y"].as_f64().ok_or("y manquant")?,
            };
            let observer = v["observateur"].as_str().map(str::to_string);
            Ok((s, observer, v["distance"].as_f64().unwrap_or(0.0)))
        })
        .collect()
}

/// Juge un couple : le verdict du noyau contre `r.viewshed` lu à la cellule de la cible. Rend
/// l'accord et la ligne du tableau, où un couple discordant est marqué « **non** » avec sa
/// distance et sa position. La lisière de GRASS n'est lue que là où le champ de vision du
/// noyau (`core_grid`, décalé de `off`) définit la cellule : hors du rayon et sur les trous,
/// le 0 de `-b` n'est pas une cellule masquée.
fn judge_pair(
    mnt: &Dem,
    qgis_grid: &Dem,
    (core_grid, off): (&Dem, (i64, i64)),
    t: &Site,
    distance: f64,
    core: bool,
) -> Result<(bool, String), String> {
    same_grid(mnt, qgis_grid, "la vue de GRASS")?;
    let grass = visible(qgis_grid.value(t.col, t.row))
        .ok_or(format!("GRASS ne définit pas la cellule de {}.", t.name))?;
    let edge = on_edge(
        &|c, r| {
            if c < 0 || r < 0 || c >= mnt.width as i64 || r >= mnt.height as i64 {
                return None;
            }
            let (c, r) = (c as u32, r as u32);
            value_at(core_grid, off, c, r)?;
            visible(qgis_grid.value(c, r))
        },
        i64::from(t.col),
        i64::from(t.row),
    );
    let agree = core == grass;
    let word = |v: bool| if v { "visible" } else { "masquée" };
    let row = format!(
        "| {} | {} m | {} ; {} | {} | {} | {} | {} |\n",
        t.name,
        num(distance, 0),
        num(t.x, 1),
        num(t.y, 1),
        word(core),
        word(grass),
        if edge { "oui" } else { "non" },
        if agree { "oui" } else { "**non**" }
    );
    Ok((agree, row))
}

fn compare(dir: &Path) -> Result<String, String> {
    let mnt_path = dir.join("mnt.tif");
    let mnt = open(&mnt_path)?;
    let points = read_json(&dir.join("points.json"))?;
    let observers = sites(&points, "observateurs")?;
    let targets = sites(&points, "cibles")?;
    let mut out = String::new();

    // Pente, exposition et ombrage : la même comparaison de grilles.
    let grids: [GridSection; 3] = [
        (
            "pente",
            "la pente",
            "Pente (degrés, Horn) — noyau contre `native:slope`",
            " °",
            linear,
            false,
        ),
        (
            "exposition",
            "l'exposition",
            "Exposition (degrés depuis le nord du quadrillage, écart sur le cercle) — `terrain.aspect` contre `native:aspect`",
            " °",
            circular,
            true,
        ),
        (
            "ombrage",
            "l'ombrage",
            "Ombrage (0 à 255, soleil 315°, 45°, facteur Z 1) — `terrain.hillshade` contre `native:hillshade`",
            "",
            linear,
            false,
        ),
    ];
    for (file, what, title, unit, diff, flat_explained) in grids {
        let comparison = compare_grids(
            &mnt,
            &open(&dir.join(format!("noyau-{file}.tif")))?,
            &open(&dir.join(format!("qgis-{file}.tif")))?,
            what,
            diff,
            flat_explained,
        )?;
        out.push_str(&comparison.report(&mnt, title, unit));
    }

    // Champs de vision.
    let _ = writeln!(
        out,
        "## Champ de vision — noyau contre `grass:r.viewshed -c -r -b`\n"
    );
    let _ = writeln!(
        out,
        "| Observateur | Comparées | Désaccords | dont noyau visible / GRASS masquée | dont noyau masquée / GRASS visible | Désaccords sur une lisière | Comparées sur une lisière | Seul le noyau / seul GRASS |\n|---|---|---|---|---|---|---|---|"
    );
    let mut bands_rows = String::new();
    for (o, _, _) in &observers {
        let core = open(&dir.join(format!("noyau-vue-{}.tif", o.name)))?;
        let qgis = open(&dir.join(format!("qgis-vue-{}.tif", o.name)))?;
        let v = compare_viewshed(&mnt, o, &core, &qgis)?;
        let _ = writeln!(
            out,
            "| {} | {} | {} ({}) | {} | {} | {} ({}) | {} ({}) | {} / {} |",
            o.name,
            v.both,
            v.disagree(),
            pct(v.disagree(), v.both),
            v.core_visible_qgis_hidden,
            v.core_hidden_qgis_visible,
            v.disagree_on_edge,
            pct(v.disagree_on_edge, v.disagree()),
            v.compared_on_edge,
            pct(v.compared_on_edge, v.both),
            v.core_only,
            v.qgis_only
        );
        let _ = write!(bands_rows, "| {} |", o.name);
        for (n, d) in v.bands {
            let _ = write!(bands_rows, " {} |", pct(d, n));
        }
        bands_rows.push('\n');
    }
    let _ = writeln!(
        out,
        "\nPart des désaccords par bande de distance :\n\n| Observateur | 0–2,5 km | 2,5–5 km | 5–7,5 km | 7,5–10 km |\n|---|---|---|---|---|\n{bands_rows}"
    );

    // Lignes de vue.
    let registry = Registry::standard();
    let mut rows = String::new();
    let (mut discord, mut core_vs_own_viewshed) = (0u32, 0u32);
    let mut elapsed = Duration::ZERO;
    // Les deux champs de vision de chaque observateur, ouverts une fois.
    let mut grids = Vec::new();
    for (o, _, _) in &observers {
        let qgis_grid = open(&dir.join(format!("qgis-vue-{}.tif", o.name)))?;
        let core_grid = open(&dir.join(format!("noyau-vue-{}.tif", o.name)))?;
        let off = offset_in(&mnt, &core_grid, "la vue du noyau")?;
        grids.push((qgis_grid, core_grid, off));
    }
    for (t, observer, distance) in &targets {
        let observer = observer.as_deref().ok_or("cible sans observateur")?;
        let index = observers
            .iter()
            .position(|(o, _, _)| o.name == observer)
            .ok_or(format!("observateur {observer} inconnu"))?;
        let o = &observers[index].0;
        let (qgis_grid, core_grid, off) = &grids[index];
        let started = Instant::now();
        let answer = registry
            .call(
                "terrain.line_of_sight",
                json!({
                    "dem": mnt_path.to_string_lossy(),
                    "observer": {"x": o.x, "y": o.y, "epsg": mnt.epsg},
                    "target": {"x": t.x, "y": t.y, "epsg": mnt.epsg},
                    "observer_height": {"value": OBSERVER_HEIGHT, "unit": "m"},
                    "target_height": {"value": TARGET_HEIGHT, "unit": "m"},
                    "curvature": true,
                    "refraction": REFRACTION,
                }),
            )
            .map_err(|e| format!("terrain.line_of_sight vers {} : {e:?}", t.name))?;
        elapsed += started.elapsed();
        let core = answer["visible"].as_bool().ok_or("verdict manquant")?;
        if visible(value_at(core_grid, *off, t.col, t.row)) != Some(core) {
            core_vs_own_viewshed += 1;
        }
        let (agree, row) = judge_pair(&mnt, qgis_grid, (core_grid, *off), t, *distance, core)?;
        if !agree {
            discord += 1;
        }
        rows.push_str(&row);
    }
    let elapsed = elapsed.as_secs_f64();
    let _ = writeln!(
        out,
        "## Ligne de vue — `terrain.line_of_sight` contre `r.viewshed` lu à la cellule de la cible\n"
    );
    let _ = writeln!(
        out,
        "{} couples, {discord} verdicts différents. Verdict du noyau différent de son propre champ de vision : {core_vs_own_viewshed}. Durée des {} appels du registre, ouverture du MNT comprise : {} s.\n",
        targets.len(),
        targets.len(),
        num(elapsed, 3)
    );
    let _ = writeln!(
        out,
        "| Cible | Distance | Position (x ; y) | Noyau | GRASS | Lisière GRASS | Accord |\n|---|---|---|---|---|---|---|\n{rows}"
    );

    // Temps.
    if let Ok(times) = read_json(&dir.join("temps.json")) {
        let _ = writeln!(out, "## Temps (secondes, durée murale)\n");
        let _ = writeln!(out, "| Appel | Noyau | QGIS |\n|---|---|---|");
        let mut keys: Vec<String> = Vec::new();
        for side in ["noyau", "qgis"] {
            if let Some(map) = times[side].as_object() {
                for k in map.keys() {
                    if !keys.contains(k) {
                        keys.push(k.clone());
                    }
                }
            }
        }
        for k in keys {
            let cell = |side: &str| {
                times[side][&k]
                    .as_f64()
                    .map_or("—".to_string(), |v| num(v, 2))
            };
            let _ = writeln!(out, "| {k} | {} | {} |", cell("noyau"), cell("qgis"));
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(width: u32, height: u32, values: Vec<f32>) -> Dem {
        Dem {
            width,
            height,
            origin_x: 650_000.0,
            origin_y: 6_860_000.0,
            cell_size_x: 25.0,
            cell_size_y: 25.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values,
        }
    }

    #[test]
    fn a_different_grid_is_refused() {
        let mnt = grid(4, 4, vec![0.0; 16]);
        assert!(same_grid(&mnt, &mnt.clone(), "x").is_ok());
        let mut other = mnt.clone();
        other.cell_size_x = 30.0;
        assert!(same_grid(&mnt, &other, "x").is_err());
        let mut other = grid(5, 4, vec![0.0; 20]);
        assert!(same_grid(&mnt, &other, "x").is_err());
        other = mnt.clone();
        other.origin_x += 12.5;
        assert!(same_grid(&mnt, &other, "x").is_err());
        other = mnt.clone();
        other.epsg = 4326;
        assert!(same_grid(&mnt, &other, "x").is_err());
    }

    #[test]
    fn a_subgrid_is_located_or_refused_when_misaligned() {
        let mnt = grid(10, 10, vec![0.0; 100]);
        let mut sub = grid(3, 3, vec![1.0; 9]);
        sub.origin_x += 2.0 * 25.0;
        sub.origin_y -= 4.0 * 25.0;
        assert_eq!(offset_in(&mnt, &sub, "x"), Ok((2, 4)));
        assert_eq!(value_at(&sub, (2, 4), 2, 4), Some(1.0));
        assert_eq!(value_at(&sub, (2, 4), 1, 4), None);
        assert_eq!(value_at(&sub, (2, 4), 5, 4), None);
        sub.origin_x += 10.0;
        assert!(offset_in(&mnt, &sub, "x").is_err());
        let mut other = grid(3, 3, vec![1.0; 9]);
        other.cell_size_y = 20.0;
        assert!(offset_in(&mnt, &other, "x").is_err());
    }

    #[test]
    fn slope_counts_masks_by_place_and_errors() {
        // MNT 4 × 4 avec un trou en (3, 3) : (2, 2) est voisine du trou.
        let mut mnt_values = vec![10.0; 16];
        mnt_values[15] = -9999.0;
        let mnt = grid(4, 4, mnt_values);
        let nd = -9999.0;
        // Noyau : seul l'intérieur, sauf la voisine du trou.
        let core = grid(
            4,
            4,
            vec![
                nd, nd, nd, nd, //
                nd, 1.0, 2.0, nd, //
                nd, 3.0, nd, nd, //
                nd, nd, nd, nd,
            ],
        );
        // QGIS : tout sauf le trou ; un écart de 0,5° en (2, 1).
        let mut q = vec![0.0; 16];
        q[5] = 1.0;
        q[6] = 2.5;
        q[9] = 3.0;
        q[15] = nd;
        let qgis = grid(4, 4, q);
        let s = compare_grids(&mnt, &core, &qgis, "la pente", linear, false).unwrap();
        assert_eq!(s.both, 3);
        assert_eq!(s.core_only, [0, 0, 0, 0]);
        // 12 cellules de bord moins le trou, définies par QGIS seul ; la voisine du trou.
        assert_eq!(s.qgis_only, [11, 1, 0, 0]);
        assert_eq!(s.neither, 1);
        assert_eq!(s.above_milli, 1);
        assert_eq!(s.max_at, (2, 1));
        assert!((s.max_abs - 0.5).abs() < 1e-9);
        assert!((s.mean_abs() - 0.5 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn the_circular_gap_wraps_around_north() {
        assert_eq!(circular(359.0, 1.0), 2.0);
        assert_eq!(circular(1.0, 359.0), 2.0);
        assert_eq!(circular(90.0, 270.0), 180.0);
        assert_eq!(circular(10.0, 10.0), 0.0);
        assert!((circular(0.25, 359.75) - 0.5).abs() < 1e-12);
        assert_eq!(linear(359.0, 1.0), 358.0);
    }

    #[test]
    fn aspect_masks_on_flat_cells_are_classed_apart_and_others_are_flagged() {
        // MNT 5 × 4 : plat à gauche (10 m), plan qui monte vers l'est à droite.
        let mnt = grid(
            5,
            4,
            vec![
                10.0, 10.0, 10.0, 11.0, 12.0, //
                10.0, 10.0, 10.0, 11.0, 12.0, //
                10.0, 10.0, 10.0, 11.0, 12.0, //
                10.0, 10.0, 10.0, 11.0, 12.0,
            ],
        );
        let nd = -9999.0;
        // Noyau : (1, 1) et (1, 2) plates, donc sans donnée ; 359,5° en (2, 1).
        let core = grid(
            5,
            4,
            vec![
                nd, nd, nd, nd, nd, //
                nd, nd, 359.5, 270.0, nd, //
                nd, nd, 270.0, nd, nd, //
                nd, nd, nd, nd, nd,
            ],
        );
        // QGIS : une valeur sur les plates, 0,5° en (2, 1), et (3, 2) que le noyau n'a pas.
        let qgis = grid(
            5,
            4,
            vec![
                nd, nd, nd, nd, nd, //
                nd, 0.0, 0.5, 270.0, nd, //
                nd, 0.0, 270.0, 270.0, nd, //
                nd, nd, nd, nd, nd,
            ],
        );
        let s = compare_grids(&mnt, &core, &qgis, "l'exposition", circular, true).unwrap();
        assert_eq!(s.both, 3);
        assert!((s.max_abs - 1.0).abs() < 1e-9 && s.max_at == (2, 1));
        assert_eq!(s.qgis_only, [0, 0, 2, 1]);
        assert_eq!(s.unexplained(), 1);
        let report = s.report(&mnt, "Exposition", " °");
        assert!(report.starts_with("## Exposition\n"));
        assert!(report
            .contains("| Masque différent, cellule plate : seul le noyau / seul QGIS | 0 / 2 |"));
        assert!(report
            .contains("1 désaccords de masque hors du bord, des trous et des cellules plates"));
    }

    #[test]
    fn a_flat_cell_mask_disagreement_is_unexplained_for_slope_and_hillshade() {
        // MNT plat 3 × 3 : la cellule centrale est plate ; le noyau la laisse sans donnée.
        let mnt = grid(3, 3, vec![10.0; 9]);
        let nd = -9999.0;
        let core = grid(3, 3, vec![nd; 9]);
        let mut q = vec![nd; 9];
        q[4] = 0.0;
        let qgis = grid(3, 3, q);
        for (what, unit) in [("la pente", " °"), ("l'ombrage", "")] {
            let s = compare_grids(&mnt, &core, &qgis, what, linear, false).unwrap();
            assert_eq!(s.qgis_only, [0, 0, 1, 0]);
            assert_eq!(s.unexplained(), 1, "{what}");
            let report = s.report(&mnt, "Grille", unit);
            assert!(report.contains("**1 désaccords de masque hors du bord et des trous :"));
        }
        // Pour l'exposition, la même cellule plate est expliquée.
        let s = compare_grids(&mnt, &core, &qgis, "l'exposition", circular, true).unwrap();
        assert_eq!(s.unexplained(), 0);
        assert!(!s.report(&mnt, "Grille", " °").contains("**"));
    }

    #[test]
    fn viewshed_counts_agreement_edges_and_one_sided_cells() {
        let mnt = grid(4, 1, vec![10.0; 4]);
        let o = site(&mnt, "O1".into(), 0, 0);
        // Noyau rogné aux 3 premières cellules ; GRASS sur tout le MNT.
        let core = grid(3, 1, vec![1.0, 1.0, 0.0]);
        let qgis = grid(4, 1, vec![1.0, 0.0, 0.0, 0.0]);
        let v = compare_viewshed(&mnt, &o, &core, &qgis).unwrap();
        assert_eq!(v.both, 3);
        assert_eq!(v.agree, 2);
        assert_eq!(v.core_visible_qgis_hidden, 1);
        assert_eq!(v.core_hidden_qgis_visible, 0);
        assert_eq!(v.disagree_on_edge, 1);
        assert_eq!(v.core_only, 0);
        assert_eq!(v.qgis_only, 1);
        assert_eq!(v.bands[0], (3, 1));
    }

    #[test]
    fn a_discordant_pair_is_listed_with_its_distance_and_position() {
        let mnt = grid(3, 1, vec![10.0; 3]);
        let qgis = grid(3, 1, vec![1.0, 0.0, 0.0]);
        let t = site(&mnt, "C01".into(), 1, 0);
        let full = grid(3, 1, vec![1.0; 3]);
        let core = (&full, (0, 0));
        let (agree, row) = judge_pair(&mnt, &qgis, core, &t, 1000.0, true).unwrap();
        assert!(!agree);
        assert!(row.starts_with("| C01 | 1000 m | 650037,5 ; 6859987,5 | visible | masquée |"));
        assert!(row.trim_end().ends_with("| oui | **non** |"));
        let (agree, row) = judge_pair(&mnt, &qgis, core, &t, 1000.0, false).unwrap();
        assert!(agree && row.trim_end().ends_with("| oui |"));
        // Cellule que GRASS ne définit pas : erreur claire, pas un verdict.
        let holed = grid(3, 1, vec![1.0, -9999.0, 0.0]);
        assert!(judge_pair(&mnt, &holed, core, &t, 1000.0, true).is_err());
    }

    #[test]
    fn grass_zeros_outside_the_disk_or_on_holes_make_no_edge() {
        // Le noyau définit les 3 premières cellules (le disque) ; GRASS écrit 0 au-delà.
        let mnt = grid(4, 1, vec![10.0; 4]);
        let o = site(&mnt, "O1".into(), 0, 0);
        let core = grid(3, 1, vec![1.0; 3]);
        let qgis = grid(4, 1, vec![1.0, 1.0, 1.0, 0.0]);
        let v = compare_viewshed(&mnt, &o, &core, &qgis).unwrap();
        assert_eq!(
            (v.both, v.agree, v.compared_on_edge, v.qgis_only),
            (3, 3, 0, 1)
        );
        // La cible au bord du disque n'est pas sur une lisière.
        let t = site(&mnt, "C01".into(), 2, 0);
        let (agree, row) = judge_pair(&mnt, &qgis, (&core, (0, 0)), &t, 1000.0, true).unwrap();
        assert!(agree && row.trim_end().ends_with("| non | oui |"));
        // Même chose pour un trou du MNT, sans donnée dans le noyau, 0 dans GRASS.
        let holed = grid(4, 1, vec![1.0, 1.0, -9999.0, 1.0]);
        let qgis = grid(4, 1, vec![1.0, 1.0, 0.0, 1.0]);
        let v = compare_viewshed(&mnt, &o, &holed, &qgis).unwrap();
        assert_eq!((v.both, v.compared_on_edge, v.qgis_only), (3, 0, 1));
    }

    #[test]
    fn a_misaligned_qgis_viewshed_is_refused() {
        let mnt = grid(4, 1, vec![10.0; 4]);
        let o = site(&mnt, "O1".into(), 0, 0);
        let core = grid(4, 1, vec![1.0; 4]);
        let qgis = grid(3, 1, vec![1.0; 3]);
        assert!(compare_viewshed(&mnt, &o, &core, &qgis).is_err());
    }

    #[test]
    fn points_are_deterministic_valid_cell_centres_at_the_wanted_distances() {
        // MNT synthétique de 15 km de côté, une bosse au centre, un trou au coin.
        let (w, h) = (600u32, 600u32);
        let mut values: Vec<f32> = (0..w * h)
            .map(|i| {
                let (c, r) = (f64::from(i % w), f64::from(i / w));
                (100.0 - ((c - 300.0).powi(2) + (r - 280.0).powi(2)).sqrt() / 10.0) as f32
            })
            .collect();
        values[0] = -9999.0;
        let dem = grid(w, h, values);
        let obs = observers(&dem).unwrap();
        assert_eq!(obs.len(), 3);
        assert_eq!((obs[1].col, obs[1].row), (300, 280));
        let tgts = targets(&dem, &obs).unwrap();
        assert_eq!(tgts.len(), 30);
        for (i, t) in tgts.iter().enumerate() {
            let k = (i % 10 + 1) as f64 * 1000.0;
            assert!((t.distance - k).abs() <= 50.0 && t.distance < RADIUS);
            assert!(dem.value(t.site.col, t.site.row).is_some());
            assert_eq!(centre(&dem, t.site.col, t.site.row), (t.site.x, t.site.y));
        }
        assert_eq!(targets(&dem, &obs).unwrap(), tgts);
    }
}
