//! Partagé par les essais réels : le modèle désigné par `CASSINI_TEST_MODEL`.

use std::path::PathBuf;

use cassini_ai::plan::{resolve_model, PromptStyle};
use cassini_ai::ModelFiles;

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
