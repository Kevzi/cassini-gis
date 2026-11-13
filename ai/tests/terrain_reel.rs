//! Essai avec le vrai modèle : les trois commandes de terrain (ticket 4.6) et l'export à une
//! échelle sans le mot « PDF » (ticket 5.40), ce que le ré-entraînement groupé de 5.41 doit avoir
//! appris. Ignoré sans `CASSINI_TEST_MODEL` (et marqué `ignore` : il charge le modèle une fois par
//! phrase).
//!
//!   CASSINI_TEST_MODEL=<gguf ou déclaration *.modele.json> cargo test -p cassini-ai --release --features cuda -- --ignored terrain_reel
//!
//! L'inventaire de l'épreuve est lu tel quel, jamais modifié : son MNT est en Lambert-93 et se
//! désigne par sa `source`. Les phrases sont fabriquées ici, pas dans `tests/epreuve-modele-local/`
//! (gelé), et le garde-fou de recouvrement de `training/` vérifie que le générateur ne les engendre
//! pas. Deux pentes, deux lignes de vue (le système écrit à chaque point, comme le garde-fou des
//! coordonnées le demande), deux champs de vision, deux exports à une échelle sans « PDF ».
//! Depuis 11.23, le plan mesuré est le plan complété par les conversions de points, comme
//! `plan.propose` le montre (11.22, `common::completed`) ; les points de ces phrases sont dans le
//! système du MNT, la complétion n'y ajoute rien.
//!
//! Depuis 5.53 (livrable 3, lot 2), la barre est élargie en place : aux 8 phrases de `SENTENCES`
//! s'ajoutent 36 phrases neuves écrites à la main (`terrain_reel/phrases.json`, schéma de
//! `par_commande/phrases.json` sans `command`), soit 44 : pentes en degrés ou en pourcentage,
//! lignes de vue et champs de vision (points en Lambert-93, hauteurs, rayon, courbure ou
//! réfraction écrits, jamais égaux au défaut), exports à une échelle sans « PDF », dont après une
//! étape de terrain, et 4 refus qui nomment dans `missing` ce que le registre n'a pas : un
//! morceau d'un identifiant réservé du catalogue (`flood` de `terrain.flood_extent`, `flow_accum`
//! de `terrain.flow_accumulation`) ou, sans identifiant au catalogue, des mots-clés anglais
//! choisis pour un futur identifiant (`cut_fill`, `volume` ; `solar`, `insolation`) : le jour où
//! un identifiant du registre contient un de ces mots, le refus devient faux et le test sans
//! modèle nomme la phrase. Les attendus passent tous
//! les contrôles communs (`common::invalid`). Une seule comparaison pour les 44
//! (`common::same_calls`) : titre d'export comparé exactement quand l'attendu en porte un, sinon
//! libre mais non vide ; entier écrit en flottant ramené à l'entier. Un refus attendu compte
//! juste contre un refus rendu ; une question est un échec. Un seul `SCORE`, sur le total.
//!
//! Seuil : décision du propriétaire du 2026-11-11, au niveau du modèle installé
//! (`ajuste-apres-5-49-b`, 33/44) moins une phrase, soit 32/44 (`NEEDED`). L'objectif
//! proportionnel (ticket 5.41, D4 du 2026-11-11 : 7/8 arrondi vers le bas, 38/44) reste
//! calculé et imprimé, sans faire échouer.

mod common;

use cassini_ai::plan::{compose_plan_with, model_files_generator, AgentError};
use cassini_core::plan::{Plan, QuestionSource};
use cassini_core::registry::Registry;
use serde_json::{json, Value};

const INVENTORY: &str = include_str!("../../tests/epreuve-modele-local/inventory.json");

/// Le MNT de l'inventaire de l'épreuve, désigné par sa `source` (forme apprise en 4.6).
const MNT: &str = "donnees/mnt_5m.tif";

/// Les phrases de l'essai : `(id, texte)`. Le plan attendu de chacune est dans `expected`.
const SENTENCES: [(&str, &str); 8] = [
    // Pente, l'unité écrite.
    ("T01", "Quelle est la pente du terrain, en degrés ?"),
    (
        "T02",
        "J'aimerais connaître la déclivité du MNT, exprimée en pourcentage.",
    ),
    // Ligne de vue, le système écrit à chaque point.
    (
        "L01",
        "Depuis le point 656 500 / 6 847 800 (Lambert-93), voit-on le point 657 300 / 6 848 400 (Lambert-93) ?",
    ),
    (
        "L02",
        "Ligne de vue entre X = 655900, Y = 6847200 en Lambert-93 et X = 656800, Y = 6848100 en Lambert-93, œil à 1,8 m.",
    ),
    // Champ de vision.
    (
        "V01",
        "Que voit-on depuis le point 656 800 / 6 848 300 (Lambert-93), à 10 m au-dessus du sol ?",
    ),
    (
        "V02",
        "Montre-moi ce qui est visible dans un rayon de 800 m autour du point X = 657000, Y = 6847600 en Lambert-93.",
    ),
    // Export à une échelle, sans le mot « PDF ».
    ("X01", "Mets la carte en page au 1:5 000."),
    ("X02", "Je voudrais une impression de la carte au 10 000e."),
];

