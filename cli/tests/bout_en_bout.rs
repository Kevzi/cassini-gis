//! Suite de bout en bout au vrai modèle (ticket 5.9, point 3 du Done when de l'epic 5), sur le vrai
//! binaire : la phrase de démo de `CLAUDE.md` devient un plan par `plan.propose`, le plan est
//! exécuté par `plan.execute` et répondu par une fausse extension, puis le journal est rejoué dans
//! un second `cassini serve` dont le dossier des modèles est vide : le rejeu se passe de l'IA.
//! Ignoré sans `CASSINI_TEST_MODEL` (et marqué `ignore` : il charge le vrai modèle).
//!
//!   CASSINI_TEST_MODEL=<gguf ou déclaration *.modele.json> \
//!     cargo test -p cassini --release --features gpu --test bout_en_bout -- --ignored --nocapture
//!
//! Le modèle est pris dans son dossier (`CASSINI_MODELS`), choisi par son nom de fichier.
//! L'inventaire est celui de l'épreuve du modèle local (`tests/epreuve-modele-local/inventory.json`),
//! traduit dans la forme que rend `project.inventory`. Aucune donnée réelle, aucun QGIS.

#[path = "support/qgis_session.rs"]
mod qgis_session;
#[path = "support/reseau.rs"]
mod reseau;

use std::path::PathBuf;
use std::time::Duration;

use qgis_session::QgisSession;
use serde_json::{json, Value};

const INVENTORY: &str = include_str!("../../tests/epreuve-modele-local/inventory.json");

/// La phrase de démo exacte de `CLAUDE.md` (A0 de `duree_reel`).
const DEMO: &str = "Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi et sors-moi la carte en PDF.";

/// Commandes que la fausse extension annonce.
const ANNOUNCED: [&str; 6] = [
    "analysis.select_by_attribute",
    "analysis.buffer",
    "analysis.intersect",
    "layout.export_pdf",
    "layer.describe",
    "project.inventory",
];

const PARCELLES: &str = "parcelles_3f9c2a1e_7b40_4d5a_9e12_0c8a6d4b71f3";
const PLU: &str = "plu_zonage_a81d5c06_2e9f_4a37_b6c4_5f13e07d92a8";
const GARES: &str = "gares_c5e0b94d_1a67_4f82_8d03_9b2e6a7c40f1";

/// Une composition au vrai modèle, chargement compris, peut être longue sur processeur.
const COMPOSING: Duration = Duration::from_secs(900);

/// L'inventaire de l'épreuve dans la forme du résultat de `project.inventory`.
fn inventory() -> Value {
    let source: Value = serde_json::from_str(INVENTORY).unwrap();
    let layers: Vec<Value> = source["layers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            let mut layer = json!({
                "layer_id": l["layer_id"], "name": l["name"], "source": l["source"],
                "crs": format!("EPSG:{}", l["epsg"]), "kind": l["kind"], "fields": l["fields"],
            });
            if !l["geometry"].is_null() {
                layer["geometry"] = l["geometry"].clone();
            }
            if !l["feature_count"].is_null() {
                layer["feature_count"] = l["feature_count"].clone();
            }
            layer
        })
        .collect();
    json!({
        "project_name": source["project"]["name"],
        "layers": layers,
        "project_crs": format!("EPSG:{}", source["project"]["epsg"]),
        "extent": null,
    })
}

/// Les quatre analyses attendues de la phrase de démo ; la cinquième étape est l'export PDF, dont
/// le titre est libre mais non vide.
fn expected_analyses() -> Value {
    json!([
        {"command": "analysis.select_by_attribute",
            "params": {"layer_id": PLU, "field": "typezone", "operator": "eq", "value": "U"}},
        {"command": "analysis.buffer",
            "params": {"layer_id": GARES, "distance": {"value": 500, "unit": "m"}}},
        {"command": "analysis.intersect",
            "params": {"layer_id": PARCELLES, "overlay_layer_id": "step:1.layer_id"}},
        {"command": "analysis.intersect",
            "params": {"layer_id": "step:3.layer_id", "overlay_layer_id": "step:2.layer_id"}},
    ])
}

