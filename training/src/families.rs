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
        weight: 9,
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
        name: "isochrone",
        weight: 4,
        make: isochrone,
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
        let prep = *rng.pick(&["à", "sur la commune de", "dans la commune de"]);
        s.push_str(&format!(" {prep} {}", rng.pick(COMMUNES)));
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
    let (op, phrase) = if rng.chance(1, 2) {
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
    let np = format!("les {} {}", t.noun.pl, phrase.replace("{f}", fragment));
    let (head, q) = open_find(rng, &t.noun, &np, true);
    let request = finish(rng, &head, q, false);
    let expected = calls(vec![select(&layer.id, &field.name, op, fragment)]);
    Some(example("filtre_texte", b, rng, request, expected))
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
    let np = match rng.below(3) {
        0 => format!("les {} qui ne sont pas {phrase}", t.noun.pl),
        1 => format!(
            "{} les {} sauf {} {phrase}",
            if t.noun.fem { "toutes" } else { "tous" },
            t.noun.pl,
            t.noun.ceux()
        ),
        _ => format!(
            "les {} dont le champ {} est différent de {}",
            t.noun.pl, field.name, val.v
        ),
    };
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
        0 => format!("Fais une zone tampon {} autour {around}", d.de()),
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

fn draft_multi(rng: &mut Rng) -> Option<Draft> {
    let mut b = Builder::new();
    match rng.below(4) {
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
            let t = pick_theme(rng, &b, |t| ordinary(t) && !text_fields(t).is_empty())?;
            let def = *rng.pick(&text_fields(t));
            let target = b.add(rng, t.key, &[def.role], &[], &[]);
            let field = target.field(def.role)?;
            let val = *rng.pick(&field.values);
            let head = qualified(rng, &t.noun, def, val, Det::Les);
            let r = pick_theme(rng, &b, ordinary)?;
            let reference = b.add(rng, r.key, &[], &[], &[]);
            let d = dist(rng);
            let rr = de_ref(rng, &r.noun);
            let np = format!("{head} {}", near(rng, &t.noun, &d, &rr));
            Some(Draft {
                calls: vec![
                    select(&target.id, &field.name, "eq", val.v),
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
        _ => {
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
    }
}

fn conditions_multiples(rng: &mut Rng) -> Option<Example> {
    let d = draft_multi(rng)?;
    let (head, q) = open_find(rng, d.noun, &d.np, true);
    let request = finish(rng, &head, q, true);
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
    let (mut body, _) = open_find(rng, d.noun, &d.np, false);
    let (clause, title) = pdf_clause(rng, &d);
    body.push_str(&clause);
    d.calls.push(pdf(&title));
    let request = finish(rng, &body, false, false);
    Some(example("export_pdf", d.b, rng, request, calls(d.calls)))
}

// -- isochrone, décompte ---------------------------------------------------

fn isochrone(rng: &mut Rng) -> Option<Example> {
    let mut b = Builder::new();
    let key = *rng.pick(&["routes", "reseau"]);
    let network = b.add(rng, key, &[], &[], &[]);
    // Une seule couche de lignes : le réseau ne doit pas être ambigu.
    for other in ["pistes", "lignes_elec", "canalisations", "rivieres"] {
        b.exclude(other);
    }
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
    let (x, y, epsg, place): (Value, Value, u32, String) = if rng.chance(1, 5) {
        let lon = rng.range(22_000, 26_000);
        let lat = rng.range(486_000, 489_000);
        let (xs, ys) = (
            format!("{}.{:04}", lon / 10_000, lon % 10_000),
            format!("{}.{:04}", lat / 10_000, lat % 10_000),
        );
        let x: f64 = xs.parse().ok()?;
        let y: f64 = ys.parse().ok()?;
        let crs = *rng.pick(&["WGS84", "EPSG:4326", "WGS 84"]);
        (
            json!(x),
            json!(y),
            4326,
            format!("le point de longitude {xs} et de latitude {ys} ({crs})"),
        )
    } else {
        let x = rng.range(60_000, 70_000) * 10;
        let y = rng.range(678_000, 690_000) * 10;
        let (xs, ys) = if rng.chance(1, 2) {
            (thousands(x), thousands(y))
        } else {
            (x.to_string(), y.to_string())
        };
        let crs = *rng.pick(&["Lambert-93", "Lambert 93", "EPSG:2154"]);
        let place = match rng.below(3) {
            0 => format!("le point {xs} / {ys} ({crs})"),
            1 => format!("le point X = {xs}, Y = {ys} en {crs}"),
            _ => format!("le point ({xs} ; {ys}), {crs}"),
        };
        (json!(x), json!(y), 2154, place)
    };
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
        1 => {
            let de = if duration.starts_with("un") {
                "d'"
            } else {
                "de "
            };
            (
                format!("Calcule l'isochrone {de}{duration} {mode} depuis {place}{via}"),
                false,
            )
        }
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
    let mode = if walk { "walk" } else { "car" };
    let expected = calls(vec![json!({"command": "analysis.isochrone", "params": {
        "layer_id": network.id, "origin": {"x": x, "y": y, "epsg": epsg},
        "mode": mode, "duration": {"value": value, "unit": unit}}})]);
    Some(example("isochrone", b, rng, request, expected))
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
    Some(example("ouverture", b, rng, request, expected))
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
            1 => (format!("Décris {which}"), false),
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
            None => (format!("Décris {}", absent.the_layer), false),
        },
        _ => (format!("Décris {}", absent.the_layer), false),
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
    let (body, reason): (String, &str) = match rng.below(17) {
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
        13 => ("Exporte la carte en PDF au format A3".to_string(), "L'export PDF du catalogue produit seulement une page A4 paysage."),
        14 => ("Publie la carte sur internet".to_string(), "Aucune commande du catalogue ne publie de carte en ligne."),
        15 => (format!("Modifie la valeur d'un champ pour toutes les {pl}"), "Aucune commande du catalogue ne modifie les attributs d'une couche."),
        _ => ("Affiche la carte en 3D".to_string(), "Aucune commande du catalogue n'affiche de vue en 3D."),
    };
    let request = finish(rng, &body, false, false);
    Some(example("refus", b, rng, request, refusal(reason)))
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
    fn family_names_are_unique() {
        let mut names: Vec<&str> = FAMILIES.iter().map(|f| f.name).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n);
    }
}
