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
//! Depuis 5.53 (livrable 3, lot 1), la barre est élargie en place : aux 25 phrases de l'épreuve
//! s'ajoutent 23 phrases neuves écrites à la main (`plan_reel/phrases.json`, schéma de
//! `par_commande/phrases.json` sans `command`), soit 48, dont 4 refus neufs qui nomment dans
//! `missing` ce que le registre n'a pas (un mot-clé cherché dans ses identifiants, jamais un
//! synonyme ; les 5 refus de l'épreuve le portent aussi, `refusal_of`) : le jour où un identifiant du registre contient un de ces
//! mots, le refus devient faux et le test sans modèle nomme la phrase. Les attendus passent tous
//! les contrôles communs (`common::invalid`). Un seul `SCORE`, sur le total. Les 25 phrases de
//! l'épreuve gardent la comparaison exacte (titre d'export compris) ; les neuves celle de la barre
//! par commande (titre libre, entiers écrits en flottant ramenés à l'entier). Le jeu
//! d'entraînement ne voit aucune de ces phrases : `training/tests/recouvrement.rs` les garde.
//!
//! Une question rendue sur une phrase claire est un échec ; seule une question sur une phrase
//! qui attend un plan compte au plafond des fausses questions, une question sur un refus attendu
//! restant un raté ordinaire (2026-11-11). Seuils (décision du propriétaire,
//! 2026-11-11, D4), proportionnels aux décisions passées et arrondis vers le bas sur le total :
//! au moins 80 % de plans conformes, arrondi vers le bas (20/25 hier, soit 38/48 aujourd'hui) ; au plus 2/25 fausses questions pour un
//! GGUF seul au long prompt (concession du 2026-10-17 : 3/48), 1/25 pour un modèle au prompt
//! court (le modèle ajusté du ticket 5.10, déclaré par `*.modele.json`, plafond strict : 1/48).

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

/// Les phrases neuves de la barre (5.53, livrable 3), écrites à la main.
const PHRASES: &str = include_str!("plan_reel/phrases.json");

/// Les phrases de l'épreuve, dans la forme des `phrases.json` : `{id, phrase, expected}`, l'attendu
/// réécrit pour le registre réel par `expected`. `exact` : le titre d'un export compte (règle
/// d'avant 5.53, gardée pour ces 25 phrases).
fn epreuve() -> Vec<Value> {
    let sentences: Vec<Value> = serde_json::from_str(SENTENCES).unwrap();
    sentences
        .iter()
        .map(|s| {
            let id = s["id"].as_str().unwrap();
            let expected = match expected(id) {
                Want::Plan(calls) => json!({"kind": "calls", "calls": calls}),
                Want::Refusal => refusal_of(id),
            };
            json!({"id": id, "phrase": s["text"], "expected": expected, "exact": true})
        })
        .collect()
}

/// L'attendu d'un refus de l'épreuve, avec ce que le registre n'a pas (écrit ici, l'épreuve
/// restant gelée) : le cadrage de la carte (S10), les statistiques d'un raster par polygone,
/// identifiant réservé `raster.zonal_statistics` (S15, S23, S24), les couches nommées d'un
/// export PDF (S18, option `layers`).
fn refusal_of(id: &str) -> Value {
    match id {
        "S10" => json!({"kind": "refusal", "missing": ["zoom", "frame", "cadr"]}),
        "S15" | "S23" | "S24" => json!({"kind": "refusal", "missing": ["raster.zonal"]}),
        "S18" => json!({"kind": "refusal", "missing_params": ["layout.export_pdf:layers"]}),
        _ => panic!("{id} : refus de l'épreuve sans ce qui manque"),
    }
}

/// Les phrases neuves de `plan_reel/phrases.json`.
fn new_phrases() -> Vec<Value> {
    common::read_phrases(PHRASES, "plan_reel/phrases.json")
}

/// Toute la barre : les 25 phrases de l'épreuve, puis les phrases neuves.
fn all_phrases() -> Vec<Value> {
    let mut all = epreuve();
    all.extend(new_phrases());
    all
}

