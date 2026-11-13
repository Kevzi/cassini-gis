//! Essai avec le vrai modèle : export PDF à une échelle écrite (ticket 5.38) et isochrone depuis
//! un point en UTM (ticket 5.37), ce que le ré-entraînement de 5.39 doit avoir appris.
//! Ignoré sans `CASSINI_TEST_MODEL` (et marqué `ignore` : il charge le modèle une fois par phrase).
//!
//!   CASSINI_TEST_MODEL=<gguf ou déclaration *.modele.json> cargo test -p cassini-ai --release --features cuda -- --ignored echelle_utm_reel
//!
//! L'inventaire de l'épreuve est lu tel quel, jamais modifié. Les phrases sont fabriquées ici, pas
//! dans `tests/epreuve-modele-local/` (gelé), et le garde-fou de recouvrement de `training/` vérifie
//! que le générateur ne les engendre pas. Cinq exports à l'échelle, toutes les formes d'écriture
//! (« 1:25 000 », « 1/25000 », « 25 000e », « 1:50k »), dont un au bout d'une recherche ; trois
//! isochrones en UTM 31N ou 32N. Le plan mesuré est le plan retraduit, sans complétion : ces
//! commandes sont exécutées par QGIS, qui reprojette l'origine d'une isochrone lui-même.
//!
//! Depuis 5.53 (livrable 3, lot 2), la barre est élargie en place : aux 8 phrases de `SENTENCES`
//! s'ajoutent 36 phrases neuves écrites à la main (`echelle_utm_reel/phrases.json`, schéma de
//! `par_commande/phrases.json` sans `command`), soit 44 : exports à une échelle sous toutes ses
//! écritures (page, orientation ou titre compris), exports au bout d'une sélection, d'un tampon ou
//! d'une isochrone, isochrones depuis un point UTM 31N ou 32N, et 4 refus qui nomment dans
//! `missing` ce qui manque au registre : un morceau d'un identifiant planifié du catalogue
//! (`atlas` de `layout.export_atlas`, `shortest_path`, `distance_matrix`) ou, sans identifiant
//! au catalogue, des mots-clés anglais choisis pour un futur identifiant (`export_image`,
//! `image`) : le jour où un identifiant du registre contient un de ces mots, le refus devient
//! faux et le test sans modèle nomme la phrase. Les attendus
//! passent tous les contrôles communs (`common::invalid`). Une seule comparaison pour les 44
//! (`common::same_calls`) : titre d'export comparé exactement quand l'attendu en porte un, sinon
//! libre mais non vide ; entier écrit en flottant ramené à l'entier. Un refus attendu compte
//! juste contre un refus rendu ; une question est un échec. Un seul `SCORE`, sur le total.
//!
//! Seuil : décision du propriétaire du 2026-11-11, au niveau du modèle installé
//! (`ajuste-apres-5-49-b`, 34/44) moins une phrase, soit 33/44 (`NEEDED`). L'objectif
//! proportionnel (ticket 5.39, D4 du 2026-11-11 : 7/8 arrondi vers le bas, 38/44) reste
//! calculé et imprimé, sans faire échouer.

mod common;

use cassini_ai::plan::{compose_plan_with, model_files_generator, AgentError};
use cassini_core::plan::{Plan, QuestionSource};
use cassini_core::registry::Registry;
use serde_json::{json, Value};

const INVENTORY: &str = include_str!("../../tests/epreuve-modele-local/inventory.json");

const PLU: &str = "plu_zonage_a81d5c06_2e9f_4a37_b6c4_5f13e07d92a8";
const ROUTES: &str = "routes_09b3d7f2_6c18_4e5b_a0d9_3e41c8f25b67";

/// Les phrases de l'essai : `(id, texte)`. Le plan attendu de chacune est dans `expected`.
const SENTENCES: [(&str, &str); 8] = [
    // Export à une échelle, la carte seule.
    ("E01", "Exporte la carte au 1:25 000."),
    ("E02", "Sors-moi la carte en PDF au 1/25000."),
    ("E03", "Je voudrais un PDF de la carte au 25 000e."),
    ("E04", "Imprime-moi la carte en PDF au 1:50k."),
    // Export à une échelle au bout d'une recherche.
    (
        "E05",
        "Garde les zones U du PLU et sors la carte en PDF au 1:10 000.",
    ),
    // Isochrones depuis un point en UTM.
    (
        "U01",
        "Calcule la zone atteignable en 10 minutes à pied depuis le point 452 300 / 5 411 800 (UTM 31N).",
    ),
    (
        "U02",
        "Jusqu'où va-t-on en voiture en 15 min depuis X = 612000, Y = 5420500, UTM 31 N ?",
    ),
    (
        "U03",
        "Montre ce qu'on atteint à pied en 20 minutes depuis le point 352 000 ; 5 380 000 en UTM zone 32 Nord.",
    ),
];

/// L'export attendu, sans titre : le titre rendu est libre, mais non vide (`common::same_calls`).
fn export(scale: u32) -> Value {
    json!({"command": "layout.export_pdf", "params": {"scale": scale}})
}

