//! Familles de tournures : chacune tire un inventaire, écrit une demande française à partir
//! de gabarits à emplacements, et construit le plan attendu (ou la question, ou le refus).
//! Les règles de chaînage sont celles du prompt de `ai/src/plan.rs` : le filtre sur un champ
//! avant le tampon ou l'intersection, le tampon sur la couche de référence, puis des
//! intersections successives dont la première porte la couche cherchée.
//!
//! Vocabulaire civil seulement (CLAUDE.md §2.9). Une phrase mal comprise par le modèle devient
//! une famille ou une tournure ici, jamais une règle de prompt.

use serde_json::{json, Value};

use crate::inventories::{
    theme, Builder, FieldDef, Geom, Kind, Layer, Noun, Theme, Val, CRITERIA, MASKS, RAW_RASTERS,
    THEMES,
};
use crate::rng::Rng;

/// Un exemple engendré, avant validation.
#[derive(Debug, Clone)]
pub struct Example {
    pub family: &'static str,
    pub inventory: Value,
    pub request: String,
    pub expected: Value,
    /// Exemple de redemande : la sortie rejetée et ses écarts, que le prompt montre avant la
    /// réponse attendue. `None` pour un premier essai.
    pub retry: Option<Retry>,
}

/// Le tour de redemande d'un exemple : la sortie rejetée par `check_plan`, telle que le
/// modèle l'aurait écrite, et les écarts au texte exact du jugement de `cassini-ai`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retry {
    pub rejected: String,
    pub errors: Vec<String>,
}

/// Une famille : nom stable, poids de tirage, fabrique.
pub struct Family {
    pub name: &'static str,
    pub weight: u64,
    pub make: fn(&mut Rng) -> Option<Example>,
}

/// Toutes les familles, dans un ordre fixe (reproductibilité).
pub static FAMILIES: &[Family] = &[
    Family {
        name: "filtre_egalite",
        weight: 10,
        make: filtre_egalite,
    },
    Family {
        name: "garde_seulement",
        weight: 6,
        make: garde_seulement,
    },
    Family {
        name: "filtre_numerique",
        weight: 8,
        make: filtre_numerique,
    },
    Family {
        name: "filtre_texte",
        weight: 4,
        make: filtre_texte,
    },
    Family {
        name: "filtre_different",
        weight: 4,
        make: filtre_different,
    },
    Family {
        name: "tampon",
        weight: 7,
        make: tampon,
    },
    Family {
        name: "tampon_filtre",
        weight: 5,
        make: tampon_filtre,
    },
    Family {
        name: "distance",
        weight: 9,
        make: distance,
    },
    Family {
        name: "distance_reference_filtree",
        weight: 6,
        make: distance_reference_filtree,
    },
    Family {
        name: "dans_une_zone",
        weight: 5,
        make: dans_une_zone,
    },
    Family {
        name: "intersection",
        weight: 4,
        make: intersection,
    },
    Family {
        name: "conditions_multiples",
        // Relevé en 5.17 : les plans à plusieurs conditions perdaient des étapes.
        weight: 13,
        make: conditions_multiples,
    },
    Family {
        name: "constructible",
        weight: 7,
        make: constructible,
    },
    Family {
        name: "export_pdf",
        weight: 6,
        make: export_pdf,
    },
    Family {
        name: "export_pdf_scale",
        weight: 3,
        make: export_pdf_scale,
    },
    Family {
        name: "export_pdf_format",
        weight: 4,
        make: export_pdf_format,
    },
    Family {
        name: "isochrone",
        weight: 4,
        make: isochrone,
    },
    Family {
        name: "isochrone_utm",
        weight: 3,
        make: isochrone_utm,
    },
    Family {
        name: "isochrone_vers_cible",
        weight: 5,
        make: isochrone_vers_cible,
    },
    Family {
        name: "decompte_par_zone",
        weight: 4,
        make: decompte_par_zone,
    },
    Family {
        name: "ouverture",
        weight: 4,
        make: ouverture,
    },
    Family {
        name: "description",
        weight: 4,
        make: description,
    },
    Family {
        name: "question_couche_absente",
        weight: 7,
        make: question_couche_absente,
    },
    Family {
        name: "question_champ_absent",
        weight: 5,
        make: question_champ_absent,
    },
    Family {
        name: "refus",
        weight: 7,
        make: refus,
    },
    Family {
        name: "refus_hors_registre",
        weight: 7,
        make: refus_hors_registre,
    },
    // Les trois commandes de terrain (4.6) : MNT désigné par sa source, points écrits.
    Family {
        name: "pente",
        weight: 4,
        make: pente,
    },
    Family {
        name: "ligne_de_vue",
        weight: 4,
        make: ligne_de_vue,
    },
    Family {
        name: "champ_de_vision",
        weight: 4,
        make: champ_de_vision,
    },
    Family {
        name: "question_terrain",
        weight: 4,
        make: question_terrain,
    },
    Family {
        name: "appartenance_commune",
        weight: 5,
        make: appartenance_commune,
    },
    Family {
        name: "question_isochrone",
        weight: 4,
        make: question_isochrone,
    },
    Family {
        name: "etape_unique",
        weight: 5,
        make: etape_unique,
    },
    // Géométrie de l'epic 11 (11.12, lot 1) : une famille par commande, et le point de terrain
    // d'un autre système reprojeté avant la commande de terrain.
    Family {
        name: "conversion_coordonnees",
        weight: 5,
        make: conversion_coordonnees,
    },
    Family {
        name: "reprojection_point",
        weight: 4,
        make: reprojection_point,
    },
    Family {
        name: "distance_azimut",
        weight: 4,
        make: distance_azimut,
    },
    Family {
        name: "point_projete",
        weight: 4,
        make: point_projete,
    },
    Family {
        name: "recoupement",
        weight: 3,
        make: recoupement,
    },
    Family {
        name: "terrain_autre_systeme",
        weight: 4,
        make: terrain_autre_systeme,
    },
    // Dérivés du MNT (11.12, lot 2a) : une famille par commande, et le seuil sur la pente
    // (`terrain.slope` puis `raster.reclassify`), qui remplace l'ancien refus.
    Family {
        name: "exposition",
        weight: 3,
        make: exposition,
    },
    Family {
        name: "ombrage",
        weight: 3,
        make: ombrage,
    },
    Family {
        name: "champ_de_vision_cumule",
        weight: 4,
        make: champ_de_vision_cumule,
    },
    Family {
        name: "points_hauts",
        weight: 4,
        make: points_hauts,
    },
    Family {
        name: "pente_seuil",
        weight: 4,
        make: pente_seuil,
    },
    // Coût de déplacement (11.12, lot 2b-1) : un raster de coût apporté par l'utilisateur,
    // jamais le MNT.
    Family {
        name: "cout_cumule",
        weight: 4,
        make: cout_cumule,
    },
    Family {
        name: "chemin_moindre_cout",
        weight: 4,
        make: chemin_moindre_cout,
    },
    // Critères, masques et superposition (11.12, lot 2b-2) : des rasters apportés par
    // l'utilisateur, cités par leur `source`, jamais une étape en liste.
    Family {
        name: "superposition_ponderee",
        weight: 4,
        make: superposition_ponderee,
    },
    Family {
        name: "reclassement_raster",
        weight: 4,
        make: reclassement_raster,
    },
    // Environ 5 % du jeu : un exemple d'une autre famille, vu au tour de redemande. Le poids
    // compte les tirages laissés (corruption acceptée par `check_plan`, premier essai qui
    // n'est pas un plan d'appels) : un sur deux environ. Relevé de 20 à 24 en 4.6 : les plans
    // de terrain, sans couche ni référence d'étape, ne se corrompent pas. Relevé de 24 à 36 en
    // 11.12 : les plans de géométrie à une étape ne se corrompent pas non plus. Relevé de 36 à
    // 44 en 11.12, lot 2b-1 : les plans de coût, à une étape, pas davantage (3,9 % sinon).
    Family {
        name: "redemande",
        weight: 44,
        make: redemande,
    },
];

// -- plans -----------------------------------------------------------------

fn select(id: &str, field: &str, op: &str, value: &str) -> Value {
    json!({"command": "analysis.select_by_attribute",
        "params": {"layer_id": id, "field": field, "operator": op, "value": value}})
}

fn buffer(id: &str, d: &Dist, dissolve: bool) -> Value {
    if dissolve {
        json!({"command": "analysis.buffer",
            "params": {"layer_id": id, "distance": d.json(), "dissolve": true}})
    } else {
        json!({"command": "analysis.buffer", "params": {"layer_id": id, "distance": d.json()}})
    }
}

fn intersect(id: &str, overlay: &str) -> Value {
    json!({"command": "analysis.intersect", "params": {"layer_id": id, "overlay_layer_id": overlay}})
}

fn pdf(title: &str) -> Value {
    json!({"command": "layout.export_pdf", "params": {"title": title}})
}

/// Export à une échelle demandée : `scale` est le dénominateur (25000 pour 1:25 000).
fn pdf_at(title: &str, scale: u64) -> Value {
    json!({"command": "layout.export_pdf", "params": {"title": title, "scale": scale}})
}

fn step(k: usize) -> String {
    format!("step:{k}.layer_id")
}

fn calls(list: Vec<Value>) -> Value {
    json!({"kind": "calls", "calls": list})
}

fn question(text: String) -> Value {
    json!({"kind": "question", "question": text})
}

fn refusal(text: &str) -> Value {
    json!({"kind": "refusal", "reason": text})
}

// -- texte -----------------------------------------------------------------

/// Communes citées comme simple contexte (« à Orly ») : sans effet sur le plan.
pub const COMMUNES: &[&str] = &[
    "Orly",
    "Ablon-sur-Seine",
    "Choisy-le-Roi",
    "Thiais",
    "Vitry-sur-Seine",
    "Athis-Mons",
    "Juvisy-sur-Orge",
    "Draveil",
    "Rungis",
    "Chevilly-Larue",
    "Créteil",
    "Valenton",
    "Montgeron",
    "Annecy",
    "Albi",
    "Colmar",
    "Niort",
    "Vannes",
    "Saumur",
    "Dole",
];

const FIND: &[&str] = &[
    "Trouve {np}",
    "Montre-moi {np}",
    "Affiche {np}",
    "Sélectionne {np}",
    "Je cherche {np}",
    "Donne-moi {np}",
    "Isole {np}",
    "Extrais {np}",
    "Repère {np}",
    "Je veux {np}",
    "Il me faut {np}",
    "Cherche {np}",
    "Fais ressortir {np}",
    "Liste {np}",
    "Identifie {np}",
    "Localise {np}",
    "Sors-moi {np}",
    "J'aimerais voir {np}",
];

const KEEP: &[&str] = &[
    "Garde seulement {np}",
    "Ne garde que {np}",
    "Conserve uniquement {np}",
    "Conserve seulement {np}",
    "Ne conserve que {np}",
    "Garde uniquement {np}",
];

const USER_TITLES: &[&str] = &[
    "Étude de faisabilité",
    "Diagnostic foncier",
    "Synthèse des contraintes",
    "Carte de travail",
    "Annexe 3",
    "Projet d'aménagement",
    "Note au conseil municipal",
    "Repérage terrain",
    "Dossier de concertation",
    "État des lieux",
];

fn pick_str(rng: &mut Rng, items: &[&'static str]) -> &'static str {
    items[rng.below(items.len())]
}

/// « de Thiais », mais « d'Annecy », « d'Orly ».
fn de_place(name: &str) -> String {
    if name.starts_with(['A', 'E', 'É', 'I', 'O', 'U', 'a', 'e', 'é', 'i', 'o', 'u']) {
        format!("d'{name}")
    } else {
        format!("de {name}")
    }
}

fn cap(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Remplit un gabarit : `{v}` valeur, `{n}` nombre, participes accordés au nom.
fn fill(template: &str, noun: &Noun, v: &str) -> String {
    template
        .replace("{v}", v)
        .replace("{n}", v)
        .replace("{situé}", &noun.agree("situé"))
        .replace("{classé}", &noun.agree("classé"))
        .replace("{localisé}", &noun.agree("localisé"))
}

/// Termine une phrase : contexte de lieu facultatif, politesse, ponctuation.
fn finish(rng: &mut Rng, body: &str, interrogative: bool, place: bool) -> String {
    let mut s = body.to_string();
    if place && rng.chance(1, 8) {
        // Seulement « à X » : « sur le territoire de X » est une tournure d'appartenance, qui
        // crée une étape (`appartenance_commune`) ; un quasi-synonyme sans étape la brouillerait.
        s.push_str(&format!(" à {}", rng.pick(COMMUNES)));
    }
    if rng.chance(1, 12) {
        s.push_str(pick_str(
            rng,
            &[", s'il te plaît", " s'il te plaît", ", stp", " svp"],
        ));
    }
    let end = if interrogative {
        *rng.pick(&[" ?", "?", " ?"])
    } else {
        *rng.pick(&[".", ".", ".", ".", "", " !"])
    };
    format!("{}{end}", cap(&s))
}

/// Tête d'une demande de recherche ; rend la phrase et si elle est interrogative.
fn open_find(rng: &mut Rng, noun: &Noun, np: &str, allow_question: bool) -> (String, bool) {
    let roll = rng.below(12);
    if allow_question && roll == 0 {
        return (format!("Peux-tu me montrer {np}"), true);
    }
    if allow_question && roll == 1 {
        return (format!("{} sont {np}", noun.quels()), true);
    }
    let template = if roll == 2 {
        rng.pick(KEEP)
    } else {
        rng.pick(FIND)
    };
    (template.replace("{np}", np), false)
}

fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// Une distance : valeur JSON, unité du registre, et sa forme écrite.
#[derive(Debug, Clone)]
pub struct Dist {
    value: Value,
    unit: &'static str,
    bare: String,
}

impl Dist {
    fn json(&self) -> Value {
        json!({"value": self.value, "unit": self.unit})
    }
    /// « de 500 m », « d'un kilomètre ».
    fn de(&self) -> String {
        if self.bare.starts_with("un ") {
            format!("d'{}", self.bare)
        } else {
            format!("de {}", self.bare)
        }
    }
}

fn dist(rng: &mut Rng) -> Dist {
    if rng.chance(1, 5) {
        let (value, forms): (Value, &[&str]) = match rng.below(6) {
            0 => (json!(1), &["1 km", "un kilomètre", "1 kilomètre"]),
            1 => (json!(2), &["2 km", "deux kilomètres", "2 kilomètres"]),
            2 => (json!(1.5), &["1,5 km", "1.5 km"]),
            3 => (json!(3), &["3 km", "3 kilomètres"]),
            4 => (json!(5), &["5 km", "cinq kilomètres"]),
            _ => (json!(0.5), &["0,5 km", "0.5 km"]),
        };
        let bare = (*rng.pick(forms)).to_string();
        return Dist {
            value,
            unit: "km",
            bare,
        };
    }
    let n = *rng.pick(&[
        20u64, 25, 30, 50, 50, 75, 100, 100, 120, 150, 150, 200, 200, 250, 300, 300, 350, 400, 500,
        500, 600, 700, 750, 800, 1000, 1200, 1500, 2000,
    ]);
    let number = if n >= 1000 && rng.chance(1, 2) {
        thousands(n)
    } else {
        n.to_string()
    };
    let bare = match rng.below(4) {
        0 => format!("{number} mètres"),
        1 => format!("{number}m"),
        _ => format!("{number} m"),
    };
    Dist {
        value: json!(n),
        unit: "m",
        bare,
    }
}

/// « à moins de 500 m d'une gare » ; `de_ref` vaut « d'une gare » ou « des gares ... ».
fn near(rng: &mut Rng, target: &Noun, d: &Dist, de_ref: &str) -> String {
    let autour = format!("autour {de_ref}");
    let t = match rng.below(7) {
        0 | 1 => format!("à moins {} {de_ref}", d.de()),
        2 => format!("{{situé}} à moins {} {de_ref}", d.de()),
        3 => format!("dans un rayon {} {autour}", d.de()),
        4 => format!("à {} maximum {de_ref}", d.bare),
        5 => format!("à {} au plus {de_ref}", d.bare),
        _ => format!("{{localisé}} à moins {} {de_ref}", d.de()),
    };
    fill(&t, target, "")
}

/// Forme d'un nom suivi d'une valeur de champ texte.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Det {
    Les,
    Des,
    /// « d'une route nationale » : seulement quand la valeur a un adjectif ; sinon « des ».
    Dun,
}

fn qualified(rng: &mut Rng, noun: &Noun, def: &FieldDef, val: &Val, det: Det) -> String {
    if let Some((sg, pl)) = val.adj {
        if rng.chance(2, 3) {
            return match det {
                Det::Les => format!("les {} {pl}", noun.pl),
                Det::Des => format!("des {} {pl}", noun.pl),
                Det::Dun => format!("{} {sg}", noun.dun()),
            };
        }
    }
    let Kind::Text { phrases, .. } = &def.kind else {
        unreachable!("qualified sur un champ non texte")
    };
    let phrase = fill(pick_str(rng, phrases), noun, val.v);
    match det {
        Det::Les => format!("les {} {phrase}", noun.pl),
        Det::Des | Det::Dun => format!("des {} {phrase}", noun.pl),
    }
}

/// Un nombre et sa tournure pour un champ numérique : (opérateur, valeur, tournure).
fn numeric(rng: &mut Rng, def: &FieldDef) -> (&'static str, String, String) {
    let Kind::Number {
        min,
        max,
        step,
        gt,
        lt,
        gte,
        lte,
        ..
    } = &def.kind
    else {
        unreachable!("numeric sur un champ non numérique")
    };
    let n = min + step * rng.range(0, (max - min) / step);
    let (op, list): (&str, &[&str]) = match rng.below(7) {
        0..=2 => ("gt", gt),
        3 | 4 => ("lt", lt),
        5 => ("gte", gte),
        _ => ("lte", lte),
    };
    let shown = if n >= 1000 && rng.chance(1, 2) {
        thousands(n)
    } else {
        n.to_string()
    };
    let phrase = rng.pick(list).replace("{n}", &shown);
    (op, n.to_string(), phrase)
}

fn text_fields(t: &Theme) -> Vec<&'static FieldDef> {
    t.fields
        .iter()
        .filter(|f| matches!(f.kind, Kind::Text { .. }))
        .collect()
}

fn number_fields(t: &Theme) -> Vec<&'static FieldDef> {
    t.fields
        .iter()
        .filter(|f| matches!(f.kind, Kind::Number { .. }))
        .collect()
}

fn vector(t: &Theme) -> bool {
    t.geom != Geom::Raster
}

/// Couche « cherchée » ou « de référence » ordinaire : vecteur, pas un réseau sans attribut,
/// pas le zonage (qui se lit comme une condition, pas comme un objet cherché).
fn ordinary(t: &Theme) -> bool {
    vector(t) && t.key != "reseau" && t.key != "zonage"
}

fn pick_theme(rng: &mut Rng, b: &Builder, pred: impl Fn(&Theme) -> bool) -> Option<&'static Theme> {
    let free = b.free(pred);
    if free.is_empty() {
        None
    } else {
        Some(*rng.pick(&free))
    }
}

fn example(
    family: &'static str,
    b: Builder,
    rng: &mut Rng,
    request: String,
    expected: Value,
) -> Example {
    let inventory = b.finish(rng);
    Example {
        family,
        inventory,
        request,
        expected,
        retry: None,
    }
}

/// Un plan en cours : de quoi écrire la phrase, et ses appels.
struct Draft {
    b: Builder,
    noun: &'static Noun,
    /// Groupe nominal complet, article compris : « les parcelles de la section AB ».
    np: String,
    calls: Vec<Value>,
}

impl Draft {
    /// Titre de carte déduit de la demande : le groupe nominal sans l'article.
    fn title(&self) -> String {
        cap(self.np.strip_prefix("les ").unwrap_or(&self.np))
    }
}

// -- filtres ---------------------------------------------------------------

fn draft_filter_eq(rng: &mut Rng) -> Option<Draft> {
    let mut b = Builder::new();
    let t = pick_theme(rng, &b, |t| ordinary(t) && !text_fields(t).is_empty())?;
    let def = *rng.pick(&text_fields(t));
    let layer = b.add(rng, t.key, &[def.role], &[], &[]);
    let field = layer.field(def.role)?;
    let val = *rng.pick(&field.values);
    let np = if rng.chance(1, 7) {
        format!(
            "les entités de la couche « {} » dont le champ {} vaut {}",
            layer.name, field.name, val.v
        )
    } else {
        qualified(rng, &t.noun, def, val, Det::Les)
    };
    Some(Draft {
        calls: vec![select(&layer.id, &field.name, "eq", val.v)],
        b,
        noun: &t.noun,
        np,
    })
}

fn filtre_egalite(rng: &mut Rng) -> Option<Example> {
    let d = draft_filter_eq(rng)?;
    let (head, q) = open_find(rng, d.noun, &d.np, true);
    let request = finish(rng, &head, q, true);
    Some(example("filtre_egalite", d.b, rng, request, calls(d.calls)))
}

fn draft_filter_numeric(rng: &mut Rng) -> Option<Draft> {
    let mut b = Builder::new();
    let t = pick_theme(rng, &b, |t| ordinary(t) && !number_fields(t).is_empty())?;
    let def = *rng.pick(&number_fields(t));
    let layer = b.add(rng, t.key, &[def.role], &[], &[]);
    let field = layer.field(def.role)?;
    let (op, value, phrase) = numeric(rng, def);
    Some(Draft {
        calls: vec![select(&layer.id, &field.name, op, &value)],
        b,
        noun: &t.noun,
        np: format!("les {} {phrase}", t.noun.pl),
    })
}

/// « Garde seulement / ne garde que / conserve » : une sélection, jamais une suppression.
fn garde_seulement(rng: &mut Rng) -> Option<Example> {
    if rng.chance(1, 4) {
        // « Dans la couche des parcelles, ne garde que celles de la section AB. »
        let mut b = Builder::new();
        let t = pick_theme(rng, &b, |t| ordinary(t) && !text_fields(t).is_empty())?;
        let def = *rng.pick(&text_fields(t));
        let layer = b.add(rng, t.key, &[def.role], &[], &[]);
        let field = layer.field(def.role)?;
        let val = *rng.pick(&field.values);
        let Kind::Text { phrases, .. } = &def.kind else {
            return None;
        };
        let phrase = fill(pick_str(rng, phrases), &t.noun, val.v);
        let verb = rng.pick(KEEP).replace(" {np}", "").to_lowercase();
        let body = format!("Dans {}, {verb} {} {phrase}", t.the_layer, t.noun.ceux());
        let request = finish(rng, &body, false, false);
        let expected = calls(vec![select(&layer.id, &field.name, "eq", val.v)]);
        return Some(example("garde_seulement", b, rng, request, expected));
    }
    let d = if rng.chance(2, 3) {
        draft_filter_eq(rng)?
    } else {
        draft_filter_numeric(rng)?
    };
    let head = rng.pick(KEEP).replace("{np}", &d.np);
    let request = finish(rng, &head, false, true);
    Some(example(
        "garde_seulement",
        d.b,
        rng,
        request,
        calls(d.calls),
    ))
}

fn filtre_numerique(rng: &mut Rng) -> Option<Example> {
    let d = draft_filter_numeric(rng)?;
    let (head, q) = open_find(rng, d.noun, &d.np, true);
    let request = finish(rng, &head, q, true);
    Some(example(
        "filtre_numerique",
        d.b,
        rng,
        request,
        calls(d.calls),
    ))
}

fn filtre_texte(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let t = pick_theme(rng, &b, |t| {
        ordinary(t) && t.fields.iter().any(|f| matches!(f.kind, Kind::Name { .. }))
    })?;
    let def = t
        .fields
        .iter()
        .find(|f| matches!(f.kind, Kind::Name { .. }))?;
    let Kind::Name { fragments } = &def.kind else {
        return None;
    };
    let fragment = *rng.pick(fragments);
    let layer = b.add(rng, t.key, &[def.role], &[], &[]);
    let field = layer.field(def.role)?;
    let (op, phrase) = name_phrase(rng, fragment);
    let np = format!("les {} {phrase}", t.noun.pl);
    let (head, q) = open_find(rng, &t.noun, &np, true);
    let request = finish(rng, &head, q, false);
    let expected = calls(vec![select(&layer.id, &field.name, op, fragment)]);
    Some(example("filtre_texte", b, rng, request, expected))
}

/// Un filtre sur le nom : (opérateur, tournure) avec le fragment placé.
fn name_phrase(rng: &mut Rng, fragment: &str) -> (&'static str, String) {
    let (op, p) = if rng.chance(1, 2) {
        let p = *rng.pick(&[
            "dont le nom contient « {f} »",
            "dont le nom comporte {f}",
            "avec « {f} » dans le nom",
            "dont le nom contient le mot {f}",
        ]);
        ("contains", p)
    } else {
        let p = *rng.pick(&[
            "dont le nom commence par « {f} »",
            "dont le nom débute par {f}",
            "dont le nom commence par {f}",
        ]);
        ("begins_with", p)
    };
    (op, p.replace("{f}", fragment))
}

/// « Les parcelles qui ne sont pas en section AB » : groupe nominal d'un filtre `ne`.
/// `chained` : la phrase porte une autre condition après, la tournure en « sauf celles ... »
/// la rattacherait à l'exception ; elle est laissée.
fn different_np(
    rng: &mut Rng,
    noun: &Noun,
    field: &str,
    v: &str,
    phrase: &str,
    chained: bool,
) -> String {
    let roll = if chained {
        *rng.pick(&[0, 2])
    } else {
        rng.below(3)
    };
    match roll {
        0 => format!("les {} qui ne sont pas {phrase}", noun.pl),
        1 => format!(
            "{} les {} sauf {} {phrase}",
            if noun.fem { "toutes" } else { "tous" },
            noun.pl,
            noun.ceux()
        ),
        _ => format!("les {} dont le champ {field} est différent de {v}", noun.pl),
    }
}

fn filtre_different(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let t = pick_theme(rng, &b, |t| ordinary(t) && !text_fields(t).is_empty())?;
    let def = *rng.pick(&text_fields(t));
    let layer = b.add(rng, t.key, &[def.role], &[], &[]);
    let field = layer.field(def.role)?;
    let val = *rng.pick(&field.values);
    let Kind::Text { phrases, .. } = &def.kind else {
        return None;
    };
    let phrase = fill(pick_str(rng, phrases), &t.noun, val.v);
    let np = different_np(rng, &t.noun, &field.name, val.v, &phrase, false);
    let (head, q) = open_find(rng, &t.noun, &np, false);
    let request = finish(rng, &head, q, true);
    let expected = calls(vec![select(&layer.id, &field.name, "ne", val.v)]);
    Some(example("filtre_different", b, rng, request, expected))
}

// -- tampons ---------------------------------------------------------------

const DISSOLVE: &[&str] = &[
    ", fusionné en une seule zone",
    " en fusionnant les zones",
    ", d'un seul tenant",
    " et fusionne les zones qui se touchent",
];

fn tampon_head(rng: &mut Rng, d: &Dist, around: &str) -> String {
    match rng.below(6) {
        // « Fais-moi », pas « Fais une zone tampon ... autour des ... » : cette tournure nue est
        // celle de phrases des essais réels (`training/tests/recouvrement.rs`).
        0 => format!("Fais-moi une zone tampon {} autour {around}", d.de()),
        1 => format!("Crée un tampon {} autour {around}", d.de()),
        2 => format!("Trace un périmètre {} autour {around}", d.de()),
        3 => format!("Génère une zone {} autour {around}", d.de()),
        4 => format!("Tampon {} autour {around}", d.de()),
        _ => format!("Dessine une zone tampon {} autour {around}", d.de()),
    }
}

fn tampon(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let t = pick_theme(rng, &b, vector)?;
    let layer = b.add(rng, t.key, &[], &[], &[]);
    let d = dist(rng);
    let around = match rng.below(4) {
        0 => format!("de la couche « {} »", layer.name),
        1 => format!("de {}", t.the_layer),
        _ => t.noun.des(),
    };
    let dissolve = rng.chance(1, 5);
    let mut body = tampon_head(rng, &d, &around);
    if dissolve {
        body.push_str(pick_str(rng, DISSOLVE));
    }
    let request = finish(rng, &body, false, false);
    let expected = calls(vec![buffer(&layer.id, &d, dissolve)]);
    Some(example("tampon", b, rng, request, expected))
}

fn tampon_filtre(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let t = pick_theme(rng, &b, |t| ordinary(t) && !text_fields(t).is_empty())?;
    let def = *rng.pick(&text_fields(t));
    let layer = b.add(rng, t.key, &[def.role], &[], &[]);
    let field = layer.field(def.role)?;
    let val = *rng.pick(&field.values);
    let d = dist(rng);
    let around = qualified(rng, &t.noun, def, val, Det::Des);
    let dissolve = rng.chance(1, 8);
    let mut body = tampon_head(rng, &d, &around);
    if dissolve {
        body.push_str(pick_str(rng, DISSOLVE));
    }
    let request = finish(rng, &body, false, false);
    let expected = calls(vec![
        select(&layer.id, &field.name, "eq", val.v),
        buffer(&step(1), &d, dissolve),
    ]);
    Some(example("tampon_filtre", b, rng, request, expected))
}

// -- distances -------------------------------------------------------------

/// Cible et référence : deux thèmes ordinaires de groupes différents.
fn pair(rng: &mut Rng, b: &mut Builder) -> Option<(Layer, Layer)> {
    let t = pick_theme(rng, b, ordinary)?;
    let target = b.add(rng, t.key, &[], &[], &[]);
    let r = pick_theme(rng, b, ordinary)?;
    let reference = b.add(rng, r.key, &[], &[], &[]);
    Some((target, reference))
}

fn de_ref(rng: &mut Rng, noun: &Noun) -> String {
    if rng.chance(1, 2) {
        noun.dun()
    } else {
        noun.des()
    }
}

fn draft_distance(rng: &mut Rng) -> Option<Draft> {
    let mut b = Builder::new();
    let (target, reference) = pair(rng, &mut b)?;
    let d = dist(rng);
    let r = de_ref(rng, &reference.theme.noun);
    let np = format!(
        "les {} {}",
        target.theme.noun.pl,
        near(rng, &target.theme.noun, &d, &r)
    );
    Some(Draft {
        calls: vec![
            buffer(&reference.id, &d, false),
            intersect(&target.id, &step(1)),
        ],
        b,
        noun: &target.theme.noun,
        np,
    })
}

fn distance(rng: &mut Rng) -> Option<Example> {
    let d = draft_distance(rng)?;
    let (head, q) = open_find(rng, d.noun, &d.np, true);
    let request = finish(rng, &head, q, true);
    Some(example("distance", d.b, rng, request, calls(d.calls)))
}

/// Référence filtrée par un champ texte : rend la couche, le groupe « des gares de la ligne D »
/// et le `select` à placer avant le tampon.
fn filtered_reference(rng: &mut Rng, b: &mut Builder) -> Option<(Layer, String, Value)> {
    let r = pick_theme(rng, b, |t| ordinary(t) && !text_fields(t).is_empty())?;
    let def = *rng.pick(&text_fields(r));
    let layer = b.add(rng, r.key, &[def.role], &[], &[]);
    let field = layer.field(def.role)?;
    let val = *rng.pick(&field.values);
    let det = if rng.chance(1, 2) { Det::Dun } else { Det::Des };
    let words = qualified(rng, &r.noun, def, val, det);
    let sel = select(&layer.id, &field.name, "eq", val.v);
    Some((layer, words, sel))
}

fn draft_distance_ref(rng: &mut Rng) -> Option<Draft> {
    let mut b = Builder::new();
    let t = pick_theme(rng, &b, ordinary)?;
    let target = b.add(rng, t.key, &[], &[], &[]);
    let (_, words, sel) = filtered_reference(rng, &mut b)?;
    let d = dist(rng);
    let np = format!("les {} {}", t.noun.pl, near(rng, &t.noun, &d, &words));
    Some(Draft {
        calls: vec![
            sel,
            buffer(&step(1), &d, false),
            intersect(&target.id, &step(2)),
        ],
        b,
        noun: &t.noun,
        np,
    })
}

fn distance_reference_filtree(rng: &mut Rng) -> Option<Example> {
    let d = draft_distance_ref(rng)?;
    let (head, q) = open_find(rng, d.noun, &d.np, true);
    let request = finish(rng, &head, q, true);
    Some(example(
        "distance_reference_filtree",
        d.b,
        rng,
        request,
        calls(d.calls),
    ))
}

// -- zones et intersections ------------------------------------------------

/// Une condition de zone : une couche de polygones filtrée par un champ.
struct Area {
    key: &'static str,
    role: &'static str,
    phrases: &'static [&'static str],
}

const AREAS: &[Area] = &[
    Area {
        key: "zonage",
        role: "zone",
        phrases: &[
            "en zone {v}",
            "{situé} en zone {v}",
            "{situé} dans la zone {v} du PLU",
            "{classé} en zone {v}",
        ],
    },
    Area {
        key: "zonage",
        role: "zone",
        phrases: &["en zone {v} du PLU", "{localisé} en zone {v}"],
    },
    Area {
        key: "inondables",
        role: "alea",
        phrases: &[
            "en zone inondable d'aléa {v}",
            "{situé} en zone d'aléa {v}",
            "dans les zones inondables d'aléa {v}",
        ],
    },
    Area {
        key: "znieff",
        role: "type",
        phrases: &[
            "dans une ZNIEFF de type {v}",
            "{situé} en ZNIEFF de type {v}",
        ],
    },
    Area {
        key: "zones_humides",
        role: "type",
        phrases: &["dans une zone humide de type {v}"],
    },
    Area {
        key: "forets",
        role: "essence",
        phrases: &["dans une forêt à dominante de {v}"],
    },
];

