//! Essai avec le vrai modèle : les quatorze commandes de l'epic 11, que le générateur enseigne
//! depuis 11.12 (cinq lots) et que le ré-entraînement groupé de 11.16 doit avoir apprises.
//! Ignoré sans `CASSINI_TEST_MODEL` (et marqué `ignore` : il charge le modèle une fois par
//! phrase).
//!
//!   CASSINI_TEST_MODEL=<gguf ou déclaration *.modele.json> cargo test -p cassini-ai --release --features cuda -- --ignored epic11_reel
//!
//! L'inventaire de l'épreuve est lu tel quel, jamais modifié ; l'essai y ajoute en mémoire les
//! rasters qu'il lui faut (un raster de coût, deux critères notés, un masque), dans la forme des
//! inventaires de QGIS, cités par leur `source`. Les phrases sont fabriquées ici, pas dans
//! `tests/epreuve-modele-local/` (gelé), et le garde-fou de recouvrement de `training/` vérifie
//! que le générateur ne les engendre pas. Deux phrases par commande ; les formes attendues sont
//! celles du générateur (`training/src/families.rs`). Le titre d'un export n'est pas comparé,
//! seulement non vide. Une commande est apprise si au moins une de ses deux phrases rend le plan
//! conforme. Un carroyage sans système (`GRID_QUESTION`) doit rendre une question, jamais un plan.

mod common;

use cassini_ai::plan::{compose_plan_with, model_files_generator};
use cassini_core::plan::Plan;
use cassini_core::point::guard_coordinates;
use cassini_core::registry::Registry;
use serde_json::{json, Value};

const INVENTORY: &str = include_str!("../../tests/epreuve-modele-local/inventory.json");

/// Le MNT de l'inventaire de l'épreuve, désigné par sa `source`.
const MNT: &str = "donnees/mnt_5m.tif";
/// Les rasters ajoutés en mémoire à l'inventaire de l'épreuve, désignés par leur `source`.
const COUT: &str = "donnees/cout_deplacement.tif";
const ACCESSIBILITE: &str = "donnees/note_accessibilite.tif";
const CALME: &str = "donnees/note_calme.tif";
const INONDABLES: &str = "donnees/zones_inondables_masque.tif";

