//! Familles de tournures : chacune tire un inventaire, écrit une demande française à partir
//! de gabarits à emplacements, et construit le plan attendu (ou la question, ou le refus).
//! Les règles de chaînage sont celles du prompt de `ai/src/plan.rs` : le filtre sur un champ
//! avant le tampon ou l'intersection, le tampon sur la couche de référence, puis des
//! intersections successives dont la première porte la couche cherchée.
//!
//! Vocabulaire civil seulement (CLAUDE.md §2.9). Une phrase mal comprise par le modèle devient
//! une famille ou une tournure ici, jamais une règle de prompt.

use serde_json::{json, Value};

use crate::inventories::{theme, Builder, FieldDef, Geom, Kind, Layer, Noun, Theme, Val, THEMES};
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
    // Environ 5 % du jeu : un exemple d'une autre famille, vu au tour de redemande. Le poids
    // compte les tirages laissés (corruption acceptée par `check_plan`, premier essai qui
    // n'est pas un plan d'appels) : un sur deux environ.
    Family {
        name: "redemande",
        weight: 20,
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
        let head = match rng.below(5) {
            0 => format!("Exporte la carte en PDF {at}"),
            1 => format!("Sors la carte {at} en PDF"),
            2 => format!("Fais-moi un PDF de la carte {at}"),
            3 => format!("Produis la carte {at} au format PDF"),
            _ => format!("Génère un PDF de la carte {at}"),
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
        let clause = match rng.below(3) {
            0 => format!(" et sors-moi la carte en PDF {at}"),
            1 => format!(", puis exporte la carte {at} en PDF"),
            _ => format!(" et fais-en une carte en PDF {at}"),
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
    let (zone, x) = if rng.chance(1, 2) {
        (31, rng.range(40_000, 70_000) * 10)
    } else {
        (32, rng.range(28_000, 45_000) * 10)
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
        11 => (format!("Calcule l'itinéraire le plus court entre deux {pl}"), "Aucune commande du catalogue ne calcule d'itinéraire."),
        12 => (format!("Télécharge le cadastre {}", de_place(commune)), "Aucune commande du catalogue ne télécharge de données : le projet utilise les fichiers de l'utilisateur."),
        // Le papier A3 vit dans `refus_hors_registre`, avec ses variantes.
        13 => ("Publie la carte sur internet".to_string(), "Aucune commande du catalogue ne publie de carte en ligne."),
        14 => (format!("Modifie la valeur d'un champ pour toutes les {pl}"), "Aucune commande du catalogue ne modifie les attributs d'une couche."),
        _ => ("Affiche la carte en 3D".to_string(), "Aucune commande du catalogue n'affiche de vue en 3D."),
    };
    let request = finish(rng, &body, false, false);
    Some(example("refus", b, rng, request, refusal(reason)))
}

// -- hors registre ---------------------------------------------------------

/// Un point en Lambert-93, écrit comme un utilisateur l'écrirait.
fn lambert_point(rng: &mut Rng) -> String {
    let x = rng.range(60_000, 70_000) * 10;
    let y = rng.range(678_000, 690_000) * 10;
    let (xs, ys) = if rng.chance(1, 2) {
        (thousands(x), thousands(y))
    } else {
        (x.to_string(), y.to_string())
    };
    // Pas « le point X / Y (Lambert-93) » : c'est la forme des phrases de terrain des essais
    // réels (`training/tests/recouvrement.rs`).
    match rng.below(2) {
        0 => format!("le point X = {xs}, Y = {ys} en EPSG:2154"),
        _ => format!("le point ({xs} ; {ys}) en Lambert 93"),
    }
}

const NO_VIEWSHED: &str = "Aucune commande du catalogue ne calcule de champ de vision.";
const NO_LINE_OF_SIGHT: &str =
    "Aucune commande du catalogue ne calcule de ligne de vue entre deux points.";
const NO_SLOPE: &str = "Aucune commande du catalogue ne calcule de pente.";
const NO_FRAMING: &str = "Aucune commande du catalogue ne cadre ni ne zoome la carte.";
const ONLY_A4: &str = "L'export PDF du catalogue produit seulement une page A4 paysage.";
const NO_RASTER_STATS: &str = "Aucune commande du catalogue ne calcule de statistiques sur un raster : le décompte par zone ne compte que des points.";
/// Décision du propriétaire (2026-10-23, option a) : « de chaque gare », « de toutes les
/// gares », « des gares » ; l'isochrone du catalogue part d'un seul point en coordonnées.
pub const NO_MULTI_ORIGIN: &str = "L'isochrone du catalogue part d'un seul point donné en coordonnées : elle ne se calcule pas depuis plusieurs lieux à la fois.";

/// Ce que le registre ne sait pas encore faire, quoi que l'inventaire contienne (un MNT pour
/// le terrain) : un refus affirmatif, jamais un plan de substitution (un tampon à la place d'un
/// champ de vision, un décompte à la place d'une moyenne d'altitude).
fn refus_hors_registre(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let roll = rng.below(7);
    // Le terrain et les statistiques portent sur un MNT : il est là, ce n'est pas lui qui manque.
    if matches!(roll, 0 | 1 | 2 | 5) {
        b.add(rng, "mnt", &[], &[], &[]);
    }
    let (body, q, reason): (String, bool, &str) = match roll {
        0 => {
            let pt = lambert_point(rng);
            let h = *rng.pick(&[1u64, 2, 5, 10, 15, 30]);
            let (body, q) = match rng.below(5) {
                0 => (format!("Détermine la zone visible depuis {pt}"), false),
                1 => (format!("Quelles zones sont visibles depuis {pt}"), true),
                2 => (
                    format!(
                        "Fais une analyse de visibilité depuis {pt}, observateur à {h} m du sol"
                    ),
                    false,
                ),
                3 => (
                    format!("Montre tout ce qu'on aperçoit depuis le clocher, {pt}"),
                    false,
                ),
                _ => (
                    format!("Jusqu'où porte la vue depuis {pt}, à {h} m de hauteur"),
                    true,
                ),
            };
            (body, q, NO_VIEWSHED)
        }
        1 => {
            let (a, c) = (lambert_point(rng), lambert_point(rng));
            let (body, q) = match rng.below(4) {
                0 => (format!("Y a-t-il une ligne de vue entre {a} et {c}"), true),
                1 => (format!("Vérifie si {a} est visible depuis {c}"), false),
                2 => (
                    format!(
                        "Calcule la ligne de vue {} vers {c}",
                        a.replacen("le point", "du point", 1)
                    ),
                    false,
                ),
                _ => (
                    format!("Est-ce que {a} et {c} se voient l'un l'autre"),
                    true,
                ),
            };
            (body, q, NO_LINE_OF_SIGHT)
        }
        2 => {
            let (body, q) = match rng.below(5) {
                0 => (
                    "Fais une carte des pentes à partir du MNT".to_string(),
                    false,
                ),
                1 => ("Calcule les pentes du terrain en degrés".to_string(), false),
                2 => (
                    "Où la pente du terrain dépasse-t-elle 15 %".to_string(),
                    true,
                ),
                3 => ("Génère la carte des pentes".to_string(), false),
                _ => (
                    "Calcule l'inclinaison du terrain à partir du modèle numérique".to_string(),
                    false,
                ),
            };
            (body, q, NO_SLOPE)
        }
        3 => {
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
        4 => {
            b.distract(rng, 1, vector);
            let body = match rng.below(6) {
                0 => "Exporte la carte en PDF au format A3".to_string(),
                1 => "Sors la carte en A3".to_string(),
                2 => "Fais un PDF de la carte sur une feuille A3".to_string(),
                3 => "Exporte la carte en PDF, en portrait".to_string(),
                4 => format!(
                    "Produis la carte en PDF A0, titre « {} »",
                    rng.pick(USER_TITLES)
                ),
                _ => "Génère un PDF de la carte au format A3 paysage".to_string(),
            };
            (body, false, ONLY_A4)
        }
        // Un trajet depuis plusieurs lieux : refus, jamais un plan ni une question.
        6 => {
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
            let (body, q) = match rng.below(4) {
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
                _ => (
                    format!("Donne l'altitude {stat} du terrain dans {}", z.les()),
                    false,
                ),
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn out_of_registry_requests_are_refused_never_planned() {
        for e in valid(refus_hors_registre, 300) {
            assert_eq!(e.expected["kind"], "refusal", "{}", e.request);
            let reason = e.expected["reason"].as_str().unwrap();
            if [NO_VIEWSHED, NO_LINE_OF_SIGHT, NO_SLOPE, NO_RASTER_STATS].contains(&reason) {
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
            // `terrain.slope` est arrivé (4.2) : le refus `NO_SLOPE` est désormais faux, mais
            // la commande s'apprend au modèle dans l'entrée dédiée de l'epic raster-et-terrain
            // (« Le modèle apprend les trois commandes de terrain »), qui retire ce refus.
            // Retirer `NO_SLOPE` doit retirer cette exception : la référence ci-dessous casse
            // la compilation si l'un part sans l'autre.
            if id == "terrain.slope" && !NO_SLOPE.is_empty() {
                continue;
            }
            // Même cas pour `terrain.line_of_sight` (4.4) et son refus `NO_LINE_OF_SIGHT`.
            if id == "terrain.line_of_sight" && !NO_LINE_OF_SIGHT.is_empty() {
                continue;
            }
            assert!(!id.starts_with("terrain."), "{id}");
            for word in ["zoom", "frame", "extent", "pan", "center", "centre", "cadr"] {
                assert!(!id.contains(word), "{id}");
            }
        }
        let pdf = registry.spec("layout.export_pdf").expect("export PDF");
        let props = pdf.params_schema["properties"]
            .as_object()
            .expect("propriétés");
        for name in props.keys() {
            for word in ["paper", "format", "size", "orientation", "page"] {
                assert!(!name.contains(word), "{name}");
            }
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

    #[test]
    fn family_names_are_unique() {
        let mut names: Vec<&str> = FAMILIES.iter().map(|f| f.name).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n);
    }
}
