//! Partagé par les essais réels : le modèle désigné par `CASSINI_TEST_MODEL`, et l'inventaire
//! tel que le noyau le montre au modèle.

use std::path::PathBuf;

use cassini_ai::plan::{resolve_model, AgentError, PromptStyle};
use cassini_ai::ModelFiles;
use cassini_core::agent_inventory::ShortIds;
use cassini_core::plan::Plan;

/// Le modèle à essayer, lu de `CASSINI_TEST_MODEL` : un GGUF seul (long prompt) ou une
/// déclaration `*.modele.json` (base, adaptateur et prompt qu'elle nomme), résolus par
/// `resolve_model`, comme dans la CLI. `None` sans la variable. Une déclaration absente ou
/// illisible fait échouer l'essai en la nommant.
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