/// Écarts du plan composé au plan attendu ; vide s'il est conforme.
fn gaps(plan: &Value) -> Vec<String> {
    let Some(calls) = plan["calls"].as_array().filter(|_| plan["kind"] == "calls") else {
        return vec![format!("le modèle n'a pas rendu de plan : {plan}")];
    };
    if calls.len() != 5 {
        return vec![format!("{} étapes au lieu de 5", calls.len())];
    }
    let mut found = Vec::new();
    for (i, want) in expected_analyses().as_array().unwrap().iter().enumerate() {
        // 500.0 vaut 500 : comparaison des nombres par valeur.
        let mut got = calls[i].clone();
        if let Some(v) = got["params"]["distance"]["value"].as_f64() {
            if v.fract() == 0.0 {
                got["params"]["distance"]["value"] = json!(v as u64);
            }
        }
        if got != *want {
            found.push(format!(
                "étape {} : attendu {want}, obtenu {}",
                i + 1,
                calls[i]
            ));
        }
    }
    let title = calls[4]["params"]["title"].as_str().unwrap_or("");
    if calls[4]["command"] != "layout.export_pdf" || title.trim().is_empty() {
        found.push(format!(
            "étape 5 : un export PDF titré attendu, obtenu {}",
            calls[4]
        ));
    }
    found
}

/// Fausse extension : écrit la sortie nommée par le noyau, rend le résultat typé de l'étape.
fn answer_step(session: &mut QgisSession, order: &Value, prefix: &str) {
    let params = &order["params"]["params"];
    if let Some(out) = params["out"].as_str() {
        std::fs::write(out, b"sortie de la fausse extension").unwrap();
    }
    let step = order["id"].as_str().unwrap_or("?").to_string();
    let result = match order["params"]["command"].as_str().unwrap() {
        "project.inventory" => inventory(),
        "layout.export_pdf" => json!({"file": params["out"], "page_count": 1, "layer_count": 6,
            "sources": ["parcelles.gpkg", "plu_zonage.gpkg", "gares.gpkg"], "truncated": false}),
        _ => json!({"layer_id": format!("{prefix}_{step}"), "name": "Résultat",
            "feature_count": 7, "crs": "EPSG:2154"}),
    };
    session.answer(order, result);
}