/// Les phrases de l'essai : `(id, texte)`, deux par commande. Le plan attendu de chacune est
/// dans `expected`, la commande mesurée dans `command`.
const SENTENCES: [(&str, &str); 28] = [
    // coord.convert
    (
        "G01",
        "Donne-moi la référence MGRS du point 48,7351 N, 2,4087 E (WGS84).",
    ),
    (
        "G02",
        "Que vaut la référence MGRS 31UDQ 48251 11750 en latitude et longitude ?",
    ),
    // data.transform_point
    (
        "P01",
        "Passe le point 656 400 / 6 847 700 (Lambert-93) en WGS84.",
    ),
    (
        "P02",
        "Quelles sont les coordonnées en Lambert-93 du point 48,7420 N, 2,4230 E (WGS84) ?",
    ),
    // geo.distance_azimuth
    (
        "D01",
        "Quelle distance sépare le point 48,7300 N, 2,4000 E (WGS84) du point 48,7500 N, 2,4400 E (WGS84), et dans quel azimut ?",
    ),
    (
        "D02",
        "Distance et azimut en grades du point (452 300 ; 5 397 800) en UTM 31N au point (455 100 ; 5 401 200) en UTM 31N.",
    ),
    // geo.destination_point
    (
        "R01",
        "Où arrive-t-on en partant du point 48,7350 N, 2,4150 E (WGS84) sur 3 km à l'azimut 45° ?",
    ),
    (
        "R02",
        "Calcule le point d'arrivée à 1 500 m du point X = 452 300, Y = 5 397 800 en UTM 31N, cap à l'azimut 120 degrés.",
    ),
    // geo.bearing_intersection
    (
        "B01",
        "J'ai visé 60° depuis le point 48,7200 N, 2,3900 E (WGS84) et 330° depuis le point 48,7150 N, 2,4500 E (WGS84) : où se coupent les deux visées ?",
    ),
    (
        "B02",
        "Recoupe l'azimut 15° pris depuis le point (452 000 ; 5 397 000) en UTM 31N et l'azimut 300° pris depuis le point (456 000 ; 5 398 000) en UTM 31N.",
    ),
    // terrain.aspect
    ("A01", "Dans quelle direction sont tournées les pentes du MNT ?"),
    (
        "A02",
        "Calcule l'orientation des versants du modèle numérique de terrain 5 m.",
    ),
    // terrain.hillshade
    ("H01", "Fais-moi un relief ombré du MNT."),
    (
        "H02",
        "Ombrage du terrain avec le soleil à l'azimut 270° et à 30° au-dessus de l'horizon.",
    ),
    // terrain.viewshed_cumulative
    (
        "C01",
        "Pour chaque endroit du terrain, combien de ces points le voient : le point 656 200 / 6 847 400 (Lambert-93), le point 657 000 / 6 848 100 (Lambert-93) et le point 655 800 / 6 848 500 (Lambert-93) ?",
    ),
    (
        "C02",
        "Champ de vision cumulé des points X = 656 500, Y = 6 847 900 en Lambert-93 et X = 657 200, Y = 6 847 300 en Lambert-93, dans un rayon de 1,5 km.",
    ),
    // terrain.peaks
    ("K01", "Où sont les points hauts du MNT ?"),
    (
        "K02",
        "Donne-moi les 10 sommets les plus hauts du terrain qui ont une proéminence d'au moins 5 m.",
    ),
    // raster.reclassify, sur la pente du MNT
    ("Z01", "Où la pente du terrain dépasse-t-elle 15 % ?"),
    (
        "Z02",
        "Range la pente du MNT en degrés en trois classes : moins de 3°, de 3° à 10°, plus de 10°.",
    ),
    // raster.weighted_overlay
    (
        "W01",
        "Combine l'accessibilité (poids 2) et le calme (poids 1) en une superposition pondérée, en excluant les zones inondables.",
    ),
    (
        "W02",
        "Fais une analyse multicritère de la note d'accessibilité et de la note de calme, à poids égaux.",
    ),
    // terrain.cost_distance
    (
        "O01",
        "Calcule le coût cumulé depuis le point 656 300 / 6 847 600 (Lambert-93) sur le raster de coût.",
    ),
    (
        "O02",
        "À partir des points X = 655 900, Y = 6 847 200 en Lambert-93 et X = 657 100, Y = 6 848 300 en Lambert-93, fais la carte de coût cumulé sur la couche « Coût de déplacement ».",
    ),
    // terrain.least_cost_path
    (
        "M01",
        "Trace le chemin le moins coûteux entre le point 655 950 / 6 847 150 (Lambert-93) et le point 657 250 / 6 848 450 (Lambert-93) sur le raster de coût.",
    ),
    (
        "M02",
        "Quel itinéraire de moindre coût relie X = 656 100, Y = 6 848 200 en Lambert-93 à X = 657 300, Y = 6 847 100 en Lambert-93, d'après le raster de coût ?",
    ),
    // layout.export_pdf au format
    ("F01", "Exporte la carte en A3 portrait."),
    (
        "F02",
        "Sors la carte en PDF au format A2 avec un carroyage Lambert-93 tous les 1 km.",
    ),
];

/// Un carroyage qui ne nomme pas son système : la réponse attendue est une question.
const GRID_QUESTION: [(&str, &str); 1] = [(
    "F03",
    "Fais un PDF de la carte avec un carroyage UTM au kilomètre.",
)];

/// Les quatorze commandes mesurées, dans l'ordre de `SENTENCES`.
const COMMANDS: [&str; 14] = [
    "coord.convert",
    "data.transform_point",
    "geo.distance_azimuth",
    "geo.destination_point",
    "geo.bearing_intersection",
    "terrain.aspect",
    "terrain.hillshade",
    "terrain.viewshed_cumulative",
    "terrain.peaks",
    "raster.reclassify",
    "raster.weighted_overlay",
    "terrain.cost_distance",
    "terrain.least_cost_path",
    "layout.export_pdf",
];

