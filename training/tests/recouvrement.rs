//! Garde-fou du jeu tenu à l'écart (ticket 5.10) : aucune de ses phrases ne doit figurer
//! dans le jeu engendré avec la graine par défaut. Depuis 5.15, il couvre aussi les phrases
//! des essais réels de `ai/tests/` : `plan_reel` (les phrases de l'épreuve), `question_reel` et
//! `duree_reel` (le scénario de démo et son amorce). Les familles visent leur sorte de demande
//! avec d'autres tournures ; elles ne les recopient jamais. Deux comparaisons, sur le texte normalisé
//! (minuscules, sans accents ni ponctuation) : à l'identique, puis à un nombre ou une commune
//! près (chiffres masqués, noms de communes remplacés par un même repère, des deux côtés).
//!
//! Le jeu tenu à l'écart a été écrit par un autre auteur que le générateur, qui ne l'a pas lu.
//! En cas d'échec, seul l'identifiant de la phrase est imprimé. On retire alors du générateur
//! la tournure fautive et on le signale ; on ne retouche jamais une famille pour coller au jeu
//! tenu à l'écart.

use std::collections::HashSet;

use cassini_training::families::COMMUNES;
use cassini_training::{generate, normalize, DEFAULT_COUNT, DEFAULT_SEED};
use serde_json::Value;

const HELD_OUT: &str = include_str!("../../ai/tests/tenu_a_l_ecart/sentences.json");
/// Les phrases de `plan_reel` : celles de l'épreuve du modèle local, qu'il lit telles quelles.
const PLAN_REEL: &str = include_str!("../../tests/epreuve-modele-local/sentences.json");
const QUESTION_REEL: &str = include_str!("../../ai/tests/question_reel.rs");
const DUREE_REEL: &str = include_str!("../../ai/tests/duree_reel.rs");

/// Texte normalisé, chiffres masqués et communes remplacées par un même repère.
fn masked(text: &str) -> String {
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

fn held_out() -> Vec<(String, String)> {
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

/// Les phrases des essais réels de `ai/tests/` : `plan_reel`, `question_reel`, `duree_reel`.
fn real_trials() -> Vec<(String, String)> {
    let mut all = from_json(PLAN_REEL);
    let questions = pairs(QUESTION_REEL, "const AMBIGUOUS");
    assert_eq!(questions.len(), 12, "{questions:?}");
    all.extend(questions);
    let scenario = pairs(DUREE_REEL, "const SCENARIO");
    assert_eq!(scenario.len(), 8, "{scenario:?}");
    all.extend(scenario);
    all.extend(pairs(DUREE_REEL, "const WARMUP"));
    all
}

/// Identifiants des phrases de `sentences` dont la forme `key` figure dans le jeu engendré.
fn overlap(sentences: Vec<(String, String)>, key: fn(&str) -> String) -> Vec<String> {
    let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
    assert_eq!(examples.len(), DEFAULT_COUNT);
    let generated: HashSet<String> = examples.iter().map(|e| key(&e.request)).collect();
    sentences
        .into_iter()
        .filter(|(_, text)| generated.contains(&key(text)))
        .map(|(id, _)| id)
        .collect()
}

#[test]
fn no_held_out_sentence_appears_in_the_generated_set() {
    let ids = overlap(held_out(), normalize);
    assert!(
        ids.is_empty(),
        "phrases tenues à l'écart engendrées telles quelles : {}",
        ids.join(", ")
    );
}

#[test]
fn no_held_out_sentence_appears_up_to_a_number_or_a_commune() {
    let ids = overlap(held_out(), masked);
    assert!(
        ids.is_empty(),
        "phrases tenues à l'écart engendrées à un nombre ou une commune près : {}",
        ids.join(", ")
    );
}

#[test]
fn no_sentence_of_the_real_trials_appears_in_the_generated_set() {
    let ids = overlap(real_trials(), masked);
    assert!(
        ids.is_empty(),
        "phrases des essais réels engendrées à un nombre ou une commune près : {}",
        ids.join(", ")
    );
}

#[test]
fn the_real_trials_are_read_whole() {
    let all = real_trials();
    assert_eq!(all.len(), 25 + 12 + 8 + 1);
    let find = |id: &str| all.iter().find(|(i, _)| i == id).unwrap().1.clone();
    assert_eq!(
        find("Q11"),
        "Garde les parcelles qui appartiennent à la commune."
    );
    assert!(find("A0").ends_with("et sors-moi la carte en PDF."));
    assert_eq!(find("amorce"), "Décris le projet.");
}

#[test]
fn the_normalization_makes_trivial_variants_collide() {
    assert_eq!(
        normalize("Trouve les écoles à moins de 300 m d'une rivière."),
        normalize("trouve les ecoles a moins de 300 m d une riviere")
    );
    assert_eq!(
        masked("Les gares à moins de 500 m, à Villeneuve-le-Roi."),
        masked("les gares a moins de 1 200 m a Orly")
    );
}
