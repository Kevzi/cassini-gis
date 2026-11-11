//! Les phrases des barres (5.54) : le jeu tenu à l'écart et les phrases des essais réels, lus
//! dans leurs fichiers, et la normalisation « à un nombre ou une commune près ». Déplacé de
//! `training/tests/recouvrement.rs`, qui les contrôle sur le jeu engendré, pour que la commande
//! refuse aussi d'écrire un jeu ciblé (`--familles`) qui en copierait une.
//!
//! Le générateur ne lit jamais ces phrases pour s'en inspirer : seul le contrôle les compare.

use std::collections::HashSet;

use serde_json::Value;

use crate::families::{Example, COMMUNES};
use crate::normalize;

const HELD_OUT: &str = include_str!("../../ai/tests/tenu_a_l_ecart/sentences.json");
/// Les phrases de `plan_reel` : celles de l'épreuve du modèle local, qu'il lit telles quelles.
const PLAN_REEL: &str = include_str!("../../tests/epreuve-modele-local/sentences.json");
const QUESTION_REEL: &str = include_str!("../../ai/tests/question_reel.rs");
const DUREE_REEL: &str = include_str!("../../ai/tests/duree_reel.rs");
const ECHELLE_UTM_REEL: &str = include_str!("../../ai/tests/echelle_utm_reel.rs");
const TERRAIN_REEL: &str = include_str!("../../ai/tests/terrain_reel.rs");
const EPIC11_REEL: &str = include_str!("../../ai/tests/epic11_reel.rs");
const STYLE_REEL: &str = include_str!("../../ai/tests/style_reel.rs");
/// Les phrases de la recette QGIS d'Annecy (5.45), écrites à la main, jouées sous QGIS.
const RECETTE_ANNECY: &str = include_str!("../../qgis-plugin/tests/recette/annecy.json");
/// Les phrases de la barre par commande (5.53), écrites à la main : `{id, command, phrase, …}`.
const PAR_COMMANDE: &str = include_str!("../../ai/tests/par_commande/phrases.json");
/// Les phrases neuves de `plan_reel` (5.53, livrable 3), écrites à la main : `{id, phrase, …}`.
const PLAN_REEL_PHRASES: &str = include_str!("../../ai/tests/plan_reel/phrases.json");
/// Les phrases neuves de `question_reel` (5.53, livrable 3), écrites à la main.
const QUESTION_REEL_PHRASES: &str = include_str!("../../ai/tests/question_reel/phrases.json");
/// Les phrases neuves de `echelle_utm_reel` (5.53, livrable 3, lot 2), écrites à la main.
const ECHELLE_UTM_REEL_PHRASES: &str = include_str!("../../ai/tests/echelle_utm_reel/phrases.json");
/// Les phrases neuves de `terrain_reel` (5.53, livrable 3, lot 2), écrites à la main.
const TERRAIN_REEL_PHRASES: &str = include_str!("../../ai/tests/terrain_reel/phrases.json");
/// Les phrases neuves d'`epic11_reel` (5.53, livrable 3, lot 3), écrites à la main.
const EPIC11_REEL_PHRASES: &str = include_str!("../../ai/tests/epic11_reel/phrases.json");
/// Les phrases neuves de `style_reel` (5.53, livrable 3, lot 3), écrites à la main.
const STYLE_REEL_PHRASES: &str = include_str!("../../ai/tests/style_reel/phrases.json");

/// Les fichiers `phrases.json` écrits à la main pour 5.53, chacun avec son nom et son compte.
pub const HAND_WRITTEN: [(&str, &str, usize); 7] = [
    ("par_commande", PAR_COMMANDE, 99),
    ("plan_reel", PLAN_REEL_PHRASES, 23),
    ("question_reel", QUESTION_REEL_PHRASES, 35),
    ("echelle_utm_reel", ECHELLE_UTM_REEL_PHRASES, 36),
    ("terrain_reel", TERRAIN_REEL_PHRASES, 36),
    // Nommées à part des constantes de leur essai (`epic11_reel`, `style_reel`), comparées à elles.
    ("epic11_reel:phrases", EPIC11_REEL_PHRASES, 20),
    ("style_reel:phrases", STYLE_REEL_PHRASES, 32),
];

