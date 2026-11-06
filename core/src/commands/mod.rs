//! Commandes du noyau : un fichier par commande, chacun s'inscrit lui-même.
//!
//! Les modules de ce dossier sont déclarés par `build.rs` : ajouter un fichier
//! ici ne demande de modifier ni `lib.rs` ni le registre.

include!(concat!(env!("OUT_DIR"), "/commands_mods.rs"));

use crate::dem::DemError;
use crate::geojson::GeojsonError;
use crate::registry::CommandFailure;

/// Une erreur du module `dem` devient l'échec typé de la commande qui l'a rencontrée, code et
/// message tels quels. La conversion vit ici et non dans `dem.rs`, qui ne dépend pas du
/// registre et reste remplaçable seul (AD-20).
impl From<DemError> for CommandFailure {
    fn from(e: DemError) -> Self {
        CommandFailure::new(e.code, e.message)
    }
}

/// Une erreur de l'écrivain GeoJSON devient l'échec typé de la commande, code
/// (`geojson.unwritable`) et message tels quels, comme pour `dem` (AD-30).
impl From<GeojsonError> for CommandFailure {
    fn from(e: GeojsonError) -> Self {
        CommandFailure::new(e.code, e.message)
    }
}

// La logique de `build.rs` est incluse ici, et non dans `lib.rs`, pour être testée
// par `cargo test` sans toucher un fichier partagé.
#[cfg(test)]
#[path = "../../build_support.rs"]
mod build_support;