fn isochrone(x: u32, y: u32, epsg: u32, mode: &str, minutes: u32) -> Value {
    json!({"command": "analysis.isochrone", "params": {
        "layer_id": ROUTES, "origin": {"x": x, "y": y, "epsg": epsg},
        "mode": mode, "duration": {"value": minutes, "unit": "min"}}})
}

/// Plan attendu d'une phrase de l'essai, titres d'export retirés.
fn expected(id: &str) -> Value {
    match id {
        "E01" | "E02" | "E03" => json!([export(25000)]),
        "E04" => json!([export(50000)]),
        "E05" => json!([
            {"command": "analysis.select_by_attribute", "params": {
                "layer_id": PLU, "field": "typezone", "operator": "eq", "value": "U"}},
            export(10000)
        ]),
        "U01" => json!([isochrone(452300, 5411800, 32631, "walk", 10)]),
        "U02" => json!([isochrone(612000, 5420500, 32631, "car", 15)]),
        "U03" => json!([isochrone(352000, 5380000, 32632, "walk", 20)]),
        _ => unreachable!("phrase inconnue : {id}"),
    }
}

/// Les phrases neuves de la barre (5.53, livrable 3, lot 2), écrites à la main.
const PHRASES: &str = include_str!("echelle_utm_reel/phrases.json");

/// Les phrases de `SENTENCES`, dans la forme des `phrases.json` : `{id, phrase, expected}`.
fn sentences() -> Vec<Value> {
    SENTENCES
        .iter()
        .map(|(id, text)| {
            json!({"id": id, "phrase": text,
                "expected": {"kind": "calls", "calls": expected(id)}})
        })
        .collect()
}

/// Les phrases neuves de `echelle_utm_reel/phrases.json`.
fn new_phrases() -> Vec<Value> {
    common::read_phrases(PHRASES, "echelle_utm_reel/phrases.json")
}

/// Toute la barre : les 8 phrases de `SENTENCES`, puis les phrases neuves.
fn all_phrases() -> Vec<Value> {
    let mut all = sentences();
    all.extend(new_phrases());
    all
}

/// La réponse rendue vaut-elle l'attendu de la phrase : des appels comparés par
/// `common::same_calls`, ou un refus contre un refus attendu.
fn answers(phrase: &Value, got: &Result<Plan, AgentError>) -> bool {
    let want = &phrase["expected"];
    match (want["kind"].as_str(), got) {
        (Some("calls"), Ok(Plan::Calls { calls })) => common::same_calls(calls, &want["calls"]),
        (Some("refusal"), Err(AgentError::Refused(_))) => true,
        _ => false,
    }
}

/// Seuil exigé : décision du propriétaire du 2026-11-11, au niveau du modèle installé
/// `ajuste-apres-5-49-b` (34/44) moins une phrase. À relever quand un modèle fait mieux.
const NEEDED: usize = 33;

