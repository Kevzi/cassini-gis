//! Barre de mesure par commande (ticket 5.53, livrable 1) : chaque commande offerte au modèle
//! local a au moins trois phrases écrites à la main (`par_commande/phrases.json`), dont au moins
//! deux attendent un plan qui l'appelle ; la troisième peut attendre une question ou un refus.
//! Les phrases ne vont jamais au jeu d'entraînement : `training/tests/recouvrement.rs` les garde.
//!
//! Sans modèle, les tests vérifient la couverture (une commande offerte, ajoutée plus tard
//! comprise, sans ses phrases fait échouer) et chaque attendu : `check_plan`, garde-fou des
//! coordonnées, complétion sans effet (les points du terrain sont écrits dans le système du MNT),
//! couches et rasters présents dans l'inventaire du jeu.
//!
//! Avec le vrai modèle, l'essai rend un score par commande, sans seuil : la barre de non-oubli
//! (livrable 2) compare deux de ces relevés.
//!
//!   CASSINI_TEST_MODEL=<gguf ou *.modele.json> cargo test -p cassini-ai --release --features cuda --test par_commande_reel -- --ignored

mod common;

use std::collections::{BTreeMap, BTreeSet};

use cassini_ai::plan::{compose_plan_with, model_files_generator, AgentError};
use cassini_core::plan::{Plan, QuestionSource};
use cassini_core::point::guard_coordinates;
use cassini_core::registry::Registry;
use serde_json::{json, Value};

const INVENTORY: &str = include_str!("par_commande/inventory.json");
const PHRASES: &str = include_str!("par_commande/phrases.json");

/// Phrases au moins, par commande offerte.
const MIN_PHRASES: usize = 3;
/// Plans attendus au moins, par commande offerte, parmi ses phrases.
const MIN_PLANS: usize = 2;

fn phrases() -> Vec<Value> {
    serde_json::from_str(PHRASES).expect("par_commande/phrases.json illisible")
}

fn inventory() -> Value {
    serde_json::from_str(INVENTORY).expect("par_commande/inventory.json illisible")
}

/// Les commandes offertes au modèle : celles du registre que le noyau ne dicte pas seul
/// (même filtre que `offered`, `ai/src/plan.rs`).
fn offered(registry: &Registry) -> Vec<String> {
    registry
        .specs()
        .filter(|s| !s.dictated_by_core)
        .map(|s| s.id.to_string())
        .collect()
}

fn expected_calls(phrase: &Value) -> Option<&Vec<Value>> {
    (phrase["expected"]["kind"] == "calls")
        .then(|| phrase["expected"]["calls"].as_array())
        .flatten()
}

/// Les commandes offertes qui n'ont pas leurs phrases : moins de `MIN_PHRASES` phrases, ou
/// moins de `MIN_PLANS` plans attendus qui l'appellent.
fn uncovered(offered: &[String], phrases: &[Value]) -> Vec<String> {
    offered
        .iter()
        .filter(|id| {
            let own: Vec<&Value> = phrases.iter().filter(|p| p["command"] == **id).collect();
            let plans = own
                .iter()
                .filter(|p| {
                    expected_calls(p)
                        .is_some_and(|calls| calls.iter().any(|c| c["command"] == **id))
                })
                .count();
            own.len() < MIN_PHRASES || plans < MIN_PLANS
        })
        .cloned()
        .collect()
}

/// Le plan attendu d'une phrase, prêt pour `check_plan` : un export sans titre en reçoit un
/// (le titre est libre et retiré à la comparaison).
fn expected_plan(calls: &[Value]) -> Plan {
    let mut calls = calls.to_vec();
    for call in &mut calls {
        if call["command"] == "layout.export_pdf" && call["params"].get("title").is_none() {
            call["params"]["title"] = json!("Carte");
        }
    }
    serde_json::from_value(json!({"kind": "calls", "calls": calls})).unwrap()
}

