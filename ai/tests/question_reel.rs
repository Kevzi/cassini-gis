//! Essai avec le vrai modèle : phrases ambiguës, où un mot de la demande ne désigne rien de
//! l'inventaire de l'épreuve. Le modèle doit poser une question au lieu de deviner.
//! Ignoré sans `CASSINI_TEST_MODEL` (et marqué `ignore` : il charge le modèle une fois par phrase).
//!
//!   CASSINI_TEST_MODEL=<gguf ou déclaration *.modele.json> cargo test -p cassini-ai --release --features cuda -- --ignored question_reel
//!
//! L'inventaire de l'épreuve est lu tel quel, jamais modifié. Les phrases sont fabriquées ici, pas
//! dans `tests/epreuve-modele-local/` (gelé). Aucune ne nomme un fichier de l'inventaire non ouvert
//! (écoles, hydrographie, modèle de terrain de la zone nord) : `layer.add` y répondrait
//! légitimement ; un test sans modèle le vérifie.
//!
//! Depuis 5.53 (livrable 3, lot 1), la barre est élargie en place : aux 12 phrases d'`AMBIGUOUS`
//! s'ajoutent 35 phrases neuves écrites à la main (`question_reel/phrases.json`, schéma de
//! `par_commande/phrases.json` sans `command`), soit 47 : 31 questions (couche, champ ou valeur
//! absents, paramètre ou point manquants) et 4 refus, qui nomment dans `missing` ce que le
//! registre n'a pas ; le jour où un identifiant du registre contient un de ces mots, le refus
//! devient faux et le test sans modèle nomme la phrase. Le score compte les **réponses
//! attendues** : une question là où une question est attendue, un refus là où un refus l'est ;
//! un plan, ou une autre sorte de réponse, compte comme un échec. Un seul `SCORE`, sur le total.
//! Seuil (décision du propriétaire, 2026-10-17, rapporté au total le 2026-11-11, D4) : 9/12 des
//! réponses attendues, arrondi vers le bas, soit 35/47. Le jeu d'entraînement ne voit aucune de ces
//! phrases : `training/tests/recouvrement.rs` les garde.

mod common;

use cassini_ai::plan::{compose_plan_with, model_files_generator, AgentError};
use cassini_core::plan::{Plan, QuestionSource};
use cassini_core::registry::Registry;
use serde_json::{json, Value};

const INVENTORY: &str = include_str!("../../tests/epreuve-modele-local/inventory.json");
/// Les phrases neuves de la barre (5.53, livrable 3), écrites à la main.
const PHRASES: &str = include_str!("question_reel/phrases.json");

/// Les phrases ambiguës : la commande existe, mais un mot ne désigne aucune couche ni aucun champ.
const AMBIGUOUS: [(&str, &str); 12] = [
    // Couche absente.
    (
        "Q01",
        "Fais une zone tampon de 200 mètres autour des pharmacies.",
    ),
    (
        "Q02",
        "Trouve les parcelles à moins de 300 m d'un arrêt de bus.",
    ),
    ("Q03", "Garde seulement les hôpitaux."),
    ("Q04", "Croise les parcelles avec les zones inondables."),
    (
        "Q05",
        "Trouve les bâtiments à moins de 50 m d'une ligne à haute tension.",
    ),
    ("Q06", "Décris la couche des parkings."),
    // Champ ou valeur absents.
    ("Q07", "Garde seulement les parcelles chères."),
    ("Q08", "Garde les bâtiments construits avant 1950."),
    ("Q09", "Garde les routes larges."),
    ("Q10", "Garde les communes les plus riches."),
    ("Q11", "Garde les parcelles qui appartiennent à la commune."),
    (
        "Q12",
        "Fais une zone tampon de 100 m autour des gares fermées.",
    ),
];

/// Les phrases neuves de `question_reel/phrases.json`.
fn new_phrases() -> Vec<Value> {
    common::read_phrases(PHRASES, "question_reel/phrases.json")
}

