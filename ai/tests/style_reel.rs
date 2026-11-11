//! Essai avec le vrai modèle : les deux commandes de style (`map.topographic_style`,
//! `map.graduated_style`, ticket 11.20), que le ré-entraînement groupé de 11.23 doit avoir
//! apprises. Ignoré sans `CASSINI_TEST_MODEL` (et marqué `ignore` : il charge le modèle une fois
//! par phrase).
//!
//!   CASSINI_TEST_MODEL=<gguf ou déclaration *.modele.json> cargo test -p cassini-ai --release --features cuda -- --ignored style_reel
//!
//! L'inventaire de l'épreuve est lu tel quel, jamais modifié. Les dix phrases sont nouvelles :
//! écrites ici, jamais par le générateur, et le garde-fou de recouvrement de `training/` vérifie
//! qu'il ne les engendre pas. Quatre mises en forme topographiques (toutes les couches, ou les
//! couches nommées dans l'ordre écrit), cinq dégradés (le champ numérique, et seulement les
//! options écrites : méthode, nombre de classes, dégradé), une question quand la couche n'a pas
//! de champ numérique pour le critère demandé. Comme ailleurs, le plan mesuré est le plan complété
//! (`common::completed`), qui ne change rien ici.
//!
//! Depuis 5.53 (livrable 3, lot 3), la barre est élargie en place : aux 10 phrases de
//! `SENTENCES` s'ajoutent 32 phrases neuves écrites à la main (`style_reel/phrases.json`, schéma
//! de `par_commande/phrases.json` sans `command`), soit 42 : 9 mises en forme topographiques,
//! 16 dégradés (carte choroplèthe, seuils naturels, effectifs égaux, amplitudes égales, dégradés
//! nommés), 3 questions (champ ou couche absents) et 4 refus (catégories par valeur, étiquettes,
//! transparence, symboles proportionnels), qui nomment dans `missing` ce que le registre n'a pas ;
//! le jour où un identifiant du registre contient un de ces mots, le refus devient faux et le
//! test sans modèle nomme la phrase. Les attendus passent tous les contrôles communs
//! (`common::invalid`) ; tout dégradé porte un champ numérique de sa couche. Les 10 phrases de
//! `SENTENCES` gardent la comparaison exacte ; les neuves celle des autres barres
//! (`common::same_calls`). Un seul `SCORE`, sur le total. Seuil : décision du propriétaire du
//! 2026-11-11, au niveau du modèle installé (`ajuste-apres-5-49-b`, 31/42) moins une phrase,
//! soit 30/42 (`NEEDED`) ; l'objectif proportionnel (ticket 11.23, D4 du 2026-11-11 : 9/10
//! arrondi vers le bas, 37/42) reste calculé et imprimé, sans faire échouer. Le jeu
//! d'entraînement ne voit aucune de ces phrases : `training/tests/recouvrement.rs` les garde.

mod common;

use cassini_ai::plan::{compose_plan_with, model_files_generator, AgentError};
use cassini_core::plan::{Plan, QuestionSource};
use cassini_core::point::guard_coordinates;
use cassini_core::registry::Registry;
use serde_json::{json, Value};

const INVENTORY: &str = include_str!("../../tests/epreuve-modele-local/inventory.json");

const PARCELLES: &str = "parcelles_3f9c2a1e_7b40_4d5a_9e12_0c8a6d4b71f3";
const PLU: &str = "plu_zonage_a81d5c06_2e9f_4a37_b6c4_5f13e07d92a8";
const GARES: &str = "gares_c5e0b94d_1a67_4f82_8d03_9b2e6a7c40f1";
const ROUTES: &str = "routes_09b3d7f2_6c18_4e5b_a0d9_3e41c8f25b67";
const BATIMENTS: &str = "batiments_d27a60e8_4b95_4c31_9f7e_8a05d1b3c649";
const COMMUNES: &str = "communes_6e1f83a5_9d20_4b74_a3c8_1f5e0b7d2946";