/// Pose la condition de zone : rend la tournure et le `select` (à placer en tête du plan).
fn area(rng: &mut Rng, b: &mut Builder, target: &Noun) -> Option<(String, Value)> {
    let candidates: Vec<&Area> = AREAS
        .iter()
        .filter(|a| !b.free(|t| t.key == a.key).is_empty())
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let a = *rng.pick(&candidates);
    let layer = b.add(rng, a.key, &[a.role], &[], &[]);
    let field = layer.field(a.role)?;
    let val = *rng.pick(&field.values);
    let words = fill(pick_str(rng, a.phrases), target, val.v);
    Some((words, select(&layer.id, &field.name, "eq", val.v)))
}

fn draft_zone(rng: &mut Rng) -> Option<Draft> {
    let mut b = Builder::new();
    let t = pick_theme(rng, &b, |t| {
        ordinary(t) && !AREAS.iter().any(|a| a.key == t.key)
    })?;
    let target = b.add(rng, t.key, &[], &[], &[]);
    let (words, sel) = area(rng, &mut b, &t.noun)?;
    Some(Draft {
        calls: vec![sel, intersect(&target.id, &step(1))],
        b,
        noun: &t.noun,
        np: format!("les {} {words}", t.noun.pl),
    })
}

fn dans_une_zone(rng: &mut Rng) -> Option<Example> {
    let d = draft_zone(rng)?;
    let (head, q) = open_find(rng, d.noun, &d.np, true);
    let request = finish(rng, &head, q, true);
    Some(example("dans_une_zone", d.b, rng, request, calls(d.calls)))
}

/// Couches de polygones qui se lisent comme un périmètre (« dans une zone inondable »).
const PERIMETERS: &[&str] = &[
    "inondables",
    "znieff",
    "zones_humides",
    "forets",
    "espaces_verts",
];

fn intersection(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let t = pick_theme(rng, &b, |t| ordinary(t) && !PERIMETERS.contains(&t.key))?;
    let target = b.add(rng, t.key, &[], &[], &[]);
    let p = pick_theme(rng, &b, |t| PERIMETERS.contains(&t.key))?;
    let perimeter = b.add(rng, p.key, &[], &[], &[]);
    let (body, q) = match rng.below(5) {
        0 | 1 => {
            let np = format!(
                "les {} {} dans {}",
                t.noun.pl,
                t.noun.agree("situé"),
                p.noun.un()
            );
            open_find(rng, &t.noun, &np, true)
        }
        2 => (
            format!("Croise {} avec {}", t.the_layer, p.the_layer),
            false,
        ),
        3 => (
            format!(
                "Intersecte la couche « {} » avec la couche « {} »",
                target.name, perimeter.name
            ),
            false,
        ),
        _ => (
            format!(
                "Quelles parties des {} sont couvertes par les {}",
                t.noun.pl, p.noun.pl
            ),
            true,
        ),
    };
    let request = finish(rng, &body, q, false);
    let expected = calls(vec![intersect(&target.id, &perimeter.id)]);
    Some(example("intersection", b, rng, request, expected))
}

// -- conditions multiples --------------------------------------------------

/// La cible filtrée d'un plan à conditions, par égalité, nombre (`gt`, `lt`, `gte`, `lte`),
/// différence (`ne`) ou nom (`contains`, `begins_with`). Rend la couche, le groupe nominal
/// (« les bâtiments de plus de 12 m de haut »), la sélection, et le lien vers la condition
/// suivante : une virgule après une différence ou un nom, que la distance ne s'y rattache pas.
fn filtered_target(
    rng: &mut Rng,
    b: &mut Builder,
    pred: impl Fn(&Theme) -> bool,
) -> Option<(Layer, String, Value, &'static str)> {
    match rng.below(6) {
        0 | 1 => {
            let t = pick_theme(rng, b, |t| pred(t) && !text_fields(t).is_empty())?;
            let def = *rng.pick(&text_fields(t));
            let target = b.add(rng, t.key, &[def.role], &[], &[]);
            let field = target.field(def.role)?;
            let val = *rng.pick(&field.values);
            let sel = select(&target.id, &field.name, "eq", val.v);
            let head = qualified(rng, &t.noun, def, val, Det::Les);
            Some((target, head, sel, " "))
        }
        2 | 3 => {
            let t = pick_theme(rng, b, |t| pred(t) && !number_fields(t).is_empty())?;
            let def = *rng.pick(&number_fields(t));
            let target = b.add(rng, t.key, &[def.role], &[], &[]);
            let field = target.field(def.role)?;
            let (op, value, phrase) = numeric(rng, def);
            let sel = select(&target.id, &field.name, op, &value);
            Some((target, format!("les {} {phrase}", t.noun.pl), sel, " "))
        }
        4 => {
            let t = pick_theme(rng, b, |t| pred(t) && !text_fields(t).is_empty())?;
            let def = *rng.pick(&text_fields(t));
            let Kind::Text { phrases, .. } = &def.kind else {
                return None;
            };
            let target = b.add(rng, t.key, &[def.role], &[], &[]);
            let field = target.field(def.role)?;
            let val = *rng.pick(&field.values);
            let phrase = fill(pick_str(rng, phrases), &t.noun, val.v);
            let head = different_np(rng, &t.noun, &field.name, val.v, &phrase, true);
            let sel = select(&target.id, &field.name, "ne", val.v);
            Some((target, head, sel, ", "))
        }
        _ => {
            let t = pick_theme(rng, b, |t| {
                pred(t) && t.fields.iter().any(|f| matches!(f.kind, Kind::Name { .. }))
            })?;
            let def = t
                .fields
                .iter()
                .find(|f| matches!(f.kind, Kind::Name { .. }))?;
            let Kind::Name { fragments } = &def.kind else {
                return None;
            };
            let fragment = *rng.pick(fragments);
            let target = b.add(rng, t.key, &[def.role], &[], &[]);
            let field = target.field(def.role)?;
            let (op, phrase) = name_phrase(rng, fragment);
            let sel = select(&target.id, &field.name, op, fragment);
            Some((target, format!("les {} {phrase}", t.noun.pl), sel, ", "))
        }
    }
}

fn draft_multi(rng: &mut Rng) -> Option<Draft> {
    let mut b = Builder::new();
    match rng.below(9) {
        // Zone puis distance : select(zone), buffer(réf), intersect(cible, 1), intersect(3, 2).
        0 => {
            let t = pick_theme(rng, &b, |t| {
                ordinary(t) && !AREAS.iter().any(|a| a.key == t.key)
            })?;
            let target = b.add(rng, t.key, &[], &[], &[]);
            let (zone, sel) = area(rng, &mut b, &t.noun)?;
            let r = pick_theme(rng, &b, ordinary)?;
            let reference = b.add(rng, r.key, &[], &[], &[]);
            let d = dist(rng);
            let rr = de_ref(rng, &r.noun);
            let link = if rng.chance(1, 3) { " et " } else { " " };
            let np = format!(
                "les {} {zone}{link}{}",
                t.noun.pl,
                near(rng, &t.noun, &d, &rr)
            );
            Some(Draft {
                calls: vec![
                    sel,
                    buffer(&reference.id, &d, false),
                    intersect(&target.id, &step(1)),
                    intersect(&step(3), &step(2)),
                ],
                b,
                noun: &t.noun,
                np,
            })
        }
        // Cible filtrée puis distance : select(cible), buffer(réf), intersect(1, 2).
        1 => {
            let (target, head, sel, sep) = filtered_target(rng, &mut b, ordinary)?;
            let t = target.theme;
            let r = pick_theme(rng, &b, ordinary)?;
            let reference = b.add(rng, r.key, &[], &[], &[]);
            let d = dist(rng);
            let rr = de_ref(rng, &r.noun);
            let np = format!("{head}{sep}{}", near(rng, &t.noun, &d, &rr));
            Some(Draft {
                calls: vec![
                    sel,
                    buffer(&reference.id, &d, false),
                    intersect(&step(1), &step(2)),
                ],
                b,
                noun: &t.noun,
                np,
            })
        }
        // Zone puis distance à une référence filtrée.
        2 => {
            let t = pick_theme(rng, &b, |t| {
                ordinary(t) && !AREAS.iter().any(|a| a.key == t.key)
            })?;
            let target = b.add(rng, t.key, &[], &[], &[]);
            let (zone, sel_zone) = area(rng, &mut b, &t.noun)?;
            let (_, words, sel_ref) = filtered_reference(rng, &mut b)?;
            let d = dist(rng);
            let np = format!(
                "les {} {zone} {}",
                t.noun.pl,
                near(rng, &t.noun, &d, &words)
            );
            Some(Draft {
                calls: vec![
                    sel_zone,
                    sel_ref,
                    buffer(&step(2), &d, false),
                    intersect(&target.id, &step(1)),
                    intersect(&step(4), &step(3)),
                ],
                b,
                noun: &t.noun,
                np,
            })
        }
        // Deux distances : buffer(r1), buffer(r2), intersect(cible, 1), intersect(3, 2).
        3 => {
            let (target, r1) = pair(rng, &mut b)?;
            let r = pick_theme(rng, &b, ordinary)?;
            let r2 = b.add(rng, r.key, &[], &[], &[]);
            let (d1, d2) = (dist(rng), dist(rng));
            let t = &target.theme.noun;
            let first = format!("à moins {} {}", d1.de(), de_ref(rng, &r1.theme.noun));
            let second = format!("à moins {} {}", d2.de(), de_ref(rng, &r2.theme.noun));
            let np = format!("les {} {first} et {second}", t.pl);
            Some(Draft {
                calls: vec![
                    buffer(&r1.id, &d1, false),
                    buffer(&r2.id, &d2, false),
                    intersect(&target.id, &step(1)),
                    intersect(&step(3), &step(2)),
                ],
                b,
                noun: t,
                np,
            })
        }
        // Trois conditions, cible filtrée, zone et distance, dans l'ordre de la phrase :
        // select(cible), select(zone), buffer(réf), intersect(1, 2), intersect(4, 3).
        4 => {
            let (target, head, sel, sep) = filtered_target(rng, &mut b, |t| {
                ordinary(t) && !AREAS.iter().any(|a| a.key == t.key)
            })?;
            let t = target.theme;
            let (zone, sel_zone) = area(rng, &mut b, &t.noun)?;
            let r = pick_theme(rng, &b, ordinary)?;
            let reference = b.add(rng, r.key, &[], &[], &[]);
            let d = dist(rng);
            let rr = de_ref(rng, &r.noun);
            let link = *rng.pick(&[" ", ", ", " et "]);
            let np = format!("{head}{sep}{zone}{link}{}", near(rng, &t.noun, &d, &rr));
            Some(Draft {
                calls: vec![
                    sel,
                    sel_zone,
                    buffer(&reference.id, &d, false),
                    intersect(&step(1), &step(2)),
                    intersect(&step(4), &step(3)),
                ],
                b,
                noun: &t.noun,
                np,
            })
        }
        // Trois conditions, une zone et deux distances : select(zone), buffer(r1), buffer(r2),
        // intersect(cible, 1), intersect(4, 2), intersect(5, 3).
        5 => {
            let t = pick_theme(rng, &b, |t| {
                ordinary(t) && !AREAS.iter().any(|a| a.key == t.key)
            })?;
            let target = b.add(rng, t.key, &[], &[], &[]);
            let (zone, sel) = area(rng, &mut b, &t.noun)?;
            let r = pick_theme(rng, &b, ordinary)?;
            let r1 = b.add(rng, r.key, &[], &[], &[]);
            let r = pick_theme(rng, &b, ordinary)?;
            let r2 = b.add(rng, r.key, &[], &[], &[]);
            let (d1, d2) = (dist(rng), dist(rng));
            let first = format!("à moins {} {}", d1.de(), de_ref(rng, &r1.theme.noun));
            let second = format!("à moins {} {}", d2.de(), de_ref(rng, &r2.theme.noun));
            let np = format!("les {} {zone}, {first} et {second}", t.noun.pl);
            Some(Draft {
                calls: vec![
                    sel,
                    buffer(&r1.id, &d1, false),
                    buffer(&r2.id, &d2, false),
                    intersect(&target.id, &step(1)),
                    intersect(&step(4), &step(2)),
                    intersect(&step(5), &step(3)),
                ],
                b,
                noun: &t.noun,
                np,
            })
        }
        // Cible filtrée et deux distances : select(cible), buffer(r1), buffer(r2),
        // intersect(1, 2), intersect(4, 3).
        6 => {
            let (target, head, sel, sep) = filtered_target(rng, &mut b, ordinary)?;
            let t = target.theme;
            let r = pick_theme(rng, &b, ordinary)?;
            let r1 = b.add(rng, r.key, &[], &[], &[]);
            let r = pick_theme(rng, &b, ordinary)?;
            let r2 = b.add(rng, r.key, &[], &[], &[]);
            let (d1, d2) = (dist(rng), dist(rng));
            let rr1 = de_ref(rng, &r1.theme.noun);
            let first = near(rng, &t.noun, &d1, &rr1);
            let second = format!("à moins {} {}", d2.de(), de_ref(rng, &r2.theme.noun));
            let np = format!("{head}{sep}{first} et {second}");
            Some(Draft {
                calls: vec![
                    sel,
                    buffer(&r1.id, &d1, false),
                    buffer(&r2.id, &d2, false),
                    intersect(&step(1), &step(2)),
                    intersect(&step(4), &step(3)),
                ],
                b,
                noun: &t.noun,
                np,
            })
        }
        // Distance et inclusion dans une couche entière (« situées dans une zone humide »),
        // cible parfois filtrée : [select(cible)], buffer(réf), puis les intersections dans
        // l'ordre de la phrase, la couche entière prise telle quelle.
        _ => {
            let pred = |t: &Theme| ordinary(t) && !PERIMETERS.contains(&t.key);
            let (target_ref, head, mut steps, sep, noun): (
                String,
                String,
                Vec<Value>,
                &str,
                &'static Noun,
            ) = if rng.chance(1, 2) {
                let (target, head, sel, sep) = filtered_target(rng, &mut b, pred)?;
                (step(1), head, vec![sel], sep, &target.theme.noun)
            } else {
                let t = pick_theme(rng, &b, pred)?;
                let target = b.add(rng, t.key, &[], &[], &[]);
                (
                    target.id,
                    format!("les {}", t.noun.pl),
                    Vec::new(),
                    " ",
                    &t.noun,
                )
            };
            let p = pick_theme(rng, &b, |t| PERIMETERS.contains(&t.key))?;
            let perimeter = b.add(rng, p.key, &[], &[], &[]);
            let r = pick_theme(rng, &b, ordinary)?;
            let reference = b.add(rng, r.key, &[], &[], &[]);
            let d = dist(rng);
            let inside = match rng.below(3) {
                0 => format!("dans {}", p.noun.un()),
                1 => format!("{} dans {}", noun.agree("situé"), p.noun.un()),
                _ => format!("{} dans {}", noun.agree("localisé"), p.noun.les()),
            };
            let rr = de_ref(rng, &r.noun);
            let close = near(rng, noun, &d, &rr);
            steps.push(buffer(&reference.id, &d, false));
            let buf = step(steps.len());
            let np = if rng.chance(1, 2) {
                steps.push(intersect(&target_ref, &perimeter.id));
                let k = steps.len();
                steps.push(intersect(&step(k), &buf));
                format!("{head}{sep}{inside} et {close}")
            } else {
                steps.push(intersect(&target_ref, &buf));
                let k = steps.len();
                steps.push(intersect(&step(k), &perimeter.id));
                format!("{head}{sep}{close} et {inside}")
            };
            Some(Draft {
                calls: steps,
                b,
                noun,
                np,
            })
        }
    }
}

/// Plafond d'étapes d'un plan engendré, PDF compris : trois conditions et l'export tiennent
/// en six étapes.
const MAX_DRAFT_STEPS: usize = 6;

fn conditions_multiples(rng: &mut Rng) -> Option<Example> {
    let mut d = draft_multi(rng)?;
    let with_pdf = d.calls.len() < MAX_DRAFT_STEPS && rng.chance(1, 4);
    let (mut head, q) = open_find(rng, d.noun, &d.np, !with_pdf);
    if with_pdf {
        let (clause, title) = pdf_clause(rng, &d);
        head.push_str(&clause);
        d.calls.push(pdf(&title));
    }
    let request = finish(rng, &head, q, !with_pdf);
    Some(example(
        "conditions_multiples",
        d.b,
        rng,
        request,
        calls(d.calls),
    ))
}

/// « Constructible » : zone urbaine du standard CNIG, `typezone` égal à `U`. Les zones `AUc`
/// sont aussi constructibles, mais le registre n'a ni `in` ni union : la tournure s'en tient à `U`.
fn draft_constructible(rng: &mut Rng) -> Option<Draft> {
    let mut b = Builder::new();
    let parcelles = b.add(rng, "parcelles", &[], &[], &[]);
    let noun = &theme("parcelles").noun;
    let words = *rng.pick(&[
        "constructibles",
        "constructibles du PLU",
        "en zone constructible",
        "{situé} en zone constructible",
        "en zone urbaine du PLU",
        "{situé} en zone U",
        "{classé} en zone U du PLU",
    ]);
    // « zone urbaine » serait ambigu avec un libellé « Zone urbaine » : pas de libellé alors.
    let forbid: &[&str] = if words.contains("urbaine") {
        &["libelle"]
    } else {
        &[]
    };
    let zonage = b.add(rng, "zonage", &["zone"], forbid, &[("zone", "U")]);
    let zone_field = zonage.field("zone")?;
    let words = fill(words, noun, "");
    let sel = select(&zonage.id, &zone_field.name, "eq", "U");
    if rng.chance(1, 4) {
        return Some(Draft {
            calls: vec![sel, intersect(&parcelles.id, &step(1))],
            b,
            noun,
            np: format!("les parcelles {words}"),
        });
    }
    let r = pick_theme(rng, &b, |t| {
        ordinary(t) && matches!(t.geom, Geom::Point | Geom::Line)
    })?;
    let reference = b.add(rng, r.key, &[], &[], &[]);
    let d = dist(rng);
    let rr = de_ref(rng, &r.noun);
    let np = format!("les parcelles {words} {}", near(rng, noun, &d, &rr));
    Some(Draft {
        calls: vec![
            sel,
            buffer(&reference.id, &d, false),
            intersect(&parcelles.id, &step(1)),
            intersect(&step(3), &step(2)),
        ],
        b,
        noun,
        np,
    })
}

fn constructible(rng: &mut Rng) -> Option<Example> {
    let mut d = draft_constructible(rng)?;
    let with_pdf = rng.chance(1, 3);
    let (mut body, q) = open_find(rng, d.noun, &d.np, !with_pdf);
    if with_pdf {
        let (clause, title) = pdf_clause(rng, &d);
        body.push_str(&clause);
        d.calls.push(pdf(&title));
    }
    let request = finish(rng, &body, q, false);
    Some(example("constructible", d.b, rng, request, calls(d.calls)))
}

// -- export PDF ------------------------------------------------------------

/// Proposition qui demande le PDF, et le titre attendu (donné par l'utilisateur ou déduit).
fn pdf_clause(rng: &mut Rng, d: &Draft) -> (String, String) {
    if rng.chance(1, 3) {
        let title = (*rng.pick(USER_TITLES)).to_string();
        let clause = match rng.below(3) {
            0 => format!(" et exporte la carte en PDF avec le titre « {title} »"),
            1 => format!(", puis sors un PDF intitulé « {title} »"),
            _ => format!(" et fais une carte PDF titrée « {title} »"),
        };
        return (clause, title);
    }
    let clause = *rng.pick(&[
        " et sors-moi la carte en PDF",
        ", puis exporte la carte en PDF",
        " et fais-en une carte en PDF",
        ", puis sors le résultat en PDF",
        " et exporte la carte au format PDF",
        ", puis produis un PDF de la carte",
        " et mets-moi ça dans un PDF",
    ]);
    (clause.to_string(), d.title())
}

/// Dénominateurs d'échelle courants (ticket 5.38).
const SCALES: [u64; 12] = [
    1000, 2000, 2500, 5000, 10000, 15000, 20000, 25000, 50000, 100000, 200000, 250000,
];

/// L'échelle telle qu'on l'écrit : « au 1:25 000 », « au 1/25000 », « au 25 000e »,
/// « à l'échelle 1/25 000 », « au 1:25000 », « au 1 : 25 000 », « au 25 000ème »,
/// « au 1:25k » (multiple de 1 000 seulement).
fn scale_text(rng: &mut Rng, scale: u64) -> String {
    let forms = if scale.is_multiple_of(1000) { 8 } else { 7 };
    match rng.below(forms) {
        0 => format!("au 1:{}", thousands(scale)),
        1 => format!("au 1/{scale}"),
        2 => format!("au {}e", thousands(scale)),
        3 => format!("à l'échelle 1/{}", thousands(scale)),
        4 => format!("au 1:{scale}"),
        5 => format!("au 1 : {}", thousands(scale)),
        6 => format!("au {}ème", thousands(scale)),
        _ => format!("au 1:{}k", scale / 1000),
    }
}

/// Export PDF à une échelle écrite dans la demande : la carte seule, ou au bout d'une
/// recherche. Le plan porte `scale`, l'entier écrit.
fn export_pdf_scale(rng: &mut Rng) -> Option<Example> {
    let scale = *rng.pick(&SCALES);
    let at = scale_text(rng, scale);
    if rng.chance(2, 3) {
        // La carte seule.
        let mut b = Builder::new();
        b.distract(rng, 2, vector);
        let titled = rng.chance(1, 2);
        let title = if titled {
            (*rng.pick(USER_TITLES)).to_string()
        } else {
            "Carte du projet".to_string()
        };
        // 5.40 : une demande à l'échelle sans le mot « PDF » est un export, jamais un refus de
        // format (ceux-là nomment un papier ou une orientation). Jamais la tête exacte
        // « Exporte la carte {at} », phrase E01 d'`echelle_utm_reel`.
        let head = match rng.below(10) {
            0 => format!("Exporte la carte en PDF {at}"),
            1 => format!("Sors la carte {at} en PDF"),
            2 => format!("Fais-moi un PDF de la carte {at}"),
            3 => format!("Produis la carte {at} au format PDF"),
            4 => format!("Génère un PDF de la carte {at}"),
            5 => format!("Imprime la carte {at}"),
            6 => format!("Sors-moi la carte {at}"),
            7 => format!("Exporte-moi la carte {at}"),
            8 => format!("Fais une mise en page de la carte {at}"),
            _ => format!("Imprime-moi la carte du projet {at}"),
        };
        let body = if titled {
            match rng.below(2) {
                0 => format!("{head} avec le titre « {title} »"),
                _ => format!("{head}, intitulé « {title} »"),
            }
        } else {
            head
        };
        let request = finish(rng, &body, false, false);
        return Some(example(
            "export_pdf_scale",
            b,
            rng,
            request,
            calls(vec![pdf_at(&title, scale)]),
        ));
    }
    let mut d = match rng.below(3) {
        0 => draft_filter_eq(rng)?,
        1 => draft_distance(rng)?,
        _ => draft_zone(rng)?,
    };
    if d.calls.len() >= MAX_DRAFT_STEPS {
        return None;
    }
    let (mut body, _) = open_find(rng, d.noun, &d.np, false);
    let (clause, title) = if rng.chance(1, 3) {
        let title = (*rng.pick(USER_TITLES)).to_string();
        (
            format!(", puis sors un PDF {at} intitulé « {title} »"),
            title,
        )
    } else {
        let clause = match rng.below(5) {
            0 => format!(" et sors-moi la carte en PDF {at}"),
            1 => format!(", puis exporte la carte {at} en PDF"),
            2 => format!(" et fais-en une carte en PDF {at}"),
            // 5.40 : sans le mot « PDF ».
            3 => format!(" et imprime la carte {at}"),
            _ => format!(", puis sors la carte {at}"),
        };
        (clause, d.title())
    };
    body.push_str(&clause);
    d.calls.push(pdf_at(&title, scale));
    let request = finish(rng, &body, false, false);
    Some(example(
        "export_pdf_scale",
        d.b,
        rng,
        request,
        calls(d.calls),
    ))
}

/// Formats de page du catalogue : valeur du plan et forme écrite (11.12, lot 3).
const PAGE_SIZES: [(&str, &str); 5] = [
    ("a4", "A4"),
    ("a3", "A3"),
    ("a2", "A2"),
    ("a1", "A1"),
    ("a0", "A0"),
];

/// Intervalles d'un graticule en degrés, tels qu'écrits à la française (« 0,01 »).
const DEG_STEPS: [f64; 8] = [0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0];
/// Intervalles d'un graticule en grades.
const GON_STEPS: [f64; 5] = [0.5, 1.0, 5.0, 10.0, 20.0];
/// Intervalles d'un quadrillage projeté : valeur et unité du registre.
const GRID_STEPS: [(u64, &str); 9] = [
    (100, "m"),
    (200, "m"),
    (250, "m"),
    (500, "m"),
    (1000, "m"),
    (1, "km"),
    (2, "km"),
    (5, "km"),
    (10, "km"),
];

/// Un nombre décimal écrit à la française : « 0,01 », « 5 ».
fn decimal_fr(v: f64) -> String {
    format!("{v}").replace('.', ",")
}

/// Valeur JSON d'un intervalle : entière si elle l'est (`1`, pas `1.0`).
fn number_json(v: f64) -> Value {
    if v.fract() == 0.0 {
        json!(v as u64)
    } else {
        json!(v)
    }
}

/// Un carroyage demandé : la proposition écrite (espace en tête) et le `grid` du plan, ou
/// `None` quand la demande ne nomme pas son système (décision 1 du propriétaire, 2026-11-07 :
/// la réponse est alors une question).
fn grid_words(rng: &mut Rng, with_system: bool) -> (String, Option<Value>) {
    if !with_system {
        let text = match rng.below(5) {
            0 => " avec un carroyage UTM au kilomètre".to_string(),
            1 => format!(
                " avec un quadrillage tous les {}",
                distance_step_text(rng, 500, "m")
            ),
            2 => format!(
                " avec un carroyage UTM tous les {}",
                distance_step_text(rng, 1, "km")
            ),
            3 => format!(" avec une grille de {}", distance_step_text(rng, 1, "km")),
            _ => " avec un carroyage kilométrique".to_string(),
        };
        return (text, None);
    }
    if rng.chance(1, 3) {
        // Graticule, en degrés ou en grades.
        let gon = rng.chance(1, 3);
        let v = if gon {
            *rng.pick(&GON_STEPS)
        } else {
            *rng.pick(&DEG_STEPS)
        };
        let n = decimal_fr(v);
        let iv = if gon {
            match rng.below(2) {
                0 => format!("{n} gr"),
                _ if v >= 2.0 => format!("{n} grades"),
                _ => format!("{n} grade"),
            }
        } else {
            match rng.below(3) {
                0 | 1 => format!("{n}°"),
                _ if v >= 2.0 => format!("{n} degrés"),
                _ => format!("{n} degré"),
            }
        };
        let text = match rng.below(4) {
            0 => format!(" avec un graticule tous les {iv}"),
            1 => format!(" avec un graticule de {iv}"),
            2 => format!(" avec un carroyage géographique tous les {iv}"),
            _ => format!(" avec un quadrillage géographique de {iv}"),
        };
        let unit = if gon { "gon" } else { "deg" };
        let grid = json!({"kind": "degrees",
            "interval": {"value": number_json(v), "unit": unit}});
        return (text, Some(grid));
    }
    let (epsg, sys) = if rng.chance(1, 2) {
        (2154, (*rng.pick(&["Lambert-93", "Lambert 93"])).to_string())
    } else {
        let zone = rng.range(30, 32);
        let sys = match rng.below(4) {
            0 => format!("UTM {zone}N"),
            1 => format!("UTM {zone} N"),
            2 => format!("UTM zone {zone} nord"),
            _ => format!("UTM {zone} Nord"),
        };
        (32600 + zone as u32, sys)
    };
    let (value, unit) = *rng.pick(&GRID_STEPS);
    let iv = distance_step_text(rng, value, unit);
    let text = match rng.below(4) {
        0 => format!(" avec un carroyage {sys} tous les {iv}"),
        1 => format!(" avec un quadrillage {sys} de {iv}"),
        2 => format!(" avec une grille {sys} tous les {iv}"),
        _ => format!(" avec un carroyage {sys} de {iv}"),
    };
    let grid = json!({"kind": "projected", "epsg": epsg,
        "interval": {"value": value, "unit": unit}});
    (text, Some(grid))
}

/// Un intervalle de quadrillage écrit avec son unité : « 1 000 m », « 500 mètres », « 2 km ».
fn distance_step_text(rng: &mut Rng, value: u64, unit: &str) -> String {
    let n = thousands(value);
    match (unit, rng.below(2)) {
        ("m", 0) => format!("{n} m"),
        ("m", _) => format!("{n} mètres"),
        (_, 0) => format!("{n} km"),
        _ if value >= 2 => format!("{n} kilomètres"),
        _ => format!("{n} kilomètre"),
    }
}

/// Format et orientation écrits (espace en tête) ; `portrait` : `Some(true)` portrait,
/// `Some(false)` paysage.
fn page_words(rng: &mut Rng, size: Option<&str>, portrait: Option<bool>) -> String {
    let o = |p: bool| if p { "portrait" } else { "paysage" };
    match (size, portrait) {
        (Some(f), None) => match rng.below(4) {
            0 => format!(" au format {f}"),
            1 => format!(" en {f}"),
            2 => format!(" sur une feuille {f}"),
            _ => format!(" en format {f}"),
        },
        (None, Some(p)) => match rng.below(4) {
            0 => format!(" en {}", o(p)),
            1 => (if p {
                " à la française"
            } else {
                " à l'italienne"
            })
            .to_string(),
            2 => format!(" en format {}", o(p)),
            _ => format!(" en mode {}", o(p)),
        },
        (Some(f), Some(p)) => match rng.below(4) {
            0 => format!(" au format {f} {}", o(p)),
            1 => format!(" en {f} {}", o(p)),
            2 => format!(" sur une feuille {f} {}", o(p)),
            _ => format!(
                " en {f}{}",
                if p {
                    " à la française"
                } else {
                    " à l'italienne"
                }
            ),
        },
        (None, None) => String::new(),
    }
}

/// Question quand le carroyage demandé ne nomme pas son système.
const GRID_SYSTEM_QUESTION: &str = "Dans quel système veux-tu le carroyage : Lambert-93, UTM avec sa zone (30N, 31N ou 32N), ou un graticule en degrés ou en grades ?";

/// Export PDF au format, à l'orientation et/ou avec le carroyage écrits dans la demande
/// (11.12, lot 3) : la carte seule, ou au bout d'une recherche une fois sur trois. Un
/// paramètre n'est au plan que si la demande le nomme ; jamais `scale` (`export_pdf_scale`).
/// Un carroyage sans système (une fois sur huit) mène à une question.
fn export_pdf_format(rng: &mut Rng) -> Option<Example> {
    let asks = rng.chance(1, 8);
    // Au moins un des trois ; le carroyage toujours quand il manque son système.
    let mask = if asks {
        4 | rng.below(4)
    } else {
        1 + rng.below(7)
    };
    let size = (mask & 1 != 0).then(|| *rng.pick(&PAGE_SIZES));
    let portrait = (mask & 2 != 0).then(|| rng.chance(1, 2));
    let pw = page_words(rng, size.map(|s| s.1), portrait);
    let (gw, grid) = if mask & 4 != 0 {
        grid_words(rng, !asks)
    } else {
        (String::new(), None)
    };
    let params = |title: &str| {
        let mut p = json!({"title": title});
        if let Some((v, _)) = size {
            p["page_size"] = json!(v);
        }
        if let Some(port) = portrait {
            p["orientation"] = json!(if port { "portrait" } else { "landscape" });
        }
        if let Some(g) = &grid {
            p["grid"] = g.clone();
        }
        json!({"command": "layout.export_pdf", "params": p})
    };
    if rng.chance(2, 3) {
        // La carte seule.
        let mut b = Builder::new();
        b.distract(rng, 2, vector);
        let titled = rng.chance(1, 2);
        let title = if titled {
            (*rng.pick(USER_TITLES)).to_string()
        } else {
            "Carte du projet".to_string()
        };
        let head = *rng.pick(&[
            "Exporte la carte en PDF",
            "Sors la carte",
            "Fais un PDF de la carte",
            "Produis la carte en PDF",
            "Génère un PDF de la carte",
            "Sors-moi la carte en PDF",
            "Exporte la carte",
        ]);
        let mut body = format!("{head}{pw}{gw}");
        if titled {
            match rng.below(2) {
                0 => body.push_str(&format!(", avec le titre « {title} »")),
                _ => body.push_str(&format!(", intitulée « {title} »")),
            }
        }
        let request = finish(rng, &body, false, false);
        let expected = if asks {
            question(GRID_SYSTEM_QUESTION.to_string())
        } else {
            calls(vec![params(&title)])
        };
        return Some(example("export_pdf_format", b, rng, request, expected));
    }
    let mut d = match rng.below(3) {
        0 => draft_filter_eq(rng)?,
        1 => draft_distance(rng)?,
        _ => draft_zone(rng)?,
    };
    if d.calls.len() >= MAX_DRAFT_STEPS {
        return None;
    }
    let (mut body, _) = open_find(rng, d.noun, &d.np, false);
    let (clause, title) = if rng.chance(1, 3) {
        let title = (*rng.pick(USER_TITLES)).to_string();
        (
            format!(", puis sors un PDF{pw}{gw} intitulé « {title} »"),
            title,
        )
    } else {
        let clause = match rng.below(3) {
            0 => format!(" et sors-moi la carte en PDF{pw}{gw}"),
            1 => format!(", puis exporte la carte{pw}{gw}"),
            _ => format!(" et fais-en une carte en PDF{pw}{gw}"),
        };
        (clause, d.title())
    };
    body.push_str(&clause);
    let request = finish(rng, &body, false, false);
    let expected = if asks {
        question(GRID_SYSTEM_QUESTION.to_string())
    } else {
        d.calls.push(params(&title));
        calls(d.calls)
    };
    Some(example("export_pdf_format", d.b, rng, request, expected))
}

