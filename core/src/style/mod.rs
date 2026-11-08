//! Mise en forme topographique des couches (changement de cap du 2026-11-08, entrée 11.20).
//!
//! - [`spec`] : `StyleSpec`, la description de style neutre envoyée à QGIS (AD-22) ;
//! - [`table`] : la table de sémiologie, chaque couleur sourcée (IGN, CNIG) ;
//! - [`recognize`] : le thème d'une couche, d'après son nom, son fichier et ses champs ;
//! - [`arrange`] : les groupes et l'ordre d'affichage.
//!
//! La commande `map.topographic_style` (`crate::commands::map_topographic_style`) assemble le tout.
//! Rien ici ne dépend de QGIS : tout se teste sur des inventaires synthétiques.

pub mod arrange;
pub mod recognize;
pub mod spec;
pub mod table;

use crate::registry::RegistryError;

/// Code du refus d'une mise en forme sans QGIS, ou sans les gestes qu'elle demande.
pub const STYLE_UNAVAILABLE: &str = "style.unavailable";

/// La mise en forme n'est pas possible : QGIS n'est pas branché (CLI `run`, MCP autonome), ou
/// l'extension n'annonce pas `project.inventory`, `layer.set_style` et `layer.arrange` (AD-25).
pub fn unavailable() -> RegistryError {
    RegistryError::Failed {
        code: STYLE_UNAVAILABLE.to_string(),
        message: "La mise en forme topographique demande QGIS et une extension Cassini qui sait \
                  lire l'inventaire, poser un style et ranger les couches : rien n'est fait."
            .to_string(),
    }
}
