//! Déclare les modules de `src/commands/` : un fichier `nom.rs` ou un dossier
//! `nom/mod.rs` devient `pub mod nom;`, sans éditer de fichier partagé.
//! La logique est dans `build_support.rs`, où elle est testée.

use std::env;
use std::fs;
use std::path::PathBuf;

mod build_support;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    println!("cargo:rerun-if-changed=src/commands");

    let modules = build_support::command_modules(&manifest.join("src").join("commands"))
        .unwrap_or_else(|message| panic!("{message}"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(
        out_dir.join("commands_mods.rs"),
        build_support::render_modules(&modules),
    )
    .expect("écriture de commands_mods.rs");
}