/// Texte normalisé, chiffres masqués et communes remplacées par un même repère.
pub fn masked(text: &str) -> String {
    let mut places: Vec<String> = COMMUNES.iter().map(|c| normalize(c)).collect();
    places.push("villeneuve le roi".to_string());
    // Les noms les plus longs d'abord : « villeneuve le roi » avant un nom plus court inclus.
    places.sort_by_key(|p| std::cmp::Reverse(p.len()));
    let mut out = format!(" {} ", normalize(text));
    for place in &places {
        out = out.replace(&format!(" {place} "), " lieu ");
    }
    let mut masked = String::new();
    let mut in_digits = false;
    for c in out.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                masked.push('0');
            }
            in_digits = true;
        } else {
            masked.push(c);
            in_digits = false;
        }
    }
    // « 1 200 » est un seul nombre : des nombres consécutifs n'en font qu'un.
    let mut words: Vec<&str> = masked.split_whitespace().collect();
    words.dedup_by(|a, b| *a == "0" && *b == "0");
    words.join(" ")
}

pub fn held_out() -> Vec<(String, String)> {
    from_json(HELD_OUT)
}

/// Les phrases `{id, text}` d'un fichier `sentences.json`.
fn from_json(text: &str) -> Vec<(String, String)> {
    let sentences: Vec<Value> = serde_json::from_str(text).expect("sentences.json lisible");
    assert!(!sentences.is_empty());
    sentences
        .iter()
        .map(|s| {
            (
                s["id"].as_str().expect("id").to_string(),
                s["text"].as_str().expect("texte").to_string(),
            )
        })
        .collect()
}

/// Les phrases `{id, phrase}` d'un `phrases.json` écrit à la main (5.53).
fn from_phrases(text: &str) -> Vec<(String, String)> {
    let phrases: Vec<Value> = serde_json::from_str(text).expect("phrases.json lisible");
    phrases
        .iter()
        .map(|p| {
            (
                p["id"].as_str().expect("id").to_string(),
                p["phrase"].as_str().expect("phrase").to_string(),
            )
        })
        .collect()
}

/// Les littéraux de chaîne de la valeur de la constante `start` (après son `=`, jusqu'au
/// premier `];` ou `);`) : les paires `("id", "texte")` d'un tableau de phrases.
fn literals(source: &str, start: &str) -> Vec<String> {
    let from = source
        .find(start)
        .unwrap_or_else(|| panic!("bloc « {start} » introuvable"));
    let block = &source[from + start.len()..];
    let block = &block[block.find('=').expect("valeur de la constante") + 1..];
    let end = block
        .find("];")
        .into_iter()
        .chain(block.find(");"))
        .min()
        .expect("fin du bloc");
    let mut out = Vec::new();
    let mut chars = block[..end].chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut text = String::new();
        while let Some(c) = chars.next() {
            match c {
                '\\' => text.extend(chars.next()),
                '"' => break,
                c => text.push(c),
            }
        }
        out.push(text);
    }
    out
}

/// Les paires `(id, texte)` d'un tableau de phrases écrit dans un fichier Rust.
fn pairs(source: &str, start: &str) -> Vec<(String, String)> {
    let found = literals(source, start);
    assert!(
        !found.is_empty() && found.len().is_multiple_of(2),
        "« {start} » : {found:?}"
    );
    found
        .chunks(2)
        .map(|p| (p[0].clone(), p[1].clone()))
        .collect()
}

/// Les phrases des essais réels de `ai/tests/` : `plan_reel`, `question_reel`, `duree_reel`,
/// `echelle_utm_reel`, `terrain_reel`, `epic11_reel`, `style_reel`, la recette QGIS d'Annecy et,
/// depuis 5.53, la barre par commande puis les phrases neuves de `plan_reel`, `question_reel`,
/// `echelle_utm_reel`, `terrain_reel`, `epic11_reel` et `style_reel`.
pub fn real_trials() -> Vec<(String, String)> {
    real_trials_by_source()
        .into_iter()
        .flat_map(|(_, s)| s)
        .collect()
}

