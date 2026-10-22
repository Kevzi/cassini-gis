//! Score du vrai modèle sur le jeu tenu à l'écart (ticket 5.10) : 40 phrases inédites sur un
//! inventaire de projet d'urbanisme, écrites par un autre auteur que le générateur d'exemples.
//! C'est la barre chiffrée que le modèle ajusté devra atteindre.
//!
//!   CASSINI_TEST_MODEL=<gguf> cargo test -p cassini-ai --release -- --ignored --nocapture ecart_reel
//!
//! Un plan est comparé exactement, sauf le `title` de `layout.export_pdf`, dont la présence
//! suffit, et les nombres sont comparés en valeur (`500.0` vaut `500`). Un plan rendu qui ne
//! passe pas `check_plan` compte comme faux ; l'essai échoue alors, après la ligne `SCORE`.
//! Une question attendue doit rendre une question, un refus attendu un refus. Aucun
//! seuil : la ligne `SCORE` est la mesure. Le texte des phrases n'est jamais imprimé, pour que
//! l'auteur du générateur ne lise pas le jeu.

use std::path::PathBuf;

use cassini_ai::plan::{compose_plan, model_generator, AgentError};
use cassini_core::plan::Plan;
use cassini_core::registry::Registry;
use serde_json::Value;

const INVENTORY: &str = include_str!("tenu_a_l_ecart/inventory.json");
const SENTENCES: &str = include_str!("tenu_a_l_ecart/sentences.json");

/// Ôte le titre de chaque `layout.export_pdf` ; rend `None` si un titre manque ou est vide.
fn without_titles(calls: &Value) -> Option<Value> {
    let mut calls = calls.clone();
    for call in calls.as_array_mut()? {
        if call["command"] == "layout.export_pdf" {
            let params = call.get_mut("params")?.as_object_mut()?;
            let title = params.remove("title")?;
            if title.as_str().is_none_or(|t| t.trim().is_empty()) {
                return None;
            }
        }
    }
    Some(calls)
}

/// Nombres ramenés à une forme unique : un flottant entier devient l'entier (`500.0` → `500`).
fn normalize_numbers(value: Value) -> Value {
    match value {
        Value::Number(n) => match n.as_f64() {
            Some(f) if n.is_f64() && f.fract() == 0.0 && f.abs() < 9.0e15 => Value::from(f as i64),
            _ => Value::Number(n),
        },
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_numbers).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, normalize_numbers(v)))
                .collect(),
        ),
        other => other,
    }
}

/// Forme comparable d'une liste d'appels : sans titres de PDF, nombres normalisés.
fn comparable(calls: &Value) -> Option<Value> {
    without_titles(calls).map(normalize_numbers)
}

/// Paramètres qui désignent une couche existante (ou une étape précédente).
const LAYER_PARAMS: [&str; 3] = ["layer_id", "overlay_layer_id", "points_layer_id"];

/// Sorte attendue d'une phrase : `calls`, `question` ou `refusal`.
fn expected_kind(expected: &Value) -> &str {
    if expected.is_array() {
        "calls"
    } else {
        expected["kind"].as_str().unwrap_or("?")
    }
}

/// Appels attendus d'un plan : la liste elle-même, ou le champ `calls` d'un plan entier
/// `{"kind":"calls","calls":[...]}`. `None` pour une question ou un refus.
fn expected_calls(expected: &Value) -> Option<&Value> {
    if expected.is_array() {
        Some(expected)
    } else if expected["kind"] == "calls" {
        expected.get("calls").filter(|c| c.is_array())
    } else {
        None
    }
}

fn got_kind(got: &Result<Plan, AgentError>) -> &'static str {
    match got {
        Ok(Plan::Calls { .. }) => "calls",
        Ok(Plan::Question { .. }) => "question",
        Ok(Plan::Refusal { .. }) | Err(AgentError::Refused(_)) => "refusal",
        Err(AgentError::PlanInvalid(_)) => "plan_invalid",
        Err(AgentError::Ai(_)) => "ai_error",
    }
}

#[test]
fn the_title_of_the_pdf_only_needs_to_be_present() {
    let want = serde_json::json!([{"command": "layout.export_pdf", "params": {"title": "A"}}]);
    let other = serde_json::json!([{"command": "layout.export_pdf", "params": {"title": "B"}}]);
    let empty = serde_json::json!([{"command": "layout.export_pdf", "params": {"title": " "}}]);
    assert_eq!(without_titles(&want), without_titles(&other));
    assert!(without_titles(&empty).is_none());
    assert_eq!(expected_kind(&want), "calls");
    let whole = serde_json::json!({"kind": "calls", "calls": want});
    assert_eq!(expected_kind(&whole), "calls");
    assert_eq!(expected_calls(&whole), Some(&want));
    assert_eq!(expected_calls(&want), Some(&want));
    assert_eq!(
        expected_calls(&serde_json::json!({"kind": "refusal"})),
        None
    );
    let float = serde_json::json!([{"command": "analysis.buffer",
        "params": {"layer_id": "a", "distance": {"value": 500.0, "unit": "m"}}}]);
    let int = serde_json::json!([{"command": "analysis.buffer",
        "params": {"layer_id": "a", "distance": {"value": 500, "unit": "m"}}}]);
    assert_ne!(float, int);
    assert_eq!(comparable(&float), comparable(&int));
    assert_eq!(
        expected_kind(&serde_json::json!({"kind": "question"})),
        "question"
    );
}

