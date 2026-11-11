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
//!
//! Depuis 11.23, le plan mesuré est le **plan complété** : le noyau ajoute les conversions de
//! points avant de montrer le plan (11.22), et l'essai applique la même complétion
//! (`common::completed`). Quatre phrases de plus (`FOREIGN_POINTS`, report de la relecture de
//! 11.16) donnent un point dans un autre système que le raster, ou passent d'un fuseau UTM à
//! l'autre : elles sont mesurées et imprimées à part, hors du score des quatorze commandes et de
//! sa barre (27 / 28).
//!
//! Depuis 5.53 (livrable 3, lot 3), la barre est élargie en place : aux 28 phrases de
//! `SENTENCES` et à F03 s'ajoutent 20 phrases neuves écrites à la main (`epic11_reel/phrases.json`,
//! schéma de `par_commande/phrases.json` ; une phrase qui attend un plan porte `command`, une
//! question ou un refus n'en porte pas), soit 49 : un plan neuf par commande (14), 3 questions
//! (point sans système, destination sans azimut, chemin sans arrivée) et 3 refus (accumulation
//! des écoulements, lissage du MNT, rugosité), qui nomment dans `missing` ce que le registre n'a
//! pas ; le jour où un identifiant du registre contient un de ces mots, le refus devient faux et
//! le test sans modèle nomme la phrase. Les attendus passent tous les contrôles communs
//! (`common::invalid`) sur l'inventaire augmenté. Les 28 phrases de `SENTENCES` gardent leur
//! comparaison (`comparable`, titre seulement non vide) ; les neuves celle des autres barres
//! (`common::same_calls`). Un seul `SCORE`, sur le total de la barre (F03 compris, les quatre
//! `FOREIGN_POINTS` toujours hors barre). Seuil (décision du coordinateur, 2026-11-11, D4) :
//! 27/28 rapporté au total, arrondi vers le bas, soit 47/49 ; F03 garde en plus son exigence
//! dure (sans question, l'essai échoue, quel que soit le total). Plus de règle par commande :
//! les lignes `COMMANDE …` restent imprimées sans seuil, avec un avertissement non bloquant pour
//! une commande sans aucun plan juste. Le jeu d'entraînement ne voit aucune de ces
//! phrases : `training/tests/recouvrement.rs` les garde.

mod common;

use cassini_ai::plan::{compose_plan_with, model_files_generator, AgentError};
use cassini_core::plan::{Plan, QuestionSource};
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

/// Des points écrits dans un autre système que le raster, et un passage d'un fuseau UTM à
/// l'autre : mesurés à part, sur le plan complété.
const FOREIGN_POINTS: [(&str, &str); 4] = [
    (
        "Y01",
        "Combien de ces deux points voient chaque endroit du MNT : le point 48,7380 N, 2,4120 E (WGS84) et le point 48,7330 N, 2,4210 E (WGS84) ?",
    ),
    (
        "Y02",
        "Coût cumulé sur le raster de coût au départ du point 455 800 / 5 398 300 (UTM 31N).",
    ),
    (
        "Y03",
        "Chemin de moindre coût sur le raster de coût, du point 48,7290 N, 2,4050 E (WGS84) au point 657 100 / 6 848 000 (Lambert-93).",
    ),
    (
        "Y04",
        "Quelles sont les coordonnées UTM 32N du point 455 800 / 5 398 300 (UTM 31N) ?",
    ),
];

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

