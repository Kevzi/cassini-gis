//! Couverture du registre par des phrases réelles d'utilisateurs (ticket 7.11, CAP-14).
//!
//! `couverture_reelle/phrases.json` est un jeu de phrases d'utilisateurs de SIG, tirées de sources
//! publiques (documentation et manuel de formation de QGIS, questions de forum publiques,
//! supports de cours publics) et réécrites : aucune n'est recopiée mot pour mot, et chacune garde
//! sa source (type, référence et licence, thème, traitement). Rien n'est téléchargé ni gratté :
//! le fichier est écrit à la main, Cassini reste hors ligne.
//!
//! Chaque phrase a un attendu posé à la main, d'après le registre actuel :
//! - `plan` : le registre couvre la demande, avec les commandes attendues (`commands`) ;
//! - `question` : la commande existe, mais la demande ne désigne rien de l'inventaire (couche,
//!   champ, point ou distance absents) : une question est la bonne réponse, donc couverte ;
//! - `refusal` : aucune commande ne fait la chose ; `missing` nomme les mots-clés de ce que le
//!   registre n'a pas. Le jour où un identifiant du registre contient un de ces mots, le refus
//!   devient faux et le test sans modèle nomme la phrase (contrôle commun de `common`).
//!
//! Part couverte = (phrases à plan + phrases à question) / total. Avec le vrai modèle, l'essai
//! mesure en plus ce que l'agent local en tire, phrase par phrase, sans seuil : c'est la mesure
//! de départ que 10.8 reprend, avec ce même harnais (`common`, inventaire de `par_commande`).
//!
//!   CASSINI_TEST_MODEL=<gguf ou *.modele.json> CASSINI_COUVERTURE_RAPPORT=<fichier.md> \
//!     cargo test -p cassini-ai --release --features cuda --test couverture_reelle_reel -- --ignored --nocapture

mod common;

use std::collections::BTreeSet;

use cassini_ai::plan::{compose_plan_with, model_files_generator, AgentError};
use cassini_core::plan::Plan;
use cassini_core::registry::Registry;
use serde_json::Value;

const INVENTORY: &str = include_str!("par_commande/inventory.json");
const PHRASES: &str = include_str!("couverture_reelle/phrases.json");

/// Phrases au moins dans le jeu.
const MIN_PHRASES: usize = 40;

fn phrases() -> Vec<Value> {
    common::read_phrases(PHRASES, "couverture_reelle/phrases.json")
}

fn kind(p: &Value) -> &str {
    p["expected"]["kind"].as_str().unwrap_or("?")
}

/// Les phrases que le registre couvre : un plan, ou une question justifiée.
fn is_covered(p: &Value) -> bool {
    matches!(kind(p), "plan" | "question")
}

/// Les mots-clés de ce que le registre n'a pas, pour une phrase non couverte.
fn missing(p: &Value) -> Vec<String> {
    p["expected"]["missing"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m.as_str().map(str::to_string))
        .collect()
}

/// Ce que l'agent a rendu, ramené à une étiquette.
#[derive(Debug, PartialEq)]
enum Answer {
    /// Un plan valide qui appelle toutes les commandes attendues.
    Plan,
    /// Un plan valide qui n'appelle pas toutes les commandes attendues.
    OtherPlan,
    Question,
    Refusal,
    /// Une sortie que le noyau écarte (JSON illisible, plan invalide).
    Failed,
}

fn classify(registry: &Registry, phrase: &Value, got: &Result<Plan, AgentError>) -> Answer {
    match got {
        Ok(Plan::Calls { calls }) => {
            if registry.check_plan(&Plan::Calls {
                calls: calls.clone(),
            }) != Ok(())
            {
                return Answer::Failed;
            }
            let called: BTreeSet<&str> = calls.iter().map(|c| c.command.as_str()).collect();
            let wanted = phrase["expected"]["commands"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str);
            if wanted.into_iter().all(|c| called.contains(c)) {
                Answer::Plan
            } else {
                Answer::OtherPlan
            }
        }
        Ok(Plan::Question { .. }) => Answer::Question,
        Ok(Plan::Refusal { .. }) | Err(AgentError::Refused(_)) => Answer::Refusal,
        Err(_) => Answer::Failed,
    }
}

/// La réponse est-elle celle qu'on attend de la phrase.
fn as_expected(p: &Value, a: &Answer) -> bool {
    match kind(p) {
        "plan" => *a == Answer::Plan,
        "question" => *a == Answer::Question,
        // Pas de plan inventé : un refus ou une question sont justifiés.
        "refusal" => matches!(a, Answer::Refusal | Answer::Question),
        _ => false,
    }
}

