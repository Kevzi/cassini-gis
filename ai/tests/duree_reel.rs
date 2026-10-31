//! Mesure de la durée par phrase (tickets 11 et 12), avec le vrai modèle.
//! Ignoré sans `CASSINI_TEST_MODEL` (et marqué `ignore` : il charge le vrai modèle).
//!
//!   CASSINI_TEST_MODEL=<gguf ou déclaration *.modele.json> CASSINI_TEST_THREADS=4 \
//!     cargo test -p cassini-ai --release -- --ignored --nocapture duree_reel
//!
//! Jamais `--features cuda`. Mesure de la tâche 5.10 4b, sous PowerShell, le binaire d'essai épinglé
//! sur 4 cœurs physiques du 5800X par le masque d'affinité `0x55` :
//!
//!   $env:CASSINI_TEST_THREADS = "4"; $env:CASSINI_TEST_MODEL = "<déclaration>"
//!   $env:CASSINI_TEST_INVENTORY = "<inventaire traduit>"
//!   $p = Start-Process C:\ct\release\deps\duree_reel-<empreinte>.exe `
//!          -ArgumentList "--ignored","--nocapture","duree_reel" -NoNewWindow -PassThru `
//!          -RedirectStandardOutput duree.txt -RedirectStandardError duree.err
//!   $p.ProcessorAffinity = [IntPtr]0x55
//!   $p.WaitForExit()
//!
//! Chaque phrase passe par `compose_plan`, comme dans le produit, redemande comprise, et toutes
//! dans le même processus : le modèle est chargé au premier appel puis gardé, et la lecture de la
//! partie fixe de l'invite est reprise d'un instantané (ticket 12). Les durées de chargement, de
//! lecture de l'invite et d'écriture sont additionnées par phrase, avec le nombre d'appels au
//! modèle et les jetons repris d'un instantané. Sans `CASSINI_TEST_THREADS`, le nombre de fils est
//! celui du produit par défaut (les cœurs physiques). Avec `CASSINI_TEST_INVENTORY=<json>`, ce sont
//! les phrases A0 à E du scénario Villeneuve-le-Roi qui passent, sur cet inventaire (hors dépôt) ;
//! A0 est la phrase de démo exacte de `CLAUDE.md`. Elles sont précédées d'une amorce (`WARMUP`),
//! qui paie seule le chargement du modèle et la lecture complète de l'invite jusqu'à l'instantané
//! du tour système, comme le premier appel d'une session de `cassini serve` : la ligne de A0 mesure
//! donc sa demande seule, chargement et instantané à part. L'amorce n'entre pas dans les moyennes.
//! Un appel qui échoue (par exemple `PromptTooLong` à la redemande, que `compose_plan` absorbe)
//! n'est ni compté ni chronométré.
//!
//! Aucune assertion sur une durée : c'est une mesure. La justesse reste portée par `plan_reel`.

mod common;

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::time::Duration;

use cassini_ai::generate_files_measured;
use cassini_ai::plan::{compose_plan_with, AgentError};
use cassini_ai::Timings;
use cassini_core::plan::Plan;
use cassini_core::registry::Registry;
use serde_json::Value;

const INVENTORY: &str = include_str!("../../tests/epreuve-modele-local/inventory.json");
const SENTENCES: &str = include_str!("../../tests/epreuve-modele-local/sentences.json");

/// Phrases du scénario de démo, mesurées chacune à part (pas encore de mémoire entre demandes).
const SCENARIO: [(&str, &str); 8] = [
    (
        "A0",
        "Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi et sors-moi la carte en PDF.",
    ),
    (
        "A",
        "Trouve les parcelles constructibles du PLU (zones U) à moins de 500 m d'une gare à Villeneuve-le-Roi et sors-moi la carte en PDF.",
    ),
    ("B1", "Garde les zones U du PLU."),
    (
        "B2",
        "Trouve les parcelles dans ces zones à moins de 500 m d'une gare.",
    ),
    ("B3", "Sors-moi la carte en PDF."),
    ("C", "Trouve les bâtiments à moins de 50 m d'une route."),
    ("D", "Trouve les parcelles près des écoles."),
    (
        "E",
        "Trouve les parcelles à 10 minutes à pied de la gare de Villeneuve-le-Roi.",
    ),
];

/// Amorce du scénario : une demande courte, sur le même inventaire, passée avant A0.
const WARMUP: (&str, &str) = ("amorce", "Décris le projet.");

/// Durées d'une phrase : somme des appels au modèle.
#[derive(Default)]
struct Row {
    calls: usize,
    load: Duration,
    prompt: Duration,
    prompt_tokens: usize,
    cached_tokens: usize,
    write: Duration,
    written_tokens: usize,
}

impl Row {
    fn add(&mut self, t: &Timings) {
        self.calls += 1;
        self.load += t.load;
        self.prompt += t.prompt;
        self.prompt_tokens += t.prompt_tokens;
        self.cached_tokens += t.cached_tokens;
        self.write += t.write;
        self.written_tokens += t.written_tokens;
    }

