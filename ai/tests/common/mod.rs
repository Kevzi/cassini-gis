//! Partagé par les essais réels : le modèle désigné par `CASSINI_TEST_MODEL`, et l'inventaire
//! tel que le noyau le montre au modèle.

use std::path::PathBuf;

use cassini_ai::plan::{resolve_model, AgentError, PromptStyle};
use cassini_ai::ModelFiles;
use cassini_core::agent_inventory::ShortIds;
use cassini_core::plan::Plan;
use cassini_core::plan_completion::complete_conversions;
use cassini_core::registry::Registry;
use serde_json::Value;

/// Le modèle à essayer, lu de `CASSINI_TEST_MODEL` : un GGUF seul (long prompt) ou une
/// déclaration `*.modele.json` (base, adaptateur et prompt qu'elle nomme), résolus par
/// `resolve_model`, comme dans la CLI. `None` sans la variable. Une déclaration absente ou
/// illisible fait échouer l'essai en la nommant.
#[allow(dead_code)]
pub fn test_model() -> Option<(ModelFiles, PromptStyle)> {
    let path = PathBuf::from(std::env::var_os("CASSINI_TEST_MODEL")?);
    let resolved = resolve_model(&path)
        .unwrap_or_else(|e| panic!("modèle illisible : {} ({e})", path.display()));
    eprintln!(
        "Modèle : {} ; adaptateur : {} ; prompt : {:?}",
        resolved.0.model.display(),
        resolved
            .0
            .adapter
            .as_ref()
            .map_or_else(|| "aucun".to_string(), |a| a.display().to_string()),
        resolved.1
    );
    Some(resolved)
}

/// L'inventaire tel que le noyau le montre au modèle depuis le ticket 5.19 : chaque identifiant
/// QGIS `nom_<uuid>` raccourci en `nom_<4 hex>`, avec la table qui retraduit le plan. Avec
/// `CASSINI_TEST_FULL_IDS` non vide, les identifiants complets d'avant 5.19 (table vide).
#[allow(dead_code)]
pub fn shown_inventory(inventory: &serde_json::Value) -> (serde_json::Value, ShortIds) {
    if std::env::var_os("CASSINI_TEST_FULL_IDS").is_some_and(|v| !v.is_empty()) {
        eprintln!("Identifiants : complets");
        return (inventory.clone(), ShortIds::default());
    }
    let ids = ShortIds::of_inventory(inventory);
    eprintln!("Identifiants : courts");
    (ids.shorten(inventory.clone()), ids)
}

/// L'inventaire d'un essai dans la forme apprise : un fichier écrit dans cette forme est rendu
/// tel quel ; la sortie brute de `project.inventory` (reconnue à sa clé `project_crs`, prise dans
/// QGIS) est traduite comme le noyau la traduit.
#[allow(dead_code)]
pub fn learned(inventory: &Value) -> Value {
    if inventory.get("project_crs").is_some() {
        cassini_core::agent_inventory::agent_inventory(inventory)
    } else {
        inventory.clone()
    }
}

/// La vue que le noyau montre au modèle pour `request` (ticket 5.48), depuis `inventory` (forme
/// apprise ou sortie brute de QGIS, aux identifiants complets) : sous le seuil du tri, l'inventaire
/// aux identifiants courts de `shown_inventory`, à l'octet près ; au-dessus, la vue triée par la
/// demande. Rend aussi le nombre de couches de l'utilisateur en tête de la liste coupée (ancrage
/// du cache) et la table des identifiants. Avec `CASSINI_TEST_FULL_IDS` non vide : l'inventaire tel
/// quel, comme `shown_inventory`.
#[allow(dead_code)]
pub fn shown_view(inventory: &Value, request: &str) -> (Value, Option<usize>, ShortIds) {
    use cassini_core::agent_inventory::{sorted_view_learned, LastPlan, ViewLimits};
    if std::env::var_os("CASSINI_TEST_FULL_IDS").is_some_and(|v| !v.is_empty()) {
        return (learned(inventory), None, ShortIds::default());
    }
    let (view, ids) = sorted_view_learned(
        learned(inventory),
        &[],
        &std::collections::HashSet::new(),
        &LastPlan::default(),
        request,
        &[],
        &ViewLimits::default(),
    );
    (view.inventory, Some(view.user_layers), ids)
}

/// La vue de `shown_view`, seule.
#[allow(dead_code)]
pub fn view(inventory: &Value, request: &str) -> Value {
    shown_view(inventory, request).0
}

/// Le plan composé, ses identifiants courts retraduits par `ids`, comme le fait le noyau avant de
/// le vérifier.
#[allow(dead_code)]
pub fn restored(composed: Result<Plan, AgentError>, ids: &ShortIds) -> Result<Plan, AgentError> {
    composed.map(|mut plan| {
        if let Plan::Calls { calls } = &mut plan {
            for call in calls {
                ids.restore(&mut call.params);
            }
        }
        plan
    })
}

/// Le plan composé, complété par les conversions de points comme le fait `plan.propose` avant de
/// le montrer (amendement d'AD-19, 11.22) : depuis 11.23, les essais réels de terrain mesurent le
/// plan complété, le modèle recopiant un point tel que la phrase le donne. Le système d'un raster
/// est celui que `inventory` donne à sa `source` (dans le produit, l'en-tête du GeoTIFF) ; une
/// question ou un refus passent inchangés.
#[allow(dead_code)]
pub fn completed(
    registry: &Registry,
    composed: Result<Plan, AgentError>,
    inventory: &Value,
) -> Result<Plan, AgentError> {
    composed.map(|plan| complete_with(registry, plan, inventory))
}

/// La complétion d'un plan sur `inventory`, comme `completed`.
#[allow(dead_code)]
pub fn complete_with(registry: &Registry, plan: Plan, inventory: &Value) -> Plan {
    let rasters: Vec<(String, u32)> = inventory["layers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| l["kind"] == "raster")
        .filter_map(|l| {
            let source = l["source"].as_str()?.to_string();
            let epsg = u32::try_from(l["epsg"].as_u64()?).ok()?;
            Some((source, epsg))
        })
        .collect();
    let raster_epsg = |raw: &str| {
        rasters
            .iter()
            .find(|(source, _)| source == raw)
            .map(|(_, epsg)| *epsg)
    };
    complete_conversions(registry, plan, &raster_epsg)
}