/// Toute la barre, dans la forme des `phrases.json` : les 12 phrases d'`AMBIGUOUS` (une question
/// attendue), puis les phrases neuves.
fn all_phrases() -> Vec<Value> {
    let mut all: Vec<Value> = AMBIGUOUS
        .iter()
        .map(|(id, text)| json!({"id": id, "phrase": text, "expected": {"kind": "question"}}))
        .collect();
    all.extend(new_phrases());
    all
}

/// La réponse rendue est-elle la réponse attendue : une question (du modèle ou du garde-fou,
/// comme avant 5.53) là où une question est attendue, un refus là où un refus l'est. La barre
/// n'attend jamais de plan (test sans modèle) : un plan est toujours un échec.
fn answers(phrase: &Value, got: &Result<Plan, AgentError>) -> bool {
    matches!(
        (phrase["expected"]["kind"].as_str(), got),
        (Some("question"), Ok(Plan::Question { .. }))
            | (Some("refusal"), Err(AgentError::Refused(_)))
    )
}

/// Réponses attendues exigées : 9/12 du total, arrondi vers le bas (D4).
fn answers_needed(total: usize) -> usize {
    common::proportional(total, 9, 12)
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn question_reel_ambiguous_or_out_of_registry_requests_get_the_expected_answer() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    // Comme dans le produit : identifiants courts montrés, plan retraduit (ticket 5.19).
    let full = inventory.clone();
    let (_, ids) = common::shown_inventory(&full);
    let mut generator = model_files_generator(files);
    let all = all_phrases();
    let (mut right, mut questions, mut guarded) = (0, 0, 0);
    for phrase in &all {
        let (id, text) = (
            phrase["id"].as_str().unwrap(),
            phrase["phrase"].as_str().unwrap(),
        );
        let got = common::restored(
            compose_plan_with(
                style,
                &registry,
                &common::view(&full, text),
                text,
                &mut generator,
            ),
            &ids,
        );
        let ok = answers(phrase, &got);
        // Score exact, phrase par phrase : jamais filtré.
        eprintln!(
            "{} {id} {text}
     attendu : {}
     obtenu  : {got:?}",
            if ok { "OK  " } else { "FAIL" },
            phrase["expected"]
        );
        // Aucun plan invalide n'est jamais rendu : tout `Ok` passe `check_plan`.
        if let Ok(plan) = &got {
            assert_eq!(registry.check_plan(plan), Ok(()), "{id}");
        }
        right += usize::from(ok);
        questions += usize::from(matches!(&got, Ok(Plan::Question { .. })));
        guarded += usize::from(matches!(
            &got,
            Ok(Plan::Question {
                source: QuestionSource::Guard,
                ..
            })
        ));
    }
    let total = all.len();
    let need = answers_needed(total);
    eprintln!(
        "SCORE {right}/{total} réponses attendues, {questions} questions posées, dont {guarded} du garde-fou"
    );
    assert!(
        right >= need,
        "{right}/{total} : sous le seuil de 9/12 arrondi vers le bas ({need}/{total})"
    );
}

#[test]
fn the_threshold_is_9_of_12_rounded_down() {
    assert_eq!(answers_needed(12), 9);
    assert_eq!(answers_needed(47), 35);
    assert_eq!(answers_needed(40), 30);
    assert_eq!(answers_needed(50), 37);
}

