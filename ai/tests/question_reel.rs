//! Essai avec le vrai modèle : phrases ambiguës, où un mot de la demande ne désigne rien de
//! l'inventaire de l'épreuve. Le modèle doit poser une question au lieu de deviner.
//! Ignoré sans `CASSINI_TEST_MODEL` (et marqué `ignore` : il charge le modèle une fois par phrase).
//!
//!   CASSINI_TEST_MODEL=<gguf ou déclaration *.modele.json> cargo test -p cassini-ai --release --features cuda -- --ignored question_reel
//!
//! L'inventaire de l'épreuve est lu tel quel, jamais modifié. Les phrases sont fabriquées ici, pas
//! dans `tests/epreuve-modele-local/` (gelé). Aucune ne nomme un fichier de l'inventaire non ouvert
//! (écoles, hydrographie, modèle de terrain de la zone nord) : `layer.add` y répondrait
//! légitimement. Seuil (décision du propriétaire, 2026-10-17) : au moins 9 questions sur 12 ; un
//! refus ou un plan compte comme un échec.

mod common;

use cassini_ai::plan::{compose_plan_with, model_files_generator};
use cassini_core::plan::{Plan, QuestionSource};
use cassini_core::registry::Registry;
use serde_json::Value;

const INVENTORY: &str = include_str!("../../tests/epreuve-modele-local/inventory.json");

/// Les phrases ambiguës : la commande existe, mais un mot ne désigne aucune couche ni aucun champ.
const AMBIGUOUS: [(&str, &str); 12] = [
    // Couche absente.
    (
        "Q01",
        "Fais une zone tampon de 200 mètres autour des pharmacies.",
    ),
    (
        "Q02",
        "Trouve les parcelles à moins de 300 m d'un arrêt de bus.",
    ),
    ("Q03", "Garde seulement les hôpitaux."),
    ("Q04", "Croise les parcelles avec les zones inondables."),
    (
        "Q05",
        "Trouve les bâtiments à moins de 50 m d'une ligne à haute tension.",
    ),
    ("Q06", "Décris la couche des parkings."),
    // Champ ou valeur absents.
    ("Q07", "Garde seulement les parcelles chères."),
    ("Q08", "Garde les bâtiments construits avant 1950."),
    ("Q09", "Garde les routes larges."),
    ("Q10", "Garde les communes les plus riches."),
    ("Q11", "Garde les parcelles qui appartiennent à la commune."),
    (
        "Q12",
        "Fais une zone tampon de 100 m autour des gares fermées.",
    ),
];

#[test]
#[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
fn question_reel_at_least_9_of_12_ambiguous_requests_get_a_question() {
    let Some((files, style)) = common::test_model() else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let registry = Registry::standard();
    let inventory: Value = serde_json::from_str(INVENTORY).unwrap();
    let mut generator = model_files_generator(files);
    let (mut questions, mut guarded) = (0, 0);
    for (id, text) in AMBIGUOUS {
        let got = compose_plan_with(style, &registry, &inventory, text, &mut generator);
        let ok = matches!(&got, Ok(Plan::Question { .. }));
        // Score exact, phrase par phrase : jamais filtré.
        eprintln!(
            "{} {id} {text}
     obtenu  : {got:?}",
            if ok { "OK  " } else { "FAIL" }
        );
        // Aucun plan invalide n'est jamais rendu : tout `Ok` passe `check_plan`.
        if let Ok(plan) = &got {
            assert_eq!(registry.check_plan(plan), Ok(()), "{id}");
        }
        questions += usize::from(ok);
        guarded += usize::from(matches!(
            &got,
            Ok(Plan::Question {
                source: QuestionSource::Guard,
                ..
            })
        ));
    }
    let total = AMBIGUOUS.len();
    eprintln!("SCORE {questions}/{total} questions posées, dont {guarded} du garde-fou");
    assert!(
        questions >= 9,
        "{questions}/{total} : sous le seuil de 9 questions"
    );
}