fn percent(n: usize, total: usize) -> String {
    format!("{:.1} %", 100.0 * n as f64 / total as f64)
}

/// Le rapport en Markdown : une ligne par phrase (source, verdict, commande manquante), puis la
/// part couverte. `answers` vaut `None` sans modèle.
fn report(phrases: &[Value], answers: Option<&[Answer]>) -> String {
    let total = phrases.len();
    let covered = phrases.iter().filter(|p| is_covered(p)).count();
    let mut out =
        String::from("# Couverture du registre par des phrases réelles (ticket 7.11)\n\n");
    out.push_str(&format!(
        "Part couverte par le registre actuel (plan valide ou question justifiée) : {covered}/{total}, soit {}.\n\n",
        percent(covered, total)
    ));
    if let Some(answers) = answers {
        let right = phrases
            .iter()
            .zip(answers)
            .filter(|(p, a)| as_expected(p, a))
            .count();
        let covered_right = phrases
            .iter()
            .zip(answers)
            .filter(|(p, a)| is_covered(p) && as_expected(p, a))
            .count();
        out.push_str(&format!(
            "Part couverte et bien rendue par l'agent local : {covered_right}/{total}, soit {}. Réponses conformes à l'attendu, refus compris : {right}/{total}, soit {}.\n\n",
            percent(covered_right, total),
            percent(right, total)
        ));
    }
    out.push_str("| Id | Source | Phrase | Attendu | Commande manquante |");
    out.push_str(if answers.is_some() {
        " Agent local |\n"
    } else {
        "\n"
    });
    out.push_str("|---|---|---|---|---|");
    out.push_str(if answers.is_some() { "---|\n" } else { "\n" });
    for (i, p) in phrases.iter().enumerate() {
        let src = &p["source"];
        let wanted = match kind(p) {
            "plan" => format!(
                "plan : {}",
                p["expected"]["commands"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" + ")
            ),
            "question" => "question justifiée".to_string(),
            _ => "non couverte".to_string(),
        };
        let missing = if is_covered(p) {
            "—".to_string()
        } else {
            missing(p).join(", ")
        };
        out.push_str(&format!(
            "| {} | {} ({}) | {} | {wanted} | {missing} |",
            p["id"].as_str().unwrap_or("?"),
            src["ref"].as_str().unwrap_or("?"),
            src["topic"].as_str().unwrap_or("?"),
            p["phrase"].as_str().unwrap_or("?"),
        ));
        if let Some(answers) = answers {
            let verdict = format!(
                "{} {:?}",
                if as_expected(p, &answers[i]) {
                    "OK"
                } else {
                    "ÉCART"
                },
                answers[i]
            );
            out.push_str(&format!(" {verdict} |"));
        }
        out.push('\n');
    }
    out
}

fn write_report(text: &str) {
    if let Some(path) = std::env::var_os("CASSINI_COUVERTURE_RAPPORT") {
        std::fs::write(&path, text).unwrap_or_else(|e| panic!("rapport illisible {path:?} : {e}"));
        eprintln!("Rapport écrit : {path:?}");
    }
}

#[test]
fn the_set_is_big_enough_and_every_sentence_has_a_unique_id_and_a_source() {
    let all = phrases();
    assert!(all.len() >= MIN_PHRASES, "{} phrases", all.len());
    let mut ids = BTreeSet::new();
    let mut texts = BTreeSet::new();
    for p in &all {
        let id = p["id"].as_str().expect("phrase sans id");
        assert!(ids.insert(id.to_string()), "id en double : {id}");
        let text = p["phrase"].as_str().unwrap_or_default();
        assert!(!text.trim().is_empty(), "{id} : phrase vide");
        assert!(texts.insert(text.to_string()), "{id} : phrase en double");
        let src = &p["source"];
        for key in ["type", "ref", "topic", "treatment"] {
            assert!(
                src[key].as_str().is_some_and(|v| !v.trim().is_empty()),
                "{id} : source sans `{key}`"
            );
        }
        // Source publique, phrase réécrite : jamais de copie littérale déclarée.
        assert!(
            matches!(src["type"].as_str(), Some("doc" | "forum" | "cours")),
            "{id} : type de source {}",
            src["type"]
        );
        assert_eq!(src["treatment"], "paraphrase", "{id}");
        assert!(
            !src["ref"].as_str().unwrap().to_lowercase().contains("esri"),
            "{id} : source Esri interdite (CLAUDE.md §2.1)"
        );
    }
}