#[test]
fn the_bar_has_40_to_50_sentences_with_at_least_3_refusals_naming_what_is_missing() {
    let all = all_phrases();
    assert!((40..=50).contains(&all.len()), "{} phrases", all.len());
    let mut ids = std::collections::BTreeSet::new();
    for p in &all {
        let id = p["id"].as_str().expect("phrase sans id");
        assert!(ids.insert(id.to_string()), "id en double : {id}");
        assert!(
            p["phrase"].as_str().is_some_and(|t| !t.trim().is_empty()),
            "{id} : phrase vide"
        );
    }
    let fresh = new_phrases();
    assert!(
        (28..=38).contains(&fresh.len()),
        "{} phrases neuves",
        fresh.len()
    );
    // Une question ou un refus, jamais un plan : `answers` ne sait pas noter un plan.
    for p in &fresh {
        assert!(
            matches!(p["expected"]["kind"].as_str(), Some("question" | "refusal")),
            "{} : attendu {}",
            p["id"],
            p["expected"]
        );
    }
    // Surtout des questions.
    let questions = fresh
        .iter()
        .filter(|p| p["expected"]["kind"] == "question")
        .count();
    assert!(questions * 2 > fresh.len(), "{questions} questions neuves");
    let refusals: Vec<&Value> = fresh
        .iter()
        .filter(|p| p["expected"]["kind"] == "refusal")
        .collect();
    assert!(refusals.len() >= 3, "{} refus neufs", refusals.len());
    for p in refusals {
        assert!(
            p["expected"]["missing"]
                .as_array()
                .is_some_and(|m| !m.is_empty() && m.iter().all(Value::is_string)),
            "{} : refus sans `missing`",
            p["id"]
        );
    }
}

#[test]
fn every_expected_answer_is_valid() {
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let errors = common::invalid(&Registry::standard(), &inventory, &all_phrases());
    assert!(
        errors.is_empty(),
        "attendus invalides :\n{}",
        errors.join("\n")
    );
}

#[test]
fn a_refusal_turns_red_when_its_command_reaches_the_registry() {
    let mut ids = common::registry_ids(&Registry::standard());
    assert!(common::refusals_now_wrong(&ids, &all_phrases()).is_empty());
    ids.push("layer.translate_labels".to_string());
    let wrong = common::refusals_now_wrong(&ids, &all_phrases());
    assert_eq!(wrong.len(), 1, "{wrong:?}");
    assert!(
        wrong[0].starts_with("QR32 : refus devenu faux"),
        "{wrong:?}"
    );
}

#[test]
fn no_sentence_names_a_file_that_is_not_open() {
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    assert!(common::naming_unloaded_files(&inventory, &all_phrases()).is_empty());
    // Le contrôle voit chaque fichier non ouvert.
    let named: Vec<Value> = [
        "Garde les écoles privées.",
        "Croise les parcelles avec l'hydrographie.",
        "Que voit-on sur la Zone-Nord ?",
        "Garde les ÉCOLES privées.",
    ]
    .iter()
    .enumerate()
    .map(|(i, t)| json!({"id": format!("X{i}"), "phrase": t}))
    .collect();
    assert_eq!(
        common::naming_unloaded_files(&inventory, &named),
        vec!["X0", "X1", "X2", "X3"]
    );
}

#[test]
fn a_question_counts_only_for_a_question_and_a_refusal_only_for_a_refusal() {
    let question = json!({"expected": {"kind": "question"}});
    let refusal = json!({"expected": {"kind": "refusal", "missing": ["mail"]}});
    let asked: Result<Plan, AgentError> = Ok(Plan::Question {
        question: "Quelle couche ?".into(),
        source: QuestionSource::Model,
    });
    let guarded: Result<Plan, AgentError> = Ok(Plan::Question {
        question: "Quel système ?".into(),
        source: QuestionSource::Guard,
    });
    let refused: Result<Plan, AgentError> = Err(AgentError::Refused("non".into()));
    let planned: Result<Plan, AgentError> = Ok(serde_json::from_value(json!({"kind": "calls",
        "calls": [{"command": "project.inventory", "params": {}}]}))
    .unwrap());
    assert!(answers(&question, &asked));
    assert!(answers(&question, &guarded));
    assert!(!answers(&question, &refused));
    assert!(!answers(&question, &planned));
    assert!(answers(&refusal, &refused));
    assert!(!answers(&refusal, &asked));
    assert!(!answers(&refusal, &planned));
}
