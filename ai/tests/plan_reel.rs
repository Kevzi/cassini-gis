//! Essai avec le vrai modèle : phrases de l'épreuve du modèle local contre le registre réel.
//! Ignoré sans `CASSINI_TEST_MODEL` (et marqué `ignore` : il charge le modèle une fois par phrase).
//!
//!   CASSINI_TEST_MODEL=<gguf ou déclaration *.modele.json> cargo test -p cassini-ai --release --features cuda -- --ignored plan_reel
//!
//! Les 25 phrases de l'épreuve (`sentences.json`) et l'inventaire de l'épreuve sont lus tels quels,
//! jamais modifiés. Les plans attendus sont réécrits pour le registre réel, chaînes `step:k.champ`
//! comprises. Là où le registre réel n'a pas la commande (retrait, reprojection, export, cadrage,
//! terrain, statistiques sur un raster) ou ne sait pas porter une option demandée (papier A3,
//! couches nommées du PDF), le refus est la réponse attendue, comme pour S20. Une question rendue
//! sur une de ces phrases claires est un échec, et il en faut au plus deux sur les 25 : concession
//! du propriétaire (2026-10-17) pour un GGUF seul au long prompt. Un modèle au prompt court (le
//! modèle ajusté du ticket 5.10, déclaré par `*.modele.json`) tient le plafond strict : au plus
//! une, avec au moins 9 questions sur 12 phrases ambiguës (`question_reel`).

mod common;

use cassini_ai::plan::{compose_plan_with, model_files_generator, AgentError, PromptStyle};
use cassini_core::plan::Plan;
use cassini_core::registry::Registry;
use serde_json::{json, Value};

const INVENTORY: &str = include_str!("../../tests/epreuve-modele-local/inventory.json");
const SENTENCES: &str = include_str!("../../tests/epreuve-modele-local/sentences.json");

const PARCELLES: &str = "parcelles_3f9c2a1e_7b40_4d5a_9e12_0c8a6d4b71f3";
const PLU: &str = "plu_zonage_a81d5c06_2e9f_4a37_b6c4_5f13e07d92a8";
const GARES: &str = "gares_c5e0b94d_1a67_4f82_8d03_9b2e6a7c40f1";
const ROUTES: &str = "routes_09b3d7f2_6c18_4e5b_a0d9_3e41c8f25b67";
const BATIMENTS: &str = "batiments_d27a60e8_4b95_4c31_9f7e_8a05d1b3c649";

fn select(layer: &str, field: &str, op: &str, value: &str) -> Value {
    json!({"command": "analysis.select_by_attribute",
        "params": {"layer_id": layer, "field": field, "operator": op, "value": value}})
}

fn buffer(layer: &str, value: u32, unit: &str) -> Value {
    json!({"command": "analysis.buffer",
        "params": {"layer_id": layer, "distance": {"value": value, "unit": unit}}})
}

fn intersect(layer: &str, overlay: &str) -> Value {
    json!({"command": "analysis.intersect", "params": {"layer_id": layer, "overlay_layer_id": overlay}})
}

