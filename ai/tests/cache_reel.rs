//! Essai avec le vrai modèle : le modèle gardé et la partie fixe de l'invite reprise (ticket 12).
//! Ignoré sans `CASSINI_TEST_MODEL` (et marqué `ignore` : il charge le vrai modèle).
//!
//!   CASSINI_TEST_MODEL=<gguf> cargo test -p cassini-ai --release --features cuda \
//!     -- --ignored --nocapture cache_reel
//!
//! Un seul test, parce que le modèle et ses instantanés sont communs à tout le processus : l'ordre
//! des appels fait partie de ce qui est vérifié. Des instantanés repris au 2ᵉ appel prouvent que
//! le modèle n'a pas été rechargé : un rechargement les libère.

use std::path::{Path, PathBuf};

use cassini_ai::generate_measured;
use cassini_ai::plan::{compose_plan, GenerationRequest};
use cassini_ai::{Role, Timings, Turn};
use cassini_core::registry::Registry;
use serde_json::Value;

const INVENTORY: &str = include_str!("../../tests/epreuve-modele-local/inventory.json");

/// Lien dur vers le modèle, effacé quoi qu'il arrive (même si l'essai panique).
struct Link(PathBuf);

impl Link {
    fn new(model: &Path) -> Self {
        let link = model.with_file_name(format!("cassini-lien-{}.gguf", std::process::id()));
        let _ = std::fs::remove_file(&link);
        std::fs::hard_link(model, &link).expect("lien vers le modèle");
        Link(link)
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Première sortie du modèle pour `request` sur `inventory`, et ses durées.
fn first_call(model: &Path, inventory: &Value, request: &str) -> (String, Timings) {
    let mut calls: Vec<(String, Timings)> = Vec::new();
    let mut generator = |r: &GenerationRequest| {
        let (completion, timings) =
            generate_measured(model, r.turns, Some(r.grammar), r.max_new_tokens, None)?;
        calls.push((completion.text.clone(), timings));
        Ok(completion)
    };
    let _ = compose_plan(&Registry::standard(), inventory, request, &mut generator);
    calls
        .into_iter()
        .next()
        .expect("au moins un appel au modèle")
}

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn cache_reel_same_request_twice_then_a_changed_inventory() {
    let Some(model) = std::env::var_os("CASSINI_TEST_MODEL") else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let model = PathBuf::from(model);
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let request = "Fais une zone tampon de 300 m autour des gares.";

    let (text_1, t1) = first_call(&model, &inventory, request);
    eprintln!("1er appel : {t1:?}\n  {text_1}");
    assert_eq!(
        t1.cached_tokens, 0,
        "premier appel du processus : rien à reprendre"
    );

    // Même demande, même projet : reprise après le tour système, sortie identique.
    let (text_2, t2) = first_call(&model, &inventory, request);
    eprintln!("2e appel : {t2:?}\n  {text_2}");
    assert_eq!(
        text_2, text_1,
        "à invite égale, la sortie gloutonne est la même"
    );
    assert!(t2.cached_tokens > 0, "le 2e appel reprend un instantané");
    assert!(t2.cached_tokens < t2.prompt_tokens);
    assert_eq!(t2.prompt_tokens, t1.prompt_tokens);

    // Inventaire changé : seule la partie fixe est reprise.
    let mut other = inventory.clone();
    other["layers"]
        .as_array_mut()
        .expect("l'inventaire de l'épreuve a des couches")
        .pop();
    let (text_3, t3) = first_call(&model, &other, request);
    eprintln!("3e appel (inventaire changé) : {t3:?}\n  {text_3}");
    assert!(t3.cached_tokens > 0, "la partie fixe est reprise");
    assert!(
        t3.cached_tokens < t2.cached_tokens,
        "reprise à la partie fixe, pas après le tour système"
    );

    // Le nouvel inventaire a remplacé l'instantané du tour système.
    let (text_4, t4) = first_call(&model, &other, request);
    eprintln!("4e appel : {t4:?}\n  {text_4}");
    assert_eq!(text_4, text_3);
    assert!(t4.cached_tokens > t3.cached_tokens);

    // Redemande : mêmes tours que la première tentative, plus la sortie rejetée et les écarts.
    // Elle reprend l'instantané du tour système, comme une première tentative.
    let mut first_turns: Vec<Turn> = Vec::new();
    let mut grammar = String::new();
    let mut recorder = |r: &GenerationRequest| {
        if first_turns.is_empty() {
            first_turns = r.turns.to_vec();
            grammar = r.grammar.to_string();
        }
        generate_measured(&model, r.turns, Some(r.grammar), r.max_new_tokens, None).map(|(c, _)| c)
    };
    let _ = compose_plan(&Registry::standard(), &other, request, &mut recorder);
    let mut retry = first_turns.clone();
    retry.push(Turn::new(Role::Assistant, text_4.clone()));
    retry.push(Turn::new(
        Role::User,
        "Ta réponse n'est pas conforme : étape 1 : essai. Redonne le plan corrigé, uniquement le JSON.",
    ));
    let (_, t5) = generate_measured(&model, &retry, Some(&grammar), 1024, None).unwrap();
    eprintln!("redemande : {t5:?}");
    assert_eq!(
        t5.cached_tokens, t4.cached_tokens,
        "la redemande reprend après le tour système"
    );

    // Autre fichier de modèle (un lien vers le même GGUF, donc un autre chemin) : le modèle gardé
    // et ses instantanés sont libérés, rien n'est repris ; revenir au premier recharge aussi.
    let link = Link::new(&model);
    let (_, t6) = first_call(&link.0, &other, request);
    let (_, t7) = first_call(&model, &other, request);
    drop(link);
    eprintln!("autre modèle : {t6:?}\nretour au premier : {t7:?}");
    // Un vrai rechargement lit le fichier : bien plus long que la reprise d'un modèle gardé.
    assert!(t6.load > t5.load * 10, "autre modèle : rechargé");
    assert!(t7.load > t5.load * 10, "retour au premier : rechargé");
    assert_eq!(t6.cached_tokens, 0, "autre modèle : rien de repris");
    assert_eq!(
        t7.cached_tokens, 0,
        "retour au premier : rechargé, rien de repris"
    );
}
