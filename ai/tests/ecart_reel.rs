//! Score du vrai modèle sur le jeu tenu à l'écart (ticket 5.10) : 40 phrases inédites sur un
//! inventaire de projet d'urbanisme, écrites par un autre auteur que le générateur d'exemples.
//! C'est la barre chiffrée que le modèle ajusté devra atteindre.
//!
//!   CASSINI_TEST_MODEL=<gguf ou déclaration *.modele.json> \
//!     cargo test -p cassini-ai --release --features cuda -- --ignored --nocapture ecart_reel
//!
//! Un plan est comparé exactement, sauf le `title` de `layout.export_pdf`, dont la présence
//! suffit, et les nombres sont comparés en valeur (`500.0` vaut `500`). Un plan rendu qui ne
//! passe pas `check_plan` compte comme faux ; l'essai échoue alors, après la ligne `SCORE`.
//! Une question attendue doit rendre une question, un refus attendu un refus. Aucun
//! seuil : la ligne `SCORE` est la mesure. Le texte des phrases n'est jamais imprimé, pour que
//! l'auteur du générateur ne lise pas le jeu : seuls l'id, la sorte et la forme de l'écart
//! (`Gap` : sorte, nombre d'étapes, ordre, commande ou nom du paramètre, jamais une valeur). La
//! ligne `REDEMANDES` compte les phrases passées par la redemande (deux appels au modèle) et
//! celles qu'elle a sauvées (justes après elle).

mod common;

use std::fmt;

use cassini_ai::plan::{compose_plan_with, model_files_generator, AgentError};
use cassini_core::plan::{Plan, QuestionSource};
use cassini_core::registry::Registry;
use serde_json::{json, Value};

const INVENTORY: &str = include_str!("tenu_a_l_ecart/inventory.json");
const SENTENCES: &str = include_str!("tenu_a_l_ecart/sentences.json");

/// Ôte le titre de chaque `layout.export_pdf` ; rend `None` si un titre manque ou est vide.
fn without_titles(calls: &Value) -> Option<Value> {
    let mut calls = calls.clone();
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

/// Nombres ramenés à une forme unique : un flottant entier devient l'entier (`500.0` → `500`).
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

/// Forme comparable d'une liste d'appels : sans titres de PDF, nombres normalisés.
fn comparable(calls: &Value) -> Option<Value> {
    without_titles(calls).map(normalize_numbers)
}

/// Paramètres qui désignent une couche existante (ou une étape précédente).
const LAYER_PARAMS: [&str; 3] = ["layer_id", "overlay_layer_id", "points_layer_id"];

/// Sorte attendue d'une phrase : `calls`, `question` ou `refusal`.
fn expected_kind(expected: &Value) -> &str {
    if expected.is_array() {
        "calls"
    } else {
        expected["kind"].as_str().unwrap_or("?")
    }
}

/// Appels attendus d'un plan : la liste elle-même, ou le champ `calls` d'un plan entier
/// `{"kind":"calls","calls":[...]}`. `None` pour une question ou un refus.
fn expected_calls(expected: &Value) -> Option<&Value> {
    if expected.is_array() {
        Some(expected)
    } else if expected["kind"] == "calls" {
        expected.get("calls").filter(|c| c.is_array())
    } else {
        None
    }
}

fn got_kind(got: &Result<Plan, AgentError>) -> &'static str {
    match got {
        Ok(Plan::Calls { .. }) => "calls",
        Ok(Plan::Question { .. }) => "question",
        Ok(Plan::Refusal { .. }) | Err(AgentError::Refused(_)) => "refusal",
        Err(AgentError::PlanInvalid(_)) => "plan_invalid",
        Err(AgentError::Ai(_)) => "ai_error",
    }
}

