//! Recette du catalogue des commandes (AD-6, amendement du 2026-11-05) : chaque commande du
//! registre standard figure au catalogue figé avec les mêmes identifiant, paramètres et
//! résultat. La recette est une fonction de la bibliothèque (`cassini_core::recipe`, entrée
//! 2.14) ; ce test l'appelle sur le catalogue public et le registre officiel.

use std::path::PathBuf;

use cassini_core::recipe::check_catalogue;
use cassini_core::registry::Registry;

fn catalogue_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("_bmad-output")
        .join("initiative-cassini-mvp")
        .join("spec-cassini-mvp")
        .join("catalogue-des-commandes.md")
}

#[test]
fn the_registry_matches_the_catalogue() {
    let path = catalogue_path();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("catalogue illisible ({}) : {e}", path.display()));
    if let Err(errors) = check_catalogue(&Registry::standard(), &text) {
        panic!(
            "le registre s'écarte du catalogue figé :
- {}",
            errors.join(
                "
- "
            )
        );
    }
}
