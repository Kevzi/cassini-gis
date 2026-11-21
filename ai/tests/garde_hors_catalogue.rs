//! Garde-fou hors catalogue (ticket 35) sur les phrases des essais : aucune phrase attendue comme
//! un plan, des appels ou une question n'est refusée par le garde ; chaque refus attendu que le
//! garde vise l'est bien. Hors ligne, sans modèle.

use cassini_core::out_of_catalogue::guard_out_of_catalogue;
use cassini_core::registry::Registry;
use serde_json::Value;

const CORPORA: [&str; 8] = [
    "couverture_reelle",
    "echelle_utm_reel",
    "epic11_reel",
    "par_commande",
    "plan_reel",
    "question_reel",
    "style_reel",
    "terrain_reel",
];

fn phrases(dir: &str) -> Vec<Value> {
    let path = format!("{}/tests/{dir}/phrases.json", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path} : {e}"));
    serde_json::from_str(&text).unwrap()
}

#[test]
fn no_covered_phrase_is_refused() {
    let registry = Registry::standard();
    let mut checked = 0;
    for dir in CORPORA {
        for p in phrases(dir) {
            if p["expected"]["kind"] == "refusal" {
                continue;
            }
            let phrase = p["phrase"].as_str().unwrap();
            assert_eq!(
                guard_out_of_catalogue(&registry, phrase),
                None,
                "{dir}/{} : {phrase}",
                p["id"]
            );
            checked += 1;
        }
    }
    assert!(checked > 300, "{checked}");
}

#[test]
fn the_ticket_refusals_are_refused() {
    let registry = Registry::standard();
    let wanted = ["CV39", "CV40", "CV41", "CV42", "PL22"];
    let mut found = 0;
    for dir in ["couverture_reelle", "plan_reel"] {
        for p in phrases(dir) {
            if wanted.contains(&p["id"].as_str().unwrap()) {
                let phrase = p["phrase"].as_str().unwrap();
                assert!(
                    guard_out_of_catalogue(&registry, phrase).is_some(),
                    "{phrase}"
                );
                found += 1;
            }
        }
    }
    assert_eq!(found, wanted.len());
}