/// Forme de l'écart entre le plan attendu et le plan rendu, sans aucune valeur tirée de la
/// phrase : seulement des sortes, des rangs d'étape (comptés depuis 1), des ids de commande et
/// des noms de paramètre.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Gap {
    /// Une autre sorte de réponse que celle attendue.
    Kind {
        want: &'static str,
        got: &'static str,
    },
    /// Un `layout.export_pdf` rendu sans titre, ou avec un titre vide, à l'étape `step`.
    Title { step: usize },
    /// Un autre nombre d'étapes.
    Steps { want: usize, got: usize },
    /// Les mêmes étapes, dans un autre ordre.
    Order,
    /// Une autre commande à l'étape `step`.
    Command {
        step: usize,
        want: String,
        got: String,
    },
    /// Même commande à l'étape `step`, autre valeur (ou présence) du paramètre `name`.
    Param { step: usize, name: String },
    /// Même commande et mêmes paramètres à l'étape `step`, mais une autre clé de l'étape.
    Step { step: usize },
    /// Un plan rendu qui ne se compare pas (liste d'appels ou paramètres mal formés).
    Unreadable,
}

impl fmt::Display for Gap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Gap::Kind { want, got } => write!(f, "sorte : attendu {want}, obtenu {got}"),
            Gap::Title { step } => write!(f, "titre du PDF absent à l'étape {step}"),
            Gap::Steps { want, got } => write!(f, "étapes : {want} attendues, {got} rendues"),
            Gap::Order => write!(f, "ordre des étapes"),
            Gap::Command { step, want, got } => {
                write!(
                    f,
                    "commande à l'étape {step} : attendu {want}, obtenu {got}"
                )
            }
            Gap::Param { step, name } => write!(f, "paramètre `{name}` à l'étape {step}"),
            Gap::Step { step } => write!(f, "autre clé de l'étape {step}"),
            Gap::Unreadable => write!(f, "plan rendu non comparable"),
        }
    }
}

/// Note la réponse `got` contre la réponse attendue `expected` : `Ok(())` si elle est juste,
/// sinon la forme de l'écart. Ne juge pas `check_plan` (fait à part, sur le registre).
fn grade(expected: &Value, got: &Result<Plan, AgentError>) -> Result<(), Gap> {
    let want = expected_kind(expected);
    let got_calls = match (got, want) {
        (Ok(Plan::Calls { calls }), "calls") => serde_json::to_value(calls).unwrap(),
        (Ok(Plan::Question { .. }), "question") => return Ok(()),
        (Err(AgentError::Refused(_)) | Ok(Plan::Refusal { .. }), "refusal") => return Ok(()),
        _ => {
            return Err(Gap::Kind {
                want: kind_name(want),
                got: got_kind(got),
            })
        }
    };
    let want_calls = expected_calls(expected)
        .and_then(comparable)
        .expect("plan attendu comparable");
    let Some(got_calls) = comparable(&got_calls) else {
        return Err(match missing_title_step(&got_calls) {
            Some(step) => Gap::Title { step },
            None => Gap::Unreadable,
        });
    };
    if want_calls == got_calls {
        return Ok(());
    }
    let (want_steps, got_steps) = (steps(&want_calls), steps(&got_calls));
    if want_steps.len() != got_steps.len() {
        return Err(Gap::Steps {
            want: want_steps.len(),
            got: got_steps.len(),
        });
    }
    if same_steps_reordered(&want_steps, &got_steps) {
        return Err(Gap::Order);
    }
    for (k, (w, g)) in want_steps.iter().zip(&got_steps).enumerate() {
        let step = k + 1;
        if w["command"] != g["command"] {
            return Err(Gap::Command {
                step,
                want: w["command"].as_str().unwrap_or("?").to_string(),
                got: g["command"].as_str().unwrap_or("?").to_string(),
            });
        }
        let empty = serde_json::Map::new();
        let (wp, gp) = (
            w["params"].as_object().unwrap_or(&empty),
            g["params"].as_object().unwrap_or(&empty),
        );
        let mut names: Vec<&String> = wp.keys().chain(gp.keys()).collect();
        names.sort();
        names.dedup();
        if let Some(name) = names.into_iter().find(|n| wp.get(*n) != gp.get(*n)) {
            return Err(Gap::Param {
                step,
                name: name.clone(),
            });
        }
        if w != g {
            return Err(Gap::Step { step });
        }
    }
    unreachable!("deux plans différents de même longueur diffèrent à une étape")
}