/// Objectif proportionnel, imprimé sans faire échouer : 7/8 du total, arrondi vers le bas (D4).
fn answers_needed(total: usize) -> usize {
    common::proportional(total, 7, 8)
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn echelle_utm_reel_answers_match_at_the_installed_level() {
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
    let (mut right, mut total) = (0, 0);
    for phrase in &all_phrases() {
        let (id, text) = (
            phrase["id"].as_str().unwrap(),
            phrase["phrase"].as_str().unwrap(),
        );
        total += 1;
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
    }
    let goal = answers_needed(total);
    eprintln!(
        "SCORE {right}/{total} réponses attendues (seuil {NEEDED}, objectif 7/8 : {goal}/{total})"
    );
    assert!(
        right >= NEEDED,
        "{right}/{total} : sous le seuil fixé (modèle installé moins une phrase, 2026-11-11) ({NEEDED}/{total})"
    );
}

#[test]
fn the_threshold_is_the_installed_level_and_the_goal_7_of_8_rounded_down() {
    assert_eq!(NEEDED, 33);
    assert_eq!(all_phrases().len(), 44);
    assert_eq!(answers_needed(8), 7);
    assert_eq!(answers_needed(44), 38);
    assert_eq!(answers_needed(40), 35);
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
    let refusals: Vec<&Value> = all
        .iter()
        .filter(|p| p["expected"]["kind"] == "refusal")
        .collect();
    assert!(refusals.len() >= 3, "{} refus", refusals.len());
    for p in refusals {
        assert!(
            p["expected"]["missing"]
                .as_array()
                .is_some_and(|m| !m.is_empty() && m.iter().all(Value::is_string)),
            "{} : refus sans `missing`",
            p["id"]
        );
    }
    // Une question n'est jamais attendue ici : la barre mesure des demandes claires.
    assert!(all.iter().all(|p| p["expected"]["kind"] != "question"));
}

#[test]
fn every_expected_answer_is_valid() {
    // `check_plan`, garde-fou des coordonnées, complétion sans effet, couches, champs et valeurs
    // présents dans l'inventaire de l'épreuve, refus bien écrits et encore justes.
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
    // Une échelle hors des bornes du schéma.
    let s = broken.iter_mut().find(|p| p["id"] == "EU01").unwrap();
    s["expected"]["calls"][0]["params"]["scale"] = json!(10);
    // Un point que la phrase ne donne pas.
    let o = broken.iter_mut().find(|p| p["id"] == "U02").unwrap();
    o["expected"]["calls"][0]["params"]["origin"]["x"] = json!(612001);
    // Une valeur que le champ ne montre pas.
    let v = broken.iter_mut().find(|p| p["id"] == "EU17").unwrap();
    v["expected"]["calls"][0]["params"]["value"] = json!("régionale");
    // Un refus qui dit absente une commande du registre.
    let r = broken.iter_mut().find(|p| p["id"] == "EU36").unwrap();
    r["expected"]["missing"] = json!(["isochrone"]);
    let errors = common::invalid(&registry, &inventory, &broken);
    for (id, reason) in [
        ("EU01", "check_plan refuse"),
        ("U02", "le garde-fou des coordonnées refuse"),
        ("EU17", "valeur absente"),
        ("EU17", "le garde-fou des attributs refuse"),
        ("EU36", "refus devenu faux"),
    ] {
        assert!(
            errors
                .iter()
                .any(|e| e.starts_with(&format!("{id} : {reason}"))),
            "{id} : {errors:?}"
        );
    }
    // EU17 compte deux fois : la valeur absente, et le garde-fou des attributs (5.57).
    assert_eq!(errors.len(), 5, "{errors:?}");
}

#[test]
fn a_refusal_turns_red_when_its_command_reaches_the_registry() {
    // Chaque mot-clé de chaque refus, essayé seul : un identifiant qui le contient fait nommer
    // cette phrase, et elle seule.
    let all = all_phrases();
    let ids = common::registry_ids(&Registry::standard());
    assert!(common::refusals_now_wrong(&ids, &all).is_empty());
    let mut tried = 0;
    for p in all.iter().filter(|p| p["expected"]["kind"] == "refusal") {
        let id = p["id"].as_str().unwrap();
        for word in p["expected"]["missing"].as_array().unwrap() {
            let mut grown = ids.clone();
            grown.push(format!("future.{}", word.as_str().unwrap()));
            let wrong = common::refusals_now_wrong(&grown, &all);
            // Un mot-clé qui en contient un autre (`export_image`, `image`) nomme la phrase deux
            // fois : elle seule compte.
            assert!(!wrong.is_empty(), "{id} {word} : rien de nommé");
            assert!(
                wrong
                    .iter()
                    .all(|w| w.starts_with(&format!("{id} : refus devenu faux"))),
                "{id} {word} : {wrong:?}"
            );
            tried += 1;
        }
    }
    assert!(tried >= 3, "{tried} mots-clés essayés");
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
fn the_score_compares_titles_numbers_and_kinds_as_documented() {
    let all = all_phrases();
    let find = |id: &str| all.iter().find(|p| p["id"] == id).unwrap().clone();
    // Sans titre attendu : libre, mais non vide ; un entier écrit en flottant vaut l'entier.
    let e01 = find("E01");
    assert!(answers(
        &e01,
        &calls(json!([
            {"command": "layout.export_pdf", "params": {"title": "Ma carte", "scale": 25000.0}}]))
    ));
    assert!(!answers(
        &e01,
        &calls(json!([
            {"command": "layout.export_pdf", "params": {"title": " ", "scale": 25000}}]))
    ));
    assert!(!answers(
        &e01,
        &calls(json!([{"command": "layout.export_pdf", "params": {"scale": 25000}}]))
    ));
    assert!(!answers(
        &e01,
        &calls(json!([
            {"command": "layout.export_pdf", "params": {"title": "Carte", "scale": 50000}}]))
    ));
    // Un titre attendu : comparé exactement.
    let eu07 = find("EU07");
    let mut titled = eu07["expected"]["calls"].clone();
    assert!(answers(&eu07, &calls(titled.clone())));
    titled[0]["params"]["title"] = json!("Situation");
    assert!(!answers(&eu07, &calls(titled)));
    // Un refus ne compte que contre un refus attendu ; une question n'est jamais juste ici.
    let refused: Result<Plan, AgentError> = Err(AgentError::Refused("non".into()));
    let asked: Result<Plan, AgentError> = Ok(Plan::Question {
        question: "Quelle zone UTM ?".into(),
        source: QuestionSource::Guard,
    });
    let u01 = find("U01");
    assert!(answers(&find("EU33"), &refused));
    assert!(!answers(&u01, &refused));
    assert!(!answers(&u01, &asked));
    assert!(!answers(&find("EU33"), &asked));
    assert!(answers(&u01, &calls(u01["expected"]["calls"].clone())));
    assert!(!answers(
        &find("EU33"),
        &calls(u01["expected"]["calls"].clone())
    ));
}