fn point(x: u32, y: u32) -> Value {
    json!({"x": x, "y": y, "epsg": 2154})
}

/// L'export attendu, sans titre : le titre rendu est libre, mais non vide (`common::same_calls`).
fn export(scale: u32) -> Value {
    json!({"command": "layout.export_pdf", "params": {"scale": scale}})
}

/// Plan attendu d'une phrase de l'essai, titres d'export retirés.
fn expected(id: &str) -> Value {
    match id {
        "T01" => json!([{"command": "terrain.slope", "params": {"dem": MNT, "unit": "degrees"}}]),
        "T02" => json!([{"command": "terrain.slope", "params": {"dem": MNT, "unit": "percent"}}]),
        "L01" => json!([{"command": "terrain.line_of_sight", "params": {
            "dem": MNT, "observer": point(656500, 6847800), "target": point(657300, 6848400)}}]),
        "L02" => json!([{"command": "terrain.line_of_sight", "params": {
            "dem": MNT, "observer": point(655900, 6847200), "target": point(656800, 6848100),
            "observer_height": {"value": 1.8, "unit": "m"}}}]),
        "V01" => json!([{"command": "terrain.viewshed", "params": {
            "dem": MNT, "observer": point(656800, 6848300),
            "observer_height": {"value": 10, "unit": "m"}}}]),
        "V02" => json!([{"command": "terrain.viewshed", "params": {
            "dem": MNT, "observer": point(657000, 6847600),
            "radius": {"value": 800, "unit": "m"}}}]),
        "X01" => json!([export(5000)]),
        "X02" => json!([export(10000)]),
        _ => unreachable!("phrase inconnue : {id}"),
    }
}

/// Les phrases neuves de la barre (5.53, livrable 3, lot 2), écrites à la main.
const PHRASES: &str = include_str!("terrain_reel/phrases.json");

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

