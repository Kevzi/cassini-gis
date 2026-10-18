//! Mesure de référence de la durée par phrase (ticket 11), avec le vrai modèle.
//! Ignoré sans `CASSINI_TEST_MODEL` (et marqué `ignore` : il charge le modèle à chaque appel).
//!
//!   CASSINI_TEST_MODEL=<gguf> CASSINI_TEST_THREADS=8 \
//!     cargo test -p cassini-ai --release -- --ignored --nocapture duree_reel
//!
//! Chaque phrase passe par `compose_plan`, comme dans le produit : modèle rechargé à chaque appel,
//! redemande comprise. Les durées de chargement, de lecture de l'invite et d'écriture sont
//! additionnées par phrase, avec le nombre d'appels au modèle. Sans `CASSINI_TEST_THREADS`, le
//! nombre de fils est celui de llama.cpp par défaut. Avec `CASSINI_TEST_INVENTORY=<json>`, ce sont
//! les phrases A à E du scénario Villeneuve-le-Roi qui passent, sur cet inventaire (hors dépôt).
//! Un appel qui échoue (par exemple `PromptTooLong` à la redemande, que `compose_plan` absorbe)
//! n'est ni compté ni chronométré.
//!
//! Aucune assertion sur une durée : c'est une mesure. La justesse reste portée par `plan_reel`.

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::time::Duration;

use cassini_ai::generate_measured;
use cassini_ai::plan::{compose_plan, AgentError};
use cassini_ai::Timings;
use cassini_core::plan::Plan;
use cassini_core::registry::Registry;
use serde_json::Value;

const INVENTORY: &str = include_str!("../../tests/epreuve-modele-local/inventory.json");
const SENTENCES: &str = include_str!("../../tests/epreuve-modele-local/sentences.json");

/// Phrases du scénario de démo, mesurées chacune à part (pas encore de mémoire entre demandes).
const SCENARIO: [(&str, &str); 7] = [
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

/// Durées d'une phrase : somme des appels au modèle.
#[derive(Default)]
struct Row {
    calls: usize,
    load: Duration,
    prompt: Duration,
    prompt_tokens: usize,
    write: Duration,
    written_tokens: usize,
}

impl Row {
    fn add(&mut self, t: &Timings) {
        self.calls += 1;
        self.load += t.load;
        self.prompt += t.prompt;
        self.prompt_tokens += t.prompt_tokens;
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

fn secs(d: Duration) -> String {
    format!("{:.1}", d.as_secs_f64()).replace('.', ",")
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn duree_reel_par_phrase() {
    let Some(model) = std::env::var_os("CASSINI_TEST_MODEL") else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let model = PathBuf::from(model);
    let threads = std::env::var("CASSINI_TEST_THREADS").ok().map(|v| {
        v.trim()
            .parse::<NonZeroU32>()
            .unwrap_or_else(|_| panic!("CASSINI_TEST_THREADS illisible : {v}"))
    });
    let (inventory, sentences): (Value, Vec<(String, String)>) =
        match std::env::var_os("CASSINI_TEST_INVENTORY") {
            Some(path) => {
                let inventory = read_inventory(&PathBuf::from(path));
                let sentences = SCENARIO
                    .iter()
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
    let registry = Registry::standard();

    let fils = threads.map_or("défaut de llama.cpp".to_string(), |n| n.to_string());
    println!("Fils : {fils}");
    println!();
    println!("| id | appels | chargement (s) | lecture du prompt (s) | jetons lus | écriture (s) | jetons écrits | total (s) | issue |");
    println!("|---|---|---|---|---|---|---|---|---|");
    let mut rows = Vec::new();
    for (id, text) in &sentences {
        let mut row = Row::default();
        let got = {
            let mut generator = |request: &cassini_ai::plan::GenerationRequest| {
                let (completion, timings) = generate_measured(
                    &model,
                    request.turns,
                    Some(request.grammar),
                    request.max_new_tokens,
                    threads,
                )?;
                row.add(&timings);
                Ok(completion)
            };
            compose_plan(&registry, &inventory, text, &mut generator)
        };
        println!(
            "| {id} | {} | {} | {} | {} | {} | {} | {} | {} |",
            row.calls,
            secs(row.load),
            secs(row.prompt),
            row.prompt_tokens,
            secs(row.write),
            row.written_tokens,
            secs(row.total()),
            outcome(&got)
        );
        rows.push(row);
    }
    let n = rows.len().max(1) as u32;
    let sum = |f: fn(&Row) -> Duration| rows.iter().map(f).sum::<Duration>() / n;
    let calls: usize = rows.iter().map(|r| r.calls).sum();
    println!();
    println!(
        "Moyennes sur {} phrases ({calls} appels) : chargement {} s, lecture du prompt {} s, écriture {} s, total {} s",
        rows.len(),
        secs(sum(|r| r.load)),
        secs(sum(|r| r.prompt)),
        secs(sum(|r| r.write)),
        secs(sum(Row::total)),
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