#[test]
fn every_expected_answer_is_well_formed_and_names_real_commands() {
    let registry = Registry::standard();
    let ids: BTreeSet<String> = common::registry_ids(&registry).into_iter().collect();
    let all = phrases();
    for p in &all {
        let id = p["id"].as_str().unwrap();
        match kind(p) {
            "plan" => {
                let commands = p["expected"]["commands"].as_array().expect("commands");
                assert!(!commands.is_empty(), "{id} : plan sans commande");
                for c in commands {
                    let c = c.as_str().unwrap_or_default();
                    assert!(ids.contains(c), "{id} : commande inconnue {c}");
                }
            }
            "question" | "refusal" => {}
            other => panic!("{id} : sorte attendue inconnue {other}"),
        }
    }
    let mut errors = common::refusals_now_wrong(&common::registry_ids(&registry), &all);
    errors.extend(common::malformed_refusals(&registry, &all));
    assert!(
        errors.is_empty(),
        "refus invalides :\n{}",
        errors.join("\n")
    );
}

#[test]
fn the_set_mixes_plans_questions_and_uncovered_requests() {
    let all = phrases();
    let count = |k: &str| all.iter().filter(|p| kind(p) == k).count();
    assert!(count("plan") >= 20, "{} plans", count("plan"));
    assert!(count("question") >= 3, "{} questions", count("question"));
    assert!(count("refusal") >= 5, "{} non couvertes", count("refusal"));
    // Chaque phrase non couverte nomme la commande qui manque.
    for p in all.iter().filter(|p| !is_covered(p)) {
        assert!(!missing(p).is_empty(), "{} : rien ne manque", p["id"]);
    }
}

#[test]
fn the_report_gives_source_verdict_and_missing_command_for_every_sentence() {
    let all = phrases();
    let text = report(&all, None);
    for p in &all {
        let id = format!("| {} |", p["id"].as_str().unwrap());
        let line = text.lines().find(|l| l.starts_with(&id)).expect("ligne");
        assert!(
            line.contains(p["source"]["ref"].as_str().unwrap()),
            "{line}"
        );
        if !is_covered(p) {
            assert!(line.contains(&missing(p).join(", ")), "{line}");
        }
    }
    let covered = all.iter().filter(|p| is_covered(p)).count();
    assert!(text.contains(&format!("{covered}/{}", all.len())));
    // Sans modèle, le rapport est quand même écrit si on le demande.
    write_report(&text);
}

#[test]
fn an_answer_is_scored_against_what_the_sentence_expects() {
    let plan = serde_json::json!({"expected": {"kind": "plan", "commands": ["analysis.buffer"]}});
    let question = serde_json::json!({"expected": {"kind": "question"}});
    let refusal = serde_json::json!({"expected": {"kind": "refusal", "missing": ["clip"]}});
    assert!(as_expected(&plan, &Answer::Plan));
    assert!(!as_expected(&plan, &Answer::OtherPlan));
    assert!(!as_expected(&plan, &Answer::Refusal));
    assert!(as_expected(&question, &Answer::Question));
    assert!(!as_expected(&question, &Answer::Plan));
    assert!(as_expected(&refusal, &Answer::Refusal));
    assert!(as_expected(&refusal, &Answer::Question));
    assert!(!as_expected(&refusal, &Answer::Plan));
    let registry = Registry::standard();
    let refused: Result<Plan, AgentError> = Err(AgentError::Refused("non".into()));
    assert_eq!(classify(&registry, &refusal, &refused), Answer::Refusal);
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn couverture_reelle_measures_what_the_local_agent_does_with_each_sentence() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let (_, ids) = common::shown_inventory(&inventory);
    let mut generator = model_files_generator(files);
    let all = phrases();
    let mut answers = Vec::new();
    for p in &all {
        let (id, text) = (p["id"].as_str().unwrap(), p["phrase"].as_str().unwrap());
        let got = common::restored(
            compose_plan_with(
                style,
                &registry,
                &common::view(&inventory, text),
                text,
                &mut generator,
            ),
            &ids,
        );
        let got = common::completed(&registry, got, &inventory);
        let answer = classify(&registry, p, &got);
        eprintln!(
            "{} {id} {text}\n     attendu : {}\n     obtenu  : {got:?}",
            if as_expected(p, &answer) {
                "OK  "
            } else {
                "FAIL"
            },
            p["expected"]
        );
        answers.push(answer);
    }
    let text = report(&all, Some(&answers));
    eprintln!("{}", text.lines().take(3).collect::<Vec<_>>().join("\n"));
    write_report(&text);
}
