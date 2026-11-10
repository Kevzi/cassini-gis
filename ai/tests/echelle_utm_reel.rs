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
//! isochrones en UTM 31N ou 32N. Le titre de l'export n'est pas comparé, seulement non vide.
//! Seuil (ticket 5.39) : au moins 7 plans conformes sur 8.

mod common;

use cassini_ai::plan::{compose_plan_with, model_files_generator};
use cassini_core::plan::Plan;
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

/// L'export attendu, titre retiré : il n'est pas comparé.
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

/// Le plan obtenu, prêt à comparer : `None` si un export n'a pas de titre non vide, sinon les
/// appels avec le titre de chaque export retiré.
fn comparable(mut calls: Value) -> Option<Value> {
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

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn echelle_utm_reel_at_least_7_of_8_plans_match() {
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
    let mut right = 0;
    for (id, text) in SENTENCES {
        let want = expected(id);
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
        let ok = match &got {
            Ok(Plan::Calls { calls }) => {
                comparable(serde_json::to_value(calls).unwrap()).is_some_and(|c| c == want)
            }
            _ => false,
        };
        // Score exact, phrase par phrase : jamais filtré.
        eprintln!(
            "{} {id} {text}
     attendu : {want}
     obtenu  : {got:?}",
            if ok { "OK  " } else { "FAIL" }
        );
        // Aucun plan invalide n'est jamais rendu : tout `Ok` passe `check_plan`.
        if let Ok(plan) = &got {
            assert_eq!(registry.check_plan(plan), Ok(()), "{id}");
        }
        right += usize::from(ok);
    }
    let total = SENTENCES.len();
    eprintln!("SCORE {right}/{total} plans conformes");
    assert!(right >= 7, "{right}/{total} : sous le seuil de 7 sur 8");
}

#[test]
fn every_sentence_has_a_valid_expected_plan() {
    let registry = Registry::standard();
    for (id, _) in SENTENCES {
        let mut calls = expected(id);
        // Le titre retiré pour la comparaison est remis pour la validation.
        for call in calls.as_array_mut().unwrap() {
            if call["command"] == "layout.export_pdf" {
                call["params"]["title"] = json!("Carte");
            }
        }
        for call in calls.as_array().unwrap() {
            let command = call["command"].as_str().unwrap();
            assert_eq!(
                registry.validate_params(command, &call["params"]),
                Ok(()),
                "{id}"
            );
        }
    }
}

#[test]
fn an_export_without_a_title_does_not_match() {
    let untitled = json!([{"command": "layout.export_pdf", "params": {"scale": 25000}}]);
    assert_eq!(comparable(untitled), None);
    let blank = json!([{"command": "layout.export_pdf", "params": {"title": " ", "scale": 25000}}]);
    assert_eq!(comparable(blank), None);
    let titled =
        json!([{"command": "layout.export_pdf", "params": {"title": "Carte", "scale": 25000}}]);
    assert_eq!(comparable(titled), Some(json!([export(25000)])));
}