fn export_pdf(rng: &mut Rng) -> Option<Example> {
    if rng.chance(1, 3) {
        // La carte seule.
        let mut b = Builder::new();
        b.distract(rng, 2, vector);
        let titled = rng.chance(1, 2);
        let title = if titled {
            (*rng.pick(USER_TITLES)).to_string()
        } else {
            "Carte du projet".to_string()
        };
        let head = *rng.pick(&[
            "Sors la carte en PDF",
            "Exporte la carte en PDF",
            "Fais-moi un PDF de la carte",
            "Produis la carte au format PDF",
            "Exporte la carte actuelle en PDF",
            "Génère un PDF de la carte",
        ]);
        let body = if titled {
            match rng.below(3) {
                0 => format!("{head} avec le titre « {title} »"),
                1 => format!("{head}, intitulé « {title} »"),
                _ => format!("{head}, titre : {title}"),
            }
        } else {
            head.to_string()
        };
        let request = finish(rng, &body, false, false);
        return Some(example(
            "export_pdf",
            b,
            rng,
            request,
            calls(vec![pdf(&title)]),
        ));
    }
    let mut d = match rng.below(5) {
        0 => draft_filter_eq(rng)?,
        1 => draft_distance(rng)?,
        2 => draft_zone(rng)?,
        3 => draft_distance_ref(rng)?,
        _ => draft_multi(rng)?,
    };
    if d.calls.len() >= MAX_DRAFT_STEPS {
        return None;
    }
    let (mut body, _) = open_find(rng, d.noun, &d.np, false);
    let (clause, title) = pdf_clause(rng, &d);
    body.push_str(&clause);
    d.calls.push(pdf(&title));
    let request = finish(rng, &body, false, false);
    Some(example("export_pdf", d.b, rng, request, calls(d.calls)))
}

// -- isochrone, décompte ---------------------------------------------------

/// Un trajet : mode et durée, tels qu'écrits et tels qu'attendus.
struct Travel {
    walk: bool,
    mode: &'static str,
    value: u64,
    unit: &'static str,
    duration: String,
}

impl Travel {
    /// « de 10 minutes », « d'un quart d'heure ».
    fn de(&self) -> String {
        de_word(&self.duration)
    }
}

/// « de 10 minutes », « d'une gare », « d'un quart d'heure ».
fn de_word(s: &str) -> String {
    if s.starts_with("un ") || s.starts_with("une ") {
        format!("d'{s}")
    } else {
        format!("de {s}")
    }
}

fn travel(rng: &mut Rng) -> Travel {
    let walk = rng.chance(1, 2);
    let mode = if walk {
        *rng.pick(&["à pied", "en marchant", "à la marche"])
    } else {
        *rng.pick(&["en voiture", "en auto", "en véhicule"])
    };
    let (value, unit, duration): (u64, &str, String) = if rng.chance(1, 6) {
        let s = *rng.pick(&[90u64, 120, 180, 300]);
        let text = if rng.chance(1, 2) {
            format!("{s} secondes")
        } else {
            format!("{s} s")
        };
        (s, "s", text)
    } else {
        let m = *rng.pick(&[5u64, 8, 10, 12, 15, 20, 25, 30, 45]);
        let text = match (m, rng.below(3)) {
            (15, 0) => "un quart d'heure".to_string(),
            (30, 0) => "une demi-heure".to_string(),
            (_, 1) => format!("{m} min"),
            _ => format!("{m} minutes"),
        };
        (m, "min", text)
    };
    Travel {
        walk,
        mode,
        value,
        unit,
        duration,
    }
}

/// Un point de départ donné en coordonnées : le point attendu, et « le point ... ».
struct Origin {
    point: Value,
    place: String,
    /// Les mêmes coordonnées, à accoler à un lieu nommé : « la gare de X (652 000 /
    /// 6 862 000, Lambert-93) », « l'école Jean Moulin, X = ..., Y = ... en EPSG:2154 ».
    coords: String,
}

fn origin_point(rng: &mut Rng) -> Option<Origin> {
    if rng.chance(1, 5) {
        let lon = rng.range(22_000, 26_000);
        let lat = rng.range(486_000, 489_000);
        let (xs, ys) = (
            format!("{}.{:04}", lon / 10_000, lon % 10_000),
            format!("{}.{:04}", lat / 10_000, lat % 10_000),
        );
        let x: f64 = xs.parse().ok()?;
        let y: f64 = ys.parse().ok()?;
        let crs = *rng.pick(&["WGS84", "EPSG:4326", "WGS 84"]);
        return Some(Origin {
            point: json!({"x": x, "y": y, "epsg": 4326}),
            place: format!("le point de longitude {xs} et de latitude {ys} ({crs})"),
            coords: format!(" (longitude {xs}, latitude {ys}, {crs})"),
        });
    }
    let x = rng.range(60_000, 70_000) * 10;
    let y = rng.range(678_000, 690_000) * 10;
    let (xs, ys) = if rng.chance(1, 2) {
        (thousands(x), thousands(y))
    } else {
        (x.to_string(), y.to_string())
    };
    let crs = *rng.pick(&["Lambert-93", "Lambert 93", "EPSG:2154"]);
    let roll = rng.below(3);
    let place = match roll {
        0 => format!("le point {xs} / {ys} ({crs})"),
        1 => format!("le point X = {xs}, Y = {ys} en {crs}"),
        _ => format!("le point ({xs} ; {ys}), {crs}"),
    };
    let coords = if roll == 1 {
        format!(", X = {xs}, Y = {ys} en {crs}")
    } else {
        format!(" ({xs} / {ys}, {crs})")
    };
    Some(Origin {
        point: json!({"x": x, "y": y, "epsg": 2154}),
        place,
        coords,
    })
}

/// Un point donné en UTM du WGS84, zone 31N ou 32N (la France métropolitaine) : le garde-fou
/// lit la zone avec son hémisphère (« UTM 31N » : 32631). Pas de zone sud au générateur.
fn utm_origin(rng: &mut Rng) -> Option<Origin> {
    let zone = if rng.chance(1, 2) { 31 } else { 32 };
    utm_origin_in(rng, zone)
}

/// Un point en UTM du WGS84, dans la zone nord `zone` (31 ou 32).
fn utm_origin_in(rng: &mut Rng, zone: u32) -> Option<Origin> {
    let x = if zone == 31 {
        rng.range(40_000, 70_000) * 10
    } else {
        rng.range(28_000, 45_000) * 10
    };
    let y = rng.range(530_000, 550_000) * 10;
    let (xs, ys) = if rng.chance(1, 2) {
        (thousands(x), thousands(y))
    } else {
        (x.to_string(), y.to_string())
    };
    let crs = match rng.below(4) {
        0 => format!("UTM {zone}N"),
        1 => format!("UTM {zone} N"),
        2 => format!("UTM zone {zone} Nord"),
        _ => format!("UTM{zone}N"),
    };
    let place = match rng.below(3) {
        0 => format!("le point {xs} / {ys} ({crs})"),
        1 => format!("le point X = {xs}, Y = {ys} en {crs}"),
        _ => format!("le point ({xs} ; {ys}), {crs}"),
    };
    Some(Origin {
        point: json!({"x": x, "y": y, "epsg": 32600 + zone}),
        coords: format!(" ({xs} / {ys}, {crs})"),
        place,
    })
}

/// Le réseau d'une isochrone : une seule couche de lignes, qu'il ne soit pas ambigu.
fn network(rng: &mut Rng, b: &mut Builder) -> Layer {
    let key = *rng.pick(&["routes", "reseau"]);
    let layer = b.add(rng, key, &[], &[], &[]);
    for other in ["pistes", "lignes_elec", "canalisations", "rivieres"] {
        b.exclude(other);
    }
    layer
}

fn isochrone_call(network: &str, origin: &Value, tr: &Travel) -> Value {
    json!({"command": "analysis.isochrone", "params": {
        "layer_id": network, "origin": origin,
        "mode": if tr.walk { "walk" } else { "car" },
        "duration": {"value": tr.value, "unit": tr.unit}}})
}

/// Une cible définie par un trajet : « à 10 min à pied de la gare de X », « accessibles en
/// 10 min à pied depuis le point ... ». `from` est le lieu tel qu'après « depuis » (« la gare
/// de X », « le point ... », « chaque gare »).
fn travel_clause(rng: &mut Rng, noun: &Noun, tr: &Travel, from: &str) -> String {
    let de_from = if let Some(rest) = from.strip_prefix("le ") {
        format!("du {rest}")
    } else if let Some(rest) = from.strip_prefix("les ") {
        format!("des {rest}")
    } else {
        de_word(from)
    };
    let (d, mode) = (&tr.duration, tr.mode);
    let mut roll = rng.below(6);
    // « à N minutes à pied de ... » nu est la tournure d'une phrase des essais réels
    // (`training/tests/recouvrement.rs`) : il passe alors par « situées à ».
    if roll < 2 && d.ends_with("minutes") && mode == "à pied" {
        roll = 2;
    }
    let t = match roll {
        0 | 1 => format!("à {d} {mode} {de_from}"),
        2 => format!("{{situé}} à {d} {mode} {de_from}"),
        3 => format!("à moins {} {mode} {de_from}", tr.de()),
        4 => format!("accessibles en {d} {mode} depuis {from}"),
        _ => format!("qu'on rejoint en {d} {mode} depuis {from}"),
    };
    fill(&t, noun, "")
}

fn isochrone(rng: &mut Rng) -> Option<Example> {
    isochrone_from(rng, "isochrone", origin_point)
}

/// L'isochrone depuis un point donné en UTM (« UTM 31N ») : même plan, code 326xx.
fn isochrone_utm(rng: &mut Rng) -> Option<Example> {
    isochrone_from(rng, "isochrone_utm", utm_origin)
}

/// Une isochrone depuis un point donné en coordonnées, tiré par `origin`.
fn isochrone_from(
    rng: &mut Rng,
    name: &'static str,
    origin: fn(&mut Rng) -> Option<Origin>,
) -> Option<Example> {
    let mut b = Builder::new();
    let network = network(rng, &mut b);
    let tr = travel(rng);
    let origin = origin(rng)?;
    let place = &origin.place;
    let (duration, mode) = (&tr.duration, tr.mode);
    let via = if rng.chance(1, 3) {
        format!(" sur la couche « {} »", network.name)
    } else {
        String::new()
    };
    let (body, q) = match rng.below(4) {
        0 => (
            format!("Quelle zone atteint-on en {duration} {mode} depuis {place}{via}"),
            true,
        ),
        1 => (
            format!("Calcule l'isochrone {} {mode} depuis {place}{via}", tr.de()),
            false,
        ),
        2 => (
            format!(
                "Jusqu'où peut-on aller {mode} en {duration} en partant {}{via}",
                place.replacen("le point", "du point", 1)
            ),
            true,
        ),
        _ => (
            format!("Montre la zone accessible {mode} en {duration} depuis {place}{via}"),
            false,
        ),
    };
    let request = finish(rng, &body, q, false);
    let expected = calls(vec![isochrone_call(&network.id, &origin.point, &tr)]);
    Some(example(name, b, rng, request, expected))
}

/// Lieux qui sont aussi des couches de points : l'origine d'un trajet, et le thème de sa
/// couche. La couche ne donne pas le point : seules des coordonnées le donnent.
fn place_with_layer(rng: &mut Rng) -> (String, &'static str) {
    let commune = *rng.pick(COMMUNES);
    match rng.below(8) {
        0..=3 => (format!("la gare {}", de_place(commune)), "gares"),
        4 => ("l'école Jean Moulin".to_string(), "ecoles"),
        5 => (format!("l'hôpital {}", de_place(commune)), "hopitaux"),
        6 => ("l'arrêt Mairie".to_string(), "arrets"),
        _ => ("la pharmacie du centre".to_string(), "pharmacies"),
    }
}

/// « Les parcelles à 10 min à pied du point ... (Lambert-93) » : le point est donné, la cible
/// est croisée avec l'isochrone. C'est le contraire de la question de `question_isochrone` :
/// la couche du lieu est souvent là aussi, mais seules les coordonnées donnent l'origine.
fn isochrone_vers_cible(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let network = network(rng, &mut b);
    // Une fois sur trois, le lieu est nommé et ses coordonnées données (« de la gare de X
    // (652 000 / 6 862 000, Lambert-93) ») : ce sont les coordonnées qui font le plan, pas
    // l'absence de nom.
    let named = rng.chance(1, 3);
    let (place, key) = place_with_layer(rng);
    if (named || rng.chance(2, 3)) && b.free(|t| t.key == key).len() == 1 {
        b.add(rng, key, &[], &[], &[]);
    }
    let t = pick_theme(rng, &b, |t| ordinary(t) && t.geom != Geom::Line)?;
    let target = b.add(rng, t.key, &[], &[], &[]);
    let tr = travel(rng);
    let origin = origin_point(rng)?;
    let from = if named {
        format!("{place}{}", origin.coords)
    } else {
        origin.place.clone()
    };
    let np = format!(
        "les {} {}",
        t.noun.pl,
        travel_clause(rng, &t.noun, &tr, &from)
    );
    let mut steps = vec![
        isochrone_call(&network.id, &origin.point, &tr),
        intersect(&target.id, &step(1)),
    ];
    let with_pdf = rng.chance(1, 6);
    let (mut head, q) = open_find(rng, &t.noun, &np, !with_pdf);
    if rng.chance(1, 4) {
        head.push_str(&format!(", sur la couche « {} »", network.name));
    }
    if with_pdf {
        let draft = Draft {
            b: Builder::new(),
            noun: &t.noun,
            np: np.clone(),
            calls: Vec::new(),
        };
        let (clause, title) = pdf_clause(rng, &draft);
        head.push_str(&clause);
        steps.push(pdf(&title));
    }
    let request = finish(rng, &head, q, false);
    Some(example(
        "isochrone_vers_cible",
        b,
        rng,
        request,
        calls(steps),
    ))
}

fn decompte_par_zone(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let zkey = *rng.pick(&["communes", "quartiers"]);
    let labelled = rng.chance(1, 3);
    let require: &[&str] = if labelled { &["nom"] } else { &[] };
    let zones = b.add(rng, zkey, require, &[], &[]);
    let z = &zones.theme.noun;
    let p = pick_theme(rng, &b, |t| t.geom == Geom::Point)?;
    let points = b.add(rng, p.key, &[], &[], &[]);
    let pl = p.noun.pl;
    // « de gares », mais « d'écoles », « d'hôpitaux ».
    let de_pl = if pl.starts_with(['a', 'e', 'é', 'i', 'o', 'u', 'h']) {
        format!("d'{pl}")
    } else {
        format!("de {pl}")
    };
    let (mut body, q) = match rng.below(5) {
        0 => (format!("Compte les {pl} par {}", z.sg), false),
        1 => (
            format!("Combien y a-t-il {de_pl} dans chaque {}", z.sg),
            true,
        ),
        2 => (format!("Nombre {de_pl} par {}", z.sg), false),
        3 => (format!("Fais le décompte des {pl} par {}", z.sg), false),
        _ => (format!("Combien {de_pl} compte chaque {}", z.sg), true),
    };
    if labelled {
        body.push_str(&match rng.below(2) {
            0 => format!(", en affichant le nom de chaque {}", z.sg),
            _ => format!(", avec le nom des {}", z.pl),
        });
    }
    let request = finish(rng, &body, q, false);
    let mut params = json!({"layer_id": zones.id, "points_layer_id": points.id});
    if labelled {
        params["label_field"] = json!(zones.field("nom")?.name);
    }
    let expected = calls(vec![
        json!({"command": "analysis.zonal_statistics", "params": params}),
    ]);
    Some(example("decompte_par_zone", b, rng, request, expected))
}

// -- ouverture et description ----------------------------------------------

fn ouverture(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    if rng.chance(1, 4) {
        // CSV à coordonnées : le système et les colonnes sont dits, donc posés.
        let t = pick_theme(rng, &b, |t| t.geom == Geom::Point)?;
        let path = b.add_file(rng, t.key, Some("csv"));
        let (xf, yf, crs, crs_text) = match rng.below(4) {
            0 => ("X", "Y", "EPSG:2154", "en Lambert-93"),
            1 => ("x", "y", "EPSG:2154", "en EPSG:2154"),
            2 => ("lon", "lat", "EPSG:4326", "en WGS84"),
            _ => ("longitude", "latitude", "EPSG:4326", "en EPSG:4326"),
        };
        let body = match rng.below(2) {
            0 => format!("Ajoute le fichier {path}, colonnes {xf} et {yf}, coordonnées {crs_text}"),
            _ => format!(
                "Ouvre {path} : les coordonnées sont dans les colonnes {xf} et {yf}, {crs_text}"
            ),
        };
        let request = finish(rng, &body, false, false);
        let expected = calls(vec![json!({"command": "layer.add",
            "params": {"path": path, "crs": crs, "x_field": xf, "y_field": yf}})]);
        other_file(rng, &mut b);
        return Some(example("ouverture", b, rng, request, expected));
    }
    let t = pick_theme(rng, &b, vector)?;
    let path = b.add_file(rng, t.key, None);
    let body = match rng.below(6) {
        0 => format!("Ouvre le fichier {path}"),
        1 => format!("Ajoute {path} au projet"),
        2 => format!("Charge la couche {path}"),
        // « la couche des captages », « la couche du zonage » : forme déjà accordée.
        3 => format!("Ajoute {} au projet", t.the_layer),
        4 => format!("Ouvre le fichier de {}", t.the_layer),
        _ => format!("Importe {path}"),
    };
    let request = finish(rng, &body, false, false);
    let expected = calls(vec![
        json!({"command": "layer.add", "params": {"path": path}}),
    ]);
    other_file(rng, &mut b);
    Some(example("ouverture", b, rng, request, expected))
}

/// Parfois un second fichier non ouvert, d'un autre thème, que la demande ne nomme pas : seul
/// le fichier demandé s'ouvre.
fn other_file(rng: &mut Rng, b: &mut Builder) {
    if rng.chance(1, 3) {
        let free = b.free(|t| t.geom != Geom::Raster);
        if !free.is_empty() {
            let t = *rng.pick(&free);
            b.add_file(rng, t.key, None);
        }
    }
}

fn description(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let roll = rng.below(10);
    if roll < 6 {
        let t = pick_theme(rng, &b, vector)?;
        let layer = b.add(rng, t.key, &[], &[], &[]);
        let which = if rng.chance(1, 2) {
            format!("la couche « {} »", layer.name)
        } else {
            t.the_layer.to_string()
        };
        let (body, q) = match rng.below(5) {
            0 => (format!("Quels sont les champs de {which}"), true),
            // « Décris la couche des ... » nu est une phrase des essais réels : « Décris-moi ».
            1 => (format!("Décris-moi {which}"), false),
            2 => (format!("Donne-moi les caractéristiques de {which}"), false),
            3 => (format!("Combien d'objets contient {which}"), true),
            _ => (
                format!("Quel est le système de coordonnées de {which}"),
                true,
            ),
        };
        let request = finish(rng, &body, q, false);
        let expected = calls(vec![
            json!({"command": "layer.describe", "params": {"layer_id": layer.id}}),
        ]);
        return Some(example("description", b, rng, request, expected));
    }
    b.distract(rng, 1, vector);
    let (body, q, command) = match roll {
        6 | 7 => {
            let (body, q) = *rng.pick(&[
                ("Qu'y a-t-il dans ce projet", true),
                ("Liste les couches du projet", false),
                ("Fais l'inventaire du projet", false),
                ("Quelles couches sont chargées", true),
                ("Dresse l'inventaire des couches ouvertes", false),
            ]);
            (body, q, "project.inventory")
        }
        8 => {
            let (body, q) = *rng.pick(&[
                ("Quelle est la version de Cassini", true),
                ("Donne-moi la version du noyau et du protocole", false),
                ("Quelle version du noyau tourne", true),
            ]);
            (body, q, "system.about")
        }
        _ => {
            let (body, q) = *rng.pick(&[
                ("Quels modèles d'IA sont disponibles", true),
                ("Liste les modèles locaux installés", false),
                ("Quels fichiers GGUF sont installés sur ce poste", true),
            ]);
            (body, q, "model.list")
        }
    };
    let request = finish(rng, body, q, false);
    let expected = calls(vec![json!({"command": command, "params": {}})]);
    Some(example("description", b, rng, request, expected))
}

// -- questions -------------------------------------------------------------

fn missing_layer_question(rng: &mut Rng, of_none: &str) -> String {
    match rng.below(4) {
        0 => format!("Aucune couche {of_none} dans le projet : laquelle faut-il utiliser ?"),
        1 => format!(
            "Je ne trouve aucune couche {of_none} dans le projet. Laquelle dois-je utiliser ?"
        ),
        2 => format!(
            "Le projet ne contient pas de couche {of_none} : quelle couche faut-il prendre ?"
        ),
        _ => format!(
            "Aucune couche {of_none} n'est dans le projet. Quelle couche faut-il utiliser ?"
        ),
    }
}

fn missing_field_question(rng: &mut Rng, layer: &str, label: &str) -> String {
    match rng.below(3) {
        0 => {
            format!("La couche « {layer} » n'a aucun champ {label} : quel champ faut-il utiliser ?")
        }
        1 => format!("Aucun champ {label} dans la couche « {layer} » : lequel faut-il utiliser ?"),
        _ => format!(
            "Je ne vois pas de champ {label} dans la couche « {layer} ». Lequel dois-je prendre ?"
        ),
    }
}

/// La demande nomme une couche dont aucun nom de couche ni aucun fichier ne parle.
fn question_couche_absente(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    if rng.chance(1, 6) {
        // « Constructible » sans zonage du PLU.
        b.exclude("zonage");
        b.add(rng, "parcelles", &[], &[], &[]);
        let r = pick_theme(rng, &b, |t| ordinary(t) && t.geom == Geom::Point)?;
        b.add(rng, r.key, &[], &[], &[]);
        let d = dist(rng);
        let rr = de_ref(rng, &r.noun);
        let np = format!(
            "les parcelles constructibles {}",
            near(rng, &theme("parcelles").noun, &d, &rr)
        );
        let (head, q) = open_find(rng, &theme("parcelles").noun, &np, true);
        let request = finish(rng, &head, q, false);
        let expected = question(missing_layer_question(rng, theme("zonage").of_none));
        return Some(example(
            "question_couche_absente",
            b,
            rng,
            request,
            expected,
        ));
    }
    let absent = *rng.pick(&THEMES.iter().filter(|t| ordinary(t)).collect::<Vec<_>>());
    b.exclude(absent.key);
    let p = pick_theme(rng, &b, ordinary)?;
    let present = b.add(rng, p.key, &[], &[], &[]);
    let a = &absent.noun;
    let d = dist(rng);
    let (body, q) = match rng.below(5) {
        0 => (tampon_head(rng, &d, &a.des()), false),
        1 => {
            let r = de_ref(rng, a);
            let np = format!("les {} {}", p.noun.pl, near(rng, &p.noun, &d, &r));
            open_find(rng, &p.noun, &np, true)
        }
        2 => {
            let np = format!(
                "les {} {}",
                a.pl,
                near(rng, a, &d, &present.theme.noun.des())
            );
            open_find(rng, a, &np, true)
        }
        3 => match text_fields(absent).first() {
            Some(def) => {
                let Kind::Text { values, .. } = &def.kind else {
                    return None;
                };
                let val = rng.pick(values);
                let np = qualified(rng, a, def, val, Det::Les);
                open_find(rng, a, &np, true)
            }
            None => (format!("Décris-moi {}", absent.the_layer), false),
        },
        _ => (format!("Décris-moi {}", absent.the_layer), false),
    };
    let request = finish(rng, &body, q, false);
    let expected = question(missing_layer_question(rng, absent.of_none));
    Some(example(
        "question_couche_absente",
        b,
        rng,
        request,
        expected,
    ))
}

/// La couche existe, mais aucun de ses champs ne porte le critère de la demande.
fn question_champ_absent(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    if rng.chance(1, 5) {
        // « Constructible » avec un zonage sans champ de type de zone.
        b.add(rng, "parcelles", &[], &[], &[]);
        // Sans libellé non plus : « Zone urbaine » contredirait la question.
        let z = b.add(rng, "zonage", &[], &["zone", "libelle"], &[]);
        let np = format!(
            "les parcelles {}",
            rng.pick(&[
                "constructibles",
                "en zone constructible",
                "constructibles du PLU"
            ])
        );
        let (head, q) = open_find(rng, &theme("parcelles").noun, &np, true);
        let request = finish(rng, &head, q, false);
        let label = theme("zonage").field("zone")?.label;
        let expected = question(missing_field_question(rng, &z.name, label));
        return Some(example("question_champ_absent", b, rng, request, expected));
    }
    let t = pick_theme(rng, &b, |t| {
        ordinary(t)
            && t.fields
                .iter()
                .any(|f| matches!(f.kind, Kind::Text { .. } | Kind::Number { .. }))
    })?;
    let defs: Vec<&FieldDef> = t
        .fields
        .iter()
        .filter(|f| matches!(f.kind, Kind::Text { .. } | Kind::Number { .. }))
        .collect();
    let def = *rng.pick(&defs);
    let layer = b.add(rng, t.key, &[], &[def.role], &[]);
    let np = match &def.kind {
        Kind::Number { .. } => {
            let (_, _, phrase) = numeric(rng, def);
            format!("les {} {phrase}", t.noun.pl)
        }
        Kind::Text { values, .. } => {
            let val = rng.pick(values);
            qualified(rng, &t.noun, def, val, Det::Les)
        }
        _ => return None,
    };
    let (head, q) = open_find(rng, &t.noun, &np, true);
    let request = finish(rng, &head, q, true);
    let expected = question(missing_field_question(rng, &layer.name, def.label));
    Some(example("question_champ_absent", b, rng, request, expected))
}

// -- refus -----------------------------------------------------------------

/// Opérations qu'aucune commande du catalogue ne fait : le refus est une phrase
/// affirmative, jamais une question.
fn refus(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let t = pick_theme(rng, &b, ordinary)?;
    let layer = b.add(rng, t.key, &[], &[], &[]);
    let pl = t.noun.pl;
    let name = layer.name.clone();
    let who = *rng.pick(&[
        "au maire",
        "à mon chef de projet",
        "au service urbanisme",
        "à la DDT",
        "à mes collègues",
    ]);
    let commune = *rng.pick(COMMUNES);
    let (body, reason): (String, &str) = match rng.below(16) {
        0 => (format!("Envoie la couche « {name} » par courriel {who}"), "Aucune commande du catalogue n'envoie de courriel."),
        1 => (format!("Envoie le résultat par mail {who}"), "Aucune commande du catalogue n'envoie de courriel."),
        2 => (format!("Supprime la couche « {name} » du projet"), "Aucune commande du catalogue ne retire de couche du projet."),
        3 => (format!("Retire les {pl} de la carte"), "Aucune commande du catalogue ne retire de couche du projet."),
        4 => match text_fields(t).first() {
            Some(def) => {
                let Kind::Text { values, phrases } = &def.kind else {
                    return None;
                };
                let val = rng.pick(values);
                let phrase = fill(pick_str(rng, phrases), &t.noun, val.v);
                (
                    format!("Supprime de la couche les {pl} {phrase}"),
                    "Aucune commande du catalogue ne supprime d'entités dans une couche : les analyses créent de nouvelles couches.",
                )
            }
            None => (format!("Efface toutes les {pl} de la couche"), "Aucune commande du catalogue ne supprime d'entités dans une couche."),
        },
        5 => (format!("Reprojette la couche « {name} » en WGS84"), "Aucune commande du catalogue ne reprojette une couche."),
        6 => (format!("Convertis les {pl} en Lambert-93"), "Aucune commande du catalogue ne reprojette une couche."),
        7 => (format!("Exporte la couche « {name} » en shapefile"), "Aucune commande du catalogue n'exporte une couche vers un fichier."),
        8 => (format!("Enregistre les {pl} dans un fichier Excel"), "Aucune commande du catalogue n'exporte une couche vers un fichier."),
        9 => (format!("Renomme la couche « {name} » en « {} »", rng.pick(USER_TITLES)), "Aucune commande du catalogue ne renomme une couche."),
        10 => (format!("Mets les {pl} en rouge sur la carte"), "Aucune commande du catalogue ne change le style d'une couche."),
        11 => (format!("Calcule l'itinéraire le plus court entre deux {pl}"), "Aucune commande du catalogue ne calcule d'itinéraire sur un réseau routier entre deux objets d'une couche : le chemin de moindre coût demande un raster de coût et deux points en coordonnées."),
        12 => (format!("Télécharge le cadastre {}", de_place(commune)), "Aucune commande du catalogue ne télécharge de données : le projet utilise les fichiers de l'utilisateur."),
        // Ce que l'export PDF ne fait pas (format hors liste, impression) vit dans
        // `refus_hors_registre` ; format, orientation et carroyage sont des plans
        // (`export_pdf_format`, 11.12, lot 3).
        13 => ("Publie la carte sur internet".to_string(), "Aucune commande du catalogue ne publie de carte en ligne."),
        14 => (format!("Modifie la valeur d'un champ pour toutes les {pl}"), "Aucune commande du catalogue ne modifie les attributs d'une couche."),
        _ => ("Affiche la carte en 3D".to_string(), "Aucune commande du catalogue n'affiche de vue en 3D."),
    };
    let request = finish(rng, &body, false, false);
    Some(example("refus", b, rng, request, refusal(reason)))
}

// -- hors registre ---------------------------------------------------------

const NO_FRAMING: &str = "Aucune commande du catalogue ne cadre ni ne zoome la carte.";
/// Décision 2 du propriétaire (2026-11-07) : ce que l'export PDF ne fait pas. Le format, hors
/// de la série du catalogue ; jamais un format ni une orientation qu'il connaît.
const NO_PAGE_SIZE: &str = "L'export PDF du catalogue ne propose pas ce format de page : ni l'A5, ni les formats américains comme la lettre US.";
/// L'impression et le nombre d'exemplaires restent hors du produit.
const NO_PRINTING: &str = "L'export PDF du catalogue écrit un fichier PDF : il n'envoie rien à une imprimante et ne fixe aucun nombre d'exemplaires.";
const NO_RASTER_STATS: &str = "Aucune commande du catalogue ne calcule de statistiques sur un raster : le décompte par zone ne compte que des points.";
/// Décision du propriétaire (2026-10-23, option a) : « de chaque gare », « de toutes les
/// gares », « des gares » ; l'isochrone du catalogue part d'un seul point en coordonnées.
pub const NO_MULTI_ORIGIN: &str = "L'isochrone du catalogue part d'un seul point donné en coordonnées : elle ne se calcule pas depuis plusieurs lieux à la fois.";

/// Ce que le registre ne sait pas encore faire, quoi que l'inventaire contienne (un MNT pour
/// les statistiques sur un raster) : un refus affirmatif, jamais un plan de substitution (un
/// décompte à la place d'une moyenne d'altitude). Depuis 5.41 : un calcul de terrain suivi
/// d'une statistique par zone (refus entier, jamais le seul calcul de terrain). Le point de
/// terrain écrit dans un autre système que le MNT n'y est plus depuis 11.12 :
/// `data.transform_point` le reprojette (`terrain_autre_systeme`). Le seuil sur la pente non
/// plus depuis 11.12, lot 2a : `raster.reclassify` le fait (`pente_seuil`). Ni le format A3, A0
/// ou portrait depuis 11.12, lot 3 (`export_pdf_format`) : seuls un format hors de la série du
/// catalogue et l'impression restent refusés.
fn refus_hors_registre(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let roll = rng.below(4);
    // Les statistiques portent sur un MNT : il est là, ce n'est pas lui qui manque.
    if roll == 2 {
        b.add(rng, "mnt", &[], &[], &[]);
    }
    let (body, q, reason): (String, bool, &str) = match roll {
        0 => {
            let t = pick_theme(rng, &b, ordinary)?;
            let layer = b.add(rng, t.key, &[], &[], &[]);
            let body = match rng.below(5) {
                0 => format!("Zoome sur {}", t.the_layer),
                1 => format!("Place {} au milieu de l'écran", t.noun.les()),
                2 => format!("Recadre la vue sur la couche « {} »", layer.name),
                3 => format!("Fais un zoom sur l'emprise des {}", t.noun.pl),
                _ => format!(
                    "Ajuste l'affichage pour voir {} les {}",
                    if t.noun.fem { "toutes" } else { "tous" },
                    t.noun.pl
                ),
            };
            (body, false, NO_FRAMING)
        }
        1 => {
            b.distract(rng, 1, vector);
            let copies = rng.range(2, 10);
            let (body, reason) = match rng.below(7) {
                0 => (
                    "Exporte la carte en PDF au format A5".to_string(),
                    NO_PAGE_SIZE,
                ),
                1 => ("Sors la carte en A5".to_string(), NO_PAGE_SIZE),
                2 => (
                    "Fais un PDF de la carte au format lettre US".to_string(),
                    NO_PAGE_SIZE,
                ),
                3 => (
                    format!(
                        "Produis la carte en PDF A5, titre « {} »",
                        rng.pick(USER_TITLES)
                    ),
                    NO_PAGE_SIZE,
                ),
                4 => (
                    format!("Imprime la carte en {copies} exemplaires"),
                    NO_PRINTING,
                ),
                5 => ("Envoie la carte à l'imprimante".to_string(), NO_PRINTING),
                _ => (
                    format!("Exporte la carte en PDF et imprime-la en {copies} exemplaires"),
                    NO_PRINTING,
                ),
            };
            (body, false, reason)
        }
        // Un trajet depuis plusieurs lieux : refus, jamais un plan ni une question.
        3 => {
            network(rng, &mut b);
            let p = pick_theme(rng, &b, |t| ordinary(t) && t.geom == Geom::Point)?;
            b.add(rng, p.key, &[], &[], &[]);
            let n = &p.noun;
            let from = match rng.below(3) {
                0 => format!("chaque {}", n.sg),
                1 => format!("{} les {}", if n.fem { "toutes" } else { "tous" }, n.pl),
                _ => n.les(),
            };
            let tr = travel(rng);
            let (duration, mode) = (&tr.duration, tr.mode);
            let (body, q) = match rng.below(4) {
                0 => (
                    format!("Calcule l'isochrone {} {mode} depuis {from}", tr.de()),
                    false,
                ),
                1 => (
                    format!("Montre la zone accessible {mode} en {duration} depuis {from}"),
                    false,
                ),
                _ => {
                    let t = pick_theme(rng, &b, |t| ordinary(t) && t.geom != Geom::Line)?;
                    b.add(rng, t.key, &[], &[], &[]);
                    let np = format!(
                        "les {} {}",
                        t.noun.pl,
                        travel_clause(rng, &t.noun, &tr, &from)
                    );
                    open_find(rng, &t.noun, &np, true)
                }
            };
            (body, q, NO_MULTI_ORIGIN)
        }
        _ => {
            let zkey = *rng.pick(&["communes", "quartiers", "parcelles"]);
            let zones = b.add(rng, zkey, &[], &[], &[]);
            let z = &zones.theme.noun;
            let stat = *rng.pick(&["moyenne", "maximale", "minimale", "médiane"]);
            let (body, q) = match rng.below(7) {
                0 => (
                    format!("Calcule l'altitude {stat} de chaque {}", z.sg),
                    false,
                ),
                1 => (
                    format!("Quelle est l'altitude {stat} de chaque {}", z.sg),
                    true,
                ),
                2 => (
                    format!("Fais les statistiques du MNT pour chaque {}", z.sg),
                    false,
                ),
                3 => (
                    format!("Donne l'altitude {stat} du terrain dans {}", z.les()),
                    false,
                ),
                // 5.41 : un calcul de terrain suivi d'une statistique par zone reste un refus
                // entier, jamais un plan à moitié.
                4 => (
                    format!(
                        "Fais la carte des pentes {}, puis donne la pente {stat} de chaque {}",
                        slope_unit(rng).1,
                        z.sg
                    ),
                    false,
                ),
                5 => (
                    format!("Calcule la pente du MNT et sa valeur {stat} par {}", z.sg),
                    false,
                ),
                _ => {
                    let p = lambert_origin(rng).place;
                    let tail = match rng.below(2) {
                        0 => format!("la part visible de chaque {}", z.sg),
                        _ => format!("fais la moyenne pour chaque {}", z.sg),
                    };
                    (
                        format!("Détermine ce qu'on voit depuis {p}, puis {tail}"),
                        false,
                    )
                }
            };
            (body, q, NO_RASTER_STATS)
        }
    };
    let request = finish(rng, &body, q, false);
    Some(example(
        "refus_hors_registre",
        b,
        rng,
        request,
        refusal(reason),
    ))
}

