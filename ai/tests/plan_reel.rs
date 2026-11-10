//! Essai avec le vrai modèle : phrases de l'épreuve du modèle local contre le registre réel.
//! Ignoré sans `CASSINI_TEST_MODEL` (et marqué `ignore` : il charge le modèle une fois par phrase).
//!
//!   CASSINI_TEST_MODEL=<gguf ou déclaration *.modele.json> cargo test -p cassini-ai --release --features cuda -- --ignored plan_reel
//!
//! Les 25 phrases de l'épreuve (`sentences.json`) et l'inventaire de l'épreuve sont lus tels quels,
//! jamais modifiés. Les plans attendus sont réécrits pour le registre réel, chaînes `step:k.champ`
//! comprises. Là où le registre réel n'a pas la commande (cadrage,
//! statistiques sur un raster) ou ne sait pas porter une option demandée (papier A3, couches
//! nommées du PDF), le refus est la réponse attendue. Depuis 5.41, le terrain a ses commandes
//! (ticket 4.6) : S20 (champ de vision), S21 (ligne de vue) et S22 (pente) attendent un plan sur
//! la `source` du MNT ; S21 écrit le système une seule fois pour deux points, ce que le garde-fou
//! des coordonnées accepte depuis le ticket 4.10 (décision du propriétaire, 2026-11-05) ; S23 et
//! S24, terrain puis moyenne par commune, restent des refus (statistiques sur un raster).
//! S25, un point écrit en WGS84 sur un MNT en Lambert-93, attend depuis 11.16 le plan
//! `data.transform_point` vers le système du MNT, puis `terrain.viewshed` sur `step:1.point`.
//! Depuis 11.23, c'est le **plan complété** qui est mesuré : le modèle recopie le point tel que
//! la phrase le donne, et le noyau ajoute la conversion avant de montrer le plan (11.22,
//! décision 3 du changement de cap du 2026-11-08) ; l'essai applique la même complétion
//! (`common::completed`). Un plan qui compose déjà la conversion n'en reçoit pas une seconde.
//! S05 « Mets les bâtiments en Lambert-93 », refus tant que le registre ne reprojetait pas une
//! couche, attend depuis le second essai de 11.23 `data.reproject` des bâtiments (en 4326 dans
//! l'inventaire de l'épreuve) vers 2154 (commande de 10.7).
//! S04 « Retire la couche des routes du projet. », refus tant que le registre ne retirait pas une
//! couche, attend depuis 10.27 `layer.remove` sur la couche des routes.
//! Un refus est un échec.
//! `tests/epreuve-modele-local/` reste gelé : son `sentences.json` et son banc Python (étape 2)
//! gardent S25 en refus ; seul cet essai porte l'attendu du registre réel.
//!
//! Une question rendue sur une phrase claire est un échec, et il en faut au plus deux sur les 25 :
//! concession du propriétaire (2026-10-17) pour un GGUF seul au long prompt. Un modèle au prompt
//! court (le modèle ajusté du ticket 5.10, déclaré par `*.modele.json`) tient le plafond strict :
//! au plus une, avec au moins 9 questions sur 12 phrases ambiguës (`question_reel`).

mod common;

use cassini_ai::plan::{compose_plan_with, model_files_generator, AgentError, PromptStyle};
use cassini_core::plan::{Plan, QuestionSource};
use cassini_core::point::guard_coordinates;
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

/// Le MNT de l'inventaire de l'épreuve, désigné par sa `source` (forme apprise en 4.6).
const MNT: &str = "donnees/mnt_5m.tif";

/// Ce qu'une phrase de l'épreuve attend sur le registre réel.
#[derive(Debug, Clone, PartialEq)]
enum Want {
    /// Ces appels, à l'identique.
    Plan(Value),
    /// Un refus : le registre n'a pas la commande ou l'option.
    Refusal,
}