    fn total(&self) -> Duration {
        self.load + self.prompt + self.write
    }
}

/// Issue de la phrase, en quelques mots.
fn outcome(got: &Result<Plan, AgentError>) -> String {
    match got {
        Ok(Plan::Calls { calls }) => format!("plan, {} étape(s)", calls.len()),
        Ok(Plan::Question { .. }) => "question".to_string(),
        Ok(Plan::Refusal { .. }) => "refus".to_string(),
        Err(AgentError::Refused(_)) => "refus".to_string(),
        Err(e) => format!("erreur : {}", e.code()),
    }
}

/// Inventaire du scénario, lu dans un fichier hors dépôt ; panique en citant le chemin.
fn read_inventory(path: &Path) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("inventaire illisible : {} ({e})", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("inventaire illisible : {} ({e})", path.display()))
}

/// Moyennes par phrase mesurée : l'amorce (`WARMUP`) en est exclue, toute autre phrase compte,
/// A0 comprise. `calls` est la somme des appels des phrases mesurées.
#[derive(Debug, PartialEq)]
struct Averages {
    sentences: usize,
    calls: usize,
    warmed: bool,
    load: Duration,
    prompt: Duration,
    write: Duration,
    total: Duration,
}

/// Moyennes des lignes `rows`, alignées une à une sur les identifiants `ids`.
fn averages(ids: &[&str], rows: &[Row]) -> Averages {
    assert_eq!(ids.len(), rows.len(), "une ligne par phrase");
    let measured: Vec<&Row> = ids
        .iter()
        .zip(rows)
        .filter(|(id, _)| **id != WARMUP.0)
        .map(|(_, r)| r)
        .collect();
    let n = measured.len().max(1) as u32;
    let mean = |f: fn(&Row) -> Duration| measured.iter().map(|r| f(r)).sum::<Duration>() / n;
    Averages {
        sentences: measured.len(),
        calls: measured.iter().map(|r| r.calls).sum(),
        warmed: ids.contains(&WARMUP.0),
        load: mean(|r| r.load),
        prompt: mean(|r| r.prompt),
        write: mean(|r| r.write),
        total: mean(Row::total),
    }
}

fn secs(d: Duration) -> String {
    format!("{:.1}", d.as_secs_f64()).replace('.', ",")
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn duree_reel_par_phrase() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let threads = std::env::var("CASSINI_TEST_THREADS").ok().map(|v| {
        v.trim()
            .parse::<NonZeroU32>()
            .unwrap_or_else(|_| panic!("CASSINI_TEST_THREADS illisible : {v}"))
    });
    let (inventory, sentences): (Value, Vec<(String, String)>) =
        match std::env::var_os("CASSINI_TEST_INVENTORY") {
            Some(path) => {
                let inventory = read_inventory(&PathBuf::from(path));
                let sentences = std::iter::once(&WARMUP)
                    .chain(&SCENARIO)
                    .map(|(id, text)| (id.to_string(), text.to_string()))
                    .collect();
                (inventory, sentences)
            }
            None => {
                let raw: Vec<Value> = serde_json::from_str(SENTENCES).unwrap();
                let sentences = raw
                    .iter()
                    .map(|s| {
                        (
                            s["id"].as_str().unwrap().to_string(),
                            s["text"].as_str().unwrap().to_string(),
                        )
                    })
                    .collect();
                (serde_json::from_str(INVENTORY).unwrap(), sentences)
            }
        };
    // Comme dans le produit : identifiants courts montrés, plan retraduit (ticket 5.19).
    let (inventory, ids) = common::shown_inventory(&inventory);
    let registry = Registry::standard();

    let fils = threads.map_or_else(
        || format!("défaut du produit ({})", cassini_ai::default_threads()),
        |n| n.to_string(),
    );
    println!("Fils : {fils}");
    println!();
    println!("| id | appels | chargement (s) | lecture du prompt (s) | jetons de l'invite | jetons repris | écriture (s) | jetons écrits | total (s) | issue |");
    println!("|---|---|---|---|---|---|---|---|---|---|");
    let mut rows = Vec::new();
    for (id, text) in &sentences {
        let mut row = Row::default();
        let got = {
            let mut generator = |request: &cassini_ai::plan::GenerationRequest| {
                let (completion, timings) = generate_files_measured(
                    &files,
                    request.turns,
                    Some(request.grammar),
                    request.max_new_tokens,
                    threads,
                )?;
                row.add(&timings);
                Ok(completion)
            };
            common::restored(
                compose_plan_with(style, &registry, &inventory, text, &mut generator),
                &ids,
            )
        };
        println!(
            "| {id} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            row.calls,
            secs(row.load),
            secs(row.prompt),
            row.prompt_tokens,
            row.cached_tokens,
            secs(row.write),
            row.written_tokens,
            secs(row.total()),
            outcome(&got)
        );
        // Une amorce sans appel laisserait le chargement et l'instantané dans la ligne de A0.
        if id == WARMUP.0 && row.calls == 0 {
            panic!(
                "l'amorce n'a fait aucun appel au modèle : la ligne de A0 ne serait pas à chaud"
            );
        }
        rows.push(row);
    }
    let ids: Vec<&str> = sentences.iter().map(|(id, _)| id.as_str()).collect();
    let mean = averages(&ids, &rows);
    println!();
    println!(
        "Moyennes sur {} phrases ({} appels{}) : chargement {} s, lecture du prompt {} s, écriture {} s, total {} s",
        mean.sentences,
        mean.calls,
        if mean.warmed { ", amorce exclue" } else { "" },
        secs(mean.load),
        secs(mean.prompt),
        secs(mean.write),
        secs(mean.total),
    );
    // Seule exigence : une ligne par phrase, et au moins un appel mesuré dans chacune (sinon le
    // modèle n'a pas répondu et la ligne ne mesure rien).
    assert_eq!(rows.len(), sentences.len());
    if let Some(((id, _), _)) = sentences.iter().zip(&rows).find(|(_, r)| r.calls == 0) {
        panic!("{id} : aucun appel au modèle n'a abouti, la ligne ne mesure rien");
    }
}

