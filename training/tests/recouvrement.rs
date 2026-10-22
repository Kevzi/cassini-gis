//! Garde-fou du jeu tenu à l'écart (ticket 5.10) : aucune de ses phrases ne doit figurer
//! dans le jeu engendré avec la graine par défaut. Deux comparaisons, sur le texte normalisé
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
    let sentences: Vec<Value> = serde_json::from_str(HELD_OUT).expect("sentences.json lisible");
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

/// Identifiants des phrases tenues à l'écart dont la forme `key` figure dans le jeu engendré.
fn overlap(key: fn(&str) -> String) -> Vec<String> {
    let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
    assert_eq!(examples.len(), DEFAULT_COUNT);
    let generated: HashSet<String> = examples.iter().map(|e| key(&e.request)).collect();
    held_out()
        .into_iter()
        .filter(|(_, text)| generated.contains(&key(text)))
        .map(|(id, _)| id)
        .collect()
}

#[test]
fn no_held_out_sentence_appears_in_the_generated_set() {
    let ids = overlap(normalize);
    assert!(
        ids.is_empty(),
        "phrases tenues à l'écart engendrées telles quelles : {}",
        ids.join(", ")
    );
}

#[test]
fn no_held_out_sentence_appears_up_to_a_number_or_a_commune() {
    let ids = overlap(masked);
    assert!(
        ids.is_empty(),
        "phrases tenues à l'écart engendrées à un nombre ou une commune près : {}",
        ids.join(", ")
    );
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
