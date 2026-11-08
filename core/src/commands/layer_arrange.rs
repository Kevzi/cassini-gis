//! Commande `layer.arrange` : range les couches du projet dans des groupes et dans un ordre
//! d'affichage (changement de cap du 2026-11-08, décision 5). Déclaration seule, entrée 11.20 :
//! le geste vit dans l'extension (entrée 21).
//!
//! Elle est **dictée par le noyau** après les `layer.set_style` de `map.topographic_style`, jamais
//! offerte au modèle, au client MCP ni au formulaire. Les groupes sont donnés de haut en bas, et
//! chaque couche du projet y figure une fois (`crate::style::arrange`). Genre de sortie `none`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};
use crate::style::arrange::LayerGroup;

/// Identifiant de la commande.
pub const ARRANGE: &str = "layer.arrange";

/// Paramètres de `layer.arrange`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerArrangeParams {
    /// Groupes à poser, de haut en bas ; leurs couches de haut en bas.
    pub groups: Vec<LayerGroup>,
}

/// Résultat de `layer.arrange`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LayerArrangeOutput {
    /// Nombre de groupes posés.
    pub group_count: u64,
    /// Nombre de couches rangées.
    pub layer_count: u64,
}

/// `layer.arrange` : commande déléguée à QGIS, dictée par le noyau.
pub struct LayerArrange;

impl Delegated for LayerArrange {
    type Params = LayerArrangeParams;
    type Output = LayerArrangeOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<LayerArrangeParams, LayerArrangeOutput>(
            ARRANGE,
            "Range les couches du projet en groupes et dans l'ordre d'affichage donnés, de haut en bas ; dictée par le noyau après une mise en forme topographique.",
            "Arranges the project layers into the given groups and display order, top to bottom; dictated by the core after a topographic styling.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(false)
        .dictated_by_core()
        .output_none()
        .example(serde_json::json!({"groups": [{"name": "Transport", "layers": ["routes_1"]},
            {"name": "Relief", "layers": ["mnt_2"]}]}))
        .tests(&[
            "layer_arrange_is_delegated_and_dictated",
            "layer_arrange_takes_named_groups_of_layer_ids",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<LayerArrange>()) }

#[cfg(test)]
mod tests {
    use super::ARRANGE;
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn layer_arrange_is_delegated_and_dictated() {
        let registry = Registry::standard();
        let spec = registry.spec(ARRANGE).unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert!(spec.dictated_by_core);
        assert!(spec.output_none);
        assert_eq!(
            spec.output_schema["required"],
            json!(["group_count", "layer_count"])
        );
        let err = registry
            .call(ARRANGE, json!({"groups": []}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn layer_arrange_takes_named_groups_of_layer_ids() {
        let registry = Registry::standard();
        for bad in [
            json!({}),
            json!({"groups": [{"name": "A"}]}),
            json!({"groups": [{"name": "A", "layers": "x"}]}),
            json!({"groups": [{"name": "A", "layers": [], "x": 1}]}),
        ] {
            let err = registry.validate_params(ARRANGE, &bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
        registry
            .validate_params(ARRANGE, &json!({"groups": [{"name": "A", "layers": ["a", "b"]}]}))
            .unwrap();
    }
}