fn kind_name(kind: &str) -> &'static str {
    match kind {
        "calls" => "calls",
        "question" => "question",
        "refusal" => "refusal",
        _ => "?",
    }
}

fn steps(calls: &Value) -> Vec<Value> {
    calls.as_array().cloned().unwrap_or_default()
}

/// Rang (depuis 1) du premier `layout.export_pdf` sans titre, ou au titre vide.
fn missing_title_step(calls: &Value) -> Option<usize> {
    calls
        .as_array()?
        .iter()
        .position(|c| {
            c["command"] == "layout.export_pdf"
                && c["params"]["title"]
                    .as_str()
                    .is_none_or(|t| t.trim().is_empty())
        })
        .map(|k| k + 1)
}

/// Vrai si les deux plans ont les mêmes étapes, à l'ordre près : chaque référence `step:k.x`
/// est remplacée par l'étape `k` elle-même (récursivement), puis les étapes sont comparées
/// comme un ensemble avec répétitions.
fn same_steps_reordered(want: &[Value], got: &[Value]) -> bool {
    let (mut w, mut g) = (resolved(want), resolved(got));
    let key = |v: &Value| v.to_string();
    w.sort_by_key(key);
    g.sort_by_key(key);
    w == g
}

/// Étapes dont les références `step:k.champ` sont remplacées par l'étape visée, résolue.
fn resolved(steps: &[Value]) -> Vec<Value> {
    fn resolve(value: &Value, done: &[Value]) -> Value {
        match value {
            Value::String(s) => s
                .strip_prefix("step:")
                .and_then(|rest| {
                    let (k, field) = rest.split_once('.')?;
                    let k: usize = k.parse().ok()?;
                    let target = done.get(k.checked_sub(1)?)?;
                    Some(json!({"step": target, "field": field}))
                })
                .unwrap_or_else(|| value.clone()),
            Value::Array(items) => Value::Array(items.iter().map(|v| resolve(v, done)).collect()),
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), resolve(v, done)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }
    let mut done = Vec::new();
    for step in steps {
        let r = resolve(step, &done);
        done.push(r);
    }
    done
}

#[test]
fn the_title_of_the_pdf_only_needs_to_be_present() {
    let want = serde_json::json!([{"command": "layout.export_pdf", "params": {"title": "A"}}]);
    let other = serde_json::json!([{"command": "layout.export_pdf", "params": {"title": "B"}}]);
    let empty = serde_json::json!([{"command": "layout.export_pdf", "params": {"title": " "}}]);
    assert_eq!(without_titles(&want), without_titles(&other));
    assert!(without_titles(&empty).is_none());
    assert_eq!(expected_kind(&want), "calls");
    let whole = serde_json::json!({"kind": "calls", "calls": want});
    assert_eq!(expected_kind(&whole), "calls");
    assert_eq!(expected_calls(&whole), Some(&want));
    assert_eq!(expected_calls(&want), Some(&want));
    assert_eq!(
        expected_calls(&serde_json::json!({"kind": "refusal"})),
        None
    );
    let float = serde_json::json!([{"command": "analysis.buffer",
        "params": {"layer_id": "a", "distance": {"value": 500.0, "unit": "m"}}}]);
    let int = serde_json::json!([{"command": "analysis.buffer",
        "params": {"layer_id": "a", "distance": {"value": 500, "unit": "m"}}}]);
    assert_ne!(float, int);
    assert_eq!(comparable(&float), comparable(&int));
    assert_eq!(
        expected_kind(&serde_json::json!({"kind": "question"})),
        "question"
    );
}