/// Plan attendu d'une phrase de l'épreuve sur le registre réel ; `None` = un refus est attendu.
fn expected(id: &str) -> Option<Value> {
    let plan = match id {
        "S01" => json!([{"command": "layer.add", "params": {"path": "donnees/ecoles.geojson"}}]),
        "S02" => json!([{"command": "layer.describe", "params": {"layer_id": GARES}}]),
        "S03" => json!([{"command": "project.inventory", "params": {}}]),
        "S06" => json!([select(PARCELLES, "section", "eq", "AB")]),
        "S07" => json!([buffer(GARES, 300, "m")]),
        "S08" => json!([{"command": "analysis.buffer", "params": {
            "layer_id": GARES, "distance": {"value": 1, "unit": "km"}, "dissolve": true}}]),
        "S11" => json!([select(BATIMENTS, "hauteur_m", "gt", "12")]),
        "S12" => json!([select(ROUTES, "classe", "eq", "départementale")]),
        "S13" => json!([intersect(PARCELLES, PLU)]),
        "S14" => json!([
            select(PLU, "libelle", "eq", "Zone à urbaniser"),
            buffer("step:1.layer_id", 50, "m")
        ]),
        "S16" => json!([{"command": "analysis.isochrone", "params": {
            "layer_id": ROUTES, "origin": {"x": 656410, "y": 6847920, "epsg": 2154},
            "mode": "walk", "duration": {"value": 10, "unit": "min"}}}]),
        "S17" => json!([
            select(PLU, "typezone", "eq", "U"),
            buffer(GARES, 500, "m"),
            intersect(PARCELLES, "step:1.layer_id"),
            intersect("step:3.layer_id", "step:2.layer_id"),
            {"command": "layout.export_pdf", "params": {"title": "Parcelles en zone U près des gares"}}
        ]),
        "S19" => json!([
            select(ROUTES, "classe", "eq", "départementale"),
            buffer("step:1.layer_id", 100, "m"),
            intersect(BATIMENTS, "step:2.layer_id")
        ]),
        // S04 S05 S09 S10 S15 S18 S20 S21 S22 S23 S24 S25 : refus.
        _ => return None,
    };
    Some(plan)
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn plan_reel_at_least_80_percent_of_the_plans_match() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let sentences: Vec<Value> = serde_json::from_str(SENTENCES).unwrap();
    let mut generator = model_files_generator(files);
    let (mut right, mut total, mut questions) = (0, 0, 0);
    for sentence in &sentences {
        let (id, text) = (
            sentence["id"].as_str().unwrap(),
            sentence["text"].as_str().unwrap(),
        );
        let want = expected(id);
        total += 1;
        let got = compose_plan_with(style, &registry, &inventory, text, &mut generator);
        let ok = match (&got, &want) {
            (Ok(Plan::Calls { calls }), Some(plan)) => {
                serde_json::to_value(calls).unwrap() == *plan
            }
            (Err(AgentError::Refused(_)), None) => true,
            _ => false,
        };
        // Score exact, phrase par phrase : jamais filtré.
        eprintln!(
            "{} {id} {text}
     attendu : {want:?}
     obtenu  : {got:?}",
            if ok { "OK  " } else { "FAIL" }
        );
        // Aucun plan invalide n'est jamais rendu : tout `Ok` passe `check_plan`.
        if let Ok(plan) = &got {
            assert_eq!(registry.check_plan(plan), Ok(()), "{id}");
        }
        right += usize::from(ok);
        // Une phrase claire ne doit pas devenir une question (déjà un échec ci-dessus).
        questions += usize::from(matches!(&got, Ok(Plan::Question { .. })));
    }
    eprintln!("SCORE {right}/{total} plans conformes, {questions} questions");
    // Les deux seuils sont jugés ensemble : un score sous la barre ne cache pas le plafond.
    let cap = question_cap(style);
    let failures: Vec<String> = [
        (right * 5 < total * 4).then(|| format!("{right}/{total} : sous le seuil de 80 %")),
        (questions > cap).then(|| {
            format!("{questions} phrases claires sont devenues des questions (au plus {cap})")
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
    assert!(failures.is_empty(), "{}", failures.join(" ; "));
}

/// Plafond de fausses questions sur les 25 phrases claires : la concession du propriétaire
/// (2026-10-17) pour le long prompt, le plafond strict du ticket 5.10 pour le prompt court.
fn question_cap(style: PromptStyle) -> usize {
    match style {
        PromptStyle::Full => 2,
        PromptStyle::Short => 1,
    }
}

#[test]
fn the_short_prompt_gets_the_strict_question_cap() {
    assert_eq!(question_cap(PromptStyle::Short), 1);
    assert_eq!(question_cap(PromptStyle::Full), 2);
}