/// Le plan qu'écrit le modèle pour une phrase de `FOREIGN_POINTS` : les points recopiés tels que
/// la phrase les donne (11.23).
fn foreign_as_written(id: &str) -> Value {
    match id {
        "Y01" => json!([{"command": "terrain.viewshed_cumulative", "params": {
            "dem": MNT, "observers": [wgs84(2.412, 48.738), wgs84(2.421, 48.733)]}}]),
        "Y02" => json!([{"command": "terrain.cost_distance", "params": {
            "cost": COUT, "sources": [utm31(455800, 5398300)]}}]),
        "Y03" => json!([{"command": "terrain.least_cost_path", "params": {
            "cost": COUT, "start": wgs84(2.405, 48.729), "end": lambert(657100, 6848000)}}]),
        "Y04" => json!([{"command": "coord.convert", "params": {
            "point": utm31(455800, 5398300), "to": ["utm"], "utm_zone": 32}}]),
        _ => unreachable!("phrase inconnue : {id}"),
    }
}

/// Le plan complété attendu d'une phrase de `FOREIGN_POINTS` : une conversion vers le système du
/// raster (Lambert-93) juste avant la commande, pour chaque point d'un autre système.
fn foreign_expected(id: &str) -> Value {
    let to_lambert = |point: Value| json!({"command": "data.transform_point", "params": {"point": point, "target_epsg": 2154}});
    match id {
        "Y01" => json!([
            to_lambert(wgs84(2.412, 48.738)),
            to_lambert(wgs84(2.421, 48.733)),
            {"command": "terrain.viewshed_cumulative", "params": {
                "dem": MNT, "observers": ["step:1.point", "step:2.point"]}}
        ]),
        "Y02" => json!([
            to_lambert(utm31(455800, 5398300)),
            {"command": "terrain.cost_distance", "params": {
                "cost": COUT, "sources": ["step:1.point"]}}
        ]),
        "Y03" => json!([
            to_lambert(wgs84(2.405, 48.729)),
            {"command": "terrain.least_cost_path", "params": {
                "cost": COUT, "start": "step:1.point", "end": lambert(657100, 6848000)}}
        ]),
        _ => foreign_as_written(id),
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

/// Les phrases neuves de la barre (5.53, livrable 3), écrites à la main.
const PHRASES: &str = include_str!("epic11_reel/phrases.json");

/// Les phrases neuves de `epic11_reel/phrases.json`.
fn new_phrases() -> Vec<Value> {
    common::read_phrases(PHRASES, "epic11_reel/phrases.json")
}

/// Toute la barre, dans la forme des `phrases.json` : les 28 phrases de `SENTENCES` (attendu de
/// `expected`, titre d'export retiré, `exact` : comparées par `comparable`), F03 (une question),
/// puis les phrases neuves.
fn bar() -> Vec<Value> {
    let mut all: Vec<Value> = SENTENCES
        .iter()
        .map(|(id, text)| {
            json!({"id": id, "command": command(id), "phrase": text,
                "expected": {"kind": "calls", "calls": expected(id)}, "exact": true})
        })
        .collect();
    all.extend(
        GRID_QUESTION
            .iter()
            .map(|(id, text)| json!({"id": id, "phrase": text, "expected": {"kind": "question"}})),
    );
    all.extend(new_phrases());
    all
}

/// La réponse rendue à une phrase de la barre est-elle l'attendue : pour `SENTENCES` (`exact`),
/// les appels de `expected` à l'identique, titre d'export seulement non vide (`comparable`) ;
/// pour les neuves, les appels par `common::same_calls` ; une question (du modèle ou du
/// garde-fou) là où une question est attendue, un refus là où un refus l'est.
fn answers(phrase: &Value, got: &Result<Plan, AgentError>) -> bool {
    let want = &phrase["expected"];
    match (want["kind"].as_str(), got) {
        (Some("calls"), Ok(Plan::Calls { calls })) if phrase["exact"] == true => {
            comparable(serde_json::to_value(calls).unwrap()).is_some_and(|c| c == want["calls"])
        }
        (Some("calls"), Ok(Plan::Calls { calls })) => common::same_calls(calls, &want["calls"]),
        (Some("question"), Ok(Plan::Question { .. })) => true,
        (Some("refusal"), Err(AgentError::Refused(_))) => true,
        _ => false,
    }
}

/// Réponses attendues exigées : 27/28 du total de la barre, arrondi vers le bas (D4).
fn answers_needed(total: usize) -> usize {
    common::proportional(total, 27, 28)
}

/// Plans mesurés par commande sur la barre : ceux qui portent la commande en `command`.
fn plans_per_command(bar: &[Value]) -> Vec<usize> {
    COMMANDS
        .iter()
        .map(|c| {
            bar.iter()
                .filter(|p| p["command"] == *c && p["expected"]["kind"] == "calls")
                .count()
        })
        .collect()
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn epic11_reel_at_least_27_of_28_rounded_down_and_the_grid_question_asked() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    // Comme dans le produit : identifiants courts montrés, plan retraduit (ticket 5.19), puis
    // complété par les conversions de points (11.22, mesuré depuis 11.23).
    let full = inventory();
    let (_, ids) = common::shown_inventory(&full);
    let mut generator = model_files_generator(files);
    let all = bar();
    let counts = plans_per_command(&all);
    let mut scores = vec![0usize; COMMANDS.len()];
    let (mut right, mut guarded, mut asked) = (0, 0, true);
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
        if let Some(index) = COMMANDS.iter().position(|c| phrase["command"] == *c) {
            scores[index] += usize::from(ok);
        }
        // F03 : exigence dure en plus de compter dans le total.
        if GRID_QUESTION.iter().any(|(g, _)| *g == id) {
            asked &= ok;
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
    let mut foreign = 0;
    for (id, text) in FOREIGN_POINTS {
        let want = foreign_expected(id);
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
        let ok = matches!(&got, Ok(Plan::Calls { calls })
            if serde_json::to_value(calls).unwrap() == want);
        eprintln!(
            "{} {id} {text}
     attendu : {want}
     obtenu  : {got:?}",
            if ok { "OK  " } else { "FAIL" }
        );
        if let Ok(plan) = &got {
            assert_eq!(registry.check_plan(plan), Ok(()), "{id}");
        }
        foreign += usize::from(ok);
    }
    // Imprimées sans seuil ; jamais `SCORE <commande>`, réservé à `par_commande_reel`.
    for ((command, score), count) in COMMANDS.iter().zip(&scores).zip(&counts) {
        eprintln!("COMMANDE {command:<28} {score}/{count}");
        if *score == 0 {
            eprintln!("attention : {command} n'a aucun plan juste (non bloquant)");
        }
    }
    let total = all.len();
    let need = answers_needed(total);
    eprintln!(
        "SCORE {right}/{total} réponses attendues ; carroyage sans système : {} ; {guarded} question(s) du garde-fou ; points d'un autre système (hors barre) : {foreign}/{}",
        if asked { "question" } else { "pas de question" },
        FOREIGN_POINTS.len()
    );
    let failures: Vec<String> = [
        (right < need).then(|| {
            format!("{right}/{total} : sous le seuil de 27/28 arrondi vers le bas ({need}/{total})")
        }),
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

#[test]
fn the_foreign_points_written_as_given_are_completed_into_the_expected_plans() {
    // 11.23 : le plan du modèle (points tels qu'écrits) est valide et passe le garde-fou ; la
    // complétion du noyau en fait l'attendu, et laisse l'attendu tel quel.
    let registry = Registry::standard();
    let inventory = inventory();
    for (id, text) in FOREIGN_POINTS {
        let raw: Plan =
            serde_json::from_value(json!({"kind": "calls", "calls": foreign_as_written(id)}))
                .unwrap();
        assert_eq!(registry.check_plan(&raw), Ok(()), "{id}");
        assert_eq!(guard_coordinates(&registry, raw.clone(), text), raw, "{id}");
        let want: Plan =
            serde_json::from_value(json!({"kind": "calls", "calls": foreign_expected(id)}))
                .unwrap();
        assert_eq!(registry.check_plan(&want), Ok(()), "{id}");
        assert_eq!(
            common::complete_with(&registry, raw, &inventory),
            want,
            "{id}"
        );
        assert_eq!(
            common::complete_with(&registry, want.clone(), &inventory),
            want,
            "{id}"
        );
    }
}

#[test]
fn every_expected_plan_is_left_unchanged_by_the_completion() {
    // Les points des phrases de `SENTENCES` sont dans le système de leur raster, ou portés par
    // une commande sans raster : la complétion n'ajoute rien à leur attendu.
    let registry = Registry::standard();
    let inventory = inventory();
    for (id, _) in SENTENCES {
        let plan = expected_plan(id);
        assert_eq!(
            common::complete_with(&registry, plan.clone(), &inventory),
            plan,
            "{id}"
        );
    }
}

#[test]
fn the_threshold_is_27_of_28_rounded_down() {
    assert_eq!(answers_needed(28), 27);
    assert_eq!(answers_needed(29), 27);
    assert_eq!(bar().len(), 49);
    assert_eq!(answers_needed(49), 47);
    assert_eq!(answers_needed(41), 39);
    assert_eq!(answers_needed(50), 48);
}

#[test]
fn the_bar_has_41_to_50_sentences_with_questions_and_refusals_naming_what_is_missing() {
    let all = bar();
    assert!((41..=50).contains(&all.len()), "{} phrases", all.len());
    let mut ids = std::collections::BTreeSet::new();
    for p in all.iter().chain(
        FOREIGN_POINTS
            .iter()
            .map(|(id, text)| json!({"id": id, "phrase": text}))
            .collect::<Vec<_>>()
            .iter(),
    ) {
        let id = p["id"].as_str().expect("phrase sans id");
        assert!(ids.insert(id.to_string()), "id en double : {id}");
        assert!(
            p["phrase"].as_str().is_some_and(|t| !t.trim().is_empty()),
            "{id} : phrase vide"
        );
    }
    let fresh = new_phrases();
    assert!(
        (12..=21).contains(&fresh.len()),
        "{} phrases neuves",
        fresh.len()
    );
    let kind = |k: &str| fresh.iter().filter(|p| p["expected"]["kind"] == k).count();
    assert!(
        kind("question") >= 2,
        "{} questions neuves",
        kind("question")
    );
    assert!(kind("refusal") >= 3, "{} refus neufs", kind("refusal"));
    for p in &fresh {
        let id = &p["id"];
        match p["expected"]["kind"].as_str() {
            // Un plan porte sa commande, une des quatorze.
            Some("calls") => assert!(
                COMMANDS.iter().any(|c| p["command"] == *c),
                "{id} : commande {}",
                p["command"]
            ),
            // Une question ou un refus n'en porte pas.
            Some("question") => assert!(p.get("command").is_none(), "{id}"),
            Some("refusal") => {
                assert!(p.get("command").is_none(), "{id}");
                assert!(
                    p["expected"]["missing"]
                        .as_array()
                        .is_some_and(|m| !m.is_empty() && m.iter().all(Value::is_string)),
                    "{id} : refus sans `missing`"
                );
            }
            other => panic!("{id} : sorte {other:?}"),
        }
    }
}

/// Les phrases neuves dont le plan attendu n'appelle pas leur `command`.
fn plans_not_calling_their_command(phrases: &[Value]) -> Vec<String> {
    phrases
        .iter()
        .filter(|p| {
            common::expected_calls(p)
                .is_some_and(|calls| !calls.iter().any(|c| c["command"] == p["command"]))
        })
        .map(|p| p["id"].as_str().unwrap_or("?").to_string())
        .collect()
}

/// Les commandes de `COMMANDS` sans plan neuf qui les appelle et les porte en `command`.
fn commands_without_a_new_plan(phrases: &[Value]) -> Vec<&'static str> {
    COMMANDS
        .iter()
        .filter(|c| {
            !phrases.iter().any(|p| {
                p["command"] == **c
                    && common::expected_calls(p)
                        .is_some_and(|calls| calls.iter().any(|call| call["command"] == **c))
            })
        })
        .copied()
        .collect()
}

#[test]
fn every_command_has_a_new_plan_that_calls_it() {
    let fresh = new_phrases();
    assert!(plans_not_calling_their_command(&fresh).is_empty());
    assert!(commands_without_a_new_plan(&fresh).is_empty());
    // Une commande privée de son plan neuf est nommée ; un plan qui n'appelle pas sa commande
    // aussi.
    let without: Vec<Value> = fresh
        .iter()
        .filter(|p| p["command"] != "terrain.peaks")
        .cloned()
        .collect();
    assert_eq!(commands_without_a_new_plan(&without), ["terrain.peaks"]);
    let mut wrong = fresh.clone();
    let p = wrong.iter_mut().find(|p| p["id"] == "EP06").unwrap();
    p["command"] = json!("terrain.hillshade");
    assert_eq!(plans_not_calling_their_command(&wrong), ["EP06"]);
    assert_eq!(commands_without_a_new_plan(&wrong), ["terrain.aspect"]);
}

#[test]
fn every_expected_answer_is_valid() {
    // `check_plan`, garde-fou des coordonnées, complétion sans effet, couches, champs, rasters
    // et fichiers présents dans l'inventaire augmenté, refus encore justes : toute la barre.
    let errors = common::invalid(&Registry::standard(), &inventory(), &bar());
    assert!(
        errors.is_empty(),
        "attendus invalides :\n{}",
        errors.join("\n")
    );
}

#[test]
fn an_invalid_expected_answer_or_a_wrong_refusal_is_named() {
    let registry = Registry::standard();
    let mut broken = bar();
    let k = broken.iter_mut().find(|p| p["id"] == "EP09").unwrap();
    k["expected"]["calls"][0]["params"]["max_count"] = json!(0);
    let w = broken.iter_mut().find(|p| p["id"] == "EP11").unwrap();
    w["expected"]["calls"][0]["params"]["criteria"][0]["raster"] = json!("donnees/note_bruit.tif");
    // Un point d'un autre système que le raster : la complétion ajouterait une conversion.
    let o = broken.iter_mut().find(|p| p["id"] == "EP12").unwrap();
    o["expected"]["calls"][0]["params"]["sources"][0] =
        json!({"x": 2.41, "y": 48.73, "epsg": 4326});
    // Un refus qui dit absente une commande du registre.
    let r = broken.iter_mut().find(|p| p["id"] == "EP19").unwrap();
    r["expected"]["missing"] = json!(["slope"]);
    let errors = common::invalid(&registry, &inventory(), &broken);
    for (id, reason) in [
        ("EP09", "check_plan refuse"),
        ("EP11", "raster absent"),
        ("EP12", "le garde-fou"),
        ("EP12", "la complétion"),
        ("EP19", "refus devenu faux"),
    ] {
        assert!(
            errors
                .iter()
                .any(|e| e.starts_with(&format!("{id} : {reason}"))),
            "{id} : {errors:?}"
        );
    }
    assert_eq!(errors.len(), 5, "{errors:?}");
}

#[test]
fn a_refusal_turns_red_when_its_command_reaches_the_registry() {
    let mut ids = common::registry_ids(&Registry::standard());
    assert!(common::refusals_now_wrong(&ids, &bar()).is_empty());
    ids.push("terrain.flow_accumulation".to_string());
    let wrong = common::refusals_now_wrong(&ids, &bar());
    assert!(!wrong.is_empty(), "{wrong:?}");
    assert!(
        wrong
            .iter()
            .all(|w| w.starts_with("EP18 : refus devenu faux")),
        "{wrong:?}"
    );
}

#[test]
fn no_sentence_names_a_file_that_is_not_open() {
    assert!(common::naming_unloaded_files(&inventory(), &bar()).is_empty());
}

/// Un plan rendu, depuis ses appels.
fn calls(value: Value) -> Result<Plan, AgentError> {
    Ok(serde_json::from_value(json!({"kind": "calls", "calls": value})).unwrap())
}

#[test]
fn the_score_of_a_new_sentence_compares_titles_numbers_and_kinds_as_documented() {
    let fresh = new_phrases();
    let find = |id: &str| fresh.iter().find(|p| p["id"] == id).unwrap().clone();
    let asked: Result<Plan, AgentError> = Ok(Plan::Question {
        question: "Quel système ?".into(),
        source: QuestionSource::Guard,
    });
    let refused: Result<Plan, AgentError> = Err(AgentError::Refused("non".into()));
    // Titre d'export libre mais non vide.
    let ep14 = find("EP14");
    let mut titled = ep14["expected"]["calls"].clone();
    titled[0]["params"]["title"] = json!("Carte");
    assert!(answers(&ep14, &calls(titled.clone())));
    titled[0]["params"]["title"] = json!(" ");
    assert!(!answers(&ep14, &calls(titled)));
    // Un entier écrit en flottant vaut l'entier ; une option de plus est un échec.
    let ep09 = find("EP09");
    let mut floats = ep09["expected"]["calls"].clone();
    floats[0]["params"]["max_count"] = json!(3.0);
    assert!(answers(&ep09, &calls(floats.clone())));
    floats[0]["params"]["min_prominence"] = json!({"value": 5, "unit": "m"});
    assert!(!answers(&ep09, &calls(floats)));
    // L'ordre des critères écrit compte.
    let ep11 = find("EP11");
    let mut swapped = ep11["expected"]["calls"].clone();
    let criteria = swapped[0]["params"]["criteria"].as_array_mut().unwrap();
    criteria.reverse();
    assert!(!answers(&ep11, &calls(swapped)));
    // Une question, du garde-fou compris, pour une question ; un refus pour un refus.
    assert!(answers(&find("EP15"), &asked));
    assert!(!answers(&find("EP15"), &refused));
    assert!(answers(&find("EP18"), &refused));
    assert!(!answers(&find("EP18"), &asked));
    assert!(!answers(&ep09, &asked));
    assert!(!answers(
        &find("EP18"),
        &calls(ep09["expected"]["calls"].clone())
    ));
}

#[test]
fn every_command_is_measured_on_its_two_sentences_and_its_new_plan() {
    assert_eq!(plans_per_command(&bar()), vec![3; COMMANDS.len()]);
}

#[test]
fn the_original_sentences_are_scored_by_comparable() {
    let all = bar();
    let f01 = all.iter().find(|p| p["id"] == "F01").unwrap().clone();
    let mut titled = f01["expected"]["calls"].clone();
    titled[0]["params"]["title"] = json!("Carte");
    assert!(answers(&f01, &calls(titled.clone())));
    // Sans titre : `comparable` le refuse.
    assert!(!answers(&f01, &calls(f01["expected"]["calls"].clone())));
    // Un nombre écrit en flottant compte faux sur les phrases d'origine (comparaison exacte).
    let k02 = all.iter().find(|p| p["id"] == "K02").unwrap().clone();
    let mut floats = k02["expected"]["calls"].clone();
    assert!(answers(&k02, &calls(floats.clone())));
    floats[0]["params"]["max_count"] = json!(10.0);
    assert!(!answers(&k02, &calls(floats)));
    // F03 : une question.
    let f03 = all.iter().find(|p| p["id"] == "F03").unwrap().clone();
    let asked: Result<Plan, AgentError> = Ok(Plan::Question {
        question: "Quel système ?".into(),
        source: QuestionSource::Model,
    });
    assert!(answers(&f03, &asked));
    assert!(!answers(&f03, &calls(titled)));
}
