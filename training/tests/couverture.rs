//! Garde-fou de couverture : chaque commande du registre doit être produite par au moins une
//! famille d'entraînement. Une commande sans famille est une commande que le modèle ne sait
//! pas écrire (essai réel de `base-22-1` : sept commandes dans ce cas).
//!
//! La liste des commandes vient du registre lui-même (`Registry::standard()`), jamais d'une
//! liste recopiée ici : une commande ajoutée au registre sans famille fait échouer ce test, en
//! la nommant. Les familles sont tirées une à une, à graine fixe, assez longtemps pour que les
//! variantes rares d'une famille (une étape en plus, un refus) apparaissent.

use std::collections::BTreeSet;

use cassini_core::registry::Registry;
use cassini_training::families::FAMILIES;
use cassini_training::{generate_families_unchecked, FOCUS_SEED};

/// Exemples tirés par famille : de quoi voir toutes les variantes d'une famille étroite.
const DRAWN_PER_FAMILY: usize = 300;

/// Commandes du registre que le modèle n'a pas à écrire :
/// - les commandes dictées par le noyau (`dictated_by_core` : `layer.open_vector`,
///   `layer.open_raster`, `layer.set_style`, `layer.arrange`, `raster.reproject_work`), que la
///   session appelle elle-même et qui ne sont offertes ni au modèle ni au client MCP ;
/// - les commandes de test et d'exemple des essais du noyau, qui ne sont pas au catalogue
///   public (`test.*`, `a.b`, `terrain.name`, `terrain.two`).
///
/// Toute autre commande doit avoir une famille ; on n'ajoute rien ici pour faire taire le test.
fn is_excluded(id: &str, dictated_by_core: bool) -> bool {
    dictated_by_core
        || id.starts_with("test.")
        || matches!(id, "a.b" | "terrain.name" | "terrain.two")
}

#[test]
fn every_public_command_of_the_registry_is_produced_by_some_family() {
    let mut produced = BTreeSet::new();
    for family in FAMILIES {
        let (examples, _) =
            generate_families_unchecked(FOCUS_SEED, DRAWN_PER_FAMILY, &[family.name]).unwrap();
        for example in examples {
            for call in example.expected["calls"].as_array().into_iter().flatten() {
                if let Some(id) = call["command"].as_str() {
                    produced.insert(id.to_string());
                }
            }
        }
    }
    let registry = Registry::standard();
    let without_family: Vec<&str> = registry
        .specs()
        .filter(|spec| !is_excluded(&spec.id, spec.dictated_by_core))
        .map(|spec| spec.id.as_str())
        .filter(|id| !produced.contains(*id))
        .collect();
    assert!(
        without_family.is_empty(),
        "commandes du registre sans aucune famille d'entraînement : {}",
        without_family.join(", ")
    );
}