/// La commande que mesure la phrase `id` : les phrases vont par deux, dans l'ordre de `COMMANDS`.
fn command(id: &str) -> &'static str {
    let index = SENTENCES
        .iter()
        .position(|(i, _)| *i == id)
        .unwrap_or_else(|| panic!("phrase inconnue : {id}"));
    COMMANDS[index / 2]
}

fn lambert(x: u32, y: u32) -> Value {
    json!({"x": x, "y": y, "epsg": 2154})
}

fn utm31(x: u32, y: u32) -> Value {
    json!({"x": x, "y": y, "epsg": 32631})
}

fn wgs84(lon: f64, lat: f64) -> Value {
    json!({"x": lon, "y": lat, "epsg": 4326})
}

fn meters(value: u32) -> Value {
    json!({"value": value, "unit": "m"})
}

fn degrees(value: u32) -> Value {
    json!({"value": value, "unit": "deg"})
}

/// Plan attendu d'une phrase de l'essai, titres d'export retirés.
fn expected(id: &str) -> Value {
    match id {
        "G01" => json!([{"command": "coord.convert", "params": {
            "point": wgs84(2.4087, 48.7351), "to": ["mgrs"]}}]),
        "G02" => json!([{"command": "coord.convert", "params": {
            "mgrs": "31UDQ4825111750", "to": ["wgs84"]}}]),
        "P01" => json!([{"command": "data.transform_point", "params": {
            "point": lambert(656400, 6847700), "target_epsg": 4326}}]),
        "P02" => json!([{"command": "data.transform_point", "params": {
            "point": wgs84(2.423, 48.742), "target_epsg": 2154}}]),
        "D01" => json!([{"command": "geo.distance_azimuth", "params": {
            "from": wgs84(2.4, 48.73), "to": wgs84(2.44, 48.75)}}]),
        "D02" => json!([{"command": "geo.distance_azimuth", "params": {
            "from": utm31(452300, 5397800), "to": utm31(455100, 5401200), "unit": "gon"}}]),
        "R01" => json!([{"command": "geo.destination_point", "params": {
            "from": wgs84(2.415, 48.735), "azimuth": degrees(45),
            "distance": {"value": 3, "unit": "km"}}}]),
        "R02" => json!([{"command": "geo.destination_point", "params": {
            "from": utm31(452300, 5397800), "azimuth": degrees(120), "distance": meters(1500)}}]),
        "B01" => json!([{"command": "geo.bearing_intersection", "params": {
            "first": wgs84(2.39, 48.72), "first_azimuth": degrees(60),
            "second": wgs84(2.45, 48.715), "second_azimuth": degrees(330)}}]),
        "B02" => json!([{"command": "geo.bearing_intersection", "params": {
            "first": utm31(452000, 5397000), "first_azimuth": degrees(15),
            "second": utm31(456000, 5398000), "second_azimuth": degrees(300)}}]),
        "A01" | "A02" => json!([{"command": "terrain.aspect", "params": {"dem": MNT}}]),
        "H01" => json!([{"command": "terrain.hillshade", "params": {"dem": MNT}}]),
        "H02" => json!([{"command": "terrain.hillshade", "params": {
            "dem": MNT, "sun_azimuth": degrees(270), "sun_altitude": degrees(30)}}]),
        "C01" => json!([{"command": "terrain.viewshed_cumulative", "params": {
            "dem": MNT, "observers": [lambert(656200, 6847400), lambert(657000, 6848100),
                lambert(655800, 6848500)]}}]),
        "C02" => json!([{"command": "terrain.viewshed_cumulative", "params": {
            "dem": MNT, "observers": [lambert(656500, 6847900), lambert(657200, 6847300)],
            "radius": {"value": 1.5, "unit": "km"}}}]),
        "K01" => json!([{"command": "terrain.peaks", "params": {"dem": MNT}}]),
        "K02" => json!([{"command": "terrain.peaks", "params": {
            "dem": MNT, "max_count": 10, "min_prominence": meters(5)}}]),
        "Z01" => json!([
            {"command": "terrain.slope", "params": {"dem": MNT, "unit": "percent"}},
            {"command": "raster.reclassify", "params": {
                "raster": "step:1", "classes": [{"min": 15, "value": 1}]}}
        ]),
        "Z02" => json!([
            {"command": "terrain.slope", "params": {"dem": MNT, "unit": "degrees"}},
            {"command": "raster.reclassify", "params": {"raster": "step:1", "classes": [
                {"max": 3, "value": 1}, {"min": 3, "max": 10, "value": 2},
                {"min": 10, "value": 3}]}}
        ]),
        "W01" => json!([{"command": "raster.weighted_overlay", "params": {
            "criteria": [{"raster": ACCESSIBILITE, "weight": 2}, {"raster": CALME, "weight": 1}],
            "exclusions": [INONDABLES]}}]),
        "W02" => json!([{"command": "raster.weighted_overlay", "params": {
            "criteria": [{"raster": ACCESSIBILITE, "weight": 1}, {"raster": CALME, "weight": 1}]}}]),
        "O01" => json!([{"command": "terrain.cost_distance", "params": {
            "cost": COUT, "sources": [lambert(656300, 6847600)]}}]),
        "O02" => json!([{"command": "terrain.cost_distance", "params": {
            "cost": COUT, "sources": [lambert(655900, 6847200), lambert(657100, 6848300)]}}]),
        "M01" => json!([{"command": "terrain.least_cost_path", "params": {
            "cost": COUT, "start": lambert(655950, 6847150), "end": lambert(657250, 6848450)}}]),
        "M02" => json!([{"command": "terrain.least_cost_path", "params": {
            "cost": COUT, "start": lambert(656100, 6848200), "end": lambert(657300, 6847100)}}]),
        "F01" => json!([{"command": "layout.export_pdf", "params": {
            "page_size": "a3", "orientation": "portrait"}}]),
        "F02" => json!([{"command": "layout.export_pdf", "params": {
            "page_size": "a2",
            "grid": {"kind": "projected", "epsg": 2154, "interval": {"value": 1, "unit": "km"}}}}]),
        _ => unreachable!("phrase inconnue : {id}"),
    }
}

