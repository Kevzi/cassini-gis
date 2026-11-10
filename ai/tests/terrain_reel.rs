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
//! coordonnées le demande), deux champs de vision, deux exports à une échelle sans « PDF ». Le
//! titre de l'export n'est pas comparé, seulement non vide. Seuil (ticket 5.41) : au moins 7 plans
//! conformes sur 8. Depuis 11.23, le plan mesuré est le plan complété par les conversions de
//! points, comme `plan.propose` le montre (11.22, `common::completed`) ; les points de ces
//! phrases sont dans le système du MNT, la complétion n'y ajoute rien.

mod common;

use cassini_ai::plan::{compose_plan_with, model_files_generator};
use cassini_core::plan::Plan;
use cassini_core::point::guard_coordinates;
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

/// L'export attendu, titre retiré : il n'est pas comparé.
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
fn terrain_reel_at_least_7_of_8_plans_match() {
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
    let mut right = 0;
    for (id, text) in SENTENCES {
        let want = expected(id);
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

/// L'attendu de `id`, titre d'export remis, en plan.
fn expected_plan(id: &str) -> Plan {
    let mut calls = expected(id);
    // Le titre retiré pour la comparaison est remis pour la validation.
    for call in calls.as_array_mut().unwrap() {
        if call["command"] == "layout.export_pdf" {
            call["params"]["title"] = json!("Carte");
        }
    }
    serde_json::from_value(json!({"kind": "calls", "calls": calls})).unwrap()
}

#[test]
fn every_sentence_has_a_valid_expected_plan() {
    let registry = Registry::standard();
    for (id, _) in SENTENCES {
        assert_eq!(registry.check_plan(&expected_plan(id)), Ok(()), "{id}");
    }
}

#[test]
fn every_expected_plan_passes_the_coordinate_guard() {
    // Un plan juste ne doit pas devenir la question du garde-fou : chaque point de l'attendu
    // vient de la phrase, système compris.
    let registry = Registry::standard();
    for (id, text) in SENTENCES {
        let plan = expected_plan(id);
        assert_eq!(
            guard_coordinates(&registry, plan.clone(), text),
            plan,
            "{id}"
        );
    }
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
    for (id, text) in SENTENCES {
        if id.starts_with('X') {
            assert!(!text.to_lowercase().contains("pdf"), "{id}");
        }
    }
}

#[test]
fn an_export_without_a_title_does_not_match() {
    let untitled = json!([{"command": "layout.export_pdf", "params": {"scale": 5000}}]);
    assert_eq!(comparable(untitled), None);
    let titled =
        json!([{"command": "layout.export_pdf", "params": {"title": "Carte", "scale": 5000}}]);
    assert_eq!(comparable(titled), Some(json!([export(5000)])));
}

#[test]
fn every_expected_plan_is_left_unchanged_by_the_completion() {
    // Les points sont écrits dans le système du MNT : la complétion n'ajoute aucune conversion.
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    for (id, _) in SENTENCES {
        let plan = expected_plan(id);
        assert_eq!(
            common::complete_with(&registry, plan.clone(), &inventory),
            plan,
            "{id}"
        );
    }
}
