//! Chaîne d'entraînement du modèle de composition (ticket 5.10, tâche 1) : un générateur
//! déterministe d'exemples « inventaire + demande française → plan attendu ».
//!
//! Chaque exemple est construit par une famille de tournures (`families`) sur un inventaire
//! synthétique (`inventories`), puis **gardé seulement s'il passe `Registry::check_plan`** du
//! registre réel, et quelques gardes de plus (couches citées présentes dans l'inventaire,
//! fichier ouvert parmi les fichiers non chargés, question et refus non vides). Les
//! enregistrements sont structurés ; leur mise en forme en prompt court (tâche 2) vit dans
//! `prompt`, seul endroit qui décide du texte vu par le modèle.

pub mod families;
pub mod inventories;
pub mod prompt;
pub mod rng;

use std::collections::{BTreeMap, HashSet};

use cassini_core::plan::Plan;
use cassini_core::registry::Registry;
use serde_json::json;

use families::{Example, Retry, FAMILIES};
use rng::Rng;

/// Graine par défaut : celle de la commande documentée et du test de recouvrement (le jeu
/// engendré n'est pas versionné, il se régénère).
pub const DEFAULT_SEED: u64 = 1;
/// Nombre d'exemples par défaut : celui de l'entraînement (décision du propriétaire, 5.15).
pub const DEFAULT_COUNT: usize = 8000;
/// Plafond d'étapes d'un plan, celui de `cassini-ai` (`MAX_STEPS`).
const MAX_STEPS: usize = 12;

/// Décompte d'une famille : exemples gardés, écartés par la validation, doublons.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Tally {
    pub kept: usize,
    pub rejected: usize,
    pub duplicates: usize,
}

/// Résumé d'une génération, par famille (ordre alphabétique).
#[derive(Debug, Default, Clone)]
pub struct Summary {
    pub families: BTreeMap<&'static str, Tally>,
    /// Premiers motifs d'écart, pour le diagnostic.
    pub reasons: Vec<String>,
}

/// Paramètres de couche qui désignent une couche existante (ou une étape précédente).
const LAYER_PARAMS: [&str; 3] = ["layer_id", "overlay_layer_id", "points_layer_id"];

/// Vérifie un exemple : plan lisible, `check_plan` du registre réel, couches et fichiers cités
/// présents dans l'inventaire, plafond d'étapes, question et refus non vides. Rend le motif
/// d'écart sinon.
pub fn validate(registry: &Registry, example: &Example) -> Result<(), String> {
    let plan: Plan = serde_json::from_value(example.expected.clone())
        .map_err(|e| format!("plan illisible : {e}"))?;
    registry
        .check_plan(&plan)
        .map_err(|e| format!("{} (étape {}) : {}", e.code, e.step, e.message))?;
    match &plan {
        Plan::Question { question, .. } => {
            if question.trim().is_empty() || !question.trim_end().ends_with('?') {
                return Err("question vide ou sans point d'interrogation".into());
            }
        }
        Plan::Refusal { reason } => {
            // Une question écrite dans un refus est le défaut à corriger, pas à apprendre.
            if reason.trim().is_empty() || reason.contains('?') {
                return Err("refus vide ou formulé en question".into());
            }
        }
        Plan::Calls { calls } => {
            if calls.is_empty() || calls.len() > MAX_STEPS {
                return Err(format!("{} étapes", calls.len()));
            }
            let ids: HashSet<&str> = example.inventory["layers"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|l| l["layer_id"].as_str())
                .collect();
            let files: HashSet<&str> = example.inventory["unloaded_files"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|f| f["path"].as_str())
                .collect();
            for (index, call) in calls.iter().enumerate() {
                for name in LAYER_PARAMS {
                    if let Some(id) = call.params[name].as_str() {
                        if !id.starts_with("step:") && !ids.contains(id) {
                            return Err(format!("étape {} : couche absente « {id} »", index + 1));
                        }
                    }
                }
                if let Some(path) = call.params["path"].as_str() {
                    if !files.contains(path) {
                        return Err(format!("étape {} : fichier absent « {path} »", index + 1));
                    }
                }
            }
        }
    }
    if example.request.trim().is_empty() {
        return Err("demande vide".into());
    }
    if let Some(retry) = &example.retry {
        validate_retry(registry, &plan, retry)?;
    }
    Ok(())
}

