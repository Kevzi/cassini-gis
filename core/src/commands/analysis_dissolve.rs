//! Commande `analysis.dissolve` : fusion des entités d'une couche par un à quatre champs. Les
//! entités qui portent les mêmes valeurs dans ces champs deviennent une seule entité ; aucun autre
//! champ n'est agrégé. Déclaration seule : le geste vit dans l'extension (AD-22 : aucun nom
//! d'algorithme ni de champ de QGIS ici). Le noyau nomme le fichier de sortie (AD-10) : une couche,
//! donc `.gpkg`.

use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

/// Nombre maximal de champs de regroupement.
const MAX_FIELDS: usize = 4;

/// Paramètres de `analysis.dissolve`, tels que les écrit l'appelant.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawParams {
    layer_id: String,
    fields: Vec<String>,
    #[serde(default)]
    out: String,
}

/// Paramètres de `analysis.dissolve`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(try_from = "RawParams")]
pub struct AnalysisDissolveParams {
    /// Id de la couche à fusionner, rendu par `layer.add`, `project.inventory` ou une analyse
    /// (valable pour la session seulement). Toute géométrie est acceptée.
    pub layer_id: String,
    /// Champs de regroupement, de un à quatre, sans doublon : des champs de l'inventaire de
    /// `layer_id`. Une entité par combinaison de valeurs ; les autres champs ne sont pas agrégés.
    #[schemars(length(min = 1, max = 4))]
    pub fields: Vec<String>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

impl TryFrom<RawParams> for AnalysisDissolveParams {
    type Error = String;

    /// Refuse une liste vide, de plus de quatre noms, un nom vide ou en double, avant tout ordre
    /// à QGIS.
    fn try_from(raw: RawParams) -> Result<Self, String> {
        if raw.fields.is_empty() {
            return Err("`fields` doit nommer au moins un champ".into());
        }
        if raw.fields.len() > MAX_FIELDS {
            return Err(format!("`fields` nomme au plus {MAX_FIELDS} champs"));
        }
        let mut seen = HashSet::new();
        for name in &raw.fields {
            if name.is_empty() {
                return Err("`fields` ne peut pas contenir un nom vide".into());
            }
            if !seen.insert(name.as_str()) {
                return Err(format!("`fields` nomme deux fois le champ « {name} »"));
            }
        }
        Ok(Self {
            layer_id: raw.layer_id,
            fields: raw.fields,
            out: raw.out,
        })
    }
}

/// Résultat de `analysis.dissolve`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AnalysisDissolveOutput {
    /// Id de la couche créée dans le projet.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
    /// Nombre d'entités de la couche créée : un par combinaison de valeurs des champs.
    pub feature_count: u64,
}

/// `analysis.dissolve` : commande déléguée à QGIS.
pub struct AnalysisDissolve;

impl Delegated for AnalysisDissolve {
    type Params = AnalysisDissolveParams;
    type Output = AnalysisDissolveOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AnalysisDissolveParams, AnalysisDissolveOutput>(
            "analysis.dissolve",
            "Fusion des entités d'une couche par un à quatre champs : les entités qui ont les mêmes valeurs dans ces champs (ex. les parcelles d'une même commune, les zones d'un même type) deviennent une seule entité. Aucun autre champ n'est agrégé. Le résultat est une nouvelle couche ajoutée au projet. Ne modifie pas l'entrée.",
            "Dissolve by field: the features of a layer that share the same values in one to four fields (e.g. the parcels of one commune, the zones of one type) become a single feature. No other field is aggregated. The result is a new layer added to the project. Does not modify the input.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .output_params(&["out"])
        .output_ext(".gpkg")
        .example(serde_json::json!({"layer_id": "parcelles_1a2b3c", "fields": ["code_insee"]}))
        .example(serde_json::json!({"layer_id": "zonage_4d5e6f", "fields": ["commune", "typezone"]}))
        .tests(&[
            "analysis_dissolve_is_delegated",
            "analysis_dissolve_needs_one_to_four_distinct_named_fields",
            "analysis_dissolve_output_is_a_gpkg_named_by_the_core",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<AnalysisDissolve>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    fn good() -> serde_json::Value {
        json!({"layer_id": "l1", "fields": ["code_insee"]})
    }

    #[test]
    fn analysis_dissolve_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.dissolve").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry.call("analysis.dissolve", good()).unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
    }

    #[test]
    fn analysis_dissolve_needs_one_to_four_distinct_named_fields() {
        let registry = Registry::standard();
        assert!(registry.validate_params("analysis.dissolve", &good()).is_ok());
        assert!(registry
            .validate_params(
                "analysis.dissolve",
                &json!({"layer_id": "l1", "fields": ["a", "b", "c", "d"]})
            )
            .is_ok());
        for bad in [
            json!({"layer_id": "l1"}),
            json!({"fields": ["a"]}),
            json!({"layer_id": "l1", "fields": []}),
            json!({"layer_id": "l1", "fields": ["a", "b", "c", "d", "e"]}),
            json!({"layer_id": "l1", "fields": ["a", "a"]}),
            json!({"layer_id": "l1", "fields": [""]}),
            json!({"layer_id": "l1", "fields": "a"}),
            json!({"layer_id": "l1", "fields": ["a"], "aggregate": "sum"}),
        ] {
            let err = registry
                .validate_params("analysis.dissolve", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn analysis_dissolve_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.dissolve").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
        assert!(registry.check_contract().is_empty());
    }
}
