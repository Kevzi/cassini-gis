//! Commande `layer.set_style` : applique un `StyleSpec` à une couche du projet (changement de cap
//! du 2026-11-08). Déclaration seule, entrée 11.20 : le geste vit dans l'extension (entrée 21).
//!
//! Elle est **dictée par le noyau**, après `map.topographic_style` ou `map.graduated_style`, jamais
//! offerte au modèle, au client MCP ni au formulaire, et refusée par `command.run`. Le style est
//! celui de la couche dans le projet : le fichier de données n'est pas touché. Genre de sortie
//! `none` (amendement d'AD-24) : son `layer_id` désigne une couche existante, pas une couche posée.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};
use crate::style::spec::StyleSpec;

/// Identifiant de la commande.
pub const SET_STYLE: &str = "layer.set_style";

/// Paramètres de `layer.set_style`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerSetStyleParams {
    /// Couche à mettre en forme.
    pub layer_id: String,
    /// Le style à poser.
    pub style: StyleSpec,
}

/// Résultat de `layer.set_style`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LayerSetStyleOutput {
    /// La couche mise en forme.
    pub layer_id: String,
}

/// `layer.set_style` : commande déléguée à QGIS, dictée par le noyau.
pub struct LayerSetStyle;

impl Delegated for LayerSetStyle {
    type Params = LayerSetStyleParams;
    type Output = LayerSetStyleOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<LayerSetStyleParams, LayerSetStyleOutput>(
            SET_STYLE,
            "Applique une description de style à une couche du projet, sans toucher son fichier ; dictée par le noyau après une mise en forme.",
            "Applies a style description to a project layer, without touching its file; dictated by the core after a styling command.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(false)
        .dictated_by_core()
        .output_none()
        .example(serde_json::json!({"layer_id": "routes_1",
            "style": {"kind": "single", "stroke": "#f3c66d", "width_mm": 0.5}}))
        .tests(&[
            "layer_set_style_is_delegated_dictated_and_creates_no_layer",
            "layer_set_style_takes_a_layer_and_a_style_spec",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<LayerSetStyle>()) }

#[cfg(test)]
mod tests {
    use super::SET_STYLE;
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn layer_set_style_is_delegated_dictated_and_creates_no_layer() {
        let registry = Registry::standard();
        let spec = registry.spec(SET_STYLE).unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert!(spec.dictated_by_core);
        assert!(spec.output_none);
        assert!(spec.path_params.is_empty());
        assert!(spec.params_schema["$defs"]["StyleSpec"].is_object());
        let err = registry
            .call(SET_STYLE, json!({"layer_id": "a", "style": {"kind": "hillshade"}}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn layer_set_style_takes_a_layer_and_a_style_spec() {
        let registry = Registry::standard();
        for bad in [
            json!({"layer_id": "a"}),
            json!({"layer_id": "a", "style": {"kind": "rainbow"}}),
            json!({"layer_id": "a", "style": {"kind": "single", "x": 1}}),
            json!({"layer_id": "a", "style": {"kind": "hillshade"}, "x": 1}),
        ] {
            let err = registry.validate_params(SET_STYLE, &bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
        registry
            .validate_params(
                SET_STYLE,
                &json!({"layer_id": "a", "style": {"kind": "elevation_ramp",
                    "stops": [{"value": 0, "color": "#000000"}, {"value": 1, "color": "#ffffff"}]}}),
            )
            .unwrap();
    }
}