/// Ce qu'attend une phrase de l'épreuve sur le registre réel.
fn expected(id: &str) -> Want {
    let plan = match id {
        "S01" => json!([{"command": "layer.add", "params": {"path": "donnees/ecoles.geojson"}}]),
        "S02" => json!([{"command": "layer.describe", "params": {"layer_id": GARES}}]),
        "S03" => json!([{"command": "project.inventory", "params": {}}]),
        // Les bâtiments sont en 4326 dans l'inventaire de l'épreuve : depuis 10.7, le registre
        // reprojette une couche (second essai de 11.23).
        "S05" => json!([{"command": "data.reproject",
            "params": {"layer_id": BATIMENTS, "target_epsg": 2154}}]),
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
        "S20" => json!([{"command": "terrain.viewshed", "params": {
            "dem": MNT, "observer": {"x": 656120, "y": 6848060, "epsg": 2154},
            "observer_height": {"value": 25, "unit": "m"},
            "radius": {"value": 2, "unit": "km"}}}]),
        "S22" => json!([{"command": "terrain.slope", "params": {"dem": MNT, "unit": "percent"}}]),
        // Deux points, un seul « Lambert-93 » : il vaut pour les deux (4.10, 2026-11-05).
        "S21" => json!([{"command": "terrain.line_of_sight", "params": {
            "dem": MNT,
            "observer": {"x": 656300, "y": 6847500, "epsg": 2154},
            "target": {"x": 657100, "y": 6848200, "epsg": 2154},
            "observer_height": {"value": 2, "unit": "m"},
            "target_height": {"value": 2, "unit": "m"}}}]),
        // Un point WGS84 sur un MNT en Lambert-93 : la reprojection est une étape visible (11.12),
        // ajoutée par le noyau au plan du modèle depuis 11.22 (plan complété).
        "S25" => json!([
            {"command": "data.transform_point", "params": {
                "point": {"x": 2.415, "y": 48.735, "epsg": 4326}, "target_epsg": 2154}},
            {"command": "terrain.viewshed", "params": {
                "dem": MNT, "observer": "step:1.point",
                "observer_height": {"value": 10, "unit": "m"},
                "radius": {"value": 1, "unit": "km"}}}
        ]),
        // L'export d'une couche en GeoPackage est une conversion de format (10.9).
        "S09" => {
            json!([{"command": "data.convert", "params": {"layer_id": PARCELLES, "format": "gpkg"}}])
        }
        // Le retrait d'une couche du projet, jamais de son fichier (10.27).
        "S04" => json!([{"command": "layer.remove", "params": {"layer_id": ROUTES}}]),
        // S10 S15 S18 S23 S24 : refus.
        _ => return Want::Refusal,
    };
    Want::Plan(plan)
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn plan_reel_at_least_80_percent_of_the_plans_match() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    let full: Value = serde_json::from_str(INVENTORY).unwrap();
    // Comme dans le produit : identifiants courts montrés, plan retraduit (ticket 5.19), puis
    // complété par les conversions de points (11.22).
    let (_, ids) = common::shown_inventory(&full);
    let sentences: Vec<Value> = serde_json::from_str(SENTENCES).unwrap();
    let mut generator = model_files_generator(files);
    let (mut right, mut total, mut questions, mut guarded) = (0, 0, 0, 0);
    for sentence in &sentences {
        let (id, text) = (
            sentence["id"].as_str().unwrap(),
            sentence["text"].as_str().unwrap(),
        );
        let want = expected(id);
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
        let guard_answer = matches!(
            &got,
            Ok(Plan::Question {
                source: QuestionSource::Guard,
                ..
            })
        );
        let ok = match (&got, &want) {
            (Ok(Plan::Calls { calls }), Want::Plan(plan)) => {
                serde_json::to_value(calls).unwrap() == *plan
            }
            (Err(AgentError::Refused(_)), Want::Refusal) => true,
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
        guarded += usize::from(guard_answer);
    }
    eprintln!(
        "SCORE {right}/{total} plans conformes, {questions} fausses questions, dont {guarded} du garde-fou"
    );
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

/// Le texte d'une phrase de l'épreuve.
fn sentence(id: &str) -> String {
    let sentences: Vec<Value> = serde_json::from_str(SENTENCES).unwrap();
    let found = sentences.iter().find(|s| s["id"] == id).unwrap();
    found["text"].as_str().unwrap().to_string()
}

#[test]
fn every_expected_plan_is_valid_and_passes_the_coordinate_guard() {
    let registry = Registry::standard();
    let sentences: Vec<Value> = serde_json::from_str(SENTENCES).unwrap();
    for s in &sentences {
        let id = s["id"].as_str().unwrap();
        let Want::Plan(calls) = expected(id) else {
            continue;
        };
        let plan: Plan = serde_json::from_value(json!({"kind": "calls", "calls": calls})).unwrap();
        assert_eq!(registry.check_plan(&plan), Ok(()), "{id}");
        let text = s["text"].as_str().unwrap();
        assert_eq!(
            guard_coordinates(&registry, plan.clone(), text),
            plan,
            "{id}"
        );
    }
}

#[test]
fn the_terrain_sentences_expect_plans_except_the_raster_statistics() {
    assert!(matches!(expected("S20"), Want::Plan(_)));
    assert!(matches!(expected("S21"), Want::Plan(_)));
    assert!(matches!(expected("S22"), Want::Plan(_)));
    assert!(matches!(expected("S25"), Want::Plan(_)));
    assert_eq!(expected("S23"), Want::Refusal);
    assert_eq!(expected("S24"), Want::Refusal);
}

#[test]
fn the_right_line_of_sight_for_s21_passes_the_guard() {
    // Le système n'est écrit qu'une fois pour deux points : il vaut pour les deux (4.10), et
    // le plan juste, l'attendu de S21, passe le garde-fou inchangé.
    let registry = Registry::standard();
    let Want::Plan(calls) = expected("S21") else {
        panic!("S21 attend un plan");
    };
    let plan: Plan = serde_json::from_value(json!({"kind": "calls", "calls": calls})).unwrap();
    assert_eq!(registry.check_plan(&plan), Ok(()));
    assert_eq!(
        guard_coordinates(&registry, plan.clone(), &sentence("S21")),
        plan
    );
}

#[test]
fn s25_reprojects_the_wgs84_point_into_the_dem_system_first() {
    // Le point écrit en WGS84 passe par `data.transform_point` vers le Lambert-93 du MNT ; le
    // champ de vision prend le point reprojeté, jamais un point en 4326.
    let Want::Plan(calls) = expected("S25") else {
        panic!("S25 attend un plan");
    };
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let dem = inventory["layers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["source"] == MNT)
        .unwrap();
    assert_eq!(calls[0]["command"], "data.transform_point");
    assert_eq!(calls[0]["params"]["point"]["epsg"], 4326);
    assert_eq!(calls[0]["params"]["target_epsg"], dem["epsg"]);
    assert_eq!(calls[1]["params"]["dem"], MNT);
    assert_eq!(calls[1]["command"], "terrain.viewshed");
    assert_eq!(calls[1]["params"]["observer"], "step:1.point");
}

/// Le plan que le modèle écrit pour S25 depuis 11.23 : le point recopié tel que la phrase le
/// donne, en WGS84.
fn s25_as_written() -> Plan {
    serde_json::from_value(json!({"kind": "calls", "calls": [
        {"command": "terrain.viewshed", "params": {
            "dem": MNT, "observer": {"x": 2.415, "y": 48.735, "epsg": 4326},
            "observer_height": {"value": 10, "unit": "m"},
            "radius": {"value": 1, "unit": "km"}}}
    ]}))
    .unwrap()
}

#[test]
fn s25_written_as_given_is_completed_into_the_expected_plan() {
    // 11.23 : le plan du modèle (point tel qu'écrit) passe le garde-fou, et la complétion du
    // noyau en fait l'attendu ; un plan qui a déjà la conversion n'en reçoit pas une seconde.
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let raw = s25_as_written();
    assert_eq!(registry.check_plan(&raw), Ok(()));
    assert_eq!(
        guard_coordinates(&registry, raw.clone(), &sentence("S25")),
        raw
    );
    let Want::Plan(calls) = expected("S25") else {
        panic!("S25 attend un plan");
    };
    let want: Plan = serde_json::from_value(json!({"kind": "calls", "calls": calls})).unwrap();
    assert_eq!(common::complete_with(&registry, raw, &inventory), want);
    assert_eq!(
        common::complete_with(&registry, want.clone(), &inventory),
        want
    );
}

#[test]
fn every_expected_plan_is_left_unchanged_by_the_completion() {
    // Un attendu est un plan complété : la complétion n'y ajoute rien.
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let sentences: Vec<Value> = serde_json::from_str(SENTENCES).unwrap();
    for s in &sentences {
        let id = s["id"].as_str().unwrap();
        let Want::Plan(calls) = expected(id) else {
            continue;
        };
        let plan: Plan = serde_json::from_value(json!({"kind": "calls", "calls": calls})).unwrap();
        assert_eq!(
            common::complete_with(&registry, plan.clone(), &inventory),
            plan,
            "{id}"
        );
    }
}