/// Un raster de l'inventaire, dans la forme des inventaires de QGIS (celle du MNT de l'épreuve).
fn raster(layer_id: &str, name: &str, source: &str) -> Value {
    json!({"layer_id": layer_id, "name": name, "kind": "raster", "geometry": null,
        "epsg": 2154, "feature_count": null, "source": source, "fields": []})
}

/// L'inventaire de l'épreuve, lu tel quel, avec en mémoire un raster de coût, deux critères
/// notés et un masque.
fn inventory() -> Value {
    let mut inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let layers = inventory["layers"].as_array_mut().unwrap();
    layers.extend([
        raster(
            "cout_deplacement_4e2a91c7_3b58_4f06_a7d1_92c0e5b8f314",
            "Coût de déplacement",
            COUT,
        ),
        raster(
            "note_accessibilite_8b31d0f4_6a27_4c95_b0e8_1d7f3a92c6e5",
            "Note d'accessibilité",
            ACCESSIBILITE,
        ),
        raster(
            "note_calme_c7f05a12_9e43_4b8d_a6c1_5e29b0d4f873",
            "Calme",
            CALME,
        ),
        raster(
            "zones_inondables_masque_2d94e6b1_7c30_4a5f_9b82_e0f15c3a7d46",
            "Zones inondables (masque)",
            INONDABLES,
        ),
    ]);
    inventory
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
fn epic11_reel_every_command_has_at_least_one_matching_plan() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    // Comme dans le produit : identifiants courts montrés, plan retraduit (ticket 5.19).
    let (inventory, ids) = common::shown_inventory(&inventory());
    let mut generator = model_files_generator(files);
    let mut scores = vec![0usize; COMMANDS.len()];
    let mut right = 0;
    for (id, text) in SENTENCES {
        let want = expected(id);
        let got = common::restored(
            compose_plan_with(style, &registry, &inventory, text, &mut generator),
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
        let index = COMMANDS.iter().position(|c| *c == command(id)).unwrap();
        scores[index] += usize::from(ok);
        right += usize::from(ok);
    }
    let mut asked = true;
    for (id, text) in GRID_QUESTION {
        let got = common::restored(
            compose_plan_with(style, &registry, &inventory, text, &mut generator),
            &ids,
        );
        let ok = matches!(&got, Ok(Plan::Question { .. }));
        eprintln!(
            "{} {id} {text}
     attendu : une question sur le système du carroyage
     obtenu  : {got:?}",
            if ok { "OK  " } else { "FAIL" }
        );
        asked &= ok;
    }
    for (command, score) in COMMANDS.iter().zip(&scores) {
        eprintln!("COMMANDE {command:<28} {score}/2");
    }
    let total = SENTENCES.len();
    eprintln!(
        "SCORE {right}/{total} plans conformes ; carroyage sans système : {}",
        if asked { "question" } else { "pas de question" }
    );
    let unlearned: Vec<&str> = COMMANDS
        .iter()
        .zip(&scores)
        .filter(|(_, s)| **s == 0)
        .map(|(c, _)| *c)
        .collect();
    let failures: Vec<String> = [
        (!unlearned.is_empty()).then(|| format!("commandes non apprises : {unlearned:?}")),
        (!asked).then(|| "le carroyage sans système n'a pas rendu de question".to_string()),
    ]
    .into_iter()
    .flatten()
    .collect();
    assert!(failures.is_empty(), "{}", failures.join(" ; "));
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
fn each_command_has_two_sentences_whose_plan_uses_it() {
    for (index, command) in COMMANDS.iter().enumerate() {
        for (id, _) in &SENTENCES[index * 2..index * 2 + 2] {
            let calls = expected(id);
            assert!(
                calls
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|c| c["command"] == *command),
                "{id} : {command}"
            );
        }
    }
}

#[test]
fn every_raster_of_a_plan_is_the_source_of_an_inventory_raster() {
    let inventory = inventory();
    let sources: Vec<&str> = inventory["layers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["kind"] == "raster")
        .map(|l| l["source"].as_str().unwrap())
        .collect();
    assert_eq!(sources, [MNT, COUT, ACCESSIBILITE, CALME, INONDABLES]);
    for (id, _) in SENTENCES {
        for call in expected(id).as_array().unwrap() {
            let params = &call["params"];
            let mut cited: Vec<&Value> = ["dem", "raster", "cost"]
                .iter()
                .filter_map(|k| params.get(*k))
                .collect();
            if let Some(criteria) = params["criteria"].as_array() {
                cited.extend(criteria.iter().map(|c| &c["raster"]));
            }
            if let Some(exclusions) = params["exclusions"].as_array() {
                cited.extend(exclusions);
            }
            for raster in cited {
                let raster = raster.as_str().unwrap();
                assert!(
                    raster.starts_with("step:") || sources.contains(&raster),
                    "{id} : {raster}"
                );
            }
        }
    }
}

#[test]
fn the_grid_question_names_no_system() {
    for (_, text) in GRID_QUESTION {
        let text = text.to_lowercase();
        assert!(text.contains("carroyage"));
        for system in [
            "lambert", "2154", "utm 3", "30n", "31n", "32n", "zone", "fuseau", "wgs", "4326",
            "epsg", "mgrs", "degré", "grade", "°",
        ] {
            assert!(!text.contains(system), "{system}");
        }
    }
}

#[test]
fn an_export_without_a_title_does_not_match() {
    let untitled = json!([{"command": "layout.export_pdf", "params": {"page_size": "a3"}}]);
    assert_eq!(comparable(untitled), None);
    let titled =
        json!([{"command": "layout.export_pdf", "params": {"title": "Carte", "page_size": "a3"}}]);
    assert_eq!(
        comparable(titled),
        Some(json!([{"command": "layout.export_pdf", "params": {"page_size": "a3"}}]))
    );
}