// Sans modèle : l'addition des appels, le nom des issues et la lecture de l'inventaire.

fn call(load: u64, prompt: u64, write: u64, read: usize, written: usize) -> Timings {
    Timings {
        load: Duration::from_millis(load),
        prompt: Duration::from_millis(prompt),
        prompt_tokens: read,
        cached_tokens: read / 2,
        write: Duration::from_millis(write),
        written_tokens: written,
    }
}

#[test]
fn a_retry_adds_both_calls_to_the_row() {
    let mut row = Row::default();
    row.add(&call(4000, 95000, 15000, 5130, 60));
    row.add(&call(4100, 99000, 9000, 5300, 40));
    assert_eq!(row.calls, 2);
    assert_eq!(row.load, Duration::from_millis(8100));
    assert_eq!(row.prompt, Duration::from_millis(194000));
    assert_eq!(row.write, Duration::from_millis(24000));
    assert_eq!((row.prompt_tokens, row.written_tokens), (10430, 100));
    assert_eq!(row.cached_tokens, 2565 + 2650);
    assert_eq!(row.total(), Duration::from_millis(226100));
    assert_eq!(secs(row.total()), "226,1");
}

#[test]
fn each_outcome_is_named_and_an_agent_error_still_gives_a_line() {
    let calls: Plan = serde_json::from_value(serde_json::json!({"kind": "calls", "calls": [
        {"command": "project.inventory", "params": {}}]}))
    .unwrap();
    assert_eq!(outcome(&Ok(calls)), "plan, 1 étape(s)");
    let question = Plan::Question {
        question: "Quelle couche ?".into(),
        source: cassini_core::plan::QuestionSource::Model,
    };
    assert_eq!(outcome(&Ok(question)), "question");
    assert_eq!(outcome(&Err(AgentError::Refused("non".into()))), "refus");
    let invalid = AgentError::PlanInvalid(vec!["écart".into()]);
    assert_eq!(outcome(&Err(invalid)), "erreur : agent.plan_invalid");
}

#[test]
#[should_panic(expected = "inventaire illisible : ")]
fn an_unreadable_inventory_panics_with_its_path() {
    read_inventory(Path::new("absent-inventaire-villeneuve.json"));
}

/// Une ligne d'un seul appel, de durées données en millisecondes.
fn row_of(load: u64, prompt: u64, write: u64) -> Row {
    let mut row = Row::default();
    row.add(&call(load, prompt, write, 100, 10));
    row
}

#[test]
fn averages_exclude_the_warmup_and_count_a0() {
    let rows = vec![
        row_of(9000, 9000, 9000),
        row_of(1000, 2000, 3000),
        row_of(3000, 4000, 5000),
    ];
    let mean = averages(&[WARMUP.0, "A0", "B1"], &rows);
    assert_eq!(
        mean,
        Averages {
            sentences: 2,
            calls: 2,
            warmed: true,
            load: Duration::from_millis(2000),
            prompt: Duration::from_millis(3000),
            write: Duration::from_millis(4000),
            total: Duration::from_millis(9000),
        }
    );
    // Sans amorce : toutes les phrases comptent.
    let mean = averages(&["A0", "B1"], &rows[1..]);
    assert_eq!((mean.sentences, mean.warmed), (2, false));
    assert_eq!(mean.total, Duration::from_millis(9000));
    // Les lignes restent alignées sur leur phrase : l'amorce en dernier n'exclut qu'elle.
    let mean = averages(
        &["A0", "B1", WARMUP.0],
        &[row_of(1000, 0, 0), row_of(3000, 0, 0), row_of(9000, 0, 0)],
    );
    assert_eq!(mean.load, Duration::from_millis(2000));
}

#[test]
fn averages_of_the_warmup_alone_are_zero() {
    let mean = averages(&[WARMUP.0], &[row_of(9000, 9000, 9000)]);
    assert_eq!((mean.sentences, mean.calls), (0, 0));
    assert_eq!(mean.total, Duration::ZERO);
}
