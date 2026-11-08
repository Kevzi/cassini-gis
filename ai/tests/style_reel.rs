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
//! (`common::completed`), qui ne change rien ici. Seuil (ticket 11.23) : au moins 9 sur 10.

mod common;

use cassini_ai::plan::{compose_plan_with, model_files_generator};
use cassini_core::plan::Plan;
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

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn style_reel_at_least_9_of_10_answers_match() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    let full: Value = serde_json::from_str(INVENTORY).unwrap();
    // Comme dans le produit : identifiants courts montrés, plan retraduit (ticket 5.19), puis
    // complété (11.22).
    let (inventory, ids) = common::shown_inventory(&full);
    let mut generator = model_files_generator(files);
    let mut right = 0;
    for (id, text) in SENTENCES {
        let want = expected(id);
        let got = common::completed(
            &registry,
            common::restored(
                compose_plan_with(style, &registry, &inventory, text, &mut generator),
                &ids,
            ),
            &full,
        );
        let ok = match (&got, &want) {
            (Ok(Plan::Calls { calls }), Want::Plan(plan)) => {
                serde_json::to_value(calls).unwrap() == *plan
            }
            (Ok(Plan::Question { .. }), Want::Question) => true,
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
    }
    let total = SENTENCES.len();
    eprintln!("SCORE {right}/{total} réponses conformes");
    assert!(right >= 9, "{right}/{total} : sous le seuil de 9 sur 10");
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
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    for (id, _) in SENTENCES {
        let Want::Plan(calls) = expected(id) else {
            continue;
        };
        let params = &calls[0]["params"];
        if calls[0]["command"] == "map.topographic_style" {
            for l in params["layers"].as_array().into_iter().flatten() {
                layer(&inventory, l.as_str().unwrap());
            }
            continue;
        }
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