/// Un dossier vide sous le dossier temporaire.
fn empty_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cassini-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn the_demo_sentence_runs_on_the_real_model_and_replays_without_it() {
    let Some(model) = std::env::var_os("CASSINI_TEST_MODEL").map(PathBuf::from) else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignoré.");
        return;
    };
    // Un nom de fichier nu n'a pas de dossier utilisable : le chemin est d'abord rendu absolu.
    let model = std::fs::canonicalize(&model).unwrap_or_else(|e| {
        panic!(
            "le modèle de CASSINI_TEST_MODEL est introuvable : {} ({e})",
            model.display()
        )
    });
    let folder = model.parent().expect("le modèle est dans un dossier");
    let name = model.file_name().unwrap().to_string_lossy().into_owned();
    let mut session = QgisSession::start(false, &folder.to_string_lossy(), &ANNOUNCED);

    // 1. La phrase devient un plan ; l'inventaire est demandé à la fausse extension.
    let started = std::time::Instant::now();
    let propose = session.start_request("plan.propose", json!({"request": DEMO, "model": name}));
    let proposed = loop {
        let message = session.wait_for_within("plan.propose", COMPOSING, |m| {
            m["method"] == "order.execute"
                || (m["id"] == json!(propose) && m.get("method").is_none())
        });
        if message["method"] != "order.execute" {
            break message;
        }
        assert_eq!(
            message["params"]["command"], "project.inventory",
            "{message}"
        );
        answer_step(&mut session, &message, "inv");
    };
    eprintln!(
        "Composition : {:.1} s\nPlan : {proposed}",
        started.elapsed().as_secs_f64()
    );
    // Question, refus ou erreur : la réponse entière est montrée.
    let plan = &proposed["result"]["plan"];
    assert!(
        plan.is_object(),
        "le modèle n'a pas rendu de plan : {proposed}"
    );
    let found = gaps(plan);
    assert!(
        found.is_empty(),
        "plan non conforme : {}",
        found.join(" ; ")
    );
    // Le modèle est chargé dans le processus : toujours aucune socket réseau.
    reseau::assert_offline("cassini serve (après la composition)", session.pid());

    // 2. Le plan validé est exécuté, un ordre à la fois.
    let plan_id = proposed["result"]["plan_id"].clone();
    let execute = session.start_request("plan.execute", json!({"plan_id": plan_id}));
    let calls = plan["calls"].as_array().unwrap().clone();
    for call in &calls {
        // Le tampon : le noyau lit d'abord l'inventaire (système de calcul, entrée 9.16).
        let order = if call["command"] == "analysis.buffer" {
            session.wait_buffer_order(GARES, "gares", "EPSG:4326", "deg")
        } else {
            session.wait_order()
        };
        assert_eq!(order["params"]["command"], call["command"], "{order}");
        answer_step(&mut session, &order, "l");
    }
    let done = session.wait_reply("plan.execute", execute);
    assert_eq!(
        done["result"]["steps"].as_array().map(Vec::len),
        Some(5),
        "{done}"
    );
    let journal = session.journal();
    assert_eq!(journal.len(), 5, "{journal:?}");
    // Le journal garde les ids résolus, jamais une référence `step:k`.
    assert!(
        journal.iter().all(|l| !l.to_string().contains("step:")),
        "{journal:?}"
    );
    let before = std::fs::read(session.journal_path()).unwrap();

    // 3. Rejeu dans une seconde session dont le dossier des modèles est vide.
    let empty = empty_dir("sans-modele");
    let mut again = session.reopen(&empty.to_string_lossy());
    let models = again.request("agent.models", json!({}));
    eprintln!("Modèles de la seconde session : {models}");
    assert_eq!(
        models["result"]["models"],
        json!([]),
        "la seconde session ne doit voir aucun modèle : {models}"
    );
    let replay = again.start_request("journal.replay", json!({}));
    for _ in 0..5 {
        let order = again.wait_order();
        answer_step(&mut again, &order, "r");
    }
    let replayed = again.wait_reply("journal.replay", replay);
    eprintln!("Rejeu sans modèle : {replayed}");
    assert_eq!(
        replayed["result"],
        json!({"steps": 5, "identical": true}),
        "{replayed}"
    );
    assert_eq!(std::fs::read(again.journal_path()).unwrap(), before);
    reseau::assert_offline("cassini serve (rejeu)", again.pid());
    again.close();
    let _ = std::fs::remove_dir_all(empty);
}

/// Sans modèle : la traduction de l'inventaire et le notateur du plan.
#[test]
fn the_test_inventory_and_grader_hold() {
    let inv = inventory();
    assert_eq!(inv["project_crs"], "EPSG:2154");
    assert!(inv["layers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["layer_id"] == PLU));
    let mut calls = expected_analyses().as_array().unwrap().clone();
    calls.push(json!({"command": "layout.export_pdf", "params": {"title": "Carte"}}));
    let mut plan = json!({"kind": "calls", "calls": calls});
    assert!(gaps(&plan).is_empty());
    plan["calls"][1]["params"]["distance"]["value"] = json!(500.0);
    assert!(gaps(&plan).is_empty());
    plan["calls"][4]["params"]["title"] = json!(" ");
    assert_eq!(gaps(&plan).len(), 1);
    assert_eq!(gaps(&json!({"question": "Laquelle ?"})).len(), 1);
}
