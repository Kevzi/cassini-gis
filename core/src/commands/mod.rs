//! Commandes du noyau : un fichier par commande, chacun s'inscrit lui-même.
//!
//! Les modules de ce dossier sont déclarés par `build.rs` : ajouter un fichier
//! ici ne demande de modifier ni `lib.rs` ni le registre.

include!(concat!(env!("OUT_DIR"), "/commands_mods.rs"));

// La logique de `build.rs` est incluse ici, et non dans `lib.rs`, pour être testée
// par `cargo test` sans toucher un fichier partagé.
#[cfg(test)]
#[path = "../../build_support.rs"]
mod build_support;