#[test]
fn the_held_out_set_is_well_formed() {
    // Lu sans rien imprimer : forme seulement, et chaque plan attendu passe `check_plan`.
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).expect("inventory.json lisible");
    let layers: Vec<&str> = inventory["layers"]
        .as_array()
        .expect("layers")
        .iter()
        .filter_map(|l| l["layer_id"].as_str())
        .collect();
    let files: Vec<&str> = inventory["unloaded_files"]
        .as_array()
        .map(|f| f.iter().filter_map(|f| f["path"].as_str()).collect())
        .unwrap_or_default();
    let sentences: Vec<Value> = serde_json::from_str(SENTENCES).expect("sentences.json lisible");
    let count = |k: &str| {
        sentences
            .iter()
            .filter(|s| expected_kind(&s["expected"]) == k)
            .count()
    };
    assert_eq!(
        (
            sentences.len(),
            count("calls"),
            count("question"),
            count("refusal")
        ),
        (40, 26, 8, 6)
    );
    for s in &sentences {
        let id = s["id"].as_str().expect("id");
        assert!(s["text"].as_str().is_some_and(|t| !t.is_empty()), "{id}");
        if expected_kind(&s["expected"]) == "calls" {
            let calls = expected_calls(&s["expected"]).expect("appels attendus");
            let plan: Plan =
                serde_json::from_value(serde_json::json!({"kind": "calls", "calls": calls}))
                    .expect("plan attendu lisible");
            assert_eq!(registry.check_plan(&plan), Ok(()), "{id}");
            // Couches et fichiers cités : présents dans l'inventaire (seul l'id est imprimé).
            for call in calls.as_array().expect("liste d'appels") {
                for name in LAYER_PARAMS {
                    if let Some(layer) = call["params"][name].as_str() {
                        assert!(
                            layer.starts_with("step:") || layers.contains(&layer),
                            "{id} : couche citée absente de l'inventaire"
                        );
                    }
                }
                if let Some(path) = call["params"]["path"].as_str() {
                    assert!(
                        files.contains(&path),
                        "{id} : fichier cité absent de l'inventaire"
                    );
                }
            }
        }
    }
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn ecart_reel_score_on_the_held_out_set() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    // Comme dans le produit : identifiants courts montrés, plan retraduit (ticket 5.19).
    let (inventory, ids) = common::shown_inventory(&inventory);
    let sentences: Vec<Value> = serde_json::from_str(SENTENCES).unwrap();
    let mut model = model_files_generator(files);
    // (justes, total) par sorte attendue.
    let mut score = [("calls", 0, 0), ("question", 0, 0), ("refusal", 0, 0)];
    let mut invalid = Vec::new();
    let (mut retried, mut saved, mut guarded) = (0, 0, 0);
    let started = std::time::Instant::now();
    for sentence in &sentences {
        let id = sentence["id"].as_str().unwrap();
        let text = sentence["text"].as_str().unwrap();
        let expected = &sentence["expected"];
        let want = expected_kind(expected);
        // Appels au générateur pour cette phrase : deux quand la redemande a eu lieu.
        let mut calls = 0;
        let got = {
            let mut generator = |r: &cassini_ai::plan::GenerationRequest| {
                calls += 1;
                model(r)
            };
            common::restored(
                compose_plan_with(style, &registry, &inventory, text, &mut generator),
                &ids,
            )
        };
        let graded = grade(expected, &got);
        guarded += usize::from(matches!(
            &got,
            Ok(Plan::Question {
                source: QuestionSource::Guard,
                ..
            })
        ));
        // Aucun plan invalide n'est jamais rendu : un `Ok` qui échoue à `check_plan` est faux,
        // et l'essai échoue après la ligne `SCORE`.
        let valid = got
            .as_ref()
            .map_or(true, |plan| registry.check_plan(plan).is_ok());
        let ok = graded.is_ok() && valid;
        if !valid {
            invalid.push(id.to_string());
        }
        if calls > 1 {
            retried += 1;
            saved += usize::from(ok);
        }
        let row = score
            .iter_mut()
            .find(|(k, _, _)| *k == want)
            .expect("sorte connue");
        row.1 += usize::from(ok);
        row.2 += 1;
        let gap = match (&graded, valid) {
            (_, false) => " ; écart : échoue à check_plan".to_string(),
            (Err(gap), true) => format!(" ; écart : {gap}"),
            (Ok(()), true) => String::new(),
        };
        eprintln!(
            "{} {id} attendu {want}, obtenu {}, {calls} appel(s){gap}",
            if ok { "OK  " } else { "FAIL" },
            got_kind(&got)
        );
    }
    let right: usize = score.iter().map(|r| r.1).sum();
    eprintln!(
        "SCORE {right}/{} : plans {}/{}, questions {}/{}, refus {}/{}, dont {guarded} question(s) du garde-fou ({style:?}, {} s)",
        sentences.len(),
        score[0].1,
        score[0].2,
        score[1].1,
        score[1].2,
        score[2].1,
        score[2].2,
        started.elapsed().as_secs()
    );
    eprintln!("REDEMANDES {retried} phrase(s) passée(s) par la redemande, dont {saved} sauvée(s)");
    assert!(
        invalid.is_empty(),
        "plans rendus qui échouent à check_plan : {}",
        invalid.join(", ")
    );
}