/// Un exemple de redemande : la sortie rejetée se relit comme un plan d'appels, `check_plan`
/// la rejette, les écarts sont ceux du jugement de `cassini-ai`, et la réponse attendue est un
/// plan d'appels qui diffère de la sortie rejetée.
fn validate_retry(registry: &Registry, expected: &Plan, retry: &Retry) -> Result<(), String> {
    let rejected: Plan = serde_json::from_str(&retry.rejected)
        .map_err(|e| format!("sortie rejetée illisible : {e}"))?;
    if !matches!(rejected, Plan::Calls { .. }) || !matches!(expected, Plan::Calls { .. }) {
        return Err("redemande hors d'un plan d'appels".into());
    }
    if rejected == *expected {
        return Err("sortie rejetée identique au plan attendu".into());
    }
    match registry.check_plan(&rejected) {
        Ok(()) => Err("corruption acceptée par check_plan".into()),
        Err(e) if retry.errors == [format!("étape {} : {}", e.step, e.message)] => Ok(()),
        Err(_) => Err("écarts différents de ceux du jugement".into()),
    }
}

/// Engendre `count` exemples valides et distincts avec la graine `seed`. Même graine, même
/// suite, à l'octet près. S'arrête plus tôt (et le résumé le montre) si les familles ne
/// rendent plus d'exemples neufs.
pub fn generate(seed: u64, count: usize) -> (Vec<Example>, Summary) {
    let registry = Registry::standard();
    let mut rng = Rng::new(seed);
    let mut summary = Summary::default();
    for family in FAMILIES {
        summary.families.insert(family.name, Tally::default());
    }
    let total: u64 = FAMILIES.iter().map(|f| f.weight).sum();
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(count);
    let mut attempts = 0usize;
    while out.len() < count && attempts < count.saturating_mul(50).max(1000) {
        attempts += 1;
        let mut roll = rng.below(total as usize) as u64;
        let family = FAMILIES
            .iter()
            .find(|f| {
                if roll < f.weight {
                    true
                } else {
                    roll -= f.weight;
                    false
                }
            })
            .expect("tirage dans la somme des poids");
        let Some(example) = (family.make)(&mut rng) else {
            continue;
        };
        let tally = summary
            .families
            .get_mut(family.name)
            .expect("famille connue");
        if let Err(reason) = validate(&registry, &example) {
            tally.rejected += 1;
            if summary.reasons.len() < 20 {
                summary
                    .reasons
                    .push(format!("{} : {reason} — {}", family.name, example.request));
            }
            continue;
        }
        let line = to_line(&example);
        if !seen.insert(line) {
            tally.duplicates += 1;
            continue;
        }
        tally.kept += 1;
        out.push(example);
    }
    (out, summary)
}

/// Un enregistrement JSON sur une ligne : `{inventory, request, expected}`. La famille reste
/// dans `Example` et le résumé, pas dans le fichier.
pub fn to_line(example: &Example) -> String {
    let mut line = json!({
        "inventory": example.inventory,
        "request": example.request,
        "expected": example.expected,
    });
    // Un exemple de redemande porte aussi sa sortie rejetée et ses écarts.
    if let Some(retry) = &example.retry {
        line["retry"] = json!({"rejected": retry.rejected, "errors": retry.errors});
    }
    line.to_string()
}

/// Le texte exact vu par le modèle pour cet exemple : le prompt du premier essai, ou celui de
/// la redemande.
pub fn prompt_text(example: &Example) -> String {
    match &example.retry {
        None => prompt::render_prompt(&example.inventory, &example.request),
        Some(retry) => prompt::render_retry_prompt(
            &example.inventory,
            &example.request,
            &retry.rejected,
            &retry.errors,
        ),
    }
}