#[test]
fn the_held_out_set_is_well_formed() {
    // Lu sans rien imprimer : forme seulement, et chaque plan attendu passe `check_plan`.
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).expect("inventory.json lisible");
    let layers: Vec<&str> = inventory["layers"]
        .as_array()
        .expect("layers")
        .iter()
        .filter_map(|l| l["layer_id"].as_str())
        .collect();
    let files: Vec<&str> = inventory["unloaded_files"]
        .as_array()
        .map(|f| f.iter().filter_map(|f| f["path"].as_str()).collect())
        .unwrap_or_default();
    let sentences: Vec<Value> = serde_json::from_str(SENTENCES).expect("sentences.json lisible");
    let count = |k: &str| {
        sentences
            .iter()
            .filter(|s| expected_kind(&s["expected"]) == k)
            .count()
    };
    assert_eq!(
        (
            sentences.len(),
            count("calls"),
            count("question"),
            count("refusal")
        ),
        (40, 26, 8, 6)
    );
    for s in &sentences {
        let id = s["id"].as_str().expect("id");
        assert!(s["text"].as_str().is_some_and(|t| !t.is_empty()), "{id}");
        if expected_kind(&s["expected"]) == "calls" {
            let calls = expected_calls(&s["expected"]).expect("appels attendus");
            let plan: Plan =
                serde_json::from_value(serde_json::json!({"kind": "calls", "calls": calls}))
                    .expect("plan attendu lisible");
            assert_eq!(registry.check_plan(&plan), Ok(()), "{id}");
            // Couches et fichiers cités : présents dans l'inventaire (seul l'id est imprimé).
            for call in calls.as_array().expect("liste d'appels") {
                for name in LAYER_PARAMS {
                    if let Some(layer) = call["params"][name].as_str() {
                        assert!(
                            layer.starts_with("step:") || layers.contains(&layer),
                            "{id} : couche citée absente de l'inventaire"
                        );
                    }
                }
                if let Some(path) = call["params"]["path"].as_str() {
                    assert!(
                        files.contains(&path),
                        "{id} : fichier cité absent de l'inventaire"
                    );
                }
            }
        }
    }
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn ecart_reel_score_on_the_held_out_set() {
    let Some(model) = std::env::var_os("CASSINI_TEST_MODEL") else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let sentences: Vec<Value> = serde_json::from_str(SENTENCES).unwrap();
    let mut generator = model_generator(PathBuf::from(model));
    // (justes, total) par sorte attendue.
    let mut score = [("calls", 0, 0), ("question", 0, 0), ("refusal", 0, 0)];
    let mut invalid = Vec::new();
    for sentence in &sentences {
        let id = sentence["id"].as_str().unwrap();
        let text = sentence["text"].as_str().unwrap();
        let expected = &sentence["expected"];
        let want = expected_kind(expected);
        let got = compose_plan(&registry, &inventory, text, &mut generator);
        let ok = match (&got, want) {
            (Ok(Plan::Calls { calls }), "calls") => {
                let calls = serde_json::to_value(calls).unwrap();
                comparable(&calls).is_some()
                    && expected_calls(expected).and_then(comparable) == comparable(&calls)
            }
            (Ok(Plan::Question { .. }), "question") => true,
            (Err(AgentError::Refused(_)), "refusal") => true,
            _ => false,
        };
        // Aucun plan invalide n'est jamais rendu : un `Ok` qui échoue à `check_plan` est faux,
        // et l'essai échoue après la ligne `SCORE`.
        let valid = got
            .as_ref()
            .map_or(true, |plan| registry.check_plan(plan).is_ok());
        let ok = ok && valid;
        if !valid {
            invalid.push(id.to_string());
        }
        let row = score
            .iter_mut()
            .find(|(k, _, _)| *k == want)
            .expect("sorte connue");
        row.1 += usize::from(ok);
        row.2 += 1;
        eprintln!(
            "{} {id} attendu {want}, obtenu {}",
            if ok { "OK  " } else { "FAIL" },
            got_kind(&got)
        );
    }
    let right: usize = score.iter().map(|r| r.1).sum();
    eprintln!(
        "SCORE {right}/{} : plans {}/{}, questions {}/{}, refus {}/{}",
        sentences.len(),
        score[0].1,
        score[0].2,
        score[1].1,
        score[1].2,
        score[2].1,
        score[2].2
    );
    assert!(
        invalid.is_empty(),
        "plans rendus qui échouent à check_plan : {}",
        invalid.join(", ")
    );
}