// Sans modèle : la matrice du notateur.

fn plan_of(value: Value) -> Result<Plan, AgentError> {
    Ok(serde_json::from_value(json!({"kind": "calls", "calls": value})).unwrap())
}

fn demo_plan(title: &str) -> Value {
    json!([
        {"command": "analysis.select_by_attribute", "params": {"layer_id": "plu", "field": "typezone", "operator": "eq", "value": "U"}},
        {"command": "analysis.buffer", "params": {"layer_id": "gares", "distance": {"value": 500, "unit": "m"}}},
        {"command": "analysis.intersect", "params": {"layer_id": "parcelles", "overlay_layer_id": "step:1.layer_id"}},
        {"command": "analysis.intersect", "params": {"layer_id": "step:3.layer_id", "overlay_layer_id": "step:2.layer_id"}},
        {"command": "layout.export_pdf", "params": {"title": title}}
    ])
}

#[test]
fn a_plan_equal_but_for_the_pdf_title_is_right() {
    let expected = json!({"kind": "calls", "calls": demo_plan("Parcelles constructibles")});
    assert_eq!(grade(&expected, &plan_of(demo_plan("Autre titre"))), Ok(()));
    // La liste nue vaut le plan entier, et 500.0 vaut 500.
    let mut float = demo_plan("T");
    float[1]["params"]["distance"]["value"] = json!(500.0);
    assert_eq!(grade(&demo_plan("A"), &plan_of(float)), Ok(()));
    // Un titre vide n'est pas un titre.
    assert_eq!(
        grade(&expected, &plan_of(demo_plan(" "))),
        Err(Gap::Title { step: 5 })
    );
}

#[test]
fn an_expected_refusal_is_right_when_the_agent_refuses() {
    let expected = json!({"kind": "refusal", "reason": "Aucune commande ne sait faire ça."});
    assert_eq!(
        grade(&expected, &Err(AgentError::Refused("non".into()))),
        Ok(())
    );
    assert_eq!(
        grade(&expected, &plan_of(demo_plan("T"))),
        Err(Gap::Kind {
            want: "refusal",
            got: "calls"
        })
    );
}

#[test]
fn an_expected_question_answered_by_a_plan_is_a_kind_gap() {
    let expected = json!({"kind": "question", "question": "Quelle couche ?"});
    assert_eq!(
        grade(&expected, &plan_of(demo_plan("T"))),
        Err(Gap::Kind {
            want: "question",
            got: "calls"
        })
    );
    let question = Ok(Plan::Question {
        question: "Laquelle ?".into(),
        source: QuestionSource::Model,
    });
    assert_eq!(grade(&expected, &question), Ok(()));
    let invalid = Err(AgentError::PlanInvalid(vec!["écart".into()]));
    assert_eq!(
        grade(&expected, &invalid),
        Err(Gap::Kind {
            want: "question",
            got: "plan_invalid"
        })
    );
}