/// Les phrases des essais réels, groupées par source (fichier ou constante qui les porte).
pub fn real_trials_by_source() -> Vec<(&'static str, Vec<(String, String)>)> {
    let mut all = vec![("plan_reel:epreuve", from_json(PLAN_REEL))];
    let questions = pairs(QUESTION_REEL, "const AMBIGUOUS");
    assert_eq!(questions.len(), 12, "{questions:?}");
    all.push(("question_reel:ambiguous", questions));
    let scenario = pairs(DUREE_REEL, "const SCENARIO");
    assert_eq!(scenario.len(), 8, "{scenario:?}");
    all.push(("duree_reel:scenario", scenario));
    all.push(("duree_reel:warmup", pairs(DUREE_REEL, "const WARMUP")));
    let scale_utm = pairs(ECHELLE_UTM_REEL, "const SENTENCES");
    assert_eq!(scale_utm.len(), 8, "{scale_utm:?}");
    all.push(("echelle_utm_reel:sentences", scale_utm));
    let terrain = pairs(TERRAIN_REEL, "const SENTENCES");
    assert_eq!(terrain.len(), 8, "{terrain:?}");
    all.push(("terrain_reel:sentences", terrain));
    let epic11 = pairs(EPIC11_REEL, "const SENTENCES");
    assert_eq!(epic11.len(), 28, "{epic11:?}");
    all.push(("epic11_reel", epic11));
    all.push((
        "epic11_reel:grid_question",
        pairs(EPIC11_REEL, "const GRID_QUESTION"),
    ));
    let foreign = pairs(EPIC11_REEL, "const FOREIGN_POINTS");
    assert_eq!(foreign.len(), 4, "{foreign:?}");
    all.push(("epic11_reel:foreign_points", foreign));
    let style = pairs(STYLE_REEL, "const SENTENCES");
    assert_eq!(style.len(), 10, "{style:?}");
    all.push(("style_reel", style));
    let annecy = from_json(RECETTE_ANNECY);
    assert_eq!(annecy.len(), 20, "{annecy:?}");
    all.push(("recette:annecy", annecy));
    for (name, text, count) in HAND_WRITTEN {
        let phrases = from_phrases(text);
        assert_eq!(phrases.len(), count, "{name} : {phrases:?}");
        all.push((name, phrases));
    }
    all
}

/// Les phrases des barres que `examples` reprennent : à l'identique ou à un nombre ou une
/// commune près pour le jeu tenu à l'écart, à un nombre ou une commune près pour les essais
/// réels. Rend leurs identifiants, jamais leur texte.
pub fn copies(examples: &[Example]) -> Vec<String> {
    let exact: HashSet<String> = examples.iter().map(|e| normalize(&e.request)).collect();
    let near: HashSet<String> = examples.iter().map(|e| masked(&e.request)).collect();
    let mut out = Vec::new();
    for (id, text) in held_out() {
        if exact.contains(&normalize(&text)) || near.contains(&masked(&text)) {
            out.push(format!("tenue à l'écart {id}"));
        }
    }
    for (id, text) in real_trials() {
        if near.contains(&masked(&text)) {
            out.push(format!("essai réel {id}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_request_that_copies_a_bar_sentence_is_named_by_its_id_only() {
        let (id, text) = held_out().into_iter().next().unwrap();
        let (real_id, real_text) = real_trials().into_iter().next().unwrap();
        let example = |request: String| Example {
            family: "essai",
            inventory: json!({"layers": [], "unloaded_files": []}),
            request,
            expected: json!({"kind": "refusal", "reason": "x"}),
            retry: None,
            focus: Vec::new(),
        };
        let found = copies(&[example(text.to_uppercase()), example(real_text.clone())]);
        assert_eq!(found[0], format!("tenue à l'écart {id}"));
        assert!(
            found.contains(&format!("essai réel {real_id}")),
            "{found:?}"
        );
        assert!(found.iter().all(|f| !f.contains(&real_text)));
        assert!(copies(&[example("Une demande neuve sans pareille.".into())]).is_empty());
    }
}