/// Le MNT de l'inventaire (raster `mnt…` qui garde sa `source`) donne son système.
fn dem_has_epsg(inventory: &Value) -> bool {
    inventory["layers"].as_array().is_some_and(|layers| {
        layers.iter().any(|l| {
            l["kind"] == "raster"
                && l.get("source").is_some()
                && l["layer_id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("mnt"))
                && l["epsg"].is_u64()
        })
    })
}

// -- terrain ---------------------------------------------------------------

/// Un point en Lambert-93, écrit comme un utilisateur l'écrirait. Pas « le point X / Y
/// (Lambert-93) » : c'est la forme des phrases de terrain des essais réels
/// (`training/tests/recouvrement.rs`).
fn lambert_origin(rng: &mut Rng) -> Origin {
    let x = rng.range(60_000, 70_000) * 10;
    let y = rng.range(678_000, 690_000) * 10;
    let (xs, ys) = if rng.chance(1, 2) {
        (thousands(x), thousands(y))
    } else {
        (x.to_string(), y.to_string())
    };
    let crs = *rng.pick(&["Lambert-93", "Lambert 93", "EPSG:2154"]);
    let place = match rng.below(2) {
        0 => format!("le point X = {xs}, Y = {ys} en {crs}"),
        _ => format!("le point ({xs} ; {ys}) en {crs}"),
    };
    Origin {
        point: json!({"x": x, "y": y, "epsg": 2154}),
        coords: format!(" ({xs} ; {ys}, {crs})"),
        place,
    }
}

/// Le système du MNT de l'inventaire d'une demande de terrain. Les familles de terrain y écrivent
/// leurs points ; `terrain_autre_systeme` écrit le sien dans un autre système et le reprojette
/// d'abord par `data.transform_point` (11.12).
fn terrain_epsg(rng: &mut Rng) -> u32 {
    match rng.below(4) {
        0 | 1 => 2154,
        2 => 32631,
        _ => 32632,
    }
}

/// Un point écrit dans le système `epsg` du MNT, son système écrit avec lui : le garde-fou
/// demande une occurrence du système par point.
fn terrain_point(rng: &mut Rng, epsg: u32) -> Option<Origin> {
    match epsg {
        2154 => Some(lambert_origin(rng)),
        32631 => utm_origin_in(rng, 31),
        _ => utm_origin_in(rng, 32),
    }
}

/// Un point écrit dans un autre système que `epsg`, celui du MNT (5.41, élargi en 11.12) : en
/// WGS84 (latitude, longitude), en UTM d'une autre zone (31N ou 32N sur un MNT en Lambert-93,
/// 32N sur un MNT en 31N et inversement), ou en Lambert-93 sur un MNT en UTM.
fn foreign_point(rng: &mut Rng, epsg: u32) -> Option<Origin> {
    let choices = if epsg == 2154 { 2 } else { 3 };
    match rng.below(choices) {
        0 => Some(wgs84_coord(rng, epsg == 32632).origin()),
        1 => {
            let zone = match epsg {
                2154 => *rng.pick(&[31, 32]),
                32631 => 32,
                _ => 31,
            };
            utm_origin_in(rng, zone)
        }
        _ => Some(lambert_origin(rng)),
    }
}

/// « du MNT », « du modèle numérique de terrain », « de la couche « MNT 1 m » », ou rien
/// (« du terrain ») : « du terrain » désigne le MNT, seul raster d'altitude de l'inventaire.
fn dem_words(rng: &mut Rng, dem: &Layer) -> String {
    match rng.below(4) {
        0 => "du MNT".to_string(),
        1 => "du modèle numérique de terrain".to_string(),
        2 => format!("de la couche « {} »", dem.name),
        _ => "du terrain".to_string(),
    }
}

/// Une hauteur au-dessus du sol écrite : valeur attendue et forme écrite du nombre.
fn height(rng: &mut Rng, values: &[(f64, &'static str)]) -> (Value, &'static str) {
    let (v, text) = *rng.pick(values);
    let value = if v.fract() == 0.0 {
        json!(v as u64)
    } else {
        json!(v)
    };
    (json!({"value": value, "unit": "m"}), text)
}

const EYE_HEIGHTS: &[(f64, &str)] = &[
    (1.6, "1,6"),
    (1.8, "1,8"),
    (2.0, "2"),
    (5.0, "5"),
    (10.0, "10"),
    (12.0, "12"),
];
const TARGET_HEIGHTS: &[(f64, &str)] = &[
    (2.0, "2"),
    (5.0, "5"),
    (10.0, "10"),
    (15.0, "15"),
    (30.0, "30"),
];

/// La hauteur de l'œil, écrite (« , œil à 2 m »).
fn eye_clause(rng: &mut Rng) -> (Value, String) {
    let (h, n) = height(rng, EYE_HEIGHTS);
    let text = match rng.below(4) {
        0 => format!(", œil à {n} m"),
        1 => format!(", observateur à {n} m du sol"),
        2 => format!(" avec une hauteur d'œil de {n} m"),
        _ => format!(", à {n} m de hauteur"),
    };
    (h, text)
}

/// La hauteur de la cible, écrite (« , cible à 10 m du sol »).
fn target_clause(rng: &mut Rng, plural: bool) -> (Value, String) {
    let (h, n) = height(rng, TARGET_HEIGHTS);
    let text = match (plural, rng.below(2)) {
        (false, 0) => format!(", cible à {n} m du sol"),
        (false, _) => format!(" pour une cible haute de {n} m"),
        (true, 0) => format!(", cibles à {n} m du sol"),
        (true, _) => format!(" pour des cibles hautes de {n} m"),
    };
    (h, text)
}

/// Le rayon d'un champ de vision : valeur attendue et distance écrite (« 800 m »).
fn radius_words(rng: &mut Rng) -> (Value, &'static str) {
    let (value, unit, forms): (Value, &str, &[&str]) = match rng.below(7) {
        0 => (json!(300), "m", &["300 m", "300 mètres"]),
        1 => (json!(500), "m", &["500 m", "500 mètres"]),
        2 => (json!(800), "m", &["800 m"]),
        3 => (json!(1), "km", &["1 km", "un kilomètre"]),
        4 => (json!(1.5), "km", &["1,5 km"]),
        5 => (json!(2), "km", &["2 km", "deux kilomètres"]),
        _ => (json!(3), "km", &["3 km"]),
    };
    (json!({"value": value, "unit": unit}), *rng.pick(forms))
}

/// Le rayon d'un champ de vision, écrit : valeur attendue et forme écrite.
fn radius_clause(rng: &mut Rng) -> (Value, String) {
    let (r, d) = radius_words(rng);
    let text = match rng.below(3) {
        0 => format!(" sur {d}"),
        1 => format!(" dans un rayon de {d}"),
        _ => format!(" jusqu'à {d}"),
    };
    (r, text)
}

/// Unité de pente écrite : `degrees` ou `percent`.
fn slope_unit(rng: &mut Rng) -> (&'static str, &'static str) {
    match rng.below(6) {
        0 => ("degrees", "en degrés"),
        1 => ("degrees", "exprimée en degrés"),
        2 => ("degrees", "en °"),
        3 => ("percent", "en pourcentage"),
        4 => ("percent", "en %"),
        _ => ("percent", "en pour cent"),
    }
}

/// « Fais la carte des pentes du MNT en pourcentage » : `terrain.slope` sur le `source` du MNT,
/// l'unité telle qu'écrite.
fn pente(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let epsg = terrain_epsg(rng);
    let dem = b.add_dem(rng, epsg);
    let (unit, unit_text) = slope_unit(rng);
    let of = dem_words(rng, &dem);
    let (body, q) = match rng.below(6) {
        // Pas « Calcule la pente du terrain en pourcentage » : c'est une phrase des essais
        // réels (`training/tests/recouvrement.rs`).
        0 => (format!("Calcule les pentes {of} {unit_text}"), false),
        1 => (format!("Fais la carte des pentes {of} {unit_text}"), false),
        2 => (
            format!("Génère un raster de pente {unit_text} à partir {of}"),
            false,
        ),
        3 => (format!("Peux-tu calculer la pente {of} {unit_text}"), true),
        4 => (format!("Donne-moi l'inclinaison {of} {unit_text}"), false),
        _ => (
            format!("Produis la carte des pentes {unit_text} {of}"),
            false,
        ),
    };
    let request = finish(rng, &body, q, true);
    let expected = calls(vec![json!({"command": "terrain.slope",
        "params": {"dem": dem.source, "unit": unit}})]);
    Some(example("pente", b, rng, request, expected))
}

/// « Y a-t-il une ligne de vue entre A et B, œil à 2 m » : `terrain.line_of_sight`, deux
/// points écrits dans le système du MNT (chacun avec son système), hauteurs seulement si
/// elles sont écrites. L'observateur est le point « depuis » ou le premier nommé.
fn ligne_de_vue(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let epsg = terrain_epsg(rng);
    let dem = b.add_dem(rng, epsg);
    let a = terrain_point(rng, epsg)?;
    let c = terrain_point(rng, epsg)?;
    let (pa, pc) = (&a.place, &c.place);
    let du_a = pa.replacen("le point", "du point", 1);
    let (body, q) = match rng.below(5) {
        0 => (
            format!("Y a-t-il une ligne de vue entre {pa} et {pc}"),
            true,
        ),
        1 => (format!("Vérifie si {pc} est visible depuis {pa}"), false),
        2 => (format!("Calcule la ligne de vue {du_a} vers {pc}"), false),
        3 => (format!("Est-ce qu'on voit {pc} depuis {pa}"), true),
        _ => (
            format!(
                "Trace le profil de visibilité {du_a} jusqu'au point {}",
                &pc["le point ".len()..]
            ),
            false,
        ),
    };
    let mut body = body;
    let mut params = json!({"dem": dem.source, "observer": a.point, "target": c.point});
    if rng.chance(1, 2) {
        let (h, text) = eye_clause(rng);
        body.push_str(&text);
        params["observer_height"] = h;
    }
    if rng.chance(1, 3) {
        let (h, text) = target_clause(rng, false);
        body.push_str(&text);
        params["target_height"] = h;
    }
    if rng.chance(1, 4) {
        body.push_str(&format!(
            ", sur {}",
            dem_words(rng, &dem)
                .replacen("du ", "le ", 1)
                .replacen("de la ", "la ", 1)
        ));
    }
    let request = finish(rng, &body, q, false);
    let expected = calls(vec![
        json!({"command": "terrain.line_of_sight", "params": params}),
    ]);
    Some(example("ligne_de_vue", b, rng, request, expected))
}

/// « Calcule le champ de vision depuis le point ... (UTM 31N) sur 2 km » :
/// `terrain.viewshed`, le point écrit dans le système du MNT, hauteurs et rayon seulement
/// s'ils sont écrits.
fn champ_de_vision(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let epsg = terrain_epsg(rng);
    let dem = b.add_dem(rng, epsg);
    let o = terrain_point(rng, epsg)?;
    let p = &o.place;
    // 5.41 : le rayon écrit avant le point, « dans un rayon de 800 m autour du point ».
    let early = rng.chance(1, 4).then(|| radius_words(rng));
    let (mut body, q) = if let Some((_, d)) = &early {
        let du_p = p.replacen("le point", "du point", 1);
        match rng.below(3) {
            0 => (
                format!("Montre ce qu'on voit dans un rayon de {d} autour {du_p}"),
                false,
            ),
            1 => (
                format!("Calcule la visibilité dans un rayon de {d} autour {du_p}"),
                false,
            ),
            _ => (
                format!("Dans un rayon de {d} autour {du_p}, quelles zones sont visibles"),
                true,
            ),
        }
    } else {
        match rng.below(6) {
            0 => (format!("Calcule le champ de vision depuis {p}"), false),
            1 => (format!("Quelles zones sont visibles depuis {p}"), true),
            2 => (format!("Détermine ce qu'on voit depuis {p}"), false),
            3 => (format!("Fais une analyse de visibilité depuis {p}"), false),
            4 => (format!("Montre la zone vue depuis {p}"), false),
            _ => (format!("Qu'est-ce qui est visible depuis {p}"), true),
        }
    };
    let mut params = json!({"dem": dem.source, "observer": o.point});
    if let Some((r, _)) = early {
        params["radius"] = r;
    }
    if rng.chance(1, 2) {
        let (h, text) = eye_clause(rng);
        body.push_str(&text);
        params["observer_height"] = h;
    }
    if rng.chance(1, 6) {
        let (h, text) = target_clause(rng, true);
        body.push_str(&text);
        params["target_height"] = h;
    }
    if params.get("radius").is_none() && rng.chance(1, 2) {
        let (r, text) = radius_clause(rng);
        body.push_str(&text);
        params["radius"] = r;
    }
    let request = finish(rng, &body, q, false);
    let expected = calls(vec![
        json!({"command": "terrain.viewshed", "params": params}),
    ]);
    Some(example("champ_de_vision", b, rng, request, expected))
}

// -- dérivés du MNT (11.12, lot 2a) -------------------------------------------
//
// Bâtis sur le MNT de l'inventaire synthétique, comme `pente` et `champ_de_vision` : le MNT
// désigné par sa `source`, tout nombre du plan écrit dans la demande, un paramètre facultatif
// seulement s'il est écrit.

/// « Calcule l'exposition des versants du MNT » : `terrain.aspect` sur le `source` du MNT.
fn exposition(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let epsg = terrain_epsg(rng);
    let dem = b.add_dem(rng, epsg);
    let of = dem_words(rng, &dem);
    let (body, q) = match rng.below(6) {
        0 => (format!("Calcule l'exposition des versants {of}"), false),
        1 => (format!("Fais la carte d'exposition {of}"), false),
        2 => (format!("Vers où sont tournés les versants {of}"), true),
        3 => (format!("Donne l'orientation des versants {of}"), false),
        4 => (
            format!("Génère un raster d'exposition à partir {of}"),
            false,
        ),
        _ => (format!("Peux-tu calculer l'exposition {of}"), true),
    };
    let request = finish(rng, &body, q, true);
    let expected = calls(vec![
        json!({"command": "terrain.aspect", "params": {"dem": dem.source}}),
    ]);
    Some(example("exposition", b, rng, request, expected))
}

/// Les directions d'où vient le soleil, rapportées au nord du quadrillage : nommées avec leur
/// azimut écrit, jamais un azimut à calculer.
const SUN_DIRECTIONS: &[(f64, &str)] = &[
    (45.0, "au nord-est"),
    (90.0, "à l'est"),
    (135.0, "au sud-est"),
    (180.0, "au sud"),
    (225.0, "au sud-ouest"),
    (270.0, "à l'ouest"),
    (315.0, "au nord-ouest"),
];
const SUN_ALTITUDES: &[f64] = &[20.0, 30.0, 35.0, 45.0, 60.0];
const Z_FACTORS: &[(f64, &str)] = &[(1.5, "1,5"), (2.0, "2"), (3.0, "3"), (5.0, "5")];

/// « Ombrage du MNT, soleil à l'ouest (azimut 270°), hauteur du soleil 30° » :
/// `terrain.hillshade`, azimut, hauteur et exagération seulement s'ils sont écrits, en degrés ou
/// en grades (tous les angles d'une demande dans la même unité).
fn ombrage(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let epsg = terrain_epsg(rng);
    let dem = b.add_dem(rng, epsg);
    let of = dem_words(rng, &dem);
    let (mut body, q) = match rng.below(5) {
        0 => (format!("Calcule l'ombrage {of}"), false),
        1 => (format!("Fais un ombrage du relief à partir {of}"), false),
        2 => (format!("Génère un estompage {of}"), false),
        3 => (format!("Produis la carte d'ombrage {of}"), false),
        _ => (format!("Peux-tu ombrer le relief {of}"), true),
    };
    let gon = rng.chance(1, 4);
    let mut params = json!({"dem": dem.source});
    if rng.chance(1, 2) {
        let (deg, dir) = *rng.pick(SUN_DIRECTIONS);
        let (angle, t) = angle_words(rng, gon, deg);
        body.push_str(&match rng.below(3) {
            0 => format!(", soleil {dir} (azimut {t})"),
            1 => format!(", lumière venant de l'azimut {t}"),
            _ => format!(", éclairage à l'azimut {t}"),
        });
        params["sun_azimuth"] = angle;
    }
    if rng.chance(1, 2) {
        let deg = *rng.pick(SUN_ALTITUDES);
        let (angle, t) = angle_words(rng, gon, deg);
        body.push_str(&match rng.below(3) {
            0 => format!(", soleil à {t} au-dessus de l'horizon"),
            1 => format!(", hauteur du soleil {t}"),
            _ => format!(" avec un soleil élevé de {t}"),
        });
        params["sun_altitude"] = angle;
    }
    if rng.chance(1, 3) {
        let (z, t) = *rng.pick(Z_FACTORS);
        body.push_str(&match rng.below(3) {
            0 => format!(", exagération verticale de {t}"),
            1 => format!(", facteur z de {t}"),
            _ => format!(" avec une exagération du relief de {t}"),
        });
        params["z_factor"] = if z.fract() == 0.0 {
            json!(z as u64)
        } else {
            json!(z)
        };
    }
    let request = finish(rng, &body, q, false);
    let expected = calls(vec![
        json!({"command": "terrain.hillshade", "params": params}),
    ]);
    Some(example("ombrage", b, rng, request, expected))
}

/// « A, B et C » : des lieux énumérés, le dernier après « et ».
fn enumerate(places: &[String]) -> String {
    match places.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, head)) => format!("{} et {last}", head.join(", ")),
        None => String::new(),
    }
}

/// « Combien de ces points voient chaque endroit : A, B et C » : `terrain.viewshed_cumulative`,
/// deux à quatre points écrits dans le système du MNT (chacun avec son système), hauteurs et
/// rayon seulement s'ils sont écrits, comme `champ_de_vision`.
fn champ_de_vision_cumule(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let epsg = terrain_epsg(rng);
    let dem = b.add_dem(rng, epsg);
    let n = rng.range(2, 4) as usize;
    let mut points = Vec::with_capacity(n);
    let mut places = Vec::with_capacity(n);
    for _ in 0..n {
        let o = terrain_point(rng, epsg)?;
        points.push(o.point);
        places.push(o.place);
    }
    let pts = enumerate(&places);
    let (mut body, q) = match rng.below(6) {
        0 => (
            format!("Combien de ces points voient chaque endroit : {pts}"),
            true,
        ),
        1 => (format!("Calcule la visibilité cumulée depuis {pts}"), false),
        2 => (
            format!("Fais un champ de vision cumulé depuis {pts}"),
            false,
        ),
        3 => (
            format!(
                "Compte, pour chaque endroit du terrain, combien de points le voient parmi {pts}"
            ),
            false,
        ),
        4 => (
            format!("Quelles zones sont vues depuis {pts}, et par combien de ces points"),
            true,
        ),
        _ => (
            format!("Superpose les champs de vision depuis {pts}"),
            false,
        ),
    };
    let mut params = json!({"dem": dem.source, "observers": points});
    if rng.chance(1, 2) {
        let (h, text) = eye_clause(rng);
        body.push_str(&text);
        params["observer_height"] = h;
    }
    if rng.chance(1, 6) {
        let (h, text) = target_clause(rng, true);
        body.push_str(&text);
        params["target_height"] = h;
    }
    if rng.chance(1, 2) {
        let (r, text) = radius_clause(rng);
        body.push_str(&text);
        params["radius"] = r;
    }
    let request = finish(rng, &body, q, false);
    let expected = calls(vec![
        json!({"command": "terrain.viewshed_cumulative", "params": params}),
    ]);
    Some(example("champ_de_vision_cumule", b, rng, request, expected))
}

const PROMINENCES: &[u64] = &[5, 10, 20, 30, 50];
const PEAK_SPACINGS: &[(u64, &str)] = &[
    (100, "m"),
    (200, "m"),
    (250, "m"),
    (500, "m"),
    (1, "km"),
    (2, "km"),
];

/// « Les 10 sommets les plus hauts du MNT, proéminence d'au moins 20 m » : `terrain.peaks`,
/// nombre de sommets, proéminence et écart seulement s'ils sont écrits.
fn points_hauts(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let epsg = terrain_epsg(rng);
    let dem = b.add_dem(rng, epsg);
    let of = dem_words(rng, &dem);
    let mut params = json!({"dem": dem.source});
    let (mut body, q) = if rng.chance(1, 2) {
        // De 2 à 100 : « le sommet le plus haut » ne dirait pas le nombre 1.
        let n = if rng.chance(1, 5) {
            *rng.pick(&[50u64, 75, 100])
        } else {
            rng.range(2, 30)
        };
        params["max_count"] = json!(n);
        match rng.below(4) {
            0 => (format!("Trouve les {n} sommets les plus hauts {of}"), false),
            1 => (format!("Liste les {n} points culminants {of}"), false),
            2 => (format!("Repère les {n} plus hauts sommets {of}"), false),
            _ => (format!("Quels sont les {n} sommets principaux {of}"), true),
        }
    } else {
        match rng.below(4) {
            0 => (format!("Trouve les sommets {of}"), false),
            1 => (format!("Quels sont les points hauts {of}"), true),
            2 => (format!("Repère les points hauts {of}"), false),
            _ => (format!("Localise les sommets {of}"), false),
        }
    };
    if rng.chance(1, 2) {
        let p = *rng.pick(PROMINENCES);
        body.push_str(&match rng.below(3) {
            0 => format!(", proéminence d'au moins {p} m"),
            1 => format!(" dont la proéminence dépasse {p} m"),
            _ => format!(" avec une proéminence minimale de {p} m"),
        });
        params["min_prominence"] = json!({"value": p, "unit": "m"});
    }
    if rng.chance(1, 3) {
        let (d, unit) = *rng.pick(PEAK_SPACINGS);
        body.push_str(&match rng.below(3) {
            0 => format!(", espacés d'au moins {d} {unit}"),
            1 => format!(", distants d'au moins {d} {unit} les uns des autres"),
            _ => format!(", séparés d'au moins {d} {unit}"),
        });
        params["min_distance"] = json!({"value": d, "unit": unit});
    }
    let request = finish(rng, &body, q, true);
    let expected = calls(vec![json!({"command": "terrain.peaks", "params": params})]);
    Some(example("points_hauts", b, rng, request, expected))
}

/// « Où la pente dépasse-t-elle 15 % » : `terrain.slope` dans l'unité du seuil, puis
/// `raster.reclassify` sur `step:1`. « dépasse, plus de n » → `{min: n, value: 1}` ; « sous,
/// moins de n » → `{max: n, value: 1}` (une cellule hors classe devient NoData : c'est la
/// sélection). Deux ou trois classes : valeurs 1, 2, 3 dans l'ordre, bornes contiguës.
fn pente_seuil(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let epsg = terrain_epsg(rng);
    let dem = b.add_dem(rng, epsg);
    let degrees = rng.chance(1, 2);
    let unit = if degrees { "degrees" } else { "percent" };
    let sym = if !degrees {
        " %"
    } else if rng.chance(1, 2) {
        "°"
    } else {
        " degrés"
    };
    let u = |n: u64| format!("{n}{sym}");
    let values: &[u64] = if degrees {
        &[5, 10, 15, 20, 30, 35, 45]
    } else {
        &[2, 5, 10, 15, 20, 30, 50]
    };
    let of = if rng.chance(1, 2) {
        format!(" {}", dem_words(rng, &dem))
    } else {
        String::new()
    };
    let (body, q, classes) = match rng.below(3) {
        0 => {
            let n = *rng.pick(values);
            let t = u(n);
            let (body, q) = match rng.below(5) {
                0 => (format!("Où la pente{of} dépasse-t-elle {t}"), true),
                1 => (
                    format!("Montre les zones où la pente{of} est supérieure à {t}"),
                    false,
                ),
                2 => (
                    format!("Garde seulement les secteurs dont la pente fait plus de {t}"),
                    false,
                ),
                3 => (
                    format!("Quelles surfaces{of} ont une pente de plus de {t}"),
                    true,
                ),
                _ => (
                    format!("Repère les terrains en pente de plus de {t}"),
                    false,
                ),
            };
            (body, q, json!([{"min": n, "value": 1}]))
        }
        1 => {
            let n = *rng.pick(values);
            let t = u(n);
            let (body, q) = match rng.below(4) {
                0 => (
                    format!("Sélectionne les endroits{of} où la pente reste sous {t}"),
                    false,
                ),
                1 => (format!("Où la pente{of} est-elle inférieure à {t}"), true),
                2 => (format!("Montre les zones de pente de moins de {t}"), false),
                _ => (
                    format!("Garde seulement les secteurs{of} dont la pente est sous {t}"),
                    false,
                ),
            };
            (body, q, json!([{"max": n, "value": 1}]))
        }
        _ => {
            let i = rng.below(values.len() - 1);
            let a = values[i];
            if rng.chance(1, 2) {
                let (ta, body) = (u(a), rng.below(3));
                let body = match body {
                    0 => format!(
                        "Classe les pentes{of} en deux catégories : moins de {ta} et plus de {ta}"
                    ),
                    1 => {
                        format!("Sépare les pentes{of} de moins de {ta} de celles de plus de {ta}")
                    }
                    _ => format!(
                        "Reclasse la pente{of} en deux classes, sous {ta} et au-dessus de {ta}"
                    ),
                };
                (
                    body,
                    false,
                    json!([{"max": a, "value": 1}, {"min": a, "value": 2}]),
                )
            } else {
                let c = values[rng.range((i + 1) as u64, (values.len() - 1) as u64) as usize];
                let (ta, tc) = (u(a), u(c));
                let body = match rng.below(3) {
                    0 => format!("Classe les pentes{of} en moins de {ta}, de {ta} à {tc} et plus de {tc}"),
                    1 => format!("Reclasse la pente{of} en trois classes : sous {ta}, entre {ta} et {tc}, au-delà de {tc}"),
                    _ => format!("Découpe la pente{of} en trois catégories : moins de {ta}, {ta} à {tc}, plus de {tc}"),
                };
                (
                    body,
                    false,
                    json!([{"max": a, "value": 1}, {"min": a, "max": c, "value": 2},
                        {"min": c, "value": 3}]),
                )
            }
        }
    };
    let request = finish(rng, &body, q, false);
    let expected = calls(vec![
        json!({"command": "terrain.slope", "params": {"dem": dem.source, "unit": unit}}),
        json!({"command": "raster.reclassify",
            "params": {"raster": "step:1", "classes": classes}}),
    ]);
    Some(example("pente_seuil", b, rng, request, expected))
}

// -- coût de déplacement (11.12, lot 2b-1) ----------------------------------
//
// Le raster de coût est apporté par l'utilisateur : un raster `cout` de l'inventaire, cité par
// sa `source` comme le MNT. Jamais le MNT ni un de ses dérivés pris comme coût : sans raster de
// coût, on le demande. Les points sont écrits dans le système du raster, chacun avec le sien.

/// « sur le raster de coût », « sur la couche « Friction » » : le raster de coût est désigné,
/// jamais « le terrain » (le MNT peut être là en distraction).
fn cost_words(rng: &mut Rng, cost: &Layer) -> String {
    match rng.below(4) {
        0 => "sur le raster de coût".to_string(),
        1 => format!("sur la couche « {} »", cost.name),
        2 => "avec le raster de coût".to_string(),
        _ => "sur le raster de coût de déplacement".to_string(),
    }
}

/// Le raster de coût cité, dans le système `epsg`, et parfois le MNT en distraction.
fn cost_inventory(rng: &mut Rng, b: &mut Builder, epsg: u32) -> Layer {
    if rng.chance(1, 3) {
        b.add(rng, "mnt", &[], &[], &[]);
    }
    b.add_raster(rng, "cout", epsg)
}

/// « Calcule le coût de déplacement cumulé depuis A, B et C sur le raster de coût » :
/// `terrain.cost_distance`, une à trois sources écrites dans le système du raster de coût,
/// dans l'ordre écrit. Un lieu nommé sans coordonnées devient la question du point.
fn cout_cumule(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let epsg = terrain_epsg(rng);
    let cost = cost_inventory(rng, &mut b, epsg);
    let on = cost_words(rng, &cost);
    let named = rng.chance(1, 6);
    let n = rng.range(1, 3) as usize;
    let mut points = Vec::with_capacity(n);
    let mut places = Vec::with_capacity(n + 1);
    for _ in 0..n {
        let o = terrain_point(rng, epsg)?;
        points.push(o.point);
        places.push(o.place);
    }
    let place = named.then(|| named_viewpoint(rng));
    if let Some(p) = &place {
        // Le lieu nommé seul, ou parmi des points écrits : aucun plan ne le devine.
        if rng.chance(1, 2) {
            places.clear();
        }
        let at = rng.below(places.len() + 1);
        places.insert(at, p.clone());
    }
    let pts = enumerate(&places);
    let (body, q) = match rng.below(6) {
        0 => (
            format!("Calcule le coût de déplacement cumulé depuis {pts} {on}"),
            false,
        ),
        1 => (
            format!("Fais la carte du coût cumulé depuis {pts}, {on}"),
            false,
        ),
        2 => (
            format!("Quel est l'effort cumulé pour rejoindre chaque endroit depuis {pts}, {on}"),
            true,
        ),
        3 => (
            format!("Calcule la distance de coût depuis {pts} {on}"),
            false,
        ),
        4 => (
            format!("Produis le raster de coût cumulé {on}, avec pour départ {pts}"),
            false,
        ),
        _ => (
            format!(
                "Combien coûte, en friction cumulée, l'accès à chaque cellule depuis {pts} {on}"
            ),
            true,
        ),
    };
    let request = finish(rng, &body, q, false);
    let expected = match &place {
        Some(p) => question(point_question(rng, p)),
        None => calls(vec![json!({"command": "terrain.cost_distance",
            "params": {"cost": cost.source, "sources": points}})]),
    };
    Some(example("cout_cumule", b, rng, request, expected))
}

/// « Trace le chemin de moindre coût du point A au point B sur le raster de coût » :
/// `terrain.least_cost_path`, départ et arrivée écrits dans le système du raster de coût, dans
/// l'ordre écrit. Un seul point écrit, l'autre nommé : la question du point. Aucun raster de
/// coût dans l'inventaire (le MNT seul, ou rien) : la question du raster, jamais le MNT.
fn chemin_moindre_cout(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let epsg = terrain_epsg(rng);
    let roll = rng.below(6);
    if roll == 0 {
        return cout_absent(rng, b, epsg);
    }
    let cost = cost_inventory(rng, &mut b, epsg);
    let on = cost_words(rng, &cost);
    let a = terrain_point(rng, epsg)?;
    let c = terrain_point(rng, epsg)?;
    let place = (roll == 1).then(|| named_viewpoint(rng));
    let (pa, pc) = match &place {
        // Le lieu nommé au départ ou à l'arrivée.
        Some(p) if rng.chance(1, 2) => (p.clone(), c.place.clone()),
        Some(p) => (a.place.clone(), p.clone()),
        None => (a.place.clone(), c.place.clone()),
    };
    let from = de_np(&pa);
    let to = a_np(&pc);
    let (body, q) = match rng.below(6) {
        0 => (
            format!("Trace le chemin de moindre coût {from} {to}, {on}"),
            false,
        ),
        1 => (
            format!("Quel est le chemin le moins coûteux pour aller {from} {to}, {on}"),
            true,
        ),
        2 => (
            format!("Depuis {pa}, trouve le trajet de moindre effort jusqu'{to} {on}"),
            false,
        ),
        3 => (
            format!("Cherche le passage le moins coûteux {from} {to} {on}"),
            false,
        ),
        4 => (
            format!("Relie {pa} {to} par le chemin de moindre coût, {on}"),
            false,
        ),
        _ => (
            format!(
                "Calcule le chemin de moindre friction en partant {from} pour arriver {to} {on}"
            ),
            false,
        ),
    };
    let request = finish(rng, &body, q, false);
    let expected = match &place {
        Some(p) => question(point_question(rng, p)),
        None => calls(vec![json!({"command": "terrain.least_cost_path",
            "params": {"cost": cost.source, "start": a.point, "end": c.point}})]),
    };
    Some(example("chemin_moindre_cout", b, rng, request, expected))
}