/// Les phrases neuves de `terrain_reel/phrases.json`.
fn new_phrases() -> Vec<Value> {
    common::read_phrases(PHRASES, "terrain_reel/phrases.json")
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
/// `ajuste-apres-5-49-b` (33/44) moins une phrase. À relever quand un modèle fait mieux.
const NEEDED: usize = 32;

/// Objectif proportionnel, imprimé sans faire échouer : 7/8 du total, arrondi vers le bas (D4).
fn answers_needed(total: usize) -> usize {
    common::proportional(total, 7, 8)
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn terrain_reel_answers_match_at_the_installed_level() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    let full: Value = serde_json::from_str(INVENTORY).unwrap();
    // Comme dans le produit : identifiants courts montrés, plan retraduit (ticket 5.19), puis
    // complété par les conversions de points (11.22, mesuré depuis 11.23).
    let (_, ids) = common::shown_inventory(&full);
    let mut generator = model_files_generator(files);
    let (mut right, mut total) = (0, 0);
    for phrase in &all_phrases() {
        let (id, text) = (
            phrase["id"].as_str().unwrap(),
            phrase["phrase"].as_str().unwrap(),
        );
        total += 1;
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
    assert_eq!(NEEDED, 32);
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
    // `check_plan`, garde-fou des coordonnées, complétion sans effet (points dans le système du
    // MNT), raster présent dans l'inventaire de l'épreuve, refus bien écrits et encore justes.
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
    // Une unité que le schéma ne connaît pas.
    let u = broken.iter_mut().find(|p| p["id"] == "TR01").unwrap();
    u["expected"]["calls"][0]["params"]["unit"] = json!("radians");
    // Un point que la phrase ne donne pas.
    let o = broken.iter_mut().find(|p| p["id"] == "L01").unwrap();
    o["expected"]["calls"][0]["params"]["target"]["y"] = json!(6848401);
    // Un système que la phrase n'écrit pas : le garde-fou refuse (et, ce système n'étant pas
    // celui du MNT, la complétion ajouterait aussi une conversion : deux erreurs).
    let c = broken.iter_mut().find(|p| p["id"] == "TR17").unwrap();
    c["expected"]["calls"][0]["params"]["observer"]["epsg"] = json!(4326);
    // Un MNT que l'inventaire n'a pas.
    let d = broken.iter_mut().find(|p| p["id"] == "TR02").unwrap();
    d["expected"]["calls"][0]["params"]["dem"] = json!("donnees/autre.tif");
    // Un refus qui dit absente une commande du registre.
    let r = broken.iter_mut().find(|p| p["id"] == "TR36").unwrap();
    r["expected"]["missing"] = json!(["hillshade"]);
    // Un point écrit en WGS 84 (garde-fou satisfait) sur le MNT en Lambert-93 : la complétion
    // ajoute une conversion, l'attendu sans elle est faux.
    broken.push(
        json!({"id": "TRW", "phrase": "Bassin visuel depuis le point 2,41 / 48,73 en WGS 84.",
        "expected": {"kind": "calls", "calls": [{"command": "terrain.viewshed", "params": {
            "dem": MNT, "observer": {"x": 2.41, "y": 48.73, "epsg": 4326}}}]}}),
    );
    let errors = common::invalid(&registry, &inventory, &broken);
    for (id, reason) in [
        ("TRW", "la complétion ajoute une conversion"),
        ("TR01", "check_plan refuse"),
        ("L01", "le garde-fou des coordonnées refuse"),
        ("TR17", "le garde-fou des coordonnées refuse"),
        ("TR02", "raster absent"),
        ("TR36", "refus devenu faux"),
    ] {
        assert!(
            errors
                .iter()
                .any(|e| e.starts_with(&format!("{id} : {reason}"))),
            "{id} : {errors:?}"
        );
    }
    // Une erreur par cas, deux pour TR17.
    assert_eq!(errors.len(), 7, "{errors:?}");
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
fn the_dem_is_the_source_of_the_inventory_raster() {
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let rasters: Vec<&Value> = inventory["layers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["kind"] == "raster")
        .collect();
    assert_eq!(rasters.len(), 1);
    assert_eq!(rasters[0]["source"], MNT);
    assert_eq!(rasters[0]["epsg"], 2154);
}

#[test]
fn no_export_sentence_names_pdf() {
    // Les exports de cette barre se demandent sans « PDF » : repérés par la commande attendue,
    // pas par l'identifiant.
    let mut exports = 0;
    for p in &all_phrases() {
        let exports_a_map = common::expected_calls(p)
            .is_some_and(|calls| calls.iter().any(|c| c["command"] == "layout.export_pdf"));
        if exports_a_map {
            exports += 1;
            let text = p["phrase"].as_str().unwrap().to_lowercase();
            assert!(!text.contains("pdf"), "{}", p["id"]);
        }
    }
    assert_eq!(exports, 2 + 9, "exports de la barre");
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
    // Sans titre attendu : libre, mais non vide.
    let x01 = find("X01");
    assert!(answers(
        &x01,
        &calls(json!([
            {"command": "layout.export_pdf", "params": {"title": "Ma carte", "scale": 5000}}]))
    ));
    assert!(!answers(
        &x01,
        &calls(json!([{"command": "layout.export_pdf", "params": {"scale": 5000}}]))
    ));
    assert!(!answers(
        &x01,
        &calls(json!([
            {"command": "layout.export_pdf", "params": {"title": " ", "scale": 5000}}]))
    ));
    // Un titre attendu : comparé exactement.
    let tr31 = find("TR31");
    let mut titled = tr31["expected"]["calls"].clone();
    assert!(answers(&tr31, &calls(titled.clone())));
    titled[0]["params"]["title"] = json!("Relief");
    assert!(!answers(&tr31, &calls(titled)));
    // Un entier écrit en flottant vaut l'entier ; une hauteur décimale reste comparée.
    let v01 = find("V01");
    let mut floats = v01["expected"]["calls"].clone();
    floats[0]["params"]["observer_height"]["value"] = json!(10.0);
    assert!(answers(&v01, &calls(floats.clone())));
    floats[0]["params"]["observer_height"]["value"] = json!(10.5);
    assert!(!answers(&v01, &calls(floats)));
    // Un paramètre de trop (le défaut écrit) ne vaut pas l'attendu qui l'omet.
    let mut with_default = v01["expected"]["calls"].clone();
    with_default[0]["params"]["target_height"] = json!({"value": 0, "unit": "m"});
    assert!(!answers(&v01, &calls(with_default)));
    // Un refus ne compte que contre un refus attendu ; une question n'est jamais juste ici.
    let refused: Result<Plan, AgentError> = Err(AgentError::Refused("non".into()));
    let asked: Result<Plan, AgentError> = Ok(Plan::Question {
        question: "Dans quel système ?".into(),
        source: QuestionSource::Guard,
    });
    assert!(answers(&find("TR34"), &refused));
    assert!(!answers(&v01, &refused));
    assert!(!answers(&v01, &asked));
    assert!(!answers(&find("TR34"), &asked));
    assert!(!answers(
        &find("TR34"),
        &calls(v01["expected"]["calls"].clone())
    ));
}
