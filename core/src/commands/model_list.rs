//! Commande `model.list` : les modèles GGUF disponibles, trouvés dans le dossier de modèles.
//!
//! Ordre de recherche fixe : la variable d'environnement `CASSINI_MODELS`, puis `models/`
//! à côté de l'exécutable du noyau. Un dossier désigné qui n'existe pas est une erreur,
//! jamais un repli sur le suivant. Aucun fichier n'est ouvert : la liste vient des noms et des tailles.

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

/// Nom de la variable d'environnement qui désigne le dossier de modèles.
pub const MODELS_ENV: &str = "CASSINI_MODELS";

/// Paramètres de `model.list` : aucun, et aucun champ inconnu accepté.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelListParams {}

/// Un modèle trouvé.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ModelEntry {
    /// Nom du fichier seul, sans dossier.
    pub file: String,
    pub size_bytes: u64,
}

/// Résultat de `model.list`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ModelListOutput {
    /// Dossier de modèles retenu.
    pub folder: String,
    /// Modèles triés par nom ; vide si le dossier n'en contient aucun.
    pub models: Vec<ModelEntry>,
}

/// Commande `model.list`.
pub struct ModelList;

fn no_folder(detail: &str) -> CommandFailure {
    CommandFailure::new(
        "models.no_folder",
        format!(
            "{detail} Cassini cherche les modèles dans le dossier désigné par la variable \
             d'environnement {MODELS_ENV}, sinon dans le dossier « models » placé à côté de \
             l'exécutable du noyau. Créez ce dossier ou désignez-en un avec {MODELS_ENV}."
        ),
    )
}

/// Choisit le dossier de modèles puis le parcourt (un seul niveau).
/// Les entrées sont des arguments pour tester sans toucher l'environnement.
fn discover(
    setting: Option<&str>,
    exe_dir: Option<&Path>,
) -> Result<ModelListOutput, CommandFailure> {
    let folder: PathBuf = match setting.filter(|s| !s.is_empty()) {
        Some(designated) => {
            let p = PathBuf::from(designated);
            if !p.is_dir() {
                return Err(no_folder(&format!(
                    "Le dossier désigné par {MODELS_ENV} n'existe pas : « {designated} »."
                )));
            }
            p
        }
        None => {
            let candidate = exe_dir.map(|d| d.join("models"));
            match candidate {
                Some(p) if p.is_dir() => p,
                _ => return Err(no_folder("Aucun dossier de modèles n'a été trouvé.")),
            }
        }
    };
    let unreadable = |e: std::io::Error| {
        CommandFailure::new(
            "models.unreadable",
            format!(
                "Le dossier de modèles « {} » n'a pas pu être lu : {e}.",
                folder.display()
            ),
        )
    };
    let mut models = Vec::new();
    for entry in std::fs::read_dir(&folder).map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        let path = entry.path();
        let is_gguf = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("gguf"));
        if !is_gguf {
            continue;
        }
        // Lien cassé ou fichier disparu entre-temps : l'entrée est ignorée, pas la liste.
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        models.push(ModelEntry {
            file: entry.file_name().to_string_lossy().into_owned(),
            size_bytes: meta.len(),
        });
    }
    models.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(ModelListOutput {
        folder: folder.display().to_string(),
        models,
    })
}

impl Command for ModelList {
    type Params = ModelListParams;
    type Output = ModelListOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<ModelListParams, ModelListOutput>(
            "model.list",
            "Liste les modèles d'IA locale (fichiers GGUF) disponibles sur ce poste.",
            "Lists the local AI models (GGUF files) available on this machine.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .example(serde_json::json!({}))
        .tests(&[
            "model_list_through_the_registry_reports_a_missing_designated_folder",
            "discover_lists_gguf_files_sorted_with_sizes",
            "discover_empty_folder_is_a_success",
            "discover_without_any_folder_is_no_folder",
            "discover_designated_missing_folder_does_not_fall_back",
            "discover_ignores_a_directory_named_like_a_model",
            "model_list_rejects_unknown_params",
        ])
    }

    fn run(&self, _params: ModelListParams) -> Result<ModelListOutput, CommandFailure> {
        let setting = std::env::var(MODELS_ENV).ok();
        let exe = std::env::current_exe().ok();
        discover(setting.as_deref(), exe.as_deref().and_then(Path::parent))
    }
}

inventory::submit! { Registration(|r| r.register(ModelList)) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Registry;
    use serde_json::json;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cassini-model-list-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn discover_lists_gguf_files_sorted_with_sizes() {
        let dir = scratch("two");
        std::fs::write(dir.join("a.gguf"), b"12345").unwrap();
        std::fs::write(dir.join("B.GGUF"), b"123").unwrap();
        std::fs::write(dir.join("notes.txt"), b"x").unwrap();
        let out = discover(dir.to_str(), None).unwrap();
        let got: Vec<_> = out
            .models
            .iter()
            .map(|m| (m.file.as_str(), m.size_bytes))
            .collect();
        assert_eq!(got, vec![("B.GGUF", 3), ("a.gguf", 5)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_empty_folder_is_a_success() {
        let dir = scratch("empty");
        let out = discover(dir.to_str(), None).unwrap();
        assert!(out.models.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_without_any_folder_is_no_folder() {
        let dir = scratch("nomodels");
        assert_eq!(
            discover(None, Some(&dir)).unwrap_err().code,
            "models.no_folder"
        );
        assert_eq!(discover(None, None).unwrap_err().code, "models.no_folder");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_uses_models_next_to_the_executable() {
        let dir = scratch("exe");
        std::fs::create_dir_all(dir.join("models")).unwrap();
        std::fs::write(dir.join("models").join("m.gguf"), b"1").unwrap();
        let out = discover(None, Some(&dir)).unwrap();
        assert_eq!(out.models.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_designated_missing_folder_does_not_fall_back() {
        let dir = scratch("fallback");
        std::fs::create_dir_all(dir.join("models")).unwrap();
        let missing = dir.join("absent");
        let err = discover(missing.to_str(), Some(&dir)).unwrap_err();
        assert_eq!(err.code, "models.no_folder");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_ignores_a_directory_named_like_a_model() {
        let dir = scratch("subdir");
        std::fs::create_dir_all(dir.join("x.gguf")).unwrap();
        let out = discover(dir.to_str(), None).unwrap();
        assert!(out.models.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn model_list_rejects_unknown_params() {
        let err = Registry::standard()
            .call("model.list", json!({"x": 1}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
    }

    // Seul test qui pose une variable de processus.
    #[test]
    fn model_list_through_the_registry_reports_a_missing_designated_folder() {
        let dir = scratch("env");
        std::fs::write(dir.join("m.gguf"), b"1234").unwrap();
        std::env::set_var(MODELS_ENV, &dir);
        let ok = Registry::standard().call("model.list", json!({})).unwrap();
        assert_eq!(ok["models"][0]["file"], "m.gguf");
        assert_eq!(ok["models"][0]["size_bytes"], 4);

        std::env::set_var(MODELS_ENV, dir.join("absent"));
        let err = Registry::standard()
            .call("model.list", json!({}))
            .unwrap_err();
        assert_eq!(err.code(), "models.no_folder");
        std::env::remove_var(MODELS_ENV);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