/// Une demande de coût sans raster de coût dans l'inventaire : le MNT seul, ou aucun raster.
/// On demande d'ouvrir ou de nommer le raster de coût ; le MNT n'en tient jamais lieu.
fn cout_absent(rng: &mut Rng, mut b: Builder, epsg: u32) -> Option<Example> {
    b.exclude("cout");
    if rng.chance(1, 2) {
        b.add_dem(rng, epsg);
    }
    let a = terrain_point(rng, epsg)?;
    let body = if rng.chance(1, 2) {
        let c = terrain_point(rng, epsg)?;
        match rng.below(2) {
            0 => format!(
                "Trace le chemin de moindre coût {} {}",
                de_np(&a.place),
                a_np(&c.place)
            ),
            _ => format!(
                "Cherche le passage le moins coûteux entre {} et {}",
                a.place, c.place
            ),
        }
    } else {
        match rng.below(2) {
            0 => format!("Calcule le coût de déplacement cumulé depuis {}", a.place),
            _ => format!("Fais la carte du coût cumulé depuis {}", a.place),
        }
    };
    let text = match rng.below(3) {
        0 => "Aucun raster de coût de déplacement n'est ouvert dans le projet : quel fichier de coût faut-il ouvrir ?".to_string(),
        1 => "Le projet ne contient pas de raster de coût. Peux-tu ouvrir le raster de coût de déplacement à utiliser, ou me dire lequel prendre ?".to_string(),
        _ => missing_layer_question(rng, theme("cout").of_none),
    };
    let request = finish(rng, &body, false, false);
    Some(example(
        "chemin_moindre_cout",
        b,
        rng,
        request,
        question(text),
    ))
}

// -- critères, masques et superposition (11.12, lot 2b-2) -------------------
//
// Les critères notés, les masques et les rasters bruts sont apportés par l'utilisateur : des
// rasters de l'inventaire, cités par leur `source` comme le MNT. Jamais le MNT, le raster de
// coût ni un dérivé du MNT comme critère ou masque, jamais une étape (`step:k`) en liste.

/// Les sujets d'un raster noté ou brut, article compris : « l'accessibilité », « le bruit ».
fn raster_subjects(key: &str) -> &'static [&'static str] {
    match key {
        "accessibilite" => &["l'accessibilité", "la note d'accessibilité"],
        "ensoleillement" => &["l'ensoleillement", "la note d'ensoleillement"],
        "proximite_services" => &["la proximité des services", "l'accès aux services"],
        "calme" => &["le calme", "la note de calme"],
        "distance_routes" => &["la distance aux routes", "l'éloignement des routes"],
        "bruit" => &["le bruit", "le niveau de bruit", "le niveau sonore"],
        _ => panic!("raster sans sujet : {key}"),
    }
}

/// Un raster désigné par son sujet, ou une fois sur quatre par le nom de sa couche (quand elle
/// est dans l'inventaire).
fn raster_words(rng: &mut Rng, key: &str, layer: Option<&Layer>) -> String {
    match layer {
        Some(l) if rng.chance(1, 4) => format!("la couche « {} »", l.name),
        _ => pick_str(rng, raster_subjects(key)).to_string(),
    }
}

/// Les mots d'un masque d'exclusion, article compris.
fn mask_words(rng: &mut Rng, key: &str) -> &'static str {
    match key {
        "masque_inondable" => pick_str(
            rng,
            &[
                "les zones inondables",
                "les secteurs inondables",
                "les zones à risque d'inondation",
            ],
        ),
        "masque_protege" => pick_str(
            rng,
            &[
                "les espaces protégés",
                "les espaces naturels protégés",
                "les secteurs protégés",
            ],
        ),
        _ => panic!("masque sans mots : {key}"),
    }
}

/// La question d'un raster nommé par la demande et absent de l'inventaire : l'ouvrir ou le
/// nommer, jamais un plan.
fn missing_raster_question(rng: &mut Rng, key: &str) -> String {
    let of_none = theme(key).of_none;
    match rng.below(3) {
        0 => format!(
            "Aucun raster {of_none} n'est ouvert dans le projet : quel fichier faut-il ouvrir ?"
        ),
        1 => format!(
            "Le projet ne contient pas de raster {of_none}. Peux-tu l'ouvrir ou me dire lequel prendre ?"
        ),
        _ => missing_layer_question(rng, of_none),
    }
}

/// Le MNT ou le raster de coût, parfois, en distraction : ni l'un ni l'autre n'est un critère.
fn raster_distractors(rng: &mut Rng, b: &mut Builder) {
    if rng.chance(1, 4) {
        b.add(rng, "mnt", &[], &[], &[]);
    }
    if rng.chance(1, 6) {
        b.add(rng, "cout", &[], &[], &[]);
    }
}

/// « Combine l'accessibilité (poids 2) et le calme (poids 1), en excluant les zones
/// inondables » : `raster.weighted_overlay`, deux à quatre critères notés dans l'ordre écrit,
/// leurs poids recopiés quand ils sont écrits (une fois sur deux), sinon tous à 1 ; zéro à
/// deux masques écrits, dans l'ordre écrit. Un critère nommé absent de l'inventaire : la
/// question du raster.
fn superposition_ponderee(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    raster_distractors(rng, &mut b);
    let n = rng.range(2, 4) as usize;
    let mut keys = CRITERIA.to_vec();
    rng.shuffle(&mut keys);
    keys.truncate(n);
    let absent = rng.chance(1, 8).then(|| rng.below(n));
    let mut subjects = Vec::with_capacity(n);
    let mut sources = Vec::with_capacity(n);
    for (i, key) in keys.iter().enumerate() {
        if absent == Some(i) {
            b.exclude(key);
            subjects.push(raster_words(rng, key, None));
            sources.push(String::new());
        } else {
            let layer = b.add_raster(rng, key, 2154);
            subjects.push(raster_words(rng, key, Some(&layer)));
            sources.push(layer.source);
        }
    }
    let mut masks = MASKS.to_vec();
    rng.shuffle(&mut masks);
    masks.truncate(rng.below(3));
    let mut mask_sources = Vec::with_capacity(masks.len());
    let mut mask_texts = Vec::with_capacity(masks.len());
    for key in &masks {
        mask_sources.push(b.add_raster(rng, key, 2154).source);
        mask_texts.push(mask_words(rng, key).to_string());
    }
    // Poids écrits une fois sur deux : dans la liste, après elle, ou « compte double ».
    let mut weights = vec![1u64; n];
    let (list, tail) = if rng.chance(1, 2) {
        match rng.below(4) {
            0 | 1 => {
                let inline = rng.chance(1, 2);
                let items: Vec<String> = subjects
                    .iter()
                    .zip(weights.iter_mut())
                    .map(|(s, w)| {
                        *w = rng.range(1, 5);
                        if inline {
                            format!("{s} (poids {w})")
                        } else {
                            format!("{s} avec un poids de {w}")
                        }
                    })
                    .collect();
                (enumerate(&items), String::new())
            }
            2 => {
                let parts: Vec<String> = subjects
                    .iter()
                    .zip(weights.iter_mut())
                    .enumerate()
                    .map(|(i, (s, w))| {
                        *w = rng.range(1, 5);
                        if i == 0 {
                            format!("avec un poids de {w} pour {s}")
                        } else {
                            format!("de {w} pour {s}")
                        }
                    })
                    .collect();
                (enumerate(&subjects), format!(", {}", enumerate(&parts)))
            }
            _ => {
                let d = rng.below(n);
                weights[d] = 2;
                let tail = if rng.chance(1, 2) {
                    format!(", {} comptant double", subjects[d])
                } else {
                    format!(", en comptant double {}", subjects[d])
                };
                (enumerate(&subjects), tail)
            }
        }
    } else {
        let tail = pick_str(
            rng,
            &[
                "",
                "",
                " à poids égaux",
                " avec le même poids pour chacun",
                ", tous au même poids",
            ],
        );
        (enumerate(&subjects), tail.to_string())
    };
    let excl = if mask_texts.is_empty() {
        String::new()
    } else {
        let m = enumerate(&mask_texts);
        match rng.below(4) {
            0 => format!(", en excluant {m}"),
            1 => {
                let bare: Vec<String> = mask_texts
                    .iter()
                    .map(|t| t.strip_prefix("les ").unwrap_or(t).to_string())
                    .collect();
                format!(", hors {}", enumerate(&bare))
            }
            2 => format!(" en écartant {m}"),
            _ => format!(", sans {m}"),
        }
    };
    let (body, q) = match rng.below(6) {
        0 => (format!("Combine {list}{tail}{excl}"), false),
        1 => (format!("Superpose {list}{tail}{excl}"), false),
        2 => (
            format!("Fais une superposition pondérée des critères suivants : {list}{tail}{excl}"),
            false,
        ),
        3 => (
            format!("Calcule un score d'aptitude en combinant {list}{tail}{excl}"),
            false,
        ),
        4 => (
            format!("Croise {list} dans une analyse multicritère{tail}{excl}"),
            false,
        ),
        _ => (
            format!("Quelles zones ressortent quand on combine {list}{tail}{excl}"),
            true,
        ),
    };
    let request = finish(rng, &body, q, false);
    let expected = match absent {
        Some(i) => question(missing_raster_question(rng, keys[i])),
        None => {
            let criteria: Vec<Value> = sources
                .iter()
                .zip(&weights)
                .map(|(s, w)| json!({"raster": s, "weight": w}))
                .collect();
            let mut params = json!({"criteria": criteria});
            if !mask_sources.is_empty() {
                params["exclusions"] = json!(mask_sources);
            }
            calls(vec![
                json!({"command": "raster.weighted_overlay", "params": params}),
            ])
        }
    };
    Some(example("superposition_ponderee", b, rng, request, expected))
}

/// « Classe la distance aux routes en moins de 100 m, de 100 à 500 m et plus de 500 m » :
/// `raster.reclassify` sur la `source` d'un raster brut (distance aux routes, bruit), sans
/// étape de terrain. Seuil unique (`{min}` ou `{max}`, valeur 1) ou deux à trois classes à
/// bornes contiguës, valeurs 1, 2, 3 dans l'ordre sauf si elles sont écrites (une fois sur
/// trois). Un raster nommé absent de l'inventaire : la question du raster.
fn reclassement_raster(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    raster_distractors(rng, &mut b);
    let key = *rng.pick(&RAW_RASTERS);
    let raster = if rng.chance(1, 8) {
        b.exclude(key);
        None
    } else {
        Some(b.add_raster(rng, key, 2154))
    };
    let s = raster_words(rng, key, raster.as_ref());
    let noise = key == "bruit";
    let values: &[u64] = if noise {
        &[45, 50, 55, 60, 65, 70, 75]
    } else {
        &[50, 100, 150, 200, 300, 500, 1000]
    };
    let u = |n: u64| {
        if noise {
            format!("{n} dB")
        } else {
            format!("{} m", thousands(n))
        }
    };
    let (body, q, classes) = match rng.below(3) {
        0 => {
            let n = *rng.pick(values);
            let t = u(n);
            let (body, q) = match rng.below(4) {
                0 => (format!("Montre les zones où {s} dépasse {t}"), false),
                1 => (
                    format!("Garde seulement les cellules où {s} est au-dessus de {t}"),
                    false,
                ),
                2 => (
                    format!("Repère les secteurs où {s} fait plus de {t}"),
                    false,
                ),
                _ => (format!("Où est-ce que {s} dépasse {t}"), true),
            };
            (body, q, json!([{"min": n, "value": 1}]))
        }
        1 => {
            let n = *rng.pick(values);
            let t = u(n);
            let (body, q) = match rng.below(4) {
                0 => (format!("Montre les zones où {s} reste sous {t}"), false),
                1 => (
                    format!("Garde seulement les cellules où {s} est en dessous de {t}"),
                    false,
                ),
                2 => (
                    format!("Sélectionne les secteurs où {s} fait moins de {t}"),
                    false,
                ),
                _ => (format!("Où est-ce que {s} reste sous {t}"), true),
            };
            (body, q, json!([{"max": n, "value": 1}]))
        }
        _ => {
            let i = rng.below(values.len() - 1);
            let a = values[i];
            let three = rng.chance(1, 2);
            let k = if three { 3 } else { 2 };
            // Valeurs écrites une fois sur trois ; sinon 1, 2, 3 dans l'ordre des classes.
            let written: Option<Vec<u64>> = rng.chance(1, 3).then(|| match rng.below(3) {
                0 => (1..=k).rev().collect(),
                1 => (1..=k).map(|v| v * 10).collect(),
                _ => (1..=k).rev().map(|v| v * 10).collect(),
            });
            let vals = written.clone().unwrap_or_else(|| (1..=k).collect());
            let (body, classes) = if three {
                let c = values[rng.range((i + 1) as u64, (values.len() - 1) as u64) as usize];
                let (ta, tc) = (u(a), u(c));
                let body = match rng.below(3) {
                    0 => format!("Classe {s} en moins de {ta}, de {a} à {tc} et plus de {tc}"),
                    1 => format!(
                        "Reclasse {s} en trois classes : sous {ta}, entre {ta} et {tc}, au-delà de {tc}"
                    ),
                    _ => format!(
                        "Découpe {s} en trois catégories : moins de {ta}, {ta} à {tc}, plus de {tc}"
                    ),
                };
                (
                    body,
                    json!([{"max": a, "value": vals[0]}, {"min": a, "max": c, "value": vals[1]},
                        {"min": c, "value": vals[2]}]),
                )
            } else {
                let ta = u(a);
                let body = match rng.below(3) {
                    0 => format!("Classe {s} en deux catégories : moins de {ta} et plus de {ta}"),
                    1 => format!("Reclasse {s} en deux classes, sous {ta} et au-dessus de {ta}"),
                    _ => format!("Découpe {s} en deux classes : moins de {ta}, plus de {ta}"),
                };
                (
                    body,
                    json!([{"max": a, "value": vals[0]}, {"min": a, "value": vals[1]}]),
                )
            };
            let body = match written {
                Some(w) => {
                    let w: Vec<String> = w.iter().map(u64::to_string).collect();
                    let w = enumerate(&w);
                    match rng.below(2) {
                        0 => format!("{body}, notées {w}"),
                        _ => format!("{body}, avec les valeurs {w}"),
                    }
                }
                None => body,
            };
            (body, false, classes)
        }
    };
    let request = finish(rng, &body, q, false);
    let expected = match &raster {
        Some(r) => calls(vec![json!({"command": "raster.reclassify",
            "params": {"raster": r.source, "classes": classes}})]),
        None => question(missing_raster_question(rng, key)),
    };
    Some(example("reclassement_raster", b, rng, request, expected))
}

// -- géométrie (11.12, lot 1) ------------------------------------------------
//
// Règle du catalogue : entre WGS84, UTM et MGRS, `coord.convert` ; dès qu'un autre système
// est en jeu (Lambert-93, ou le système d'un MNT), `data.transform_point`. Une conversion ou
// une reprojection est toujours une étape visible du plan, jamais insérée en silence.

/// Un point écrit pour une demande de géométrie : le point attendu, la phrase « le point … »
/// qui le porte avec son système, ses coordonnées nues et son système écrit (pour plusieurs
/// points écrits dans un seul système, nommé une fois).
struct Coord {
    point: Value,
    place: String,
    bare: String,
    crs: String,
}

impl Coord {
    fn origin(self) -> Origin {
        Origin {
            coords: format!(" ({}, {})", self.bare, self.crs),
            point: self.point,
            place: self.place,
        }
    }
}

/// Un point en WGS84, quatre décimales écrites à la virgule : près de Paris (zone UTM 31), ou
/// dans l'Est (`east`, zone 32).
fn wgs84_coord(rng: &mut Rng, east: bool) -> Coord {
    let (lon, lat) = if east {
        (rng.range(62_000, 78_000), rng.range(478_000, 490_000))
    } else {
        (rng.range(22_000, 26_000), rng.range(486_000, 490_000))
    };
    let text = |n: u64| format!("{},{:04}", n / 10_000, n % 10_000);
    let (xs, ys) = (text(lon), text(lat));
    let crs = (*rng.pick(&["WGS84", "EPSG:4326", "WGS 84"])).to_string();
    let bare = format!("{ys} N, {xs} E");
    let place = match rng.below(3) {
        0 => format!("le point de longitude {xs} et de latitude {ys} ({crs})"),
        1 => format!("le point GPS {bare} ({crs})"),
        _ => format!("le point {bare} ({crs})"),
    };
    Coord {
        point: json!({"x": lon as f64 / 10_000.0, "y": lat as f64 / 10_000.0, "epsg": 4326}),
        place,
        bare,
        crs,
    }
}

/// Coordonnées projetées écrites « x / y », « X = x, Y = y » ou « (x ; y) ».
fn projected_coord(rng: &mut Rng, x: u64, y: u64, epsg: u32, crs: String) -> Coord {
    let (xs, ys) = if rng.chance(1, 2) {
        (thousands(x), thousands(y))
    } else {
        (x.to_string(), y.to_string())
    };
    let (bare, place) = match rng.below(3) {
        0 => {
            let bare = format!("{xs} / {ys}");
            let place = format!("le point {bare} ({crs})");
            (bare, place)
        }
        1 => {
            let bare = format!("X = {xs}, Y = {ys}");
            let place = format!("le point {bare} en {crs}");
            (bare, place)
        }
        _ => {
            let bare = format!("({xs} ; {ys})");
            let place = format!("le point {bare} en {crs}");
            (bare, place)
        }
    };
    Coord {
        point: json!({"x": x, "y": y, "epsg": epsg}),
        place,
        bare,
        crs,
    }
}

/// Un point en UTM du WGS84, zone nord 31 ou 32, sa zone écrite avec l'hémisphère.
fn utm_coord(rng: &mut Rng, zone: u32) -> Coord {
    let x = if zone == 31 {
        rng.range(40_000, 70_000) * 10
    } else {
        rng.range(28_000, 45_000) * 10
    };
    let y = rng.range(530_000, 550_000) * 10;
    let crs = match rng.below(4) {
        0 => format!("UTM {zone}N"),
        1 => format!("UTM {zone} N"),
        2 => format!("UTM zone {zone} Nord"),
        _ => format!("UTM{zone}N"),
    };
    projected_coord(rng, x, y, 32600 + zone, crs)
}

/// Un point en Lambert-93, près de Paris.
fn lambert_coord(rng: &mut Rng) -> Coord {
    let x = rng.range(60_000, 70_000) * 10;
    let y = rng.range(678_000, 690_000) * 10;
    let crs = (*rng.pick(&["Lambert-93", "Lambert 93", "EPSG:2154"])).to_string();
    projected_coord(rng, x, y, 2154, crs)
}

/// Les systèmes d'un point de géométrie : WGS84, ou UTM de la zone normale de la région.
fn wgs84_or_utm(rng: &mut Rng, east: bool) -> Coord {
    if rng.chance(1, 2) {
        wgs84_coord(rng, east)
    } else {
        utm_coord(rng, if east { 32 } else { 31 })
    }
}

/// Une référence MGRS valide, tirée par `Mgrs::from_lat_lon` d'un point WGS84 de la région :
/// sa forme canonique (celle du plan) et sa forme écrite (espacée, compacte ou mixte).
struct MgrsText {
    canonical: String,
    written: String,
}

fn mgrs_ref(rng: &mut Rng, east: bool) -> Option<MgrsText> {
    let c = wgs84_coord(rng, east);
    let (lon, lat) = (c.point["x"].as_f64()?, c.point["y"].as_f64()?);
    let precision = *rng.pick(&[5u8, 5, 4, 3]);
    let m = cassini_core::geodesy::Mgrs::from_lat_lon(lat, lon, precision)?;
    let p = usize::from(precision);
    let (e, n) = (format!("{:0p$}", m.easting), format!("{:0p$}", m.northing));
    let written = match rng.below(3) {
        0 => format!("{}{} {}{} {e} {n}", m.zone, m.band, m.column, m.row),
        1 => m.canonical(),
        _ => format!("{}{}{}{} {e} {n}", m.zone, m.band, m.column, m.row),
    };
    Some(MgrsText {
        canonical: m.canonical(),
        written,
    })
}

/// « le point … » → « du point … », « la référence … » → « de la référence … ».
fn de_np(np: &str) -> String {
    if let Some(rest) = np.strip_prefix("le ") {
        format!("du {rest}")
    } else if let Some(rest) = np.strip_prefix("les ") {
        format!("des {rest}")
    } else {
        de_word(np)
    }
}

/// « le point … » → « au point … ».
fn a_np(np: &str) -> String {
    match np.strip_prefix("le ") {
        Some(rest) => format!("au {rest}"),
        None => format!("à {np}"),
    }
}

fn transform_call(point: Value, target: u32) -> Value {
    json!({"command": "data.transform_point",
        "params": {"point": point, "target_epsg": target}})
}

/// Un azimut écrit, toujours avec son unité : degrés (« 45° », « 45 degrés ») ou grades
/// (« 50 gr », « 50 grades ») ; `lo` et `hi` bornent la valeur en degrés.
fn azimuth_words(rng: &mut Rng, gon: bool, lo: u64, hi: u64) -> (Value, String) {
    if gon {
        let n = rng.range(lo * 10 / 9 + 1, hi * 10 / 9);
        let unit = *rng.pick(&["gr", "grades"]);
        (json!({"value": n, "unit": "gon"}), format!("{n} {unit}"))
    } else {
        let n = rng.range(lo, hi);
        let text = if rng.chance(1, 2) {
            format!("{n}°")
        } else {
            format!("{n} degrés")
        };
        (json!({"value": n, "unit": "deg"}), text)
    }
}

/// L'azimut `degrees` (dans [0, 360)) écrit arrondi à l'unité, en degrés ou en grades, avec son
/// unité, comme `azimuth_words`.
fn angle_words(rng: &mut Rng, gon: bool, degrees: f64) -> (Value, String) {
    if gon {
        let n = ((degrees * 400.0 / 360.0).round() as u64) % 400;
        let unit = *rng.pick(&["gr", "grades"]);
        (json!({"value": n, "unit": "gon"}), format!("{n} {unit}"))
    } else {
        let n = (degrees.round() as u64) % 360;
        let text = if rng.chance(1, 2) {
            format!("{n}°")
        } else {
            format!("{n} degrés")
        };
        (json!({"value": n, "unit": "deg"}), text)
    }
}

/// Latitude et longitude d'un point de géométrie, en WGS84 ou en UTM nord.
fn lat_lon(c: &Coord) -> Option<(f64, f64)> {
    let (x, y) = (c.point["x"].as_f64()?, c.point["y"].as_f64()?);
    let epsg = u32::try_from(c.point["epsg"].as_u64()?).ok()?;
    if epsg == 4326 {
        return Some((y, x));
    }
    let (zone, north) = cassini_core::geodesy::Utm::zone_of_epsg(epsg)?;
    Some(cassini_core::geodesy::from_utm(
        &cassini_core::geodesy::Utm {
            zone,
            north,
            easting: x,
            northing: y,
        },
    ))
}

/// Une distance géodésique écrite en chiffres : la `Distance` attendue et sa forme écrite.
fn geo_distance(rng: &mut Rng) -> (Value, String) {
    match rng.below(4) {
        0 => {
            let n = rng.range(4, 60) * 50;
            let unit = *rng.pick(&["m", "mètres"]);
            (
                json!({"value": n, "unit": "m"}),
                format!("{} {unit}", thousands(n)),
            )
        }
        1 => {
            let (v, text) = *rng.pick(&[(1.5, "1,5"), (2.5, "2,5"), (3.2, "3,2"), (7.5, "7,5")]);
            (json!({"value": v, "unit": "km"}), format!("{text} km"))
        }
        _ => {
            let n = rng.range(2, 25);
            let unit = *rng.pick(&["km", "kilomètres"]);
            (json!({"value": n, "unit": "km"}), format!("{n} {unit}"))
        }
    }
}

/// Le résultat demandé en MGRS : la phrase et l'étape `coord.convert` sur le point de l'étape
/// `k`, la précision seulement si elle est écrite.
fn mgrs_result(rng: &mut Rng, k: usize) -> (String, Value) {
    let mut params = json!({"point": format!("step:{k}.point"), "to": ["mgrs"]});
    let text = if rng.chance(1, 3) {
        let p = rng.range(1, 5);
        params["mgrs_precision"] = json!(p);
        format!(", résultat en MGRS à {p} chiffres")
    } else {
        (*rng.pick(&[
            ", et donne le résultat en MGRS",
            ", résultat en référence MGRS",
            ", puis rends-le en MGRS",
        ]))
        .to_string()
    };
    (text, json!({"command": "coord.convert", "params": params}))
}

/// Ce qu'une conversion demande, écrit : (« en UTM », « les coordonnées UTM »).
fn convert_words(
    rng: &mut Rng,
    target: &str,
    precision: Option<u64>,
    zone: Option<u64>,
) -> (String, String) {
    match target {
        "wgs84" => match rng.below(3) {
            0 => (
                "en latitude et longitude".into(),
                "la latitude et la longitude".into(),
            ),
            1 => ("en WGS84".into(), "les coordonnées WGS84".into()),
            _ => (
                "en coordonnées géographiques".into(),
                "les coordonnées géographiques".into(),
            ),
        },
        "utm" => {
            let (en, noun) = if rng.chance(1, 2) {
                ("en UTM", "les coordonnées UTM")
            } else {
                ("en coordonnées UTM", "les coordonnées UTM")
            };
            match zone {
                Some(z) => (
                    format!("{en} dans le fuseau {z}"),
                    format!("{noun} dans le fuseau {z}"),
                ),
                None => (en.into(), noun.into()),
            }
        }
        _ => {
            let (en, noun) = if rng.chance(1, 2) {
                ("en MGRS", "la référence MGRS")
            } else {
                ("en référence MGRS", "la référence MGRS")
            };
            match precision {
                Some(p) => (
                    format!("{en} à {p} chiffres"),
                    format!("{noun} à {p} chiffres"),
                ),
                None => (en.into(), noun.into()),
            }
        }
    }
}

/// « Donne la référence MGRS du point 48,8566 N, 2,3522 E (WGS84) », « Convertis
/// 31U DQ 52000 12000 en latitude et longitude » : `coord.convert` entre WGS84, UTM et MGRS,
/// `mgrs_precision` et `utm_zone` seulement s'ils sont écrits. Un point en Lambert-93 passe
/// d'abord par `data.transform_point` vers le WGS84 (deux étapes).
fn conversion_coordonnees(rng: &mut Rng) -> Option<Example> {
    let b = Builder::new();
    let east = rng.chance(1, 3);
    let normal_zone: u64 = if east { 32 } else { 31 };
    let roll = rng.below(4);
    // Le point à convertir, ou l'étape de reprojection d'un point en Lambert-93.
    let (np, mut params, offered, first): (String, Value, [&str; 2], Option<Value>) = match roll {
        0 => {
            let c = wgs84_coord(rng, east);
            (c.place, json!({"point": c.point}), ["utm", "mgrs"], None)
        }
        1 => {
            let c = utm_coord(rng, normal_zone as u32);
            (c.place, json!({"point": c.point}), ["wgs84", "mgrs"], None)
        }
        2 => {
            let m = mgrs_ref(rng, east)?;
            let np = match rng.below(3) {
                0 => m.written.clone(),
                1 => format!("la référence MGRS {}", m.written),
                _ => format!("le point MGRS {}", m.written),
            };
            (np, json!({"mgrs": m.canonical}), ["wgs84", "utm"], None)
        }
        _ => {
            let c = lambert_coord(rng);
            let first = transform_call(c.point, 4326);
            (
                c.place,
                json!({"point": "step:1.point"}),
                ["utm", "mgrs"],
                Some(first),
            )
        }
    };
    let mut targets: Vec<&str> = if rng.chance(1, 3) {
        offered.to_vec()
    } else {
        vec![*rng.pick(&offered)]
    };
    if targets.len() == 2 && rng.chance(1, 2) {
        targets.reverse();
    }
    let precision = (targets.contains(&"mgrs") && rng.chance(1, 3)).then(|| rng.range(1, 5));
    // Le fuseau imposé reste à une zone de celle du point (`coord.out_of_range` au-delà).
    let zone = (targets.contains(&"utm") && roll != 3 && rng.chance(1, 4)).then(|| {
        if rng.chance(1, 2) {
            normal_zone + 1
        } else {
            normal_zone - 1
        }
    });
    let (mut en, mut noun) = (Vec::new(), Vec::new());
    for t in &targets {
        let (e, n) = convert_words(rng, t, precision, zone);
        en.push(e);
        noun.push(n);
    }
    let (en, noun) = (en.join(" et "), noun.join(" et "));
    let (body, q) = match rng.below(5) {
        0 => (format!("Convertis {np} {en}"), false),
        1 => (format!("Donne {noun} {}", de_np(&np)), false),
        2 => (format!("Exprime {np} {en}"), false),
        3 => (format!("Peux-tu convertir {np} {en}"), true),
        _ => (format!("Traduis {np} {en}"), false),
    };
    params["to"] = json!(targets);
    if let Some(p) = precision {
        params["mgrs_precision"] = json!(p);
    }
    if let Some(z) = zone {
        params["utm_zone"] = json!(z);
    }
    let convert = json!({"command": "coord.convert", "params": params});
    let list = match first {
        Some(call) => vec![call, convert],
        None => vec![convert],
    };
    let request = finish(rng, &body, q, false);
    Some(example(
        "conversion_coordonnees",
        b,
        rng,
        request,
        calls(list),
    ))
}

/// « Reprojette le point … (Lambert-93) en WGS84 », « Donne les coordonnées du point … (UTM 31N)
/// en Lambert-93 » : dès qu'un système hors WGS84, UTM et MGRS est en jeu, une seule étape
/// `data.transform_point`, le système d'arrivée écrit.
fn reprojection_point(rng: &mut Rng) -> Option<Example> {
    let b = Builder::new();
    let east = rng.chance(1, 3);
    const TO_LAMBERT: &[&str] = &["en Lambert-93", "en Lambert 93", "vers l'EPSG:2154"];
    let (np, point, target, words): (String, Value, u32, String) = match rng.below(4) {
        0 => {
            let c = lambert_coord(rng);
            let w = *rng.pick(&[
                "en WGS84",
                "en latitude et longitude WGS84",
                "vers l'EPSG:4326",
                "en coordonnées GPS (WGS84)",
            ]);
            (c.place, c.point, 4326, w.to_string())
        }
        1 => {
            let c = wgs84_coord(rng, east);
            (c.place, c.point, 2154, rng.pick(TO_LAMBERT).to_string())
        }
        2 => {
            let c = utm_coord(rng, if east { 32 } else { 31 });
            (c.place, c.point, 2154, rng.pick(TO_LAMBERT).to_string())
        }
        _ => {
            let c = wgs84_or_utm(rng, east);
            let (target, w) = *rng.pick(&[
                (27572, "en Lambert II étendu (EPSG:27572)"),
                (3857, "en Web Mercator (EPSG:3857)"),
                (27572, "vers l'EPSG:27572"),
                (3857, "vers l'EPSG:3857"),
            ]);
            (c.place, c.point, target, w.to_string())
        }
    };
    let (body, q) = match rng.below(5) {
        0 => (format!("Reprojette {np} {words}"), false),
        1 => (format!("Convertis {np} {words}"), false),
        2 => (
            format!("Donne les coordonnées {} {words}", de_np(&np)),
            false,
        ),
        3 => (format!("Transforme {np} {words}"), false),
        _ => (
            format!("Quelles sont les coordonnées {} {words}", de_np(&np)),
            true,
        ),
    };
    let request = finish(rng, &body, q, false);
    Some(example(
        "reprojection_point",
        b,
        rng,
        request,
        calls(vec![transform_call(point, target)]),
    ))
}