/// Les phrases de l'essai : `(id, texte)`. La réponse attendue de chacune est dans `expected`.
const SENTENCES: [(&str, &str); 10] = [
    // map.topographic_style : toutes les couches.
    (
        "ST01",
        "Donne au projet l'allure d'une carte topographique, s'il te plaît.",
    ),
    (
        "ST02",
        "Je voudrais que la carte ressemble à une carte IGN : applique la sémiologie officielle à toutes les couches.",
    ),
    // map.topographic_style : les couches nommées, dans l'ordre écrit.
    (
        "ST03",
        "Mets uniquement les routes et les gares aux couleurs de la carte topographique.",
    ),
    ("ST04", "Habille le zonage du PLU selon le standard du CNIG."),
    // map.graduated_style : le champ, et seulement les options écrites.
    (
        "ST05",
        "Fais varier la couleur des communes avec leur nombre d'habitants.",
    ),
    (
        "ST06",
        "Range les bâtiments en 6 classes de hauteur et colore-les en dégradé.",
    ),
    (
        "ST07",
        "Dégradé de verts sur les parcelles, d'après leur surface, en quantiles.",
    ),
    (
        "ST08",
        "Les routes colorées selon la vitesse autorisée, à intervalles égaux, en bleus.",
    ),
    (
        "ST09",
        "Montre le nombre d'étages des bâtiments par un dégradé de violets.",
    ),
    // Aucun champ numérique ne porte le critère : une question.
    ("ST10", "Colore les gares selon leur fréquentation."),
];

/// Ce qu'une phrase de l'essai attend.
#[derive(Debug, Clone, PartialEq)]
enum Want {
    /// Ces appels, à l'identique.
    Plan(Value),
    /// Une question.
    Question,
}

fn topographic(params: Value) -> Value {
    json!([{"command": "map.topographic_style", "params": params}])
}

fn graduated(params: Value) -> Value {
    json!([{"command": "map.graduated_style", "params": params}])
}

/// La réponse attendue d'une phrase de l'essai.
fn expected(id: &str) -> Want {
    Want::Plan(match id {
        "ST01" | "ST02" => topographic(json!({})),
        "ST03" => topographic(json!({"layers": [ROUTES, GARES]})),
        "ST04" => topographic(json!({"layers": [PLU]})),
        "ST05" => graduated(json!({"layer_id": COMMUNES, "field": "population"})),
        "ST06" => graduated(json!({"layer_id": BATIMENTS, "field": "hauteur_m", "class_count": 6})),
        "ST07" => graduated(json!({"layer_id": PARCELLES, "field": "surface_m2",
            "method": "quantile", "ramp": "greens"})),
        "ST08" => graduated(json!({"layer_id": ROUTES, "field": "vitesse_kmh",
            "method": "equal_interval", "ramp": "blues"})),
        "ST09" => {
            graduated(json!({"layer_id": BATIMENTS, "field": "nb_etages", "ramp": "purples"}))
        }
        "ST10" => return Want::Question,
        _ => unreachable!("phrase inconnue : {id}"),
    })
}

/// Les phrases neuves de la barre (5.53, livrable 3), écrites à la main.
const PHRASES: &str = include_str!("style_reel/phrases.json");

/// Les phrases neuves de `style_reel/phrases.json`.
fn new_phrases() -> Vec<Value> {
    common::read_phrases(PHRASES, "style_reel/phrases.json")
}

/// Toute la barre, dans la forme des `phrases.json` : les 10 phrases de `SENTENCES`, l'attendu
/// tiré de `expected` (`exact` : comparaison à l'identique, règle d'avant 5.53), puis les
/// phrases neuves.
fn all_phrases() -> Vec<Value> {
    let mut all: Vec<Value> = SENTENCES
        .iter()
        .map(|(id, text)| {
            let expected = match expected(id) {
                Want::Plan(calls) => json!({"kind": "calls", "calls": calls}),
                Want::Question => json!({"kind": "question"}),
            };
            json!({"id": id, "phrase": text, "expected": expected, "exact": true})
        })
        .collect();
    all.extend(new_phrases());
    all
}

