//! Essai avec le vrai modèle : ignoré tant que `CASSINI_TEST_MODEL` n'est pas posé.

use std::path::PathBuf;

#[test]
fn the_real_model_answers_a_sentence() {
    let Some(model) = std::env::var_os("CASSINI_TEST_MODEL") else {
        eprintln!("CASSINI_TEST_MODEL absent : essai ignore.");
        return;
    };
    let done = cassini_ai::complete(&PathBuf::from(model), "Dis bonjour.").unwrap();
    assert!(!done.text.trim().is_empty());
    assert!(!done.truncated);
}