/// « Quelle distance sépare le point … du point … », « distance et azimut entre les points …
/// et … (UTM 31N), azimut en grades » : `geo.distance_azimuth`, `unit` seulement si elle est
/// écrite (`gon` pour les grades). Des points en Lambert-93 sont d'abord reprojetés en WGS84,
/// chacun par son étape.
fn distance_azimut(rng: &mut Rng) -> Option<Example> {
    let b = Builder::new();
    let east = rng.chance(1, 3);
    let roll = rng.below(3);
    let make = |rng: &mut Rng| match roll {
        0 => wgs84_coord(rng, east),
        1 => utm_coord(rng, if east { 32 } else { 31 }),
        _ => lambert_coord(rng),
    };
    let a = make(rng);
    let c = make(rng);
    if a.point == c.point {
        return None;
    }
    let (mut body, q) = if rng.chance(1, 3) {
        // Le système écrit une fois pour les deux points.
        let suffix = if a.bare.starts_with('X') || a.bare.starts_with('(') {
            format!("en {}", a.crs)
        } else {
            format!("({})", a.crs)
        };
        let between = format!("les points {} et {} {suffix}", a.bare, c.bare);
        match rng.below(3) {
            0 => (
                format!("Donne la distance et l'azimut entre {between}"),
                false,
            ),
            1 => (format!("Quelle est la distance entre {between}"), true),
            _ => (format!("Calcule distance et azimut entre {between}"), false),
        }
    } else {
        let (pa, pc) = (&a.place, &c.place);
        match rng.below(5) {
            0 => (format!("Quelle distance sépare {pa} {}", de_np(pc)), true),
            1 => (
                format!("Calcule la distance et l'azimut {} {}", de_np(pa), a_np(pc)),
                false,
            ),
            2 => (
                format!("Donne la distance et l'azimut entre {pa} et {pc}"),
                false,
            ),
            3 => (format!("Quel est l'azimut {} vers {pc}", de_np(pa)), true),
            _ => (
                format!("Mesure la distance géodésique entre {pa} et {pc}"),
                false,
            ),
        }
    };
    let unit = match rng.below(4) {
        0 => {
            body.push_str(pick_str(
                rng,
                &[", azimut en grades", " (angles en grades)"],
            ));
            Some("gon")
        }
        1 => {
            body.push_str(pick_str(
                rng,
                &[", azimut en degrés", " (angles en degrés)"],
            ));
            Some("deg")
        }
        _ => None,
    };
    let mut list = Vec::new();
    let (from, to) = if roll == 2 {
        list.push(transform_call(a.point, 4326));
        list.push(transform_call(c.point, 4326));
        (json!("step:1.point"), json!("step:2.point"))
    } else {
        (a.point, c.point)
    };
    let mut params = json!({"from": from, "to": to});
    if let Some(u) = unit {
        params["unit"] = json!(u);
    }
    list.push(json!({"command": "geo.distance_azimuth", "params": params}));
    let request = finish(rng, &body, q, false);
    Some(example("distance_azimut", b, rng, request, calls(list)))
}

/// « Quel point atteint-on à 3 km du point …, azimut 45° » : `geo.destination_point`, l'azimut
/// avec son unité et la distance tels qu'écrits ; résultat demandé en MGRS → `coord.convert`
/// sur `step:1.point`.
fn point_projete(rng: &mut Rng) -> Option<Example> {
    let b = Builder::new();
    let east = rng.chance(1, 3);
    let from = wgs84_or_utm(rng, east);
    let gon = rng.chance(1, 4);
    let (az, az_text) = azimuth_words(rng, gon, 10, 350);
    let (d, d_text) = geo_distance(rng);
    let p = &from.place;
    let de_p = de_np(p);
    let (mut body, q) = match rng.below(5) {
        0 => (
            format!("Quel point atteint-on à {d_text} {de_p}, azimut {az_text}"),
            true,
        ),
        1 => (
            format!("Calcule le point situé à {d_text} {de_p} dans l'azimut {az_text}"),
            false,
        ),
        2 => (
            format!("Depuis {p}, avance de {d_text} à l'azimut {az_text}"),
            false,
        ),
        3 => (
            format!("Donne les coordonnées du point à {d_text} {de_p}, à l'azimut {az_text}"),
            false,
        ),
        _ => (
            format!("Projette un point à {d_text} {de_p}, azimut {az_text}"),
            false,
        ),
    };
    let mut list = vec![json!({"command": "geo.destination_point",
        "params": {"from": from.point, "azimuth": az, "distance": d}})];
    if rng.chance(1, 3) {
        let (text, call) = mgrs_result(rng, 1);
        body.push_str(&text);
        list.push(call);
    }
    let request = finish(rng, &body, q, false);
    Some(example("point_projete", b, rng, request, calls(list)))
}

/// « Recoupe la visée du point … à l'azimut 40° avec celle du point … à l'azimut 310° » :
/// `geo.bearing_intersection`, chaque station avec son azimut écrit (unité comprise) ;
/// résultat demandé en MGRS → `coord.convert` sur `step:1.point`.
fn recoupement(rng: &mut Rng) -> Option<Example> {
    let b = Builder::new();
    let east = rng.chance(1, 3);
    let utm = rng.chance(1, 2);
    let make = |rng: &mut Rng| {
        if utm {
            utm_coord(rng, if east { 32 } else { 31 })
        } else {
            wgs84_coord(rng, east)
        }
    };
    let (mut a, mut c) = (make(rng), make(rng));
    // La première station à l'ouest, visée vers le nord-est ; la seconde à l'est, vers le
    // nord-ouest : les deux visées se coupent en avant des stations.
    let x = |p: &Coord| p.point["x"].as_f64().unwrap_or(0.0);
    if x(&a) == x(&c) {
        return None;
    }
    if x(&a) > x(&c) {
        std::mem::swap(&mut a, &mut c);
    }
    // Les azimuts visent un vrai point au nord des deux stations, à une distance du même ordre
    // que leur écart : tirés au hasard, les deux visées ne se coupaient pas toujours en avant
    // des stations (`geo.no_intersection`).
    let (lat_a, lon_a) = lat_lon(&a)?;
    let (lat_c, lon_c) = lat_lon(&c)?;
    let k = rng.range(40, 120) as f64 / 100.0;
    let lat_t = lat_a.max(lat_c) + (lon_c - lon_a) * k;
    let lon_t = (lon_a + lon_c) / 2.0 + (rng.range(0, 20) as f64 - 10.0) / 100.0 * (lon_c - lon_a);
    let bearing = |lat: f64, lon: f64| {
        cassini_core::geodesic::inverse(lat, lon, lat_t, lon_t)
            .azi1
            .rem_euclid(360.0)
    };
    let (b1, b2) = (bearing(lat_a, lon_a), bearing(lat_c, lon_c));
    // Les visées se coupent sous un angle franc (bien plus que le degré du seuil de parallélisme).
    if !(30.0..=150.0).contains(&(b1 - b2).rem_euclid(360.0)) {
        return None;
    }
    let gon = rng.chance(1, 4);
    let (az1, t1) = angle_words(rng, gon, b1);
    let (az2, t2) = angle_words(rng, gon, b2);
    let (pa, pc) = (&a.place, &c.place);
    let (mut body, q) = match rng.below(5) {
        0 => (
            format!(
                "Recoupe la visée {} à l'azimut {t1} avec celle {} à l'azimut {t2}",
                de_np(pa),
                de_np(pc)
            ),
            false,
        ),
        1 => (
            format!(
                "Où se croisent l'azimut {t1} relevé depuis {pa} et l'azimut {t2} relevé depuis {pc}"
            ),
            true,
        ),
        2 => (
            format!("Calcule l'intersection des visées : {t1} depuis {pa}, {t2} depuis {pc}"),
            false,
        ),
        3 => (
            format!("Depuis {pa} je relève {t1}, depuis {pc} je relève {t2} : trouve le point visé"),
            false,
        ),
        _ => (
            format!("Fais un recoupement entre {pa} (azimut {t1}) et {pc} (azimut {t2})"),
            false,
        ),
    };
    let mut list = vec![json!({"command": "geo.bearing_intersection", "params": {
        "first": a.point, "first_azimuth": az1, "second": c.point, "second_azimuth": az2}})];
    if rng.chance(1, 3) {
        let (text, call) = mgrs_result(rng, 1);
        body.push_str(&text);
        list.push(call);
    }
    let request = finish(rng, &body, q, false);
    Some(example("recoupement", b, rng, request, calls(list)))
}

/// Les étapes qui portent un point d'un autre système dans celui du MNT, ajoutées à `steps` :
/// `data.transform_point` vers `epsg` ; une référence MGRS passe d'abord par `coord.convert`
/// vers le WGS84. Rend la phrase du point et la référence `step:k.point` du point reprojeté.
fn into_dem_system(rng: &mut Rng, epsg: u32, steps: &mut Vec<Value>) -> Option<(String, String)> {
    let place = if rng.chance(1, 4) {
        let m = mgrs_ref(rng, epsg == 32632)?;
        steps.push(json!({"command": "coord.convert",
            "params": {"mgrs": m.canonical, "to": ["wgs84"]}}));
        let k = steps.len();
        steps.push(transform_call(json!(format!("step:{k}.wgs84")), epsg));
        match rng.below(2) {
            0 => format!("le point MGRS {}", m.written),
            _ => format!("la référence MGRS {}", m.written),
        }
    } else {
        let o = foreign_point(rng, epsg)?;
        steps.push(transform_call(o.point, epsg));
        o.place
    };
    Some((place, format!("step:{}.point", steps.len())))
}

/// Un champ de vision ou une ligne de vue depuis un point écrit dans un autre système que le
/// MNT (WGS84, UTM d'une autre zone, Lambert-93 sur un MNT en UTM, MGRS) : la reprojection vers
/// le système du MNT est une étape visible, puis la commande de terrain prend `step:k.point`.
/// Remplace le refus `NO_TERRAIN_CRS` de 5.41, devenu faux avec `data.transform_point`. Sans le
/// système du MNT à l'inventaire, l'exemple n'est pas gardé.
fn terrain_autre_systeme(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let epsg = terrain_epsg(rng);
    let dem = b.add_dem(rng, epsg);
    let mut steps = Vec::new();
    let (pa, observer) = into_dem_system(rng, epsg, &mut steps)?;
    let line = rng.chance(1, 3);
    let (mut body, q, mut params, command) = if line {
        let (pc, target) = into_dem_system(rng, epsg, &mut steps)?;
        let (body, q) = match rng.below(3) {
            0 => (
                format!("Y a-t-il une ligne de vue entre {pa} et {pc}"),
                true,
            ),
            1 => (format!("Vérifie si {pc} est visible depuis {pa}"), false),
            _ => (
                format!("Calcule la ligne de vue {} vers {pc}", de_np(&pa)),
                false,
            ),
        };
        let params = json!({"dem": dem.source, "observer": observer, "target": target});
        (body, q, params, "terrain.line_of_sight")
    } else {
        let (body, q) = match rng.below(4) {
            0 => (format!("Calcule le champ de vision depuis {pa}"), false),
            1 => (format!("Quelles zones sont visibles depuis {pa}"), true),
            2 => (format!("Montre la zone vue depuis {pa}"), false),
            _ => (format!("Détermine ce qu'on voit depuis {pa}"), false),
        };
        let params = json!({"dem": dem.source, "observer": observer});
        (body, q, params, "terrain.viewshed")
    };
    if rng.chance(1, 2) {
        let (h, text) = eye_clause(rng);
        body.push_str(&text);
        params["observer_height"] = h;
    }
    if !line && rng.chance(1, 3) {
        let (r, text) = radius_clause(rng);
        body.push_str(&text);
        params["radius"] = r;
    }
    steps.push(json!({"command": command, "params": params}));
    let request = finish(rng, &body, q, false);
    let e = example("terrain_autre_systeme", b, rng, request, calls(steps));
    // Le système d'arrivée se lit dans l'inventaire : sans le système du MNT (forme QGIS qui
    // l'a perdu), l'exemple n'est pas gardé.
    dem_has_epsg(&e.inventory).then_some(e)
}

/// Lieux nommés sans coordonnées, d'où partirait une vue : aucune commande ne trouve un point
/// à partir d'un nom.
fn named_viewpoint(rng: &mut Rng) -> String {
    let commune = *rng.pick(COMMUNES);
    match rng.below(5) {
        0 => format!("le clocher {}", de_place(commune)),
        1 => format!("le château d'eau {}", de_place(commune)),
        2 => format!("la mairie {}", de_place(commune)),
        3 => "le sommet de la colline".to_string(),
        _ => "le belvédère du parc".to_string(),
    }
}

/// La question qui demande le point manquant, en X, Y et système.
fn point_question(rng: &mut Rng, place: &str) -> String {
    // « du clocher », « de la mairie » : jamais « de le ».
    let of_place = match place.strip_prefix("le ") {
        Some(rest) => format!("du {rest}"),
        None => format!("de {place}"),
    };
    match rng.below(4) {
        0 => format!(
            "Je n'ai pas les coordonnées {of_place} : peux-tu donner ce point en X et Y, avec son système de coordonnées ?"
        ),
        1 => format!(
            "Quelles sont les coordonnées {of_place} (X, Y et système, par exemple Lambert-93) ?"
        ),
        2 => "Il me faut le point en coordonnées : quelles sont ses valeurs X et Y, et dans quel système ?".to_string(),
        _ => cassini_core::point::GUARD_QUESTION.to_string(),
    }
}

/// Ce qui manque à une demande de terrain devient une question, jamais un plan deviné : le MNT
/// absent de l'inventaire, un point sans coordonnées, l'unité d'une pente.
fn question_terrain(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let roll = rng.below(4);
    let (body, q, text) = if roll == 0 {
        // Aucun MNT (un raster de coût peut venir en distraction) : on demande d'ouvrir ou de
        // nommer le MNT.
        b.exclude("mnt");
        let epsg = terrain_epsg(rng);
        let (body, q) = match rng.below(3) {
            0 => {
                let (_, unit) = slope_unit(rng);
                (format!("Calcule les pentes du terrain {unit}"), false)
            }
            1 => {
                let p = terrain_point(rng, epsg)?;
                (
                    format!("Calcule le champ de vision depuis {}", p.place),
                    false,
                )
            }
            _ => {
                let (a, c) = (terrain_point(rng, epsg)?, terrain_point(rng, epsg)?);
                (
                    format!("Y a-t-il une ligne de vue entre {} et {}", a.place, c.place),
                    true,
                )
            }
        };
        let text = match rng.below(3) {
            0 => "Aucun modèle numérique de terrain n'est ouvert dans le projet : quel fichier MNT faut-il ouvrir ?".to_string(),
            1 => "Le projet ne contient pas de MNT. Peux-tu ouvrir le modèle numérique de terrain à utiliser, ou me dire lequel prendre ?".to_string(),
            _ => missing_layer_question(rng, theme("mnt").of_none),
        };
        (body, q, text)
    } else if roll == 1 {
        // Pente sans unité : degrés ou pourcentage, jamais une unité devinée.
        let epsg = terrain_epsg(rng);
        let dem = b.add_dem(rng, epsg);
        let of = dem_words(rng, &dem);
        let body = match rng.below(3) {
            0 => format!("Fais la carte des pentes {of}"),
            1 => format!("Calcule la pente {of}"),
            _ => format!("Génère un raster de pente à partir {of}"),
        };
        let text = match rng.below(2) {
            0 => "Dans quelle unité faut-il calculer la pente : en degrés ou en pourcentage ?",
            _ => "La pente en degrés ou en pourcentage ?",
        };
        (body, false, text.to_string())
    } else {
        // Un point sans coordonnées : un lieu nommé, ou une ligne de vue avec un seul point.
        let epsg = terrain_epsg(rng);
        b.add_dem(rng, epsg);
        let place = named_viewpoint(rng);
        let (body, q) = if roll == 2 {
            match rng.below(3) {
                0 => (format!("Calcule le champ de vision depuis {place}"), false),
                1 => (format!("Quelles zones sont visibles depuis {place}"), true),
                _ => (
                    format!("Fais une analyse de visibilité depuis {place}"),
                    false,
                ),
            }
        } else {
            let a = terrain_point(rng, epsg)?;
            match rng.below(3) {
                0 => (
                    format!("Y a-t-il une ligne de vue entre {} et {place}", a.place),
                    true,
                ),
                1 => (
                    format!("Vérifie si {place} est visible depuis {}", a.place),
                    false,
                ),
                _ => (
                    format!("Est-ce qu'on voit {} depuis {place}", a.place),
                    true,
                ),
            }
        };
        (body, q, point_question(rng, &place))
    };
    let request = finish(rng, &body, q, false);
    Some(example("question_terrain", b, rng, request, question(text)))
}

// -- appartenance à une commune --------------------------------------------

/// Tournure explicite d'appartenance à une commune nommée : seule elle crée une étape. Une
/// commune citée seule (« à Orly », ajoutée par `finish`) reste un contexte, sans effet.
fn belonging(rng: &mut Rng, noun: &Noun, commune: &str) -> String {
    let t = match rng.below(5) {
        0 | 1 => format!("qui appartiennent à la commune {}", de_place(commune)),
        2 => format!("{{situé}} sur le territoire {}", de_place(commune)),
        3 => format!("sur le territoire communal {}", de_place(commune)),
        _ => format!(
            "qui se trouvent sur le territoire de la commune {}",
            de_place(commune)
        ),
    };
    fill(&t, noun, "")
}

/// « Les écoles qui appartiennent à la commune de Thiais » : avec une couche de communes,
/// sélection de la commune par son nom, puis intersection ; sans elle, une question ; sans
/// nom de commune, une question aussi.
fn appartenance_commune(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let roll = rng.below(10);
    let t = pick_theme(rng, &b, |t| {
        ordinary(t) && t.key != "communes" && t.key != "quartiers"
    })?;
    let target = b.add(rng, t.key, &[], &[], &[]);
    let commune = *rng.pick(COMMUNES);
    if roll < 6 {
        // La couche de communes est là, avec un champ de nom.
        let communes = b.add(rng, "communes", &["nom"], &[], &[]);
        let name = communes.field("nom")?.name.clone();
        let np = format!("les {} {}", t.noun.pl, belonging(rng, &t.noun, commune));
        let mut steps = vec![
            select(&communes.id, &name, "eq", commune),
            intersect(&target.id, &step(1)),
        ];
        let with_pdf = rng.chance(1, 5);
        let (mut head, q) = open_find(rng, &t.noun, &np, !with_pdf);
        if with_pdf {
            let draft = Draft {
                b: Builder::new(),
                noun: &t.noun,
                np: np.clone(),
                calls: Vec::new(),
            };
            let (clause, title) = pdf_clause(rng, &draft);
            head.push_str(&clause);
            steps.push(pdf(&title));
        }
        let request = finish(rng, &head, q, false);
        return Some(example(
            "appartenance_commune",
            b,
            rng,
            request,
            calls(steps),
        ));
    }
    if roll < 9 {
        // Aucune couche de communes : rien ne dit où passe la limite.
        b.exclude("communes");
        let np = format!("les {} {}", t.noun.pl, belonging(rng, &t.noun, commune));
        let (head, q) = open_find(rng, &t.noun, &np, true);
        let request = finish(rng, &head, q, false);
        let expected = question(missing_layer_question(rng, theme("communes").of_none));
        return Some(example("appartenance_commune", b, rng, request, expected));
    }
    // La couche est là, mais la demande ne dit pas quelle commune.
    let communes = b.add(rng, "communes", &["nom"], &[], &[]);
    let words = match rng.below(3) {
        0 => "qui appartiennent à la commune",
        1 => "{situé} sur le territoire communal",
        _ => "qui se trouvent sur le territoire de la commune",
    };
    let np = format!("les {} {}", t.noun.pl, fill(words, &t.noun, ""));
    let (head, q) = open_find(rng, &t.noun, &np, true);
    let request = finish(rng, &head, q, false);
    let expected = question(match rng.below(2) {
        0 => format!(
            "Quelle commune faut-il retenir, sous le nom qu'elle porte dans la couche « {} » ?",
            communes.name
        ),
        _ => "Quelle commune faut-il retenir ?".to_string(),
    });
    Some(example("appartenance_commune", b, rng, request, expected))
}

// -- isochrone sans coordonnées --------------------------------------------

/// Une isochrone dont l'origine est un lieu nommé, sans coordonnées : aucune commande ne
/// trouve un point à partir d'un nom, la réponse est une question qui demande le point. La
/// couche du lieu est le plus souvent dans l'inventaire (depuis 5.17) : c'est là que le modèle
/// inventait l'origine. « D'une gare », sans dire laquelle, est une question aussi.
fn question_isochrone(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    network(rng, &mut b);
    let commune = *rng.pick(COMMUNES);
    // L'origine : un lieu, le plus souvent présent comme couche (la couche ne donne pas le
    // point), parfois un lieu sans couche.
    let (origin, theme_key) = if rng.chance(5, 6) {
        let (origin, key) = place_with_layer(rng);
        (origin, Some(key))
    } else {
        let origin = match rng.below(3) {
            0 => format!("la mairie {}", de_place(commune)),
            1 => "la place du marché".to_string(),
            _ => format!("l'église {}", de_place(commune)),
        };
        (origin, None)
    };
    let mut present = None;
    if let Some(k) = theme_key {
        if rng.chance(5, 6) && b.free(|t| t.key == k).len() == 1 {
            b.add(rng, k, &[], &[], &[]);
            present = Some(theme(k));
        }
    }
    // « D'une gare » : la couche est là, la demande ne dit pas laquelle.
    let indefinite = present.filter(|_| rng.chance(1, 5));
    let origin = match indefinite {
        Some(t) => t.noun.un(),
        None => origin,
    };
    let tr = travel(rng);
    let (duration, mode) = (&tr.duration, tr.mode);
    let (body, q) = match rng.below(8) {
        0 => (
            format!("Calcule l'isochrone {} {mode} depuis {origin}", tr.de()),
            false,
        ),
        1 => (
            format!("Quelle zone atteint-on en {duration} {mode} depuis {origin}"),
            true,
        ),
        2 => (
            format!("Montre la zone accessible {mode} en {duration} depuis {origin}"),
            false,
        ),
        3 => (
            format!(
                "Jusqu'où peut-on aller {mode} en {duration} en partant {}",
                de_word(&origin)
            ),
            true,
        ),
        // La cible citée : « les parcelles à 10 min à pied de la gare de X ».
        _ => {
            let t = pick_theme(rng, &b, |t| ordinary(t) && t.geom != Geom::Line)?;
            b.add(rng, t.key, &[], &[], &[]);
            let np = format!(
                "les {} {}",
                t.noun.pl,
                travel_clause(rng, &t.noun, &tr, &origin)
            );
            open_find(rng, &t.noun, &np, true)
        }
    };
    let request = finish(rng, &body, q, false);
    let expected = question(match indefinite {
        Some(t) => match rng.below(2) {
            0 => format!(
                "Depuis {} {} faut-il partir : quelles sont ses coordonnées X et Y, avec leur système ?",
                if t.noun.fem { "quelle" } else { "quel" },
                t.noun.sg
            ),
            _ => format!(
                "{} {} faut-il prendre pour départ, et quelles sont ses coordonnées (X, Y et système) ?",
                if t.noun.fem { "Quelle" } else { "Quel" },
                t.noun.sg
            ),
        },
        None => match rng.below(4) {
            0 => format!(
                "Je n'ai pas les coordonnées de {origin} : peux-tu donner le point de départ en X et Y, avec son système de coordonnées ?"
            ),
            1 => "Quelles sont les coordonnées du point de départ (X, Y et système, par exemple Lambert-93) ?".to_string(),
            2 => format!(
                "Depuis quel point exact faut-il partir : quelles sont les coordonnées de {origin} et leur système (Lambert-93 ou WGS84) ?"
            ),
            _ => cassini_core::point::GUARD_QUESTION.to_string(),
        },
    });
    Some(example("question_isochrone", b, rng, request, expected))
}

// -- une seule étape -------------------------------------------------------