/// Le fichier JSONL entier, une ligne par exemple, fin de ligne `\n`.
pub fn to_jsonl(examples: &[Example]) -> String {
    let mut text = String::new();
    for e in examples {
        text.push_str(&to_line(e));
        text.push('\n');
    }
    text
}

/// Un enregistrement de prompt sur une ligne : `{prompt, completion}`, mis en forme par
/// `prompt` (tâche 2). C'est ce que lit la chaîne d'entraînement, sans rien reformater.
pub fn to_chat_line(example: &Example) -> String {
    let plan: Plan = serde_json::from_value(example.expected.clone())
        .expect("un exemple écrit a passé `validate`, son plan se relit");
    json!({
        "prompt": prompt_text(example),
        "completion": prompt::completion(&plan),
    })
    .to_string()
}

/// Le fichier JSONL de prompts entier, une ligne par exemple, fin de ligne `\n`.
pub fn to_chat_jsonl(examples: &[Example]) -> String {
    let mut text = String::new();
    for e in examples {
        text.push_str(&to_chat_line(e));
        text.push('\n');
    }
    text
}

/// Forme normalisée d'une phrase pour comparer deux jeux : minuscules, sans accents ni
/// ponctuation, espaces réduits.
pub fn normalize(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        let mapped = match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => "a",
            'ç' => "c",
            'é' | 'è' | 'ê' | 'ë' => "e",
            'î' | 'ï' | 'í' | 'ì' => "i",
            'ô' | 'ö' | 'ó' | 'ò' => "o",
            'ù' | 'û' | 'ü' | 'ú' => "u",
            'ÿ' => "y",
            'œ' => "oe",
            'æ' => "ae",
            c if c.is_alphanumeric() => {
                out.push(c);
                continue;
            }
            _ => " ",
        };
        out.push_str(mapped);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn the_same_seed_gives_the_same_file_and_another_seed_another() {
        let (a, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let (b, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        assert_eq!(to_jsonl(&a), to_jsonl(&b));
        let (c, _) = generate(DEFAULT_SEED + 1, DEFAULT_COUNT);
        assert_ne!(to_jsonl(&a), to_jsonl(&c));
    }

    #[test]
    fn every_written_example_passes_check_plan_again() {
        let registry = Registry::standard();
        let (examples, summary) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        assert_eq!(examples.len(), DEFAULT_COUNT, "{:?}", summary.families);
        for line in to_jsonl(&examples).lines() {
            let record: Value = serde_json::from_str(line).unwrap();
            let plan: Plan = serde_json::from_value(record["expected"].clone()).unwrap();
            assert_eq!(registry.check_plan(&plan), Ok(()), "{line}");
            assert!(record["request"].as_str().is_some_and(|r| !r.is_empty()));
            assert!(record["inventory"]["layers"].is_array());
        }
    }

    #[test]
    fn an_invalid_plan_is_never_written() {
        let registry = Registry::standard();
        let inventory = json!({"layers": [{"layer_id": "gares_1"}], "unloaded_files": []});
        let make = |expected: Value| Example {
            family: "essai",
            inventory: inventory.clone(),
            request: "Tampon de 300 m autour des gares.".into(),
            expected,
            retry: None,
        };
        // Référence vers une étape qui n'existe pas : refusée par `check_plan`.
        let bad = make(json!({"kind": "calls", "calls": [
            {"command": "analysis.buffer", "params": {"layer_id": "step:3.layer_id",
                "distance": {"value": 300, "unit": "m"}}}]}));
        assert!(validate(&registry, &bad)
            .unwrap_err()
            .contains("plan.bad_reference"));
        // Couche absente de l'inventaire, commande inconnue, refus en forme de question.
        let ghost = make(json!({"kind": "calls", "calls": [
            {"command": "analysis.buffer", "params": {"layer_id": "routes_9",
                "distance": {"value": 300, "unit": "m"}}}]}));
        assert!(validate(&registry, &ghost)
            .unwrap_err()
            .contains("routes_9"));
        let unknown =
            make(json!({"kind": "calls", "calls": [{"command": "no.such", "params": {}}]}));
        assert!(validate(&registry, &unknown).is_err());
        let asking = make(json!({"kind": "refusal", "reason": "Quelle couche ?"}));
        assert!(validate(&registry, &asking).is_err());
        let good = make(json!({"kind": "calls", "calls": [
            {"command": "analysis.buffer", "params": {"layer_id": "gares_1",
                "distance": {"value": 300, "unit": "m"}}}]}));
        assert_eq!(validate(&registry, &good), Ok(()));
    }

    #[test]
    fn every_family_gives_at_least_one_valid_example() {
        let registry = Registry::standard();
        for family in FAMILIES {
            let mut rng = Rng::new(11);
            let valid = (0..50)
                .filter_map(|_| (family.make)(&mut rng))
                .filter(|e| validate(&registry, e).is_ok())
                .count();
            assert!(valid > 0, "{} ne rend aucun exemple valide", family.name);
        }
    }

    #[test]
    fn no_family_produces_rejected_examples() {
        // Un écart signale une famille mal écrite : le filtre est un garde-fou, pas un tamis.
        let (_, summary) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let rejected: usize = summary.families.values().map(|t| t.rejected).sum();
        assert_eq!(rejected, 0, "{:#?}", summary.reasons);
    }

    #[test]
    fn the_decided_families_and_vocabulary_are_present() {
        let (examples, summary) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        assert_eq!(examples.len(), DEFAULT_COUNT);
        for (name, tally) in &summary.families {
            assert!(tally.kept > 0, "famille absente : {name}");
        }
        let requests: Vec<String> = examples.iter().map(|e| e.request.to_lowercase()).collect();
        for needle in [
            "constructible",
            "garde seulement",
            "ne garde que",
            "conserve",
            "à moins de",
            "pdf",
        ] {
            assert!(requests.iter().any(|r| r.contains(needle)), "{needle}");
        }
        // Environ 5 % de redemandes (au moins 4 %, jamais plus de 7 %).
        let retries = examples.iter().filter(|e| e.retry.is_some()).count();
        assert!(
            retries * 25 >= examples.len() && retries * 100 <= examples.len() * 7,
            "{retries} redemandes"
        );
        let kinds = |k: &str| examples.iter().filter(|e| e.expected["kind"] == k).count();
        assert!(kinds("question") > 0 && kinds("refusal") > 0 && kinds("calls") > 0);
        // Chaînes à plusieurs conditions : au moins deux intersections successives.
        assert!(examples.iter().any(|e| {
            e.expected["calls"].as_array().is_some_and(|c| {
                c.iter()
                    .filter(|s| s["command"] == "analysis.intersect")
                    .count()
                    >= 2
            })
        }));
    }

    #[test]
    fn a_commune_named_only_as_context_adds_no_step() {
        // « … à Orly » : sans tournure d'appartenance, la commune ne crée aucune sélection.
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let mut seen = 0;
        for e in &examples {
            let r = e.request.to_lowercase();
            if r.contains("appartien") || r.contains("territoire") {
                continue;
            }
            let Some(calls) = e.expected["calls"].as_array() else {
                continue;
            };
            for commune in families::COMMUNES {
                if !e.request.contains(&format!(" à {commune}")) {
                    continue;
                }
                seen += 1;
                assert!(
                    calls.iter().all(|c| c["params"]["value"] != *commune),
                    "{}",
                    e.request
                );
            }
        }
        assert!(seen > 100, "{seen} demandes avec une commune de contexte");
    }

    #[test]
    fn the_chat_line_of_a_retry_example_carries_the_retry_prompt() {
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let mut retries = 0;
        for e in &examples {
            let line: Value = serde_json::from_str(&to_chat_line(e)).unwrap();
            let want = match &e.retry {
                Some(r) => {
                    retries += 1;
                    prompt::render_retry_prompt(&e.inventory, &e.request, &r.rejected, &r.errors)
                }
                None => prompt::render_prompt(&e.inventory, &e.request),
            };
            assert_eq!(line["prompt"], want, "{}", e.request);
        }
        assert!(retries > 0);
    }

    /// Une distance écrite dans la demande : un nombre suivi de « m » ou « km » (espace
    /// facultative, pas de lettre après), ou les mots entiers « mètre(s) », « kilomètre(s) ».
    /// Une durée (« 10 min ») ou « périmètre » n'en sont pas.
    fn has_distance(request: &str) -> bool {
        let lower = request.to_lowercase();
        let words = lower
            .split(|c: char| !c.is_alphabetic())
            .any(|w| matches!(w, "mètre" | "mètres" | "kilomètre" | "kilomètres"));
        let chars: Vec<char> = lower.chars().collect();
        let unit = (0..chars.len()).any(|i| {
            if !chars[i].is_ascii_digit() {
                return false;
            }
            let mut j = i + 1;
            if chars.get(j) == Some(&' ') {
                j += 1;
            }
            if chars.get(j) == Some(&'k') {
                j += 1;
            }
            chars.get(j) == Some(&'m') && !chars.get(j + 1).is_some_and(|c| c.is_alphabetic())
        });
        words || unit
    }

    #[test]
    fn a_travel_without_coordinates_never_becomes_a_plan() {
        // Une durée de trajet depuis un lieu sans coordonnées mène à une question (un refus
        // depuis plusieurs lieux, décision a) ; une question de coordonnées ne porte jamais sur
        // une distance en mètres, qui reste un plan.
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let modes = [
            "à pied",
            "en marchant",
            "à la marche",
            "en voiture",
            "en auto",
            "en véhicule",
        ];
        let mut travels = 0;
        for e in &examples {
            let r = e.request.to_lowercase();
            let travel = modes.iter().any(|m| r.contains(m));
            let coordinates = ["lambert", "epsg", "wgs"].iter().any(|c| r.contains(c));
            if travel && !coordinates {
                travels += 1;
                let refused = e.expected["reason"] == families::NO_MULTI_ORIGIN;
                assert!(
                    e.expected["kind"] == "question" || refused,
                    "{} → {}",
                    e.request,
                    e.expected
                );
            }
            let asks_point = e.expected["question"]
                .as_str()
                .is_some_and(|q| q.contains("coordonnées"));
            if asks_point {
                assert!(travel && !has_distance(&e.request), "{}", e.request);
            }
        }
        assert!(travels > 50, "{travels}");
        assert!(has_distance("à moins de 500 m d'une gare"));
        assert!(has_distance("à 300m"));
        assert!(has_distance("à moins de 2 km"));
        assert!(has_distance("à deux kilomètres"));
        assert!(!has_distance("à 10 min à pied"));
        assert!(!has_distance("dans le périmètre"));
    }

    #[test]
    fn the_isochrone_still_starts_from_a_single_point() {
        // Le refus des origines multiples tient tant que l'origine est un seul point.
        let registry = Registry::standard();
        let spec = registry.spec("analysis.isochrone").expect("isochrone");
        let schema = &spec.params_schema;
        let mut origin = &schema["properties"]["origin"];
        // `{"$ref": "#/$defs/Point"}` : la forme est dans les définitions du schéma.
        if let Some(name) = origin["$ref"]
            .as_str()
            .and_then(|r| r.strip_prefix("#/$defs/"))
        {
            origin = &schema["$defs"][name];
        }
        assert!(origin.is_object(), "{schema}");
        assert_ne!(origin["type"], "array", "{origin}");
        for key in ["x", "y", "epsg"] {
            assert!(origin["properties"][key].is_object(), "{key} : {origin}");
        }
    }

    #[test]
    fn no_plan_of_the_generated_set_passes_six_steps() {
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        for e in &examples {
            let n = e.expected["calls"].as_array().map_or(0, Vec::len);
            assert!(n <= 6, "{n} étapes : {}", e.request);
        }
    }

    #[test]
    fn normalization_drops_case_accents_and_punctuation() {
        assert_eq!(
            normalize("Trouve les parcelles « constructibles » à 500 m d'une gare !"),
            "trouve les parcelles constructibles a 500 m d une gare"
        );
        assert_eq!(normalize("Écoles   Œuvre"), "ecoles oeuvre");
    }
}