/// Ce que l'inventaire du jeu connaît : les couches (`layer_id`) et leurs champs, les rasters
/// (`source`) et les fichiers non chargés (`unloaded_files`).
struct Known {
    fields: BTreeMap<String, BTreeSet<String>>,
    rasters: BTreeSet<String>,
    unloaded: BTreeSet<String>,
}

fn known(inventory: &Value) -> Known {
    let mut fields = BTreeMap::new();
    let mut rasters = BTreeSet::new();
    for layer in inventory["layers"].as_array().unwrap() {
        let names = layer["fields"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["name"].as_str().map(str::to_string))
            .collect();
        fields.insert(layer["layer_id"].as_str().unwrap().to_string(), names);
        if layer["kind"] == "raster" {
            rasters.insert(layer["source"].as_str().unwrap().to_string());
        }
    }
    let unloaded = inventory["unloaded_files"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f["path"].as_str().map(str::to_string))
        .collect();
    Known {
        fields,
        rasters,
        unloaded,
    }
}

/// Les identifiants de couche et les chemins de raster que nomme un appel.
fn references(call: &Value) -> (Vec<String>, Vec<String>) {
    let params = &call["params"];
    let text = |v: &Value| v.as_str().map(str::to_string);
    let mut layers: Vec<String> = ["layer_id", "overlay_layer_id", "points_layer_id"]
        .iter()
        .filter_map(|k| text(&params[*k]))
        .collect();
    for key in ["leave_out", "layers"] {
        layers.extend(
            params[key]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(text),
        );
    }
    let mut rasters: Vec<String> = ["dem", "cost", "raster"]
        .iter()
        .filter_map(|k| text(&params[*k]))
        .collect();
    for criterion in params["criteria"].as_array().into_iter().flatten() {
        rasters.extend(text(&criterion["raster"]));
    }
    rasters.extend(
        params["exclusions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(text),
    );
    (layers, rasters)
}

/// Les phrases dont l'attendu est invalide, avec la raison.
fn invalid(registry: &Registry, inventory: &Value, phrases: &[Value]) -> Vec<String> {
    let known = known(inventory);
    let mut out = Vec::new();
    for p in phrases {
        let id = p["id"].as_str().unwrap_or("?");
        let text = p["phrase"].as_str().unwrap_or_default();
        match p["expected"]["kind"].as_str() {
            Some("question" | "refusal") => continue,
            Some("calls") => {}
            other => {
                out.push(format!("{id} : sorte attendue inconnue {other:?}"));
                continue;
            }
        }
        let Some(calls) = expected_calls(p) else {
            out.push(format!("{id} : plan attendu sans appels"));
            continue;
        };
        if !calls.iter().any(|c| c["command"] == p["command"]) {
            out.push(format!(
                "{id} : le plan attendu n'appelle pas {}",
                p["command"]
            ));
        }
        let plan = expected_plan(calls);
        if let Err(e) = registry.check_plan(&plan) {
            out.push(format!("{id} : check_plan refuse ({e:?})"));
            continue;
        }
        if guard_coordinates(registry, plan.clone(), text) != plan {
            out.push(format!(
                "{id} : le garde-fou des coordonnées refuse le plan attendu"
            ));
        }
        if common::complete_with(registry, plan.clone(), inventory) != plan {
            out.push(format!(
                "{id} : la complétion ajoute une conversion au plan attendu"
            ));
        }
        for call in calls {
            let (named_layers, named_rasters) = references(call);
            for layer in named_layers.iter().filter(|l| !l.starts_with("step:")) {
                if !known.fields.contains_key(layer) {
                    out.push(format!("{id} : couche absente de l'inventaire {layer}"));
                }
            }
            for raster in named_rasters.iter().filter(|r| !r.starts_with("step:")) {
                if !known.rasters.contains(raster) {
                    out.push(format!("{id} : raster absent de l'inventaire {raster}"));
                }
            }
            // Un champ nommé doit exister dans la couche qu'il désigne directement.
            let params = &call["params"];
            if let Some(own) = params["layer_id"]
                .as_str()
                .and_then(|l| known.fields.get(l))
            {
                for key in ["field", "label_field"] {
                    if let Some(field) = params[key].as_str() {
                        if !own.contains(field) {
                            out.push(format!("{id} : champ absent de la couche {field}"));
                        }
                    }
                }
            }
            if call["command"] == "layer.add" {
                let path = params["path"].as_str().unwrap_or_default();
                if !known.unloaded.contains(path) {
                    out.push(format!(
                        "{id} : fichier absent des fichiers non chargés {path}"
                    ));
                }
            }
        }
    }
    out
}

#[test]
fn every_offered_command_has_its_sentences() {
    let registry = Registry::standard();
    let offered = offered(&registry);
    let missing = uncovered(&offered, &phrases());
    assert!(
        missing.is_empty(),
        "commandes offertes sans leurs {MIN_PHRASES} phrases (dont {MIN_PLANS} plans) : {}",
        missing.join(", ")
    );
    // Plancher : un filtre qui ne rendrait plus rien laisserait la couverture verte.
    assert!(offered.len() >= 33, "{} commandes offertes", offered.len());
    eprintln!("{} commandes offertes couvertes", offered.len());
}

#[test]
fn every_sentence_names_an_offered_command_and_has_a_unique_id() {
    let offered: BTreeSet<String> = offered(&Registry::standard()).into_iter().collect();
    let mut ids = BTreeSet::new();
    for p in phrases() {
        let id = p["id"].as_str().expect("phrase sans id").to_string();
        assert!(ids.insert(id.clone()), "id en double : {id}");
        let command = p["command"].as_str().expect("phrase sans commande");
        assert!(
            offered.contains(command),
            "{id} : commande non offerte {command}"
        );
        assert!(
            p["phrase"].as_str().is_some_and(|t| !t.trim().is_empty()),
            "{id} : phrase vide"
        );
    }
}

#[test]
fn every_expected_answer_is_valid() {
    let errors = invalid(&Registry::standard(), &inventory(), &phrases());
    assert!(
        errors.is_empty(),
        "attendus invalides :\n{}",
        errors.join("\n")
    );
}

#[test]
fn a_command_without_its_sentences_is_named() {
    let registry = Registry::standard();
    let offered = offered(&registry);
    let without: Vec<Value> = phrases()
        .into_iter()
        .filter(|p| p["command"] != "terrain.peaks")
        .collect();
    assert_eq!(uncovered(&offered, &without), vec!["terrain.peaks"]);
    // Une commande ajoutée au registre sans phrases est nommée aussi.
    let mut grown = offered.clone();
    grown.push("terrain.nouvelle".to_string());
    assert_eq!(uncovered(&grown, &phrases()), vec!["terrain.nouvelle"]);
    // Trois phrases mais une seule qui attend un plan : pas assez.
    let mut thin = phrases();
    for p in thin
        .iter_mut()
        .filter(|p| p["command"] == "terrain.slope")
        .skip(1)
    {
        p["expected"] = json!({"kind": "question"});
    }
    assert_eq!(uncovered(&offered, &thin), vec!["terrain.slope"]);
}

#[test]
fn an_invalid_expected_answer_is_named() {
    let registry = Registry::standard();
    let mut broken = phrases();
    let p = broken.iter_mut().find(|p| p["id"] == "SL2").unwrap();
    p["expected"]["calls"][0]["params"]["unit"] = json!("radians");
    let q = broken.iter_mut().find(|p| p["id"] == "LD1").unwrap();
    q["expected"]["calls"][0]["params"]["layer_id"] = json!("batiments_inconnus");
    let r = broken.iter_mut().find(|p| p["id"] == "LA1").unwrap();
    r["expected"]["calls"][0]["params"]["path"] = json!("donnees/hydrographie.gpkg");
    let f = broken.iter_mut().find(|p| p["id"] == "SEL1").unwrap();
    f["expected"]["calls"][0]["params"]["field"] = json!("usages");
    let h = broken.iter_mut().find(|p| p["id"] == "HS3").unwrap();
    h["expected"]["calls"][0]["params"]["dem"] = json!("donnees/mnt_inconnu.tif");
    let k = broken.iter_mut().find(|p| p["id"] == "PK2").unwrap();
    k["command"] = json!("terrain.slope");
    let errors = invalid(&registry, &inventory(), &broken);
    // L'ordre des erreurs suit le fichier : on les compare comme un ensemble.
    assert_eq!(errors.len(), 6, "{errors:?}");
    for (id, reason) in [
        ("LD1", "couche absente"),
        ("SL2", "check_plan refuse"),
        ("LA1", "fichier absent"),
        ("SEL1", "champ absent"),
        ("HS3", "raster absent"),
        ("PK2", "le plan attendu n'appelle pas"),
    ] {
        assert!(
            errors
                .iter()
                .any(|e| e.starts_with(&format!("{id} : {reason}"))),
            "{id} : {errors:?}"
        );
    }
}

/// Un nombre entier écrit en flottant (`2.0`) vaut l'entier (`2`) : serde_json les distingue.
fn normalize_numbers(value: Value) -> Value {
    match value {
        Value::Number(n) => match n.as_f64() {
            Some(f) if n.is_f64() && f.fract() == 0.0 && f.abs() < 9.0e15 => Value::from(f as i64),
            _ => Value::Number(n),
        },
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_numbers).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, normalize_numbers(v)))
                .collect(),
        ),
        other => other,
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
fn par_commande_reel_scores_every_offered_command() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    // Comme dans le produit : identifiants courts montrés, plan retraduit (5.19), complété (11.22).
    let full = inventory();
    let (_, ids) = common::shown_inventory(&full);
    let mut generator = model_files_generator(files);
    let mut scores: BTreeMap<String, (usize, usize)> = offered(&registry)
        .into_iter()
        .map(|id| (id, (0, 0)))
        .collect();
    for p in phrases() {
        let id = p["id"].as_str().unwrap();
        let text = p["phrase"].as_str().unwrap();
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
        let want = &p["expected"];
        let ok = match (want["kind"].as_str(), &got) {
            (Some("calls"), Ok(Plan::Calls { calls })) => {
                comparable(serde_json::to_value(calls).unwrap()).map(normalize_numbers)
                    == Some(normalize_numbers(want["calls"].clone()))
            }
            // Une question du garde-fou des coordonnées répond à autre chose que la phrase.
            (
                Some("question"),
                Ok(Plan::Question {
                    source: QuestionSource::Model,
                    ..
                }),
            ) => true,
            (Some("refusal"), Err(AgentError::Refused(_))) => true,
            _ => false,
        };
        eprintln!(
            "{} {id} {text}\n     attendu : {want}\n     obtenu  : {got:?}",
            if ok { "OK  " } else { "FAIL" }
        );
        // Un plan invalide compte en échec ; l'essai continue, pour que chaque commande ait
        // sa ligne `SCORE` (le relevé que lit la barre de non-oubli).
        let ok = ok
            && match &got {
                Ok(plan) => registry.check_plan(plan).is_ok(),
                Err(_) => true,
            };
        let entry = scores
            .get_mut(p["command"].as_str().unwrap())
            .expect("commande offerte");
        entry.0 += usize::from(ok);
        entry.1 += 1;
    }
    let (mut right, mut total) = (0, 0);
    for (command, (ok, n)) in &scores {
        eprintln!("SCORE {command} {ok}/{n}");
        right += ok;
        total += n;
    }
    eprintln!("SCORE total {right}/{total}");
}