/// Met en minuscule la première lettre.
fn uncap(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Une seule opération demandée ; une autre couche est citée pour le contexte, sans étape.
fn etape_unique(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let (op, step_call) = match rng.below(3) {
        0 => {
            let t = pick_theme(rng, &b, vector)?;
            let layer = b.add(rng, t.key, &[], &[], &[]);
            let d = dist(rng);
            (
                tampon_head(rng, &d, &t.noun.des()),
                buffer(&layer.id, &d, false),
            )
        }
        1 => {
            let t = pick_theme(rng, &b, |t| ordinary(t) && !text_fields(t).is_empty())?;
            let def = *rng.pick(&text_fields(t));
            let layer = b.add(rng, t.key, &[def.role], &[], &[]);
            let field = layer.field(def.role)?;
            let val = *rng.pick(&field.values);
            let np = qualified(rng, &t.noun, def, val, Det::Les);
            let head = rng.pick(KEEP).replace("{np}", &np);
            (head, select(&layer.id, &field.name, "eq", val.v))
        }
        _ => {
            let t = pick_theme(rng, &b, vector)?;
            let layer = b.add(rng, t.key, &[], &[], &[]);
            (
                format!("Décris la couche « {} »", layer.name),
                json!({"command": "layer.describe", "params": {"layer_id": layer.id}}),
            )
        }
    };
    let o = pick_theme(rng, &b, ordinary)?;
    b.add(rng, o.key, &[], &[], &[]);
    let other = o.noun.pl;
    let body = match rng.below(6) {
        0 => format!("{op}, je m'occuperai des {other} plus tard"),
        1 => format!("Pour mon étude sur les {other}, {}", uncap(&op)),
        2 => format!("Avant de regarder les {other}, {}", uncap(&op)),
        3 => format!("{op} ; ne touche pas aux {other} pour l'instant"),
        4 => format!("{op}, sans t'occuper des {other}"),
        _ => format!(
            "Pour commencer, {} ; on verra les {other} après",
            uncap(&op)
        ),
    };
    let request = finish(rng, &body, false, false);
    Some(example(
        "etape_unique",
        b,
        rng,
        request,
        calls(vec![step_call]),
    ))
}

// -- redemande -------------------------------------------------------------

thread_local! {
    static REGISTRY: cassini_core::registry::Registry = cassini_core::registry::Registry::standard();
}

/// Paramètres qui désignent une couche (ou une étape précédente).
const LAYER_KEYS: [&str; 3] = ["layer_id", "overlay_layer_id", "points_layer_id"];

/// Un exemple d'une autre famille vu au tour de redemande : son plan corrompu comme un modèle
/// le corromprait, rejeté par `check_plan`, puis les écarts au texte exact du jugement de
/// `cassini-ai` (`judge` : « étape k : message »). La réponse attendue reste le plan juste.
/// Une corruption que `check_plan` accepte (un identifiant de couche inconnu, que le
/// registre ne peut pas voir) n'apprend rien : l'exemple est laissé, pas écrit.
fn redemande(rng: &mut Rng) -> Option<Example> {
    let bases: Vec<&Family> = FAMILIES.iter().filter(|f| f.name != "redemande").collect();
    let total: u64 = bases.iter().map(|f| f.weight).sum();
    let mut roll = rng.below(total as usize) as u64;
    let family = bases.iter().find(|f| {
        if roll < f.weight {
            true
        } else {
            roll -= f.weight;
            false
        }
    })?;
    let base = (family.make)(rng)?;
    let calls = base.expected["calls"].as_array()?.clone();
    let corrupted = corrupt(rng, &calls)?;
    let plan: cassini_core::plan::Plan =
        serde_json::from_value(json!({"kind": "calls", "calls": corrupted})).ok()?;
    let error = REGISTRY.with(|r| r.check_plan(&plan)).err()?;
    let rejected = serde_json::to_string(&plan).ok()?;
    Some(Example {
        family: "redemande",
        retry: Some(Retry {
            rejected,
            errors: vec![format!("étape {} : {}", error.step, error.message)],
        }),
        ..base
    })
}

/// Corrompt un plan juste d'une des trois façons d'un modèle : identifiant de couche inconnu,
/// référence `step:k` hors plage, étape retirée qui casse une référence.
fn corrupt(rng: &mut Rng, calls: &[Value]) -> Option<Vec<Value>> {
    let mut out = calls.to_vec();
    match rng.below(3) {
        0 => {
            let (i, key) = layer_param(rng, &out, |v| !v.starts_with("step:"))?;
            let id = out[i]["params"][key].as_str()?.to_string();
            let cut = id.find('_').map_or(id.len(), |p| p + 1);
            out[i]["params"][key] = json!(format!("{}{}", &id[..cut], rng.hex(4)));
        }
        1 => {
            let (i, key) = layer_param(rng, &out, |_| true)?;
            // `step:0` (on compte à partir de 1), l'étape elle-même ou une suivante : jamais une
            // étape déjà exécutée.
            let k = match rng.below(3) {
                0 => 0,
                n => i + n,
            };
            out[i]["params"][key] = json!(step(k));
        }
        _ => {
            if out.len() < 2 {
                return None;
            }
            let j = rng.below(out.len() - 1);
            out.remove(j);
        }
    }
    Some(out)
}

/// Une étape et un de ses paramètres de couche dont la valeur vérifie `pred`.
fn layer_param(
    rng: &mut Rng,
    calls: &[Value],
    pred: impl Fn(&str) -> bool,
) -> Option<(usize, &'static str)> {
    let mut found = Vec::new();
    for (i, call) in calls.iter().enumerate() {
        for key in LAYER_KEYS {
            if call["params"][key].as_str().is_some_and(&pred) {
                found.push((i, key));
            }
        }
    }
    if found.is_empty() {
        None
    } else {
        Some(*rng.pick(&found))
    }
}

/// Un refus qui dit qu'aucune commande ne reprojette un point : faux depuis
/// `data.transform_point` (11.12). Sert de garde-fou aux tests.
#[cfg(test)]
pub(crate) fn reprojects_no_point(expected: &Value) -> bool {
    expected["kind"] == "refusal"
        && expected["reason"]
            .as_str()
            // Toute tournure qui parle de reprojection et de point (« ne reprojette pas de point »,
            // « aucune reprojection d'un point ») ; le refus de reprojeter une couche n'en parle pas.
            .is_some_and(|r| {
                let r = r.to_lowercase();
                r.contains("reproj") && r.contains("point")
            })
}

/// Un refus qui dit qu'aucune commande ne sélectionne les cellules d'un raster par leur
/// valeur : faux depuis `raster.reclassify` (11.9, famille `pente_seuil` en 11.12, lot 2a).
/// Sert de garde-fou aux tests.
#[cfg(test)]
pub(crate) fn selects_no_cell(expected: &Value) -> bool {
    expected["kind"] == "refusal"
        && expected["reason"].as_str().is_some_and(|r| {
            let r = r.to_lowercase();
            (r.contains("sélectionn") || r.contains("seuil")) && r.contains("cellule")
        })
}

/// Un refus qui cite un format, une orientation ou un carroyage que l'export connaît
/// (« seulement une page A4 paysage », A3, portrait, quadrillage) : faux depuis `page_size`,
/// `orientation` et `grid` (11.11, famille
/// `export_pdf_format` en 11.12, lot 3). Sert de garde-fou aux tests.
#[cfg(test)]
pub(crate) fn refuses_a_known_page(expected: &Value) -> bool {
    expected["kind"] == "refusal"
        && expected["reason"].as_str().is_some_and(|r| {
            let r = r.to_lowercase();
            [
                "a4",
                "a3",
                "a2",
                "a1",
                "a0",
                "portrait",
                "paysage",
                "orientation",
                "carroyage",
                "quadrillage",
                "graticule",
            ]
            .iter()
            .any(|w| r.contains(w))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn thousands_are_separated_by_spaces() {
        assert_eq!(thousands(12500), "12 500");
        assert_eq!(thousands(1000), "1 000");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_234_567), "1 234 567");
    }

    #[test]
    fn distances_write_their_preposition() {
        let km = Dist {
            value: json!(1),
            unit: "km",
            bare: "un kilomètre".into(),
        };
        assert_eq!(km.de(), "d'un kilomètre");
        let m = Dist {
            value: json!(500),
            unit: "m",
            bare: "500 m".into(),
        };
        assert_eq!(m.de(), "de 500 m");
        assert_eq!(m.json(), json!({"value": 500, "unit": "m"}));
    }

    #[test]
    fn a_distance_in_meters_stays_a_buffer_then_an_intersection() {
        // Le contraire d'un trajet : « à moins de 500 m d'une gare » reste un plan, jamais une
        // question de coordonnées, même quand la couche des gares est là.
        let examples = valid(distance, 300);
        for e in &examples {
            assert_eq!(
                commands(e),
                ["analysis.buffer", "analysis.intersect"],
                "{}",
                e.request
            );
        }
        assert!(examples.iter().any(|e| has_theme(e, "gares")));
    }

    /// Exemples valides d'une famille, sur `n` tirages.
    fn valid(make: fn(&mut Rng) -> Option<Example>, n: usize) -> Vec<Example> {
        let registry = cassini_core::registry::Registry::standard();
        let mut rng = Rng::new(7);
        let out: Vec<Example> = (0..n)
            .filter_map(|_| make(&mut rng))
            .inspect(|e| assert_eq!(crate::validate(&registry, e), Ok(()), "{}", e.request))
            .collect();
        assert!(!out.is_empty());
        out
    }

    fn commands(e: &Example) -> Vec<&str> {
        e.expected["calls"]
            .as_array()
            .map(|c| c.iter().map(|s| s["command"].as_str().unwrap()).collect())
            .unwrap_or_default()
    }

    fn has_theme(e: &Example, slug_of: &str) -> bool {
        let t = theme(slug_of);
        e.inventory["layers"].as_array().unwrap().iter().any(|l| {
            let id = l["layer_id"].as_str().unwrap();
            t.slugs.iter().any(|s| id.starts_with(&format!("{s}_")))
        })
    }

    // -- géométrie (11.12, lot 1) ---------------------------------------------

    /// Les exemples d'une famille de géométrie : valides, et leur plan laissé tel quel par le
    /// garde-fou des coordonnées avec leur demande.
    fn geometry_examples(make: fn(&mut Rng) -> Option<Example>) -> Vec<Example> {
        let registry = cassini_core::registry::Registry::standard();
        let examples = valid(make, 600);
        for e in &examples {
            let plan: cassini_core::plan::Plan =
                serde_json::from_value(e.expected.clone()).unwrap();
            let guarded =
                cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
            assert_eq!(guarded, plan, "{}", e.request);
        }
        examples
    }

    /// Le nombre `value` est écrit dans la demande, lu comme le lit le garde-fou.
    fn written(e: &Example, value: &Value) {
        let v = value
            .as_f64()
            .unwrap_or_else(|| panic!("{value} : {}", e.request));
        let numbers = cassini_core::point::request_numbers(&e.request);
        assert!(
            numbers.iter().any(|n| (n - v).abs() < 1e-9),
            "{v} absent : {}",
            e.request
        );
    }

    fn call(e: &Example, k: usize) -> &Value {
        &e.expected["calls"][k]
    }

    /// Un angle du plan porte toujours son unité, et sa valeur est écrite dans la demande.
    fn written_angle(e: &Example, angle: &Value) {
        let unit = angle["unit"]
            .as_str()
            .unwrap_or_else(|| panic!("{}", e.request));
        let r = e.request.as_str();
        match unit {
            "gon" => assert!(r.contains(" gr"), "{r}"),
            _ => assert!(r.contains('°') || r.contains("degrés"), "{r}"),
        }
        written(e, &angle["value"]);
    }

    /// Un résultat demandé en MGRS : `coord.convert` sur le point de l'étape 1, la précision
    /// seulement si elle est écrite.
    fn mgrs_tail(examples: &[Example], command: &str) {
        let (mut with, mut without) = (0, 0);
        for e in examples {
            assert_eq!(commands(e)[0], command, "{}", e.request);
            let asks = e.request.contains("MGRS");
            assert_eq!(commands(e).len(), if asks { 2 } else { 1 }, "{}", e.request);
            if asks {
                with += 1;
                let p = &call(e, 1)["params"];
                assert_eq!(p["point"], "step:1.point", "{}", e.request);
                assert_eq!(p["to"], json!(["mgrs"]), "{}", e.request);
                assert_eq!(
                    p.get("mgrs_precision").is_some(),
                    e.request.contains("chiffres"),
                    "{}",
                    e.request
                );
                if let Some(n) = p.get("mgrs_precision") {
                    written(e, n);
                }
            } else {
                without += 1;
            }
        }
        assert!(with > 0 && without > 0);
    }

    #[test]
    fn a_conversion_between_wgs84_utm_and_mgrs_is_one_coord_convert() {
        let examples = geometry_examples(conversion_coordonnees);
        let mut inputs = std::collections::BTreeSet::new();
        for e in &examples {
            let r = &e.request;
            let last = call(e, commands(e).len() - 1);
            assert_eq!(last["command"], "coord.convert", "{r}");
            let p = &last["params"];
            let to = p["to"].as_array().unwrap();
            assert!((1..=2).contains(&to.len()), "{r}");
            // Un point en Lambert-93 passe d'abord par `data.transform_point` vers le WGS84.
            if commands(e).len() == 2 {
                assert_eq!(commands(e)[0], "data.transform_point", "{r}");
                assert_eq!(call(e, 0)["params"]["point"]["epsg"], 2154, "{r}");
                assert_eq!(call(e, 0)["params"]["target_epsg"], 4326, "{r}");
                assert_eq!(p["point"], "step:1.point", "{r}");
                assert!(!to.contains(&json!("wgs84")), "{r}");
                inputs.insert("lambert");
            } else if let Some(m) = p["mgrs"].as_str() {
                // MGRS valide et canonique, recopié de la demande.
                let parsed = cassini_core::geodesy::Mgrs::parse(m).unwrap();
                assert_eq!(parsed.canonical(), m, "{r}");
                assert!(!to.contains(&json!("mgrs")), "{r}");
                inputs.insert("mgrs");
            } else {
                let epsg = p["point"]["epsg"].as_u64().unwrap();
                let utm = (32601..=32660).contains(&epsg);
                assert!(epsg == 4326 || utm, "{r}");
                assert!(
                    !to.contains(&json!(if utm { "utm" } else { "wgs84" })),
                    "{r}"
                );
                inputs.insert(if utm { "utm" } else { "wgs84" });
            }
            assert_eq!(
                p.get("mgrs_precision").is_some(),
                r.contains("chiffres"),
                "{r}"
            );
            assert_eq!(p.get("utm_zone").is_some(), r.contains("fuseau"), "{r}");
            for key in ["mgrs_precision", "utm_zone"] {
                if let Some(n) = p.get(key) {
                    written(e, n);
                }
            }
        }
        assert_eq!(inputs.len(), 4, "{inputs:?}");
        assert!(examples.iter().any(|e| call(e, 0)["params"]["to"]
            .as_array()
            .is_some_and(|t| t.len() == 2)));
    }

    #[test]
    fn a_point_in_another_system_is_one_transform_point() {
        let examples = geometry_examples(reprojection_point);
        let mut targets = std::collections::BTreeSet::new();
        for e in &examples {
            let r = &e.request;
            assert_eq!(commands(e), ["data.transform_point"], "{r}");
            let p = &call(e, 0)["params"];
            let (from, to) = (
                p["point"]["epsg"].as_u64().unwrap(),
                p["target_epsg"].clone(),
            );
            assert_ne!(json!(from), to, "{r}");
            // Un système hors WGS84, UTM et MGRS est toujours en jeu.
            assert!(from == 2154 || ![json!(4326)].contains(&to), "{r}");
            assert!(
                from == 2154 || !(32601..=32660).contains(&to.as_u64().unwrap()),
                "{r}"
            );
            written(e, &to);
            targets.insert(to.as_u64().unwrap());
        }
        assert_eq!(
            targets.into_iter().collect::<Vec<_>>(),
            [2154, 3857, 4326, 27572]
        );
    }

    #[test]
    fn a_distance_and_azimuth_takes_the_written_unit_and_reprojects_lambert_first() {
        let examples = geometry_examples(distance_azimut);
        let (mut lambert, mut once) = (0, 0);
        for e in &examples {
            let r = &e.request;
            let last = call(e, commands(e).len() - 1);
            assert_eq!(last["command"], "geo.distance_azimuth", "{r}");
            let p = &last["params"];
            let unit = p.get("unit").and_then(Value::as_str);
            assert_eq!(unit == Some("gon"), r.contains("grades"), "{r}");
            assert_eq!(unit == Some("deg"), r.contains("degrés"), "{r}");
            if commands(e).len() == 3 {
                lambert += 1;
                for k in 0..2 {
                    assert_eq!(call(e, k)["command"], "data.transform_point", "{r}");
                    assert_eq!(call(e, k)["params"]["point"]["epsg"], 2154, "{r}");
                    assert_eq!(call(e, k)["params"]["target_epsg"], 4326, "{r}");
                }
                assert_eq!(p["from"], "step:1.point", "{r}");
                assert_eq!(p["to"], "step:2.point", "{r}");
            } else {
                assert_eq!(commands(e).len(), 1, "{r}");
                assert_ne!(p["from"]["epsg"], 2154, "{r}");
            }
            once += usize::from(r.contains("les points"));
        }
        assert!(lambert > 0 && once > 0);
    }

    #[test]
    fn a_destination_point_takes_the_written_azimuth_and_distance() {
        let examples = geometry_examples(point_projete);
        for e in &examples {
            let p = &call(e, 0)["params"];
            written_angle(e, &p["azimuth"]);
            written(e, &p["distance"]["value"]);
            assert!(["m", "km"].contains(&p["distance"]["unit"].as_str().unwrap()));
        }
        assert!(examples
            .iter()
            .any(|e| call(e, 0)["params"]["azimuth"]["unit"] == "gon"));
        mgrs_tail(&examples, "geo.destination_point");
    }

    #[test]
    fn the_geometry_steps_of_the_core_run_without_error() {
        // Les commandes de géométrie du noyau, exécutées sur chaque exemple : un recoupement
        // dont les visées ne se coupent pas en avant, ou un point projeté hors de la grille UTM,
        // apprendrait un plan qui échoue.
        let registry = cassini_core::registry::Registry::standard();
        for make in [
            point_projete as fn(&mut Rng) -> Option<Example>,
            recoupement,
            distance_azimut,
            conversion_coordonnees,
        ] {
            for e in valid(make, 600) {
                let step = call(&e, 0);
                let params = &step["params"];
                if step["command"] == "data.transform_point" {
                    continue;
                }
                let out = registry.call(step["command"].as_str().unwrap(), params.clone());
                assert!(out.is_ok(), "{out:?} : {}", e.request);
            }
        }
    }

    #[test]
    fn a_bearing_intersection_takes_both_written_azimuths() {
        let examples = geometry_examples(recoupement);
        for e in &examples {
            let p = &call(e, 0)["params"];
            written_angle(e, &p["first_azimuth"]);
            written_angle(e, &p["second_azimuth"]);
            assert_eq!(p["first_azimuth"]["unit"], p["second_azimuth"]["unit"]);
        }
        mgrs_tail(&examples, "geo.bearing_intersection");
    }

    #[test]
    fn a_terrain_point_in_another_system_is_reprojected_into_the_dem_system_first() {
        // 11.12 remplace le refus de 5.41 : WGS84, UTM d'une autre zone, Lambert-93 sur un MNT
        // en UTM, ou MGRS, puis la commande de terrain sur `step:k.point`.
        let examples = geometry_examples(terrain_autre_systeme);
        let mut seen = std::collections::BTreeSet::new();
        for e in &examples {
            let r = &e.request;
            let names = commands(e);
            let last = call(e, names.len() - 1);
            assert!(
                ["terrain.viewshed", "terrain.line_of_sight"].contains(&names[names.len() - 1]),
                "{r}"
            );
            let dem = cited_dem_of(e, last);
            let epsg = dem["epsg"].as_u64().unwrap_or_else(|| panic!("{r}"));
            for (k, step) in e.expected["calls"].as_array().unwrap().iter().enumerate() {
                let p = &step["params"];
                match step["command"].as_str().unwrap() {
                    "data.transform_point" => {
                        assert_eq!(p["target_epsg"], epsg, "{r}");
                        match p["point"]["epsg"].as_u64() {
                            Some(from) => {
                                assert_ne!(from, epsg, "{r}");
                                seen.insert((epsg, from));
                            }
                            None => {
                                // Une référence MGRS : convertie en WGS84 à l'étape d'avant.
                                assert_eq!(p["point"], format!("step:{k}.wgs84"), "{r}");
                                assert_eq!(call(e, k - 1)["command"], "coord.convert", "{r}");
                                assert_eq!(call(e, k - 1)["params"]["to"], json!(["wgs84"]));
                                seen.insert((epsg, 0));
                            }
                        }
                    }
                    "coord.convert" => assert!(p["mgrs"].is_string(), "{r}"),
                    _ => {
                        for key in ["observer", "target"] {
                            if let Some(point) = p.get(key) {
                                assert!(point.as_str().is_some_and(|s| s.starts_with("step:")));
                            }
                        }
                    }
                }
            }
        }
        for pair in [
            (2154, 4326),
            (2154, 32631),
            (2154, 32632),
            (2154, 0),
            (32631, 32632),
            (32632, 32631),
            (32631, 2154),
            (32632, 2154),
            (32632, 4326),
            (32631, 0),
        ] {
            assert!(seen.contains(&pair), "{pair:?} : {seen:?}");
        }
    }

    /// Le MNT cité par l'étape de terrain `last`.
    fn cited_dem_of<'a>(e: &'a Example, last: &Value) -> &'a Value {
        let dem = last["params"]["dem"].as_str().unwrap();
        e.inventory["layers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["kind"] == "raster" && l["source"] == dem)
            .unwrap_or_else(|| panic!("MNT « {dem} » absent : {}", e.request))
    }

    #[test]
    fn a_terrain_computation_then_a_statistic_per_zone_is_refused() {
        // 5.41 : « la pente puis sa moyenne par commune » est un refus entier.
        let examples = valid(refus_hors_registre, 600);
        let stats: Vec<&Example> = examples
            .iter()
            .filter(|e| e.expected["reason"] == NO_RASTER_STATS)
            .collect();
        assert!(stats.iter().any(|e| e.request.contains("pente")));
        assert!(stats.iter().any(|e| e.request.contains("voit")));
        for e in stats {
            assert_eq!(e.expected["kind"], "refusal", "{}", e.request);
        }
    }

    #[test]
    fn a_radius_written_before_the_point_is_a_viewshed_radius() {
        // 5.41 : « dans un rayon de 800 m autour du point … » rend `terrain.viewshed` à `radius`.
        let around: Vec<Example> = valid(champ_de_vision, 400)
            .into_iter()
            .filter(|e| e.request.contains(" autour du point"))
            .collect();
        assert!(!around.is_empty());
        for e in &around {
            let p = &e.expected["calls"][0]["params"];
            assert_eq!(e.expected["calls"][0]["command"], "terrain.viewshed");
            assert!(p["radius"].is_object(), "{}", e.request);
            assert!(
                e.request.to_lowercase().contains("dans un rayon de"),
                "{}",
                e.request
            );
            // Un seul rayon, celui écrit avant le point : aucune seconde clause après lui.
            let text = e.request.to_lowercase();
            let (before, after) = text.split_once(" autour d").unwrap();
            let forms: &[&str] = match (
                p["radius"]["value"].to_string().as_str(),
                p["radius"]["unit"].as_str().unwrap(),
            ) {
                ("300", "m") => &["300 m", "300 mètres"],
                ("500", "m") => &["500 m", "500 mètres"],
                ("800", "m") => &["800 m"],
                ("1", "km") => &["1 km", "un kilomètre"],
                ("1.5", "km") => &["1,5 km"],
                ("2", "km") => &["2 km", "deux kilomètres"],
                ("3", "km") => &["3 km"],
                other => panic!("rayon inattendu {other:?} : {}", e.request),
            };
            assert!(
                forms
                    .iter()
                    .any(|f| before.ends_with(&format!("dans un rayon de {f}"))),
                "{}",
                e.request
            );
            for marker in ["rayon", " sur ", "jusqu'à"] {
                assert!(!after.contains(marker), "{}", e.request);
            }
        }
    }

    #[test]
    fn out_of_registry_requests_are_refused_never_planned() {
        let registry = cassini_core::registry::Registry::standard();
        for e in valid(refus_hors_registre, 300) {
            assert_eq!(e.expected["kind"], "refusal", "{}", e.request);
            let reason = e.expected["reason"].as_str().unwrap();
            // Aucun refus ne nomme une commande du registre, ni le terrain qu'il calcule (4.6).
            for spec in registry.specs() {
                assert!(!reason.contains(spec.id.as_str()), "{reason}");
            }
            for word in ["champ de vision", "ligne de vue", "pente", "visib"] {
                assert!(!reason.contains(word), "{reason}");
            }
            assert!(!selects_no_cell(&e.expected), "{}", e.request);
            if reason == NO_RASTER_STATS {
                assert!(has_theme(&e, "mnt"), "MNT présent : {}", e.request);
            }
        }
    }

    #[test]
    fn belonging_to_a_commune_selects_it_or_asks() {
        let examples = valid(appartenance_commune, 300);
        let (mut planned, mut asked) = (0, 0);
        for e in &examples {
            if e.expected["kind"] == "calls" {
                planned += 1;
                let calls = e.expected["calls"].as_array().unwrap();
                assert_eq!(calls[0]["command"], "analysis.select_by_attribute");
                let commune = calls[0]["params"]["value"].as_str().unwrap();
                assert!(COMMUNES.contains(&commune) && e.request.contains(commune));
                assert_eq!(
                    calls[1],
                    intersect(calls[1]["params"]["layer_id"].as_str().unwrap(), &step(1))
                );
                assert!(has_theme(e, "communes"));
            } else {
                asked += 1;
                assert_eq!(e.expected["kind"], "question", "{}", e.request);
            }
        }
        assert!(planned > 0 && asked > 0);
        // Sans couche de communes, toujours une question.
        assert!(examples
            .iter()
            .filter(|e| !has_theme(e, "communes"))
            .all(|e| e.expected["kind"] == "question"));
    }

    #[test]
    fn an_isochrone_without_coordinates_asks_for_the_point() {
        for e in valid(question_isochrone, 200) {
            assert_eq!(e.expected["kind"], "question", "{}", e.request);
            assert!(e.expected["question"]
                .as_str()
                .unwrap()
                .contains("coordonnées"));
            assert!(!e.request.contains("Lambert") && !e.request.contains("EPSG"));
        }
    }

    /// Thèmes des lieux qui sont aussi des couches de points (`place_with_layer`).
    const PLACE_THEMES: [&str; 5] = ["gares", "ecoles", "hopitaux", "arrets", "pharmacies"];

    #[test]
    fn a_travel_time_from_a_place_layer_asks_for_the_point() {
        // Phrase E (constat de 5.15) : la couche des gares est là, la demande ne donne pas de
        // coordonnées ; la réponse est une question, jamais une origine inventée.
        let examples = valid(question_isochrone, 600);
        let with_layer = examples
            .iter()
            .filter(|e| PLACE_THEMES.iter().any(|t| has_theme(e, t)))
            .count();
        // Taux attendu 25/36 : une marge sous lui, que le tirage ne la franchisse pas.
        assert!(with_layer * 5 >= examples.len() * 3, "{with_layer}");
        let mut targeted = 0;
        for e in &examples {
            assert_eq!(e.expected["kind"], "question", "{}", e.request);
            let r = e.request.to_lowercase();
            if (r.contains(" de la gare ") || r.contains(" d'une gare"))
                && (r.contains(" min") || r.contains("heure") || r.contains("seconde"))
                && has_theme(e, "gares")
            {
                targeted += 1;
            }
        }
        assert!(targeted > 0);
        assert!(examples
            .iter()
            .any(|e| e.expected["question"] == cassini_core::point::GUARD_QUESTION));
        // « D'une gare » : on demande laquelle.
        assert!(examples.iter().any(|e| e.request.contains(" d'une gare")
            && e.expected["question"]
                .as_str()
                .unwrap()
                .contains("quelle gare")
            || e.request.contains(" d'une gare")
                && e.expected["question"]
                    .as_str()
                    .unwrap()
                    .starts_with("Quelle gare")));
    }

    #[test]
    fn a_travel_time_from_a_given_point_is_an_isochrone_crossed_with_the_target() {
        // Le contraire de la phrase E : même tournure, point donné ; isochrone, puis la cible
        // croisée avec l'étape 1. Le garde-fou de 5.16 laisse le plan tel quel.
        let registry = cassini_core::registry::Registry::standard();
        let examples = valid(isochrone_vers_cible, 400);
        for e in &examples {
            let c = commands(e);
            assert_eq!(
                c[..2],
                ["analysis.isochrone", "analysis.intersect"],
                "{}",
                e.request
            );
            assert!(c.len() == 2 || c[2] == "layout.export_pdf");
            let calls = e.expected["calls"].as_array().unwrap();
            assert_eq!(calls[1]["params"]["overlay_layer_id"], step(1).as_str());
            assert!(!calls[1]["params"]["layer_id"]
                .as_str()
                .unwrap()
                .starts_with("step:"));
            let plan: cassini_core::plan::Plan =
                serde_json::from_value(e.expected.clone()).unwrap();
            let guarded =
                cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
            assert_eq!(guarded, plan, "{}", e.request);
        }
        assert!(examples.iter().any(|e| has_theme(e, "gares")));
        assert!(examples
            .iter()
            .any(|e| e.request.contains(" à pied du point ")));
        // Un lieu nommé avec ses coordonnées : c'est encore un plan.
        assert!(examples.iter().any(|e| {
            (e.request.contains("la gare ") || e.request.contains("l'école Jean Moulin"))
                && !e.request.contains(" point ")
        }));
    }

    #[test]
    fn a_point_given_in_utm_is_an_isochrone_left_unchanged_by_the_guard() {
        // 5.37 : la zone UTM écrite avec son hémisphère donne le code du plan (32631, 32632).
        let registry = cassini_core::registry::Registry::standard();
        let examples = valid(isochrone_utm, 400);
        for e in &examples {
            assert_eq!(e.family, "isochrone_utm");
            assert_eq!(commands(e), ["analysis.isochrone"], "{}", e.request);
            let epsg = &e.expected["calls"][0]["params"]["origin"]["epsg"];
            assert!(epsg == 32631 || epsg == 32632, "{epsg} {}", e.request);
            let plan: cassini_core::plan::Plan =
                serde_json::from_value(e.expected.clone()).unwrap();
            let guarded =
                cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
            assert_eq!(guarded, plan, "{}", e.request);
        }
        for form in ["UTM 31N", "UTM 32 N", "UTM zone 31 Nord", "UTM32N"] {
            assert!(examples.iter().any(|e| e.request.contains(form)), "{form}");
        }
    }

    #[test]
    fn the_travel_mode_and_duration_are_mapped_as_written() {
        let mut examples = valid(isochrone, 400);
        examples.extend(valid(isochrone_vers_cible, 400));
        for e in &examples {
            let calls = e.expected["calls"].as_array().unwrap();
            let p = &calls[0]["params"];
            let r = &e.request;
            let walk = ["à pied", "en marchant", "à la marche"]
                .iter()
                .any(|m| r.contains(m));
            assert_eq!(p["mode"] == "walk", walk, "{r}");
            assert_eq!(p["mode"] == "car", !walk, "{r}");
            let chars: Vec<char> = r.chars().collect();
            let seconds = r.contains("seconde")
                || (0..chars.len().saturating_sub(2)).any(|i| {
                    chars[i].is_ascii_digit()
                        && chars[i + 1] == ' '
                        && chars[i + 2] == 's'
                        && !chars
                            .get(i + 3)
                            .is_some_and(|c| c.is_alphabetic() || *c == '\'')
                });
            assert_eq!(p["duration"]["unit"] == "s", seconds, "{r}");
            if r.contains("un quart d'heure") {
                assert_eq!(p["duration"], json!({"value": 15, "unit": "min"}), "{r}");
            }
            if r.contains("une demi-heure") {
                assert_eq!(p["duration"], json!({"value": 30, "unit": "min"}), "{r}");
            }
        }
    }

    #[test]
    fn a_travel_time_from_several_places_is_refused() {
        // Décision a : « de chaque gare », « de toutes les gares », « des gares » → refus.
        let examples: Vec<Example> = valid(refus_hors_registre, 1500)
            .into_iter()
            .filter(|e| e.expected["reason"] == NO_MULTI_ORIGIN)
            .collect();
        assert!(!examples.is_empty());
        for e in &examples {
            let r = e.request.to_lowercase();
            // L'origine, au pluriel, juste après « depuis » ou le mode.
            let modes = [
                "à pied",
                "en marchant",
                "à la marche",
                "en voiture",
                "en auto",
                "en véhicule",
            ];
            let plural = [
                "depuis les ",
                "depuis chaque ",
                "depuis toutes les ",
                "depuis tous les ",
            ]
            .iter()
            .any(|p| r.contains(p))
                || modes.iter().any(|m| {
                    ["des ", "de chaque ", "de toutes les ", "de tous les "]
                        .iter()
                        .any(|p| r.contains(&format!("{m} {p}")))
                });
            assert!(plural, "{}", e.request);
            assert!(
                !r.contains("depuis une ") && !r.contains("depuis un "),
                "{}",
                e.request
            );
        }
    }

    #[test]
    fn several_conditions_vary_operators_and_lose_no_step() {
        // Constats H17 à H22 : étapes perdues, mauvais opérateur, mauvaise commande.
        let examples = valid(conditions_multiples, 3000);
        let mut ops = std::collections::BTreeSet::new();
        let (mut five, mut whole_layer) = (0, 0);
        for e in &examples {
            let calls = e.expected["calls"].as_array().unwrap();
            let chain: Vec<&Value> = calls
                .iter()
                .filter(|c| c["command"] != "layout.export_pdf")
                .collect();
            for c in &chain {
                if c["command"] == "analysis.select_by_attribute" {
                    ops.insert(c["params"]["operator"].as_str().unwrap().to_string());
                }
                if c["command"] == "analysis.intersect"
                    && !c["params"]["overlay_layer_id"]
                        .as_str()
                        .unwrap()
                        .starts_with("step:")
                {
                    whole_layer += 1;
                }
            }
            // Aucune étape perdue : chaque étape sauf la dernière sert à une étape suivante.
            for k in 1..chain.len() {
                let r = step(k);
                assert!(
                    chain[k..]
                        .iter()
                        .any(|c| LAYER_KEYS.iter().any(|key| c["params"][*key] == r.as_str())),
                    "étape {k} perdue : {}",
                    e.request
                );
            }
            assert_eq!(chain.last().unwrap()["command"], "analysis.intersect");
            let names: Vec<&str> = chain
                .iter()
                .map(|c| c["command"].as_str().unwrap())
                .collect();
            if names
                == [
                    "analysis.select_by_attribute",
                    "analysis.buffer",
                    "analysis.buffer",
                    "analysis.intersect",
                    "analysis.intersect",
                ]
            {
                five += 1;
                assert_eq!(chain[3]["params"]["layer_id"], step(1).as_str());
            }
        }
        for op in [
            "eq",
            "gt",
            "lt",
            "gte",
            "lte",
            "ne",
            "contains",
            "begins_with",
        ] {
            assert!(ops.contains(op), "{op} : {ops:?}");
        }
        assert!(five > 0 && whole_layer > 0, "{five} {whole_layer}");
    }

    #[test]
    fn a_single_operation_with_another_layer_in_context_is_one_step() {
        for e in valid(etape_unique, 200) {
            assert_eq!(commands(&e).len(), 1, "{}", e.request);
            assert!(e.inventory["layers"].as_array().unwrap().len() >= 2);
        }
    }

    #[test]
    fn three_conditions_keep_every_step_and_never_pass_six() {
        let examples = valid(conditions_multiples, 600);
        let mut lengths: Vec<usize> = examples.iter().map(|e| commands(e).len()).collect();
        lengths.sort_unstable();
        lengths.dedup();
        assert!(lengths.contains(&5) && lengths.contains(&6), "{lengths:?}");
        assert!(
            lengths.iter().all(|n| (3..=MAX_DRAFT_STEPS).contains(n)),
            "{lengths:?}"
        );
        assert!(examples
            .iter()
            .any(|e| commands(e).contains(&"layout.export_pdf")));
        // Cible filtrée, zone, distance : deux sélections, un tampon, deux intersections.
        assert!(examples.iter().any(|e| {
            let c = commands(e);
            c.iter()
                .filter(|s| **s == "analysis.select_by_attribute")
                .count()
                == 2
                && c.iter().filter(|s| **s == "analysis.intersect").count() == 2
                && c.contains(&"analysis.buffer")
        }));
    }

    #[test]
    fn a_retry_example_shows_a_rejected_output_and_answers_the_right_plan() {
        let registry = cassini_core::registry::Registry::standard();
        for e in valid(redemande, 300) {
            let retry = e.retry.as_ref().expect("redemande");
            let rejected: cassini_core::plan::Plan = serde_json::from_str(&retry.rejected).unwrap();
            let err = registry.check_plan(&rejected).unwrap_err();
            assert_eq!(
                retry.errors,
                [format!("étape {} : {}", err.step, err.message)]
            );
            assert_eq!(e.expected["kind"], "calls");
        }
    }

    #[test]
    fn a_corruption_that_check_plan_accepts_is_left_out() {
        // Un identifiant de couche inconnu : `check_plan` ne voit pas l'inventaire.
        let calls = vec![buffer(
            "gares_1",
            &Dist {
                value: json!(300),
                unit: "m",
                bare: "300 m".into(),
            },
            false,
        )];
        let registry = cassini_core::registry::Registry::standard();
        let mut rng = Rng::new(1);
        loop {
            let out = corrupt(&mut rng, &calls);
            let Some(out) = out else { continue };
            if out[0]["params"]["layer_id"] != "step:1.layer_id"
                && out[0]["params"]["layer_id"] != "step:2.layer_id"
            {
                let plan: cassini_core::plan::Plan =
                    serde_json::from_value(json!({"kind": "calls", "calls": out})).unwrap();
                assert_eq!(registry.check_plan(&plan), Ok(()));
                break;
            }
        }
    }

    #[test]
    fn what_is_refused_out_of_registry_is_still_missing_from_the_registry() {
        // Le jour où une de ces commandes arrive, ce test échoue : le refus devient faux.
        let registry = cassini_core::registry::Registry::standard();
        for spec in registry.specs() {
            let id = spec.id.as_str();
            // Ni calculatrice raster. Le seuil sur la pente n'est plus un refus depuis 11.12,
            // lot 2a : `terrain.slope` puis `raster.reclassify` (`pente_seuil`).
            for word in ["threshold", "raster_calc"] {
                assert!(!id.contains(word), "{id}");
            }
            for word in ["zoom", "frame", "extent", "pan", "center", "centre", "cadr"] {
                assert!(!id.contains(word), "{id}");
            }
        }
        // Point reprojeté (11.12) : `data.transform_point` est au registre, donc aucun refus
        // ne dit plus qu'aucune commande ne reprojette un point. Ce volet échoue si le refus
        // de 5.41 revient dans une famille de refus.
        assert!(registry.spec("data.transform_point").is_some());
        for make in [
            refus as fn(&mut Rng) -> Option<Example>,
            refus_hors_registre,
        ] {
            let mut rng = Rng::new(3);
            for _ in 0..2000 {
                let Some(e) = make(&mut rng) else {
                    continue;
                };
                assert!(!reprojects_no_point(&e.expected), "{}", e.request);
                assert!(!selects_no_cell(&e.expected), "{}", e.request);
                // Format, orientation et carroyage (11.11) : plus aucun refus « A4 paysage »
                // ni d'un format ou d'une orientation du catalogue (11.12, lot 3).
                assert!(!refuses_a_known_page(&e.expected), "{}", e.request);
            }
        }
        assert!(registry.spec("layout.export_pdf").is_some_and(|s| {
            let params = &s.params_schema["properties"];
            ["page_size", "orientation", "grid"]
                .iter()
                .all(|p| params.get(p).is_some())
        }));
        assert!(refuses_a_known_page(&json!({"kind": "refusal", "reason":
            "L'export PDF du catalogue produit seulement une page A4 paysage."})));
        assert!(!refuses_a_known_page(&refusal(NO_PAGE_SIZE)));
        assert!(!refuses_a_known_page(&refusal(NO_PRINTING)));
        assert!(reprojects_no_point(&json!({"kind": "refusal", "reason":
            "Les commandes de terrain du catalogue prennent un point écrit dans le système du MNT : aucune ne reprojette un point."})));
    }

    #[test]
    fn a_page_out_of_the_series_and_printing_stay_refusals() {
        // 11.12, lot 3 (décision 2) : A5, lettre US, impression et exemplaires restent des
        // refus, chacun avec sa raison.
        let mut rng = Rng::new(5);
        let (mut pages, mut prints) = (0, 0);
        for _ in 0..2000 {
            let Some(e) = refus_hors_registre(&mut rng) else {
                continue;
            };
            let r = e.request.to_lowercase();
            if r.contains("a5") || r.contains("lettre us") {
                pages += 1;
                assert_eq!(e.expected, refusal(NO_PAGE_SIZE), "{}", e.request);
            } else if r.contains("exemplaires") || r.contains("imprimante") {
                prints += 1;
                assert_eq!(e.expected, refusal(NO_PRINTING), "{}", e.request);
            }
        }
        assert!(pages > 0 && prints > 0);
    }

    #[test]
    fn an_export_at_a_format_carries_only_the_written_parameters() {
        // 11.12, lot 3 : clés attendues seules, valeurs du catalogue, chaque valeur retrouvée
        // écrite dans la demande, intervalle et unité recopiés, un seul export par plan.
        let registry = cassini_core::registry::Registry::standard();
        let examples = valid(export_pdf_format, 800);
        let (mut questions, mut searches, mut alone) = (0, 0, 0);
        let mut seen = BTreeSet::new();
        for e in &examples {
            assert_eq!(e.family, "export_pdf_format");
            let r = e.request.to_lowercase();
            assert!(
                !r.contains("mgrs") && !r.contains("exemplaire"),
                "{}",
                e.request
            );
            if e.expected["kind"] == "question" {
                questions += 1;
                assert_eq!(e.expected["question"], GRID_SYSTEM_QUESTION);
                // Un carroyage écrit sans son système : ni Lambert, ni zone UTM, ni degrés.
                let named = ["lambert", "utm 3", "utm zone", "°", "degré", "grade", "gr "]
                    .iter()
                    .any(|w| r.contains(w));
                assert!(!named, "{}", e.request);
                assert!(
                    ["carroyage", "quadrillage", "grille"]
                        .iter()
                        .any(|w| r.contains(w)),
                    "{}",
                    e.request
                );
                continue;
            }
            let calls = e.expected["calls"].as_array().unwrap();
            let exports: Vec<&Value> = calls
                .iter()
                .filter(|c| c["command"] == "layout.export_pdf")
                .collect();
            assert_eq!(exports.len(), 1, "{}", e.request);
            assert_eq!(
                calls.last().unwrap()["command"],
                "layout.export_pdf",
                "{}",
                e.request
            );
            if calls.len() > 1 {
                searches += 1;
            } else {
                alone += 1;
            }
            let params = exports[0]["params"].as_object().unwrap();
            assert!(params["title"].as_str().is_some_and(|t| !t.is_empty()));
            let keys: BTreeSet<&str> = params.keys().map(String::as_str).collect();
            assert!(keys.len() >= 2, "{}", e.request);
            for k in &keys {
                assert!(
                    ["title", "page_size", "orientation", "grid"].contains(k),
                    "{k} : {}",
                    e.request
                );
            }
            if let Some(size) = params.get("page_size") {
                let size = size.as_str().unwrap();
                let (_, written) = PAGE_SIZES.iter().find(|(v, _)| *v == size).unwrap();
                assert!(e.request.contains(written), "{size} : {}", e.request);
                seen.insert(size.to_string());
            } else {
                for (_, written) in PAGE_SIZES {
                    assert!(!e.request.contains(written), "{}", e.request);
                }
            }
            match params.get("orientation").and_then(Value::as_str) {
                Some("portrait") => {
                    assert!(r.contains("portrait") || r.contains("à la française"));
                    seen.insert("portrait".into());
                }
                Some("landscape") => {
                    assert!(r.contains("paysage") || r.contains("à l'italienne"));
                    seen.insert("landscape".into());
                }
                Some(o) => panic!("orientation {o}"),
                None => {
                    for w in ["portrait", "paysage", "française", "italienne"] {
                        assert!(!r.contains(w), "{}", e.request);
                    }
                }
            }
            match params.get("grid") {
                Some(grid) => {
                    let interval = &grid["interval"];
                    let unit = interval["unit"].as_str().unwrap();
                    let value = interval["value"].as_f64().unwrap();
                    assert!(value > 0.0);
                    let words: &[&str] = match grid["kind"].as_str().unwrap() {
                        "degrees" => {
                            assert_eq!(grid.as_object().unwrap().len(), 2);
                            seen.insert("degrees".into());
                            match unit {
                                "deg" => &["°", "degré"],
                                "gon" => &[" gr", "grade"],
                                u => panic!("unité {u}"),
                            }
                        }
                        "projected" => {
                            seen.insert("projected".into());
                            let epsg = grid["epsg"].as_u64().unwrap();
                            if epsg == 2154 {
                                assert!(r.contains("lambert"), "{}", e.request);
                            } else {
                                assert!((32630..=32632).contains(&epsg), "{epsg}");
                                assert!(
                                    {
                                        let z = epsg - 32600;
                                        r.contains(&format!("utm {z}"))
                                            || r.contains(&format!("utm zone {z}"))
                                    },
                                    "{}",
                                    e.request
                                );
                            }
                            match unit {
                                "m" => &[" m", "mètre"],
                                "km" => &["km", "kilomètre"],
                                u => panic!("unité {u}"),
                            }
                        }
                        k => panic!("grille {k}"),
                    };
                    let number = if grid["kind"] == "degrees" {
                        decimal_fr(value)
                    } else {
                        thousands(value as u64)
                    };
                    assert!(
                        words
                            .iter()
                            .any(|w| e.request.contains(&format!("{number}{w}"))
                                || e.request.contains(&format!("{number} {}", w.trim()))),
                        "{number} {unit} : {}",
                        e.request
                    );
                }
                None => {
                    for w in ["carroyage", "quadrillage", "graticule", "grille"] {
                        assert!(!r.contains(w), "{}", e.request);
                    }
                }
            }
            let plan: cassini_core::plan::Plan =
                serde_json::from_value(e.expected.clone()).unwrap();
            let guarded =
                cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
            assert_eq!(guarded, plan, "{}", e.request);
        }
        assert!(questions > 0 && searches > 0 && alone > 0);
        for v in [
            "a4",
            "a3",
            "a2",
            "a1",
            "a0",
            "portrait",
            "landscape",
            "degrees",
            "projected",
        ] {
            assert!(seen.contains(v), "{v} absent");
        }
    }

    #[test]
    fn page_words_and_grid_steps_are_written_with_their_unit() {
        assert_eq!(decimal_fr(0.01), "0,01");
        assert_eq!(decimal_fr(5.0), "5");
        assert_eq!(number_json(1.0), json!(1));
        assert_eq!(number_json(0.5), json!(0.5));
        let mut rng = Rng::new(5);
        for _ in 0..50 {
            let t = distance_step_text(&mut rng, 1000, "m");
            assert!(t == "1 000 m" || t == "1 000 mètres", "{t}");
            let t = distance_step_text(&mut rng, 1, "km");
            assert!(t == "1 km" || t == "1 kilomètre", "{t}");
            let t = page_words(&mut rng, Some("A3"), None);
            assert!(t.starts_with(' ') && t.contains("A3"), "{t}");
            for p in [true, false] {
                let t = page_words(&mut rng, None, Some(p));
                let words: &[&str] = if p {
                    &["portrait", "à la française"]
                } else {
                    &["paysage", "à l'italienne"]
                };
                assert!(words.iter().any(|w| t.contains(w)), "{t}");
                let t = page_words(&mut rng, Some("A0"), Some(p));
                assert!(t.contains("A0"), "{t}");
                assert!(words.iter().any(|w| t.contains(w)), "{t}");
            }
            assert_eq!(page_words(&mut rng, None, None), "");
        }
    }

    #[test]
    fn an_export_at_a_scale_carries_the_written_number_and_passes_the_guard() {
        // 5.38 : `scale` est l'entier écrit dans la demande, quelle que soit sa forme ; ce
        // n'est pas un `Point`, le garde-fou laisse le plan.
        let registry = cassini_core::registry::Registry::standard();
        let examples = valid(export_pdf_scale, 400);
        for e in &examples {
            assert_eq!(e.family, "export_pdf_scale");
            let calls = e.expected["calls"].as_array().unwrap();
            let last = calls.last().unwrap();
            assert_eq!(last["command"], "layout.export_pdf", "{}", e.request);
            let scale = last["params"]["scale"].as_u64().expect("scale entier");
            assert!(SCALES.contains(&scale), "{scale}");
            let written = [
                format!("1:{}", thousands(scale)),
                format!("1/{scale}"),
                format!("{}e", thousands(scale)),
                format!("1/{}", thousands(scale)),
                format!("1:{scale}"),
                format!("1 : {}", thousands(scale)),
                format!("{}ème", thousands(scale)),
                format!("1:{}k", scale / 1000),
            ];
            assert!(
                written.iter().any(|w| e.request.contains(w.as_str())),
                "{scale} {}",
                e.request
            );
            if e.request.contains(&format!("1:{}k", scale / 1000)) {
                assert!(scale.is_multiple_of(1000), "{}", e.request);
            }
            let plan: cassini_core::plan::Plan =
                serde_json::from_value(e.expected.clone()).unwrap();
            let guarded =
                cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
            assert_eq!(guarded, plan, "{}", e.request);
        }
        assert!(examples.iter().any(|e| {
            let scale = e.expected["calls"].as_array().unwrap().last().unwrap()["params"]["scale"]
                .as_u64()
                .unwrap();
            e.request.contains(&format!("1:{}k", scale / 1000))
        }));
        for form in [
            "au 1:",
            "au 1/",
            "000e",
            "à l'échelle 1/",
            "au 1 : ",
            "000ème",
        ] {
            assert!(examples.iter().any(|e| e.request.contains(form)), "{form}");
        }
        // « au 1:25000 » : le nombre collé, sans espace de milliers.
        assert!(examples.iter().any(|e| {
            let scale = e.expected["calls"].as_array().unwrap().last().unwrap()["params"]["scale"]
                .as_u64()
                .unwrap();
            scale >= 10000 && e.request.contains(&format!("au 1:{scale}"))
        }));
        assert!(examples.iter().any(|e| commands(e).len() > 1));
        assert!(examples.iter().any(|e| commands(e).len() == 1));
    }

    /// Le MNT d'un plan de terrain : la couche raster dont `dem` est la `source`.
    fn cited_dem(e: &Example) -> &Value {
        cited_raster(e, "dem")
    }

    /// La couche raster dont le paramètre `key` de la première étape est la `source`.
    fn cited_raster<'a>(e: &'a Example, key: &str) -> &'a Value {
        let path = e.expected["calls"][0]["params"][key].as_str().unwrap();
        e.inventory["layers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["kind"] == "raster" && l["source"] == path)
            .unwrap_or_else(|| panic!("raster « {path} » absent : {}", e.request))
    }

    /// Les exemples d'une famille de terrain : une étape, la commande attendue, le `dem` source
    /// d'un raster de l'inventaire, aucun paramètre de plus que ceux qu'elle écrit, et le
    /// garde-fou qui laisse le plan.
    fn terrain_examples(
        make: fn(&mut Rng) -> Option<Example>,
        command: &str,
        allowed: &[&str],
    ) -> Vec<Example> {
        let registry = cassini_core::registry::Registry::standard();
        let examples = valid(make, 400);
        for e in &examples {
            assert_eq!(commands(e), [command], "{}", e.request);
            cited_dem(e);
            let params = e.expected["calls"][0]["params"].as_object().unwrap();
            for key in params.keys() {
                assert!(allowed.contains(&key.as_str()), "{key} : {}", e.request);
            }
            let plan: cassini_core::plan::Plan =
                serde_json::from_value(e.expected.clone()).unwrap();
            let guarded =
                cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
            assert_eq!(guarded, plan, "{}", e.request);
        }
        examples
    }

    /// Les points d'un plan de terrain sont dans le système du MNT, quand l'inventaire le
    /// donne ; les deux zones UTM et le Lambert-93 sont vus.
    fn points_in_the_dem_system(examples: &[Example], keys: &[&str]) {
        let mut systems = std::collections::BTreeSet::new();
        for e in examples {
            let dem = cited_dem(e);
            for key in keys {
                let epsg = &e.expected["calls"][0]["params"][key]["epsg"];
                if let Some(d) = dem.get("epsg") {
                    assert_eq!(d, epsg, "{}", e.request);
                }
                systems.insert(epsg.as_u64().unwrap());
            }
        }
        assert_eq!(
            systems.into_iter().collect::<Vec<_>>(),
            [2154, 32631, 32632]
        );
        for form in [
            "UTM 31N",
            "UTM 32 N",
            "UTM zone 31 Nord",
            "UTM32N",
            "Lambert",
        ] {
            assert!(examples.iter().any(|e| e.request.contains(form)), "{form}");
        }
    }

    /// Un paramètre facultatif n'est au plan que si la demande l'écrit.
    fn written_only(examples: &[Example], key: &str, markers: &[&str]) {
        let (mut with, mut without) = (0, 0);
        for e in examples {
            let written = markers.iter().any(|m| e.request.contains(m));
            let planned = e.expected["calls"][0]["params"].get(key).is_some();
            assert_eq!(written, planned, "{key} : {}", e.request);
            if planned {
                with += 1;
            } else {
                without += 1;
            }
        }
        assert!(with > 0 && without > 0, "{key}");
    }

    const EYE_MARKERS: &[&str] = &["œil", "observateur", "de hauteur"];

    #[test]
    fn a_slope_carries_the_dem_source_and_the_written_unit() {
        let examples = terrain_examples(pente, "terrain.slope", &["dem", "unit"]);
        for e in &examples {
            let unit = &e.expected["calls"][0]["params"]["unit"];
            let r = &e.request;
            let degrees = r.contains("degrés") || r.contains('°');
            assert_eq!(unit == "degrees", degrees, "{r}");
            assert_eq!(
                unit == "percent",
                r.contains("pourcentage") || r.contains('%') || r.contains("pour cent"),
                "{r}"
            );
        }
        assert!(examples.iter().any(|e| e.request.contains("« ")));
    }

    #[test]
    fn a_line_of_sight_takes_two_written_points_and_only_the_written_heights() {
        let examples = terrain_examples(
            ligne_de_vue,
            "terrain.line_of_sight",
            &[
                "dem",
                "observer",
                "target",
                "observer_height",
                "target_height",
            ],
        );
        points_in_the_dem_system(&examples, &["observer", "target"]);
        written_only(&examples, "observer_height", EYE_MARKERS);
        written_only(&examples, "target_height", &["cible"]);
    }

    #[test]
    fn a_viewshed_takes_the_written_point_and_only_the_written_radius() {
        let examples = terrain_examples(
            champ_de_vision,
            "terrain.viewshed",
            &[
                "dem",
                "observer",
                "observer_height",
                "target_height",
                "radius",
            ],
        );
        points_in_the_dem_system(&examples, &["observer"]);
        written_only(&examples, "observer_height", EYE_MARKERS);
        written_only(&examples, "target_height", &["cible"]);
        written_only(&examples, "radius", &[" sur ", "rayon", "jusqu'à"]);
        assert!(examples.iter().any(|e| {
            let p = &e.expected["calls"][0]["params"];
            p["observer"]["epsg"] == 32631 && p["radius"]["unit"] == "km"
        }));
    }

    #[test]
    fn what_a_terrain_request_lacks_is_asked() {
        let examples = valid(question_terrain, 400);
        let (mut no_dem, mut no_point, mut no_unit) = (0, 0, 0);
        for e in &examples {
            assert_eq!(e.expected["kind"], "question", "{}", e.request);
            let q = e.expected["question"].as_str().unwrap();
            assert!(!q.contains(" de le "), "{q}");
            // Le raster de coût peut venir en distraction (11.12, lot 2b-1) : c'est le MNT
            // qui manque, pas tout raster.
            if !has_theme(e, "mnt") {
                no_dem += 1;
                assert!(q.contains("MNT") || q.contains("modèle numérique"), "{q}");
            } else if q.contains("degrés") {
                no_unit += 1;
                assert!(q.contains("pourcentage"), "{q}");
            } else {
                no_point += 1;
                let q = q.to_lowercase();
                assert!(
                    q.contains('x') && q.contains('y') && q.contains("système"),
                    "{q}"
                );
            }
        }
        assert!(no_dem > 0 && no_point > 0 && no_unit > 0);
        // Une ligne de vue dont un seul point est écrit, un lieu nommé sans coordonnées.
        assert!(examples
            .iter()
            .any(|e| e.request.contains("ligne de vue") && e.request.contains(" point ")));
        assert!(examples.iter().any(|e| e.request.contains("clocher")));
    }

    // -- dérivés du MNT (11.12, lot 2a) ---------------------------------------

    /// Les paramètres de la première étape d'un plan.
    fn params0(e: &Example) -> &Value {
        &e.expected["calls"][0]["params"]
    }

    /// Un angle facultatif d'un plan : unité écrite, valeur écrite ; jamais présent sans être
    /// écrit (repères `markers`).
    fn written_angle_only(examples: &[Example], key: &str, markers: &[&str]) {
        written_only(examples, key, markers);
        for e in examples {
            if let Some(angle) = params0(e).get(key) {
                written_angle(e, angle);
            }
        }
    }

    #[test]
    fn an_aspect_carries_the_dem_source_only() {
        let examples = terrain_examples(exposition, "terrain.aspect", &["dem"]);
        assert!(examples.iter().any(|e| e.request.contains("exposition")));
        assert!(examples.iter().any(|e| e.request.contains("versants")));
    }

    #[test]
    fn a_hillshade_takes_only_the_written_sun_and_exaggeration() {
        let examples = terrain_examples(
            ombrage,
            "terrain.hillshade",
            &["dem", "sun_azimuth", "sun_altitude", "z_factor"],
        );
        written_angle_only(&examples, "sun_azimuth", &["azimut"]);
        written_angle_only(
            &examples,
            "sun_altitude",
            &["horizon", "hauteur du soleil", "élevé de"],
        );
        written_only(&examples, "z_factor", &["exagération", "facteur z"]);
        for e in &examples {
            if let Some(z) = params0(e).get("z_factor") {
                written(e, z);
            }
        }
        // Degrés et grades vus ; une direction nommée porte son azimut écrit.
        let units: std::collections::BTreeSet<&str> = examples
            .iter()
            .filter_map(|e| params0(e)["sun_azimuth"]["unit"].as_str())
            .collect();
        assert_eq!(units.into_iter().collect::<Vec<_>>(), ["deg", "gon"]);
        assert!(examples
            .iter()
            .any(|e| e.request.contains("à l'ouest (azimut")));
    }

    #[test]
    fn a_cumulative_viewshed_takes_two_to_four_written_points_of_the_dem_system() {
        let examples = terrain_examples(
            champ_de_vision_cumule,
            "terrain.viewshed_cumulative",
            &[
                "dem",
                "observers",
                "observer_height",
                "target_height",
                "radius",
            ],
        );
        let mut counts = std::collections::BTreeSet::new();
        let mut systems = std::collections::BTreeSet::new();
        for e in &examples {
            let dem = cited_dem(e);
            let observers = params0(e)["observers"].as_array().unwrap();
            counts.insert(observers.len());
            for o in observers {
                if let Some(d) = dem.get("epsg") {
                    assert_eq!(d, &o["epsg"], "{}", e.request);
                }
                systems.insert(o["epsg"].as_u64().unwrap());
                written(e, &o["x"]);
                written(e, &o["y"]);
            }
            // Le rayon peut s'écrire en lettres (« un kilomètre »), comme pour `champ_de_vision`.
            for key in ["observer_height", "target_height"] {
                if let Some(v) = params0(e).get(key) {
                    written(e, &v["value"]);
                }
            }
        }
        assert_eq!(counts.into_iter().collect::<Vec<_>>(), [2, 3, 4]);
        assert_eq!(
            systems.into_iter().collect::<Vec<_>>(),
            [2154, 32631, 32632]
        );
        written_only(&examples, "observer_height", EYE_MARKERS);
        written_only(&examples, "target_height", &["cible"]);
        written_only(&examples, "radius", &[" sur ", "rayon", "jusqu'à"]);
    }

    #[test]
    fn peaks_take_only_the_written_count_prominence_and_spacing() {
        let examples = terrain_examples(
            points_hauts,
            "terrain.peaks",
            &["dem", "max_count", "min_prominence", "min_distance"],
        );
        written_only(&examples, "min_prominence", &["proéminence"]);
        written_only(
            &examples,
            "min_distance",
            &["espacés", "distants", "séparés"],
        );
        let (mut counted, mut uncounted) = (0, 0);
        for e in &examples {
            let p = params0(e);
            match p.get("max_count") {
                Some(n) => {
                    counted += 1;
                    let n = n.as_u64().unwrap();
                    assert!((1..=100).contains(&n), "{}", e.request);
                    written(e, &json!(n));
                }
                None => uncounted += 1,
            }
            for key in ["min_prominence", "min_distance"] {
                if let Some(v) = p.get(key) {
                    written(e, &v["value"]);
                }
            }
        }
        assert!(counted > 0 && uncounted > 0);
    }

    #[test]
    fn a_slope_threshold_is_a_slope_then_a_reclassification_of_step_1() {
        let registry = cassini_core::registry::Registry::standard();
        let examples = valid(pente_seuil, 600);
        let (mut above, mut below, mut two, mut three) = (0, 0, 0, 0);
        for e in &examples {
            assert_eq!(
                commands(e),
                ["terrain.slope", "raster.reclassify"],
                "{}",
                e.request
            );
            cited_dem(e);
            let plan: cassini_core::plan::Plan =
                serde_json::from_value(e.expected.clone()).unwrap();
            let guarded =
                cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
            assert_eq!(guarded, plan, "{}", e.request);
            // L'unité de la pente est celle du seuil.
            let r = &e.request;
            let degrees = r.contains("degrés") || r.contains('°');
            assert_eq!(degrees, !r.contains('%'), "{r}");
            let unit = &params0(e)["unit"];
            assert_eq!(unit == "degrees", degrees, "{r}");
            let p = &call(e, 1)["params"];
            assert_eq!(p["raster"], "step:1", "{r}");
            assert_eq!(p.as_object().unwrap().len(), 2, "{r}");
            let classes = p["classes"].as_array().unwrap();
            // Valeurs 1, 2, 3 dans l'ordre, bornes contiguës et croissantes, toutes écrites.
            for (k, c) in classes.iter().enumerate() {
                assert_eq!(c["value"], json!(k + 1), "{r}");
                for bound in ["min", "max"] {
                    if let Some(v) = c.get(bound) {
                        written(e, v);
                    }
                }
                if let (Some(lo), Some(hi)) = (c["min"].as_f64(), c["max"].as_f64()) {
                    assert!(lo < hi, "{r}");
                }
            }
            for pair in classes.windows(2) {
                assert_eq!(pair[0]["max"], pair[1]["min"], "{r}");
            }
            assert!(classes.first().unwrap().get("min").is_none() || classes.len() == 1);
            assert!(classes.last().unwrap().get("max").is_none() || classes.len() == 1);
            match classes.len() {
                1 if classes[0].get("min").is_some() => above += 1,
                1 => below += 1,
                2 => two += 1,
                3 => three += 1,
                n => panic!("{n} classes : {r}"),
            }
        }
        assert!(above > 0 && below > 0 && two > 0 && three > 0);
        assert!(examples
            .iter()
            .any(|e| e.request.contains("dépasse-t-elle")));
    }

    #[test]
    fn the_cell_selection_guard_catches_the_old_refusal() {
        // Le garde-fou de l'entrée 11.12, lot 2a : il reconnaît l'ancien refus, s'il revenait.
        assert!(selects_no_cell(&refusal(
            "Aucune commande du catalogue ne sélectionne les cellules d'un raster par leur valeur."
        )));
        assert!(!selects_no_cell(&refusal(NO_RASTER_STATS)));
    }

    // -- coût de déplacement (11.12, lot 2b-1) ---------------------------------

    /// Le raster de coût d'un plan : la couche `cout` dont `cost` est la `source`, jamais le MNT.
    fn cited_cost(e: &Example) -> &Value {
        let layer = cited_raster(e, "cost");
        let id = layer["layer_id"].as_str().unwrap();
        assert!(
            theme("cout")
                .slugs
                .iter()
                .any(|s| id.starts_with(&format!("{s}_"))),
            "{id} : {}",
            e.request
        );
        layer
    }

    /// Les plans d'appels d'une famille de coût : une étape `command`, ses clés exactement
    /// `keys`, `cost` source du raster de coût, le plan laissé par le garde-fou ; et ses
    /// questions, sans plan. Rend les deux parts.
    fn cost_examples(
        make: fn(&mut Rng) -> Option<Example>,
        command: &str,
        keys: &[&str],
    ) -> (Vec<Example>, Vec<Example>) {
        let registry = cassini_core::registry::Registry::standard();
        let (planned, asked): (Vec<Example>, Vec<Example>) = valid(make, 600)
            .into_iter()
            .partition(|e| e.expected["kind"] == "calls");
        for e in &planned {
            assert_eq!(commands(e), [command], "{}", e.request);
            let params = params0(e).as_object().unwrap();
            let mut got: Vec<&str> = params.keys().map(String::as_str).collect();
            got.sort_unstable();
            let mut want = keys.to_vec();
            want.sort_unstable();
            assert_eq!(got, want, "{}", e.request);
            cited_cost(e);
            let plan: cassini_core::plan::Plan =
                serde_json::from_value(e.expected.clone()).unwrap();
            let guarded =
                cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
            assert_eq!(guarded, plan, "{}", e.request);
        }
        for e in &asked {
            assert_eq!(e.expected["kind"], "question", "{}", e.request);
            assert!(e.expected.get("calls").is_none());
        }
        assert!(!planned.is_empty() && !asked.is_empty());
        (planned, asked)
    }

    /// Un point du plan : écrit dans la demande, dans le système du raster de coût. Rend le
    /// rang de son X parmi les nombres de la demande, pour l'ordre.
    fn cost_point(
        e: &Example,
        point: &Value,
        systems: &mut std::collections::BTreeSet<u64>,
    ) -> usize {
        if let Some(epsg) = cited_cost(e).get("epsg") {
            assert_eq!(epsg, &point["epsg"], "{}", e.request);
        }
        systems.insert(point["epsg"].as_u64().unwrap());
        written(e, &point["x"]);
        written(e, &point["y"]);
        let x = point["x"].as_f64().unwrap();
        cassini_core::point::request_numbers(&e.request)
            .iter()
            .position(|n| (n - x).abs() < 1e-9)
            .unwrap()
    }

    /// Une question de point manquant : X, Y et système.
    fn asks_for_the_point(q: &str) -> bool {
        let q = q.to_lowercase();
        q.contains('x') && q.contains('y') && q.contains("système")
    }

    #[test]
    fn a_cost_distance_takes_one_to_three_written_sources_of_the_cost_system() {
        let (planned, asked) =
            cost_examples(cout_cumule, "terrain.cost_distance", &["cost", "sources"]);
        let mut counts = std::collections::BTreeSet::new();
        let mut systems = std::collections::BTreeSet::new();
        for e in &planned {
            let sources = params0(e)["sources"].as_array().unwrap();
            counts.insert(sources.len());
            let ranks: Vec<usize> = sources
                .iter()
                .map(|s| cost_point(e, s, &mut systems))
                .collect();
            let distinct: std::collections::BTreeSet<String> =
                sources.iter().map(|s| s["x"].to_string()).collect();
            if distinct.len() == sources.len() {
                assert!(ranks.is_sorted(), "ordre : {}", e.request);
            }
            for s in sources {
                assert!(!s.to_string().contains("step:"), "{}", e.request);
            }
        }
        assert_eq!(counts.into_iter().collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(
            systems.into_iter().collect::<Vec<_>>(),
            [2154, 32631, 32632]
        );
        // Le MNT en distraction ne change pas le raster cité.
        assert!(planned.iter().any(|e| has_theme(e, "mnt")));
        for e in &asked {
            assert!(has_theme(e, "cout"), "{}", e.request);
            assert!(asks_for_the_point(e.expected["question"].as_str().unwrap()));
        }
        assert!(planned.iter().any(|e| e.request.contains("« ")));
    }

    #[test]
    fn a_least_cost_path_goes_from_the_written_start_to_the_written_end() {
        let (planned, asked) = cost_examples(
            chemin_moindre_cout,
            "terrain.least_cost_path",
            &["cost", "start", "end"],
        );
        let mut systems = std::collections::BTreeSet::new();
        for e in &planned {
            let p = params0(e);
            let start = cost_point(e, &p["start"], &mut systems);
            let end = cost_point(e, &p["end"], &mut systems);
            if p["start"]["x"] != p["end"]["x"] {
                assert!(start < end, "départ puis arrivée : {}", e.request);
            }
        }
        assert_eq!(
            systems.into_iter().collect::<Vec<_>>(),
            [2154, 32631, 32632]
        );
        assert!(planned.iter().any(|e| has_theme(e, "mnt")));
        let (mut no_cost, mut no_cost_with_dem, mut no_point) = (0, 0, 0);
        for e in &asked {
            let q = e.expected["question"].as_str().unwrap();
            if has_theme(e, "cout") {
                no_point += 1;
                assert!(asks_for_the_point(q), "{q}");
            } else {
                // Sans raster de coût, on le demande ; le MNT n'en tient jamais lieu.
                no_cost += 1;
                no_cost_with_dem += usize::from(has_theme(e, "mnt"));
                assert!(q.contains("raster de coût"), "{q}");
                assert!(!q.contains("MNT"), "{q}");
            }
        }
        assert!(no_cost > 0 && no_cost_with_dem > 0 && no_point > 0);
    }

    // -- critères, masques et superposition (11.12, lot 2b-2) ------------------

    /// Le thème et la couche du raster de source `path` : présent dans l'inventaire, sinon échec.
    fn listed_raster<'a>(e: &'a Example, path: &Value) -> (&'static str, &'a Value) {
        assert!(
            !path.to_string().contains("step:"),
            "étape en liste : {}",
            e.request
        );
        let layer = e.inventory["layers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["kind"] == "raster" && l.get("source") == Some(path))
            .unwrap_or_else(|| panic!("raster {path} absent : {}", e.request));
        let id = layer["layer_id"].as_str().unwrap();
        let t = THEMES
            .iter()
            .find(|t| t.slugs.iter().any(|s| id.starts_with(&format!("{s}_"))))
            .unwrap_or_else(|| panic!("{id} sans thème"));
        (t.key, layer)
    }

    /// Les plans d'appels d'une famille de rasters en liste : une étape `command`, ses clés
    /// parmi `keys` (les `required` toujours là) ; et ses questions, sans plan, qui nomment un
    /// raster de `named` absent de l'inventaire.
    fn listed_examples(
        make: fn(&mut Rng) -> Option<Example>,
        command: &str,
        required: &[&str],
        keys: &[&str],
        named: &[&str],
    ) -> (Vec<Example>, Vec<Example>) {
        let (planned, asked): (Vec<Example>, Vec<Example>) = valid(make, 800)
            .into_iter()
            .partition(|e| e.expected["kind"] == "calls");
        for e in &planned {
            assert_eq!(commands(e), [command], "{}", e.request);
            let params = params0(e).as_object().unwrap();
            for k in params.keys() {
                assert!(keys.contains(&k.as_str()), "{k} : {}", e.request);
            }
            for k in required {
                assert!(params.contains_key(*k), "{k} : {}", e.request);
            }
        }
        for e in &asked {
            assert_eq!(e.expected["kind"], "question", "{}", e.request);
            assert!(e.expected.get("calls").is_none());
            let q = e.expected["question"].as_str().unwrap();
            assert!(
                named
                    .iter()
                    .any(|k| q.contains(theme(k).of_none) && !has_theme(e, k)),
                "{q} : {}",
                e.request
            );
        }
        assert!(!planned.is_empty() && !asked.is_empty());
        (planned, asked)
    }

    /// Rang de la première mention d'un raster dans la demande : un de ses sujets, ou le nom de
    /// sa couche entre guillemets.
    fn mention(e: &Example, key: &str, layer: &Value) -> usize {
        let name = format!("« {} »", layer["name"].as_str().unwrap());
        let words: Vec<&str> = match key {
            "masque_inondable" => vec!["inondables", "inondation"],
            "masque_protege" => vec!["protégés"],
            _ => raster_subjects(key).to_vec(),
        };
        words
            .iter()
            .copied()
            .chain([name.as_str()])
            .filter_map(|w| e.request.find(w))
            .min()
            .unwrap_or_else(|| panic!("{key} non écrit : {}", e.request))
    }

    #[test]
    fn a_weighted_overlay_takes_the_written_criteria_weights_and_masks_in_order() {
        let (planned, _) = listed_examples(
            superposition_ponderee,
            "raster.weighted_overlay",
            &["criteria"],
            &["criteria", "exclusions"],
            &CRITERIA,
        );
        let (mut counts, mut masks) = (BTreeSet::new(), BTreeSet::new());
        let (mut weighted, mut equal, mut double) = (0, 0, 0);
        for e in &planned {
            let p = params0(e);
            let criteria = p["criteria"].as_array().unwrap();
            counts.insert(criteria.len());
            let mut ranks = Vec::new();
            for c in criteria {
                let mut keys: Vec<&str> =
                    c.as_object().unwrap().keys().map(String::as_str).collect();
                keys.sort_unstable();
                assert_eq!(keys, ["raster", "weight"], "{}", e.request);
                let (key, layer) = listed_raster(e, &c["raster"]);
                assert!(CRITERIA.contains(&key), "critère {key} : {}", e.request);
                ranks.push(mention(e, key, layer));
            }
            assert!(ranks.is_sorted(), "ordre des critères : {}", e.request);
            let weights: Vec<u64> = criteria
                .iter()
                .map(|c| c["weight"].as_u64().unwrap())
                .collect();
            if e.request.contains("double") {
                double += 1;
                let mut w = weights.clone();
                w.sort_unstable();
                let mut want = vec![1; w.len() - 1];
                want.push(2);
                assert_eq!(w, want, "{}", e.request);
            } else if cassini_core::point::request_numbers(&e.request).is_empty() {
                equal += 1;
                assert!(weights.iter().all(|w| *w == 1), "{}", e.request);
            } else {
                weighted += 1;
                for c in criteria {
                    written(e, &c["weight"]);
                }
            }
            let exclusions = p["exclusions"].as_array().cloned().unwrap_or_default();
            masks.insert(exclusions.len());
            let mut ranks = Vec::new();
            for m in &exclusions {
                let (key, layer) = listed_raster(e, m);
                assert!(MASKS.contains(&key), "masque {key} : {}", e.request);
                ranks.push(mention(e, key, layer));
            }
            assert!(ranks.is_sorted(), "ordre des masques : {}", e.request);
            if exclusions.is_empty() {
                assert!(
                    !e.request.contains("inond") && !e.request.contains("protégé"),
                    "masque non écrit : {}",
                    e.request
                );
            }
        }
        assert_eq!(counts.into_iter().collect::<Vec<_>>(), [2, 3, 4]);
        assert_eq!(masks.into_iter().collect::<Vec<_>>(), [0, 1, 2]);
        assert!(weighted > 0 && equal > 0 && double > 0);
        // Le MNT ou le raster de coût en distraction ne deviennent jamais un critère.
        assert!(planned.iter().any(|e| has_theme(e, "mnt")));
        assert!(planned.iter().any(|e| has_theme(e, "cout")));
    }

    #[test]
    fn a_raw_raster_is_reclassified_directly_into_contiguous_classes() {
        let (planned, _) = listed_examples(
            reclassement_raster,
            "raster.reclassify",
            &["raster", "classes"],
            &["raster", "classes"],
            &RAW_RASTERS,
        );
        let (mut lengths, mut rasters) = (BTreeSet::new(), BTreeSet::new());
        let mut written_values = 0;
        for e in &planned {
            let p = params0(e);
            let (key, _) = listed_raster(e, &p["raster"]);
            assert!(RAW_RASTERS.contains(&key), "{key} : {}", e.request);
            rasters.insert(key);
            let noise = key == "bruit";
            if !noise {
                assert!(!e.request.contains("dB"), "{}", e.request);
            }
            let classes = p["classes"].as_array().unwrap();
            lengths.insert(classes.len());
            for c in classes {
                for bound in ["min", "max"] {
                    if let Some(b) = c.get(bound) {
                        written(e, b);
                        // La borne est écrite avec son unité : « 65 dB », « 1 000 m ».
                        let n = b.as_u64().unwrap();
                        let with_unit = if noise {
                            format!("{n} dB")
                        } else {
                            format!("{} m", thousands(n))
                        };
                        assert!(
                            e.request.contains(&with_unit),
                            "{with_unit} : {}",
                            e.request
                        );
                    }
                }
            }
            if let [only] = classes.as_slice() {
                assert_eq!(only["value"], 1, "{}", e.request);
                let above = ["dépasse", "au-dessus", "plus de"];
                let below = ["sous", "en dessous", "moins de"];
                let words = if only.get("min").is_some() {
                    assert!(only.get("max").is_none());
                    above
                } else {
                    assert!(only.get("max").is_some());
                    below
                };
                assert!(words.iter().any(|w| e.request.contains(w)), "{}", e.request);
                continue;
            }
            // Bornes contiguës : ouverte en bas, ouverte en haut, chaque min la max d'avant.
            assert!(classes[0].get("min").is_none(), "{}", e.request);
            assert!(classes[classes.len() - 1].get("max").is_none());
            for pair in classes.windows(2) {
                assert_eq!(pair[1]["min"], pair[0]["max"], "{}", e.request);
            }
            let values: Vec<u64> = classes
                .iter()
                .map(|c| c["value"].as_u64().unwrap())
                .collect();
            let default: Vec<u64> = (1..=values.len() as u64).collect();
            if values != default {
                written_values += 1;
                for c in classes {
                    written(e, &c["value"]);
                }
            }
        }
        assert_eq!(lengths.into_iter().collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(
            rasters.into_iter().collect::<Vec<_>>(),
            ["bruit", "distance_routes"]
        );
        assert!(written_values > 0);
        assert!(planned.iter().any(|e| has_theme(e, "mnt")));
    }

    #[test]
    fn family_names_are_unique() {
        let mut names: Vec<&str> = FAMILIES.iter().map(|f| f.name).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n);
    }
}