#[test]
fn the_same_steps_in_another_order_are_an_order_gap() {
    // Tampon avant sélection : les références sont renumérotées, les étapes sont les mêmes.
    let reordered = json!([
        {"command": "analysis.buffer", "params": {"layer_id": "gares", "distance": {"value": 500, "unit": "m"}}},
        {"command": "analysis.select_by_attribute", "params": {"layer_id": "plu", "field": "typezone", "operator": "eq", "value": "U"}},
        {"command": "analysis.intersect", "params": {"layer_id": "parcelles", "overlay_layer_id": "step:2.layer_id"}},
        {"command": "analysis.intersect", "params": {"layer_id": "step:3.layer_id", "overlay_layer_id": "step:1.layer_id"}},
        {"command": "layout.export_pdf", "params": {"title": "T"}}
    ]);
    assert_eq!(grade(&demo_plan("T"), &plan_of(reordered)), Err(Gap::Order));
}

#[test]
fn another_value_of_a_parameter_names_the_parameter_and_the_step() {
    let mut other = demo_plan("T");
    other[1]["params"]["distance"]["value"] = json!(300);
    let gap = grade(&demo_plan("T"), &plan_of(other)).unwrap_err();
    assert_eq!(
        gap,
        Gap::Param {
            step: 2,
            name: "distance".into()
        }
    );
    // L'écart imprimé ne porte que le nom du paramètre, jamais sa valeur.
    assert_eq!(gap.to_string(), "paramètre `distance` à l'étape 2");
    // Les intersections dans l'autre ordre de leurs couches : un paramètre, pas un ordre.
    let mut swapped = demo_plan("T");
    swapped[2]["params"]["overlay_layer_id"] = json!("step:2.layer_id");
    swapped[3]["params"]["overlay_layer_id"] = json!("step:1.layer_id");
    assert_eq!(
        grade(&demo_plan("T"), &plan_of(swapped)),
        Err(Gap::Param {
            step: 3,
            name: "overlay_layer_id".into()
        })
    );
    // Un paramètre en trop compte aussi.
    let mut extra = demo_plan("T");
    extra[1]["params"]["dissolve"] = json!(true);
    assert_eq!(
        grade(&demo_plan("T"), &plan_of(extra)),
        Err(Gap::Param {
            step: 2,
            name: "dissolve".into()
        })
    );
}

#[test]
fn another_command_or_another_count_of_steps_is_named() {
    let mut other = demo_plan("T");
    other[0]["command"] = json!("analysis.buffer");
    assert_eq!(
        grade(&demo_plan("T"), &plan_of(other)),
        Err(Gap::Command {
            step: 1,
            want: "analysis.select_by_attribute".into(),
            got: "analysis.buffer".into()
        })
    );
    let shorter = json!(demo_plan("T").as_array().unwrap()[..4]);
    assert_eq!(
        grade(&demo_plan("T"), &plan_of(shorter)),
        Err(Gap::Steps { want: 5, got: 4 })
    );
}

#[test]
fn a_step_differing_outside_command_and_params_is_named() {
    let mut expected = demo_plan("T");
    expected[2]["note"] = json!("x");
    let gap = grade(&expected, &plan_of(demo_plan("T"))).unwrap_err();
    assert_eq!(gap, Gap::Step { step: 3 });
    assert_eq!(gap.to_string(), "autre clé de l'étape 3");
}

#[test]
fn a_missing_title_names_its_step_and_only_a_found_one() {
    let mut untitled = demo_plan("T");
    untitled[4]["params"] = json!({});
    assert_eq!(
        grade(&demo_plan("T"), &plan_of(untitled.clone())),
        Err(Gap::Title { step: 5 })
    );
    assert_eq!(missing_title_step(&untitled), Some(5));
    assert_eq!(missing_title_step(&demo_plan("T")), None);
    assert_eq!(missing_title_step(&json!({"pas": "une liste"})), None);
}