/// La réponse rendue est-elle la réponse attendue : les appels à l'identique pour `SENTENCES`,
/// par `common::same_calls` pour les neuves ; une question (du modèle ou du garde-fou) là où une
/// question est attendue ; un refus là où un refus l'est.
fn answers(phrase: &Value, got: &Result<Plan, AgentError>) -> bool {
    let want = &phrase["expected"];
    match (want["kind"].as_str(), got) {
        (Some("calls"), Ok(Plan::Calls { calls })) if phrase["exact"] == true => {
            serde_json::to_value(calls).unwrap() == want["calls"]
        }
        (Some("calls"), Ok(Plan::Calls { calls })) => common::same_calls(calls, &want["calls"]),
        (Some("question"), Ok(Plan::Question { .. })) => true,
        (Some("refusal"), Err(AgentError::Refused(_))) => true,
        _ => false,
    }
}

/// Seuil exigé : décision du propriétaire du 2026-11-11, au niveau du modèle installé
/// `ajuste-apres-5-49-b` (31/42) moins une phrase. À relever quand un modèle fait mieux.
const NEEDED: usize = 30;

/// Objectif proportionnel, imprimé sans faire échouer : 9/10 du total, arrondi vers le bas (D4).
fn answers_needed(total: usize) -> usize {
    common::proportional(total, 9, 10)
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn style_reel_answers_match_at_the_installed_level_on_the_whole_bar() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    let full: Value = serde_json::from_str(INVENTORY).unwrap();
    // Comme dans le produit : identifiants courts montrés, plan retraduit (ticket 5.19), puis
    // complété (11.22).
    let (_, ids) = common::shown_inventory(&full);
    let mut generator = model_files_generator(files);
    let all = all_phrases();
    let (mut right, mut guarded) = (0, 0);
    for phrase in &all {
        let (id, text) = (
            phrase["id"].as_str().unwrap(),
            phrase["phrase"].as_str().unwrap(),
        );
        let got = common::completed(
            &registry,
            common::restored(
                compose_plan_with(
                    style,
                    &registry,
                    &common::view(&full, text),
                    text,
                    &mut generator,
                ),
                &ids,
            ),
            &full,
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
        guarded += usize::from(matches!(
            &got,
            Ok(Plan::Question {
                source: QuestionSource::Guard,
                ..
            })
        ));
    }
    let total = all.len();
    let goal = answers_needed(total);
    eprintln!(
        "SCORE {right}/{total} réponses conformes, dont {guarded} question(s) du garde-fou (seuil {NEEDED}, objectif 9/10 : {goal}/{total})"
    );
    assert!(
        right >= NEEDED,
        "{right}/{total} : sous le seuil fixé (modèle installé moins une phrase, 2026-11-11) ({NEEDED}/{total})"
    );
}

/// L'attendu de `id`, en plan.
fn expected_plan(id: &str) -> Option<Plan> {
    match expected(id) {
        Want::Plan(calls) => {
            Some(serde_json::from_value(json!({"kind": "calls", "calls": calls})).unwrap())
        }
        Want::Question => None,
    }
}

#[test]
fn every_expected_plan_is_valid_passes_the_guard_and_the_completion() {
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    for (id, text) in SENTENCES {
        let Some(plan) = expected_plan(id) else {
            continue;
        };
        assert_eq!(registry.check_plan(&plan), Ok(()), "{id}");
        assert_eq!(
            guard_coordinates(&registry, plan.clone(), text),
            plan,
            "{id}"
        );
        assert_eq!(
            common::complete_with(&registry, plan.clone(), &inventory),
            plan,
            "{id}"
        );
    }
}

/// La couche `id` de l'inventaire de l'épreuve.
fn layer(inventory: &Value, id: &str) -> Value {
    inventory["layers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["layer_id"] == id)
        .unwrap_or_else(|| panic!("couche absente : {id}"))
        .clone()
}

#[test]
fn every_graduated_field_is_a_numeric_field_of_its_layer() {
    // Toute la barre, phrases neuves comprises : un dégradé porte toujours un champ numérique de
    // sa couche, une mise en forme topographique des couches de l'inventaire.
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let mut graduated = 0;
    for phrase in all_phrases() {
        let id = phrase["id"].as_str().unwrap();
        let Some(calls) = common::expected_calls(&phrase) else {
            continue;
        };
        for call in calls {
            let params = &call["params"];
            if call["command"] == "map.topographic_style" {
                for l in params["layers"].as_array().into_iter().flatten() {
                    layer(&inventory, l.as_str().unwrap());
                }
                continue;
            }
            assert_eq!(call["command"], "map.graduated_style", "{id}");
            graduated += 1;
            let l = layer(&inventory, params["layer_id"].as_str().unwrap());
            let field = l["fields"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["name"] == params["field"])
                .unwrap_or_else(|| panic!("{id} : champ absent"));
            assert!(
                ["integer", "number"].contains(&field["type"].as_str().unwrap()),
                "{id}"
            );
        }
    }
    assert_eq!(graduated, 5 + 16);
}

#[test]
fn the_question_names_a_layer_without_a_numeric_field() {
    // ST10 : les gares n'ont aucun champ numérique, la fréquentation n'est pas dans l'inventaire.
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let gares = layer(&inventory, GARES);
    assert!(gares["fields"]
        .as_array()
        .unwrap()
        .iter()
        .all(|f| f["type"] == "string"));
    assert_eq!(expected("ST10"), Want::Question);
}

#[test]
fn the_sentences_split_into_four_topographic_five_graduated_and_one_question() {
    let mut counts = [0usize; 3];
    for (id, _) in SENTENCES {
        match expected(id) {
            Want::Plan(calls) if calls[0]["command"] == "map.topographic_style" => counts[0] += 1,
            Want::Plan(_) => counts[1] += 1,
            Want::Question => counts[2] += 1,
        }
    }
    assert_eq!(counts, [4, 5, 1]);
}

#[test]
fn the_threshold_is_the_installed_level_and_the_goal_9_of_10_rounded_down() {
    assert_eq!(NEEDED, 30);
    assert_eq!(answers_needed(10), 9);
    assert_eq!(all_phrases().len(), 42);
    assert_eq!(answers_needed(42), 37);
    assert_eq!(answers_needed(40), 36);
    assert_eq!(answers_needed(50), 45);
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
        (30..=40).contains(&fresh.len()),
        "{} phrases neuves",
        fresh.len()
    );
    let mut counts = [0usize; 4];
    for p in &fresh {
        match p["expected"]["kind"].as_str() {
            Some("calls") if p["expected"]["calls"][0]["command"] == "map.topographic_style" => {
                counts[0] += 1
            }
            Some("calls") => counts[1] += 1,
            Some("question") => counts[2] += 1,
            Some("refusal") => counts[3] += 1,
            other => panic!("{} : sorte {other:?}", p["id"]),
        }
    }
    // Mises en forme topographiques, dégradés, questions, refus.
    assert_eq!(counts, [9, 16, 3, 4]);
    for p in fresh.iter().filter(|p| p["expected"]["kind"] == "refusal") {
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
    // `check_plan`, garde-fou des coordonnées, complétion sans effet, couches et champs présents
    // dans l'inventaire de l'épreuve, refus encore justes.
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let errors = common::invalid(&Registry::standard(), &inventory, &all_phrases());
    assert!(
        errors.is_empty(),
        "attendus invalides :\n{}",
        errors.join("\n")
    );
}

#[test]
fn an_invalid_expected_answer_or_a_wrong_refusal_is_named() {
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let mut broken = all_phrases();
    let f = broken.iter_mut().find(|p| p["id"] == "SR02").unwrap();
    f["expected"]["calls"][0]["params"]["field"] = json!("superficie");
    let m = broken.iter_mut().find(|p| p["id"] == "SR04").unwrap();
    m["expected"]["calls"][0]["params"]["method"] = json!("pretty");
    let l = broken.iter_mut().find(|p| p["id"] == "SR20").unwrap();
    l["expected"]["calls"][0]["params"]["layers"] = json!(["forets_0000"]);
    // Un refus qui dit absente une commande du registre.
    let r = broken.iter_mut().find(|p| p["id"] == "SR29").unwrap();
    r["expected"]["missing"] = json!(["graduated"]);
    let errors = common::invalid(&registry, &inventory, &broken);
    for (id, reason) in [
        ("SR02", "champ absent"),
        ("SR04", "check_plan refuse"),
        ("SR20", "couche absente"),
        ("SR29", "refus devenu faux"),
    ] {
        assert!(
            errors
                .iter()
                .any(|e| e.starts_with(&format!("{id} : {reason}"))),
            "{id} : {errors:?}"
        );
    }
    assert_eq!(errors.len(), 4, "{errors:?}");
}

#[test]
fn a_refusal_turns_red_when_its_command_reaches_the_registry() {
    let mut ids = common::registry_ids(&Registry::standard());
    assert!(common::refusals_now_wrong(&ids, &all_phrases()).is_empty());
    ids.push("map.labels".to_string());
    let wrong = common::refusals_now_wrong(&ids, &all_phrases());
    assert_eq!(wrong.len(), 1, "{wrong:?}");
    assert!(
        wrong[0].starts_with("SR30 : refus devenu faux"),
        "{wrong:?}"
    );
}

#[test]
fn no_sentence_names_a_file_that_is_not_open() {
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    assert!(common::naming_unloaded_files(&inventory, &all_phrases()).is_empty());
}

/// Un plan rendu, depuis ses appels.
fn calls(value: Value) -> Result<Plan, AgentError> {
    Ok(serde_json::from_value(json!({"kind": "calls", "calls": value})).unwrap())
}

#[test]
fn the_score_compares_plans_questions_and_refusals_as_documented() {
    let all = all_phrases();
    let find = |id: &str| all.iter().find(|p| p["id"] == id).unwrap().clone();
    let asked: Result<Plan, AgentError> = Ok(Plan::Question {
        question: "Quel champ ?".into(),
        source: QuestionSource::Model,
    });
    let guarded: Result<Plan, AgentError> = Ok(Plan::Question {
        question: "Quel système ?".into(),
        source: QuestionSource::Guard,
    });
    let refused: Result<Plan, AgentError> = Err(AgentError::Refused("non".into()));
    // Une phrase de `SENTENCES` : à l'identique ; une option de plus est un échec.
    let st06 = find("ST06");
    let mut right = st06["expected"]["calls"].clone();
    assert!(answers(&st06, &calls(right.clone())));
    right[0]["params"]["ramp"] = json!("greys");
    assert!(!answers(&st06, &calls(right)));
    // Un entier écrit en flottant : faux sur une phrase d'origine (comparaison exacte), juste sur
    // une neuve (ci-dessous, SR04).
    let mut float = st06["expected"]["calls"].clone();
    float[0]["params"]["class_count"] = json!(6.0);
    assert!(!answers(&st06, &calls(float)));
    // Une phrase neuve : un entier écrit en flottant vaut l'entier ; une option de plus ou
    // l'ordre des couches changé sont des échecs.
    let sr04 = find("SR04");
    let mut floats = sr04["expected"]["calls"].clone();
    floats[0]["params"]["class_count"] = json!(5.0);
    assert!(answers(&sr04, &calls(floats.clone())));
    floats[0]["params"]["ramp"] = json!("reds");
    assert!(!answers(&sr04, &calls(floats)));
    let sr21 = find("SR21");
    let mut swapped = sr21["expected"]["calls"].clone();
    assert!(answers(&sr21, &calls(swapped.clone())));
    swapped[0]["params"]["layers"] = json!([ROUTES, COMMUNES]);
    assert!(!answers(&sr21, &calls(swapped)));
    // Une question compte pour une question, du modèle ou du garde-fou ; un refus pour un refus.
    assert!(answers(&find("ST10"), &asked));
    assert!(answers(&find("SR26"), &guarded));
    assert!(!answers(&find("SR26"), &refused));
    assert!(answers(&find("SR29"), &refused));
    assert!(!answers(&find("SR29"), &asked));
    assert!(!answers(&sr04, &asked));
    assert!(!answers(&sr04, &refused));
    assert!(!answers(
        &find("SR29"),
        &calls(sr04["expected"]["calls"].clone())
    ));
}

#[test]
fn the_new_sentences_carry_neither_command_nor_exact() {
    // `exact` réservé aux 10 phrases de `SENTENCES` ; pas de `command` dans cette barre.
    for p in new_phrases() {
        assert!(p.get("command").is_none(), "{} : `command`", p["id"]);
        assert!(p.get("exact").is_none(), "{} : `exact`", p["id"]);
    }
    let exact: Vec<Value> = all_phrases()
        .into_iter()
        .filter(|p| p["exact"] == true)
        .map(|p| p["id"].clone())
        .collect();
    let originals: Vec<Value> = SENTENCES.iter().map(|(id, _)| json!(id)).collect();
    assert_eq!(exact, originals);
}