/// Le plan rendu vaut-il l'attendu de la phrase : à l'identique pour l'épreuve, titre d'export
/// libre pour les phrases neuves (`common::same_calls`) ; un refus attendu, un refus rendu.
fn answers(phrase: &Value, got: &Result<Plan, AgentError>) -> bool {
    let want = &phrase["expected"];
    match (want["kind"].as_str(), got) {
        (Some("calls"), Ok(Plan::Calls { calls })) if phrase["exact"] == true => {
            serde_json::to_value(calls).unwrap() == want["calls"]
        }
        (Some("calls"), Ok(Plan::Calls { calls })) => common::same_calls(calls, &want["calls"]),
        (Some("refusal"), Err(AgentError::Refused(_))) => true,
        _ => false,
    }
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
    let mut generator = model_files_generator(files);
    let (mut right, mut total, mut questions, mut guarded) = (0, 0, 0, 0);
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
        let guard_answer = matches!(
            &got,
            Ok(Plan::Question {
                source: QuestionSource::Guard,
                ..
            })
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
        // Une phrase claire ne doit pas devenir une question (déjà un échec ci-dessus). Seule
        // une phrase qui attend un plan compte : une question sur un refus attendu est un raté
        // ordinaire (2026-11-11).
        let false_question = is_false_question(phrase, &got);
        questions += usize::from(false_question);
        guarded += usize::from(false_question && guard_answer);
    }
    eprintln!(
        "SCORE {right}/{total} plans conformes, {questions} fausses questions, dont {guarded} du garde-fou"
    );
    // Les deux seuils sont jugés ensemble : un score sous la barre ne cache pas le plafond.
    let (need, cap) = (plans_needed(total), question_cap(style, total));
    let failures: Vec<String> = [
        (right < need).then(|| {
            format!("{right}/{total} : sous le seuil de 80 % arrondi vers le bas ({need}/{total})")
        }),
        (questions > cap).then(|| {
            format!("{questions} phrases claires sont devenues des questions (au plus {cap})")
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
    assert!(failures.is_empty(), "{}", failures.join(" ; "));
}

/// Une fausse question : une question rendue sur une phrase qui attend un plan. Sur une phrase
/// qui attend un refus, la question est un raté ordinaire, hors du plafond (2026-11-11).
fn is_false_question(phrase: &Value, got: &Result<Plan, AgentError>) -> bool {
    phrase["expected"]["kind"] == "calls" && matches!(got, Ok(Plan::Question { .. }))
}

#[test]
fn only_a_question_on_a_sentence_expecting_a_plan_is_a_false_question() {
    let question: Result<Plan, AgentError> = Ok(Plan::Question {
        question: "Quelle couche ?".into(),
        source: QuestionSource::Model,
    });
    let all = all_phrases();
    let plan = all.iter().find(|p| p["id"] == "PL01").unwrap();
    let refusal = all.iter().find(|p| p["id"] == "S10").unwrap();
    assert_eq!(plan["expected"]["kind"], "calls");
    assert_eq!(refusal["expected"]["kind"], "refusal");
    assert!(is_false_question(plan, &question));
    assert!(!is_false_question(refusal, &question));
    // Ni un plan ni un refus rendus ne sont des questions.
    let refused: Result<Plan, AgentError> = Err(AgentError::Refused("absent".into()));
    assert!(!is_false_question(plan, &refused));
    // Un raté reste un raté : la question sur un refus attendu ne vaut pas le refus.
    assert!(!answers(refusal, &question));
}

/// Plans conformes exigés : 80 % du total, arrondi vers le bas (D4).
fn plans_needed(total: usize) -> usize {
    common::proportional(total, 4, 5)
}

/// Plafond de fausses questions sur les phrases claires : la concession du propriétaire
/// (2026-10-17) pour le long prompt, 2 sur 25, le plafond strict du ticket 5.10 pour le prompt
/// court, 1 sur 25 ; rapportés au total, arrondis vers le bas (D4).
fn question_cap(style: PromptStyle, total: usize) -> usize {
    match style {
        PromptStyle::Full => common::proportional(total, 2, 25),
        PromptStyle::Short => common::proportional(total, 1, 25),
    }
}

#[test]
fn the_short_prompt_gets_the_strict_question_cap() {
    // Sur les 25 phrases de l'épreuve : les décisions passées, à l'identique.
    assert_eq!(question_cap(PromptStyle::Short, 25), 1);
    assert_eq!(question_cap(PromptStyle::Full, 25), 2);
    assert_eq!(plans_needed(25), 20);
    // Sur la barre élargie : la même proportion, arrondie vers le bas.
    assert_eq!(all_phrases().len(), 48);
    assert_eq!(plans_needed(48), 38);
    assert_eq!(question_cap(PromptStyle::Full, 48), 3);
    assert_eq!(question_cap(PromptStyle::Short, 48), 1);
    assert_eq!(plans_needed(47), 37);
    assert_eq!(question_cap(PromptStyle::Full, 47), 3);
    assert_eq!(question_cap(PromptStyle::Short, 47), 1);
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
        (15..=25).contains(&fresh.len()),
        "{} phrases neuves",
        fresh.len()
    );
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
    // `check_plan`, garde-fou des coordonnées, complétion sans effet, couches, champs, rasters
    // et fichiers présents dans l'inventaire de l'épreuve, refus encore justes.
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
    let f = broken.iter_mut().find(|p| p["id"] == "PL01").unwrap();
    f["expected"]["calls"][0]["params"]["field"] = json!("superficie");
    let u = broken.iter_mut().find(|p| p["id"] == "PL15").unwrap();
    u["expected"]["calls"][0]["params"]["unit"] = json!("radians");
    // Un refus qui dit absente une commande du registre.
    let r = broken.iter_mut().find(|p| p["id"] == "PL21").unwrap();
    r["expected"]["missing"] = json!(["reproject"]);
    // Un mot vide ou qui n'est pas un mot-clé en minuscules ASCII.
    let e = broken.iter_mut().find(|p| p["id"] == "PL20").unwrap();
    e["expected"]["missing"] = json!([""]);
    let m = broken.iter_mut().find(|p| p["id"] == "PL22").unwrap();
    m["expected"]["missing"] = json!(["Géocod"]);
    // Un refus sans ce qui manque, une option que la commande porte.
    let n = broken.iter_mut().find(|p| p["id"] == "S10").unwrap();
    n["expected"] = json!({"kind": "refusal"});
    let o = broken.iter_mut().find(|p| p["id"] == "S18").unwrap();
    o["expected"]["missing_params"] = json!(["layout.export_pdf:page_size"]);
    // Une égalité sur une valeur que le champ ne montre pas.
    let v = broken.iter_mut().find(|p| p["id"] == "PL06").unwrap();
    v["expected"]["calls"][0]["params"]["value"] = json!("B");
    let errors = common::invalid(&registry, &inventory, &broken);
    for (id, reason) in [
        ("PL01", "champ absent"),
        ("PL15", "check_plan refuse"),
        ("PL21", "refus devenu faux"),
        ("PL20", "mot `missing` invalide"),
        ("PL22", "mot `missing` invalide"),
        ("S10", "refus sans `missing`"),
        ("S18", "refus devenu faux"),
        ("PL06", "valeur absente"),
        ("PL01", "le garde-fou des attributs refuse"),
        ("PL06", "le garde-fou des attributs refuse"),
    ] {
        assert!(
            errors
                .iter()
                .any(|e| e.starts_with(&format!("{id} : {reason}"))),
            "{id} : {errors:?}"
        );
    }
    // `contains("")` est toujours vrai : le mot vide est refusé, sans autre refus devenu faux.
    // PL01 et PL06 comptent deux fois : leur contrôle, et le garde-fou des attributs (5.57).
    assert_eq!(errors.len(), 10, "{errors:?}");
}

#[test]
fn a_refusal_turns_red_when_its_command_reaches_the_registry() {
    let mut ids = common::registry_ids(&Registry::standard());
    assert!(common::refusals_now_wrong(&ids, &all_phrases()).is_empty());
    ids.push("terrain.contour_lines".to_string());
    let wrong = common::refusals_now_wrong(&ids, &all_phrases());
    assert_eq!(wrong.len(), 1, "{wrong:?}");
    assert!(
        wrong[0].starts_with("PL23 : refus devenu faux"),
        "{wrong:?}"
    );
}

/// Le texte d'une phrase de l'épreuve.
fn sentence(id: &str) -> String {
    let sentences: Vec<Value> = serde_json::from_str(SENTENCES).unwrap();
    let found = sentences.iter().find(|s| s["id"] == id).unwrap();
    found["text"].as_str().unwrap().to_string()
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
fn only_the_sentences_that_open_a_file_name_a_file_that_is_not_open() {
    // Une phrase qui nomme un fichier non ouvert attend son `layer.add` (S01, PL11) ; aucune
    // autre ne le nomme.
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let all = all_phrases();
    let opening: Vec<String> = all
        .iter()
        .filter(|p| {
            common::expected_calls(p)
                .is_some_and(|calls| calls.iter().any(|c| c["command"] == "layer.add"))
        })
        .map(|p| p["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(common::naming_unloaded_files(&inventory, &all), opening);
    assert_eq!(opening, ["S01", "PL11"]);
}

/// Un plan rendu, depuis ses appels.
fn calls(value: Value) -> Result<Plan, AgentError> {
    Ok(serde_json::from_value(json!({"kind": "calls", "calls": value})).unwrap())
}

#[test]
fn the_score_compares_titles_numbers_and_kinds_as_documented() {
    let all = all_phrases();
    let find = |id: &str| all.iter().find(|p| p["id"] == id).unwrap().clone();
    // L'épreuve : comparaison exacte, titre compris.
    let s17 = find("S17");
    let mut right = s17["expected"]["calls"].clone();
    assert!(answers(&s17, &calls(right.clone())));
    right[4]["params"]["title"] = json!("Autre titre");
    assert!(!answers(&s17, &calls(right)));
    // Une phrase neuve dont l'attendu porte un titre : comparé exactement.
    let pl16 = find("PL16");
    let mut titled = pl16["expected"]["calls"].clone();
    assert!(answers(&pl16, &calls(titled.clone())));
    titled[0]["params"]["title"] = json!("Routes");
    assert!(!answers(&pl16, &calls(titled)));
    // Sans titre attendu : libre, mais non vide ; un entier écrit en flottant vaut l'entier.
    let free = json!({"id": "X1", "phrase": "Exporte la carte en PDF au 1:25 000.",
        "expected": {"kind": "calls", "calls": [
            {"command": "layout.export_pdf", "params": {"scale": 25000}}]}});
    assert!(answers(
        &free,
        &calls(json!([
        {"command": "layout.export_pdf", "params": {"title": "Ma carte", "scale": 25000.0}}]))
    ));
    assert!(!answers(
        &free,
        &calls(json!([
        {"command": "layout.export_pdf", "params": {"title": " ", "scale": 25000}}]))
    ));
    let pl06 = find("PL06");
    let mut floats = pl06["expected"]["calls"].clone();
    floats[1]["params"]["distance"]["value"] = json!(750.0);
    assert!(answers(&pl06, &calls(floats)));
    // Un refus ne compte que contre un refus attendu ; une question n'est jamais juste ici.
    let refused: Result<Plan, AgentError> = Err(AgentError::Refused("non".into()));
    let asked: Result<Plan, AgentError> = Ok(Plan::Question {
        question: "Laquelle ?".into(),
        source: QuestionSource::Model,
    });
    assert!(answers(&find("PL20"), &refused));
    assert!(answers(&find("S10"), &refused));
    assert!(!answers(&pl06, &refused));
    assert!(!answers(&find("PL20"), &asked));
    assert!(!answers(&pl06, &asked));
    assert!(!answers(
        &find("PL20"),
        &calls(pl06["expected"]["calls"].clone())
    ));
}
