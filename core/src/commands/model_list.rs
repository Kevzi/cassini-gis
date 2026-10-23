//! Commande `model.list` : les modèles GGUF disponibles, trouvés dans le dossier de modèles.
//!
//! Ordre de recherche fixe : la variable d'environnement `CASSINI_MODELS`, puis `models/`
//! à côté de l'exécutable du noyau. Un dossier désigné qui n'existe pas est une erreur,
//! jamais un repli sur le suivant. Aucun GGUF n'est ouvert : la liste vient des noms et des tailles ;
//! seules les déclarations `*.modele.json` (4 Kio au plus) sont ouvertes et lues.
//!
//! Un modèle peut aussi se **déclarer** par un petit fichier `*.modele.json` posé à côté des
//! GGUF (ticket 5.10, tâche 3) : il nomme le GGUF du modèle, l'adaptateur LoRA facultatif qui
//! se pose sur lui, et le prompt qu'il attend. Une déclaration devient une entrée de la liste,
//! et les GGUF qu'elle nomme n'y figurent plus seuls. Une déclaration illisible, ou qui nomme
//! autre chose qu'un nom de fichier du même dossier, est ignorée par `model.list` ; le
//! diagnostic `diagnose` (sélecteur du panneau, ticket 5.14) la garde avec son problème.

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

/// Fin du nom d'un fichier de déclaration de modèle.
pub const DECLARATION_SUFFIX: &str = ".modele.json";

/// Taille maximale d'une déclaration, en octets : au-delà, ce n'en est pas une.
const MAX_DECLARATION_BYTES: u64 = 4096;

/// Le prompt qu'un modèle attend.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PromptKind {
    /// Le long prompt de l'agent : consignes, catalogue et exemples.
    #[default]
    Full,
    /// Le prompt court appris par le modèle ajusté (ticket 5.10).
    Short,
}

/// Déclaration d'un modèle : noms de fichiers seuls, dans le dossier de la déclaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelDeclaration {
    /// GGUF du modèle (la base, quand un adaptateur est nommé).
    pub model: String,
    /// GGUF de l'adaptateur LoRA posé sur le modèle, s'il y en a un.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    /// Prompt attendu ; absent : le long prompt.
    #[serde(default)]
    pub prompt: PromptKind,
}

impl ModelDeclaration {
    /// Les GGUF nommés : le modèle, puis l'adaptateur.
    pub fn files(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.model.as_str()).chain(self.adapter.as_deref())
    }
}

/// Vrai si `name` se lit comme le nom d'une déclaration de modèle.
pub fn is_declaration(name: &str) -> bool {
    name.len() > DECLARATION_SUFFIX.len()
        && name
            .get(name.len() - DECLARATION_SUFFIX.len()..)
            .is_some_and(|end| end.eq_ignore_ascii_case(DECLARATION_SUFFIX))
}

/// Vrai si `name` est un nom de fichier GGUF seul : ni dossier, ni séparateur, ni `..`.
fn is_plain_gguf_name(name: &str) -> bool {
    let path = Path::new(name);
    !name.is_empty()
        && !name.contains(['/', '\\', ':'])
        && !name.contains("..")
        && path.file_name().and_then(|n| n.to_str()) == Some(name)
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("gguf"))
}

/// Lit la déclaration `path`. L'erreur est un message en français, sans chemin.
pub fn read_declaration(path: &Path) -> Result<ModelDeclaration, String> {
    let meta = std::fs::metadata(path)
        .map_err(|_| "la déclaration de modèle est introuvable".to_string())?;
    if !meta.is_file() || meta.len() > MAX_DECLARATION_BYTES {
        return Err("la déclaration de modèle n'est pas un petit fichier JSON".to_string());
    }
    let text = std::fs::read_to_string(path)
        .map_err(|_| "la déclaration de modèle n'a pas pu être lue".to_string())?;
    // Seules la ligne et la colonne de l'erreur sont gardées : le texte de serde (en anglais)
    // peut citer une valeur du fichier, un chemin par exemple (AD-9).
    let declaration: ModelDeclaration = serde_json::from_str(&text).map_err(|e| {
        format!(
            "la déclaration de modèle n'est pas lisible (ligne {}, colonne {})",
            e.line(),
            e.column()
        )
    })?;
    if let Some(bad) = declaration.files().find(|f| !is_plain_gguf_name(f)) {
        // Un nom qui porte un séparateur peut être un chemin : il n'est pas recopié.
        return Err(if bad.contains(['/', '\\', ':']) {
            "la déclaration de modèle nomme un chemin au lieu d'un nom de fichier GGUF du même dossier"
                .to_string()
        } else {
            format!(
                "la déclaration de modèle nomme « {bad} », qui n'est pas un nom de fichier GGUF du même dossier"
            )
        });
    }
    Ok(declaration)
}

/// Un modèle trouvé.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ModelEntry {
    /// Nom du fichier seul, sans dossier : le GGUF, ou la déclaration.
    pub file: String,
    /// Taille du GGUF ; pour une déclaration, celle des GGUF nommés présents.
    pub size_bytes: u64,
    /// Ce que déclare le fichier `*.modele.json` ; absent pour un GGUF seul.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration: Option<ModelDeclaration>,
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

/// Choisit le dossier de modèles : `setting` (la valeur de `CASSINI_MODELS`), sinon `models/`
/// à côté de `exe_dir`. Un dossier désigné absent est une erreur, jamais un repli.
fn choose_folder(setting: Option<&str>, exe_dir: Option<&Path>) -> Result<PathBuf, CommandFailure> {
    match setting.filter(|s| !s.is_empty()) {
        Some(designated) => {
            let p = PathBuf::from(designated);
            if !p.is_dir() {
                return Err(no_folder(&format!(
                    "Le dossier désigné par {MODELS_ENV} n'existe pas : « {designated} »."
                )));
            }
            Ok(p)
        }
        None => match exe_dir.map(|d| d.join("models")) {
            Some(p) if p.is_dir() => Ok(p),
            _ => Err(no_folder("Aucun dossier de modèles n'a été trouvé.")),
        },
    }
}

/// Une entrée trouvée par le parcours, avant mise en forme.
struct Scanned {
    file: String,
    size_bytes: u64,
    /// `None` pour un GGUF seul ; pour une déclaration, sa lecture.
    declaration: Option<Result<ModelDeclaration, String>>,
    /// GGUF nommés par la déclaration et absents du dossier.
    missing: Vec<String>,
}

/// Parcourt le dossier (un seul niveau) : les GGUF seuls et les déclarations, lisibles ou
/// non, triés par nom. Aucun GGUF n'est encore caché : voir `hide_named`.
fn scan(folder: &Path) -> Result<Vec<Scanned>, CommandFailure> {
    let unreadable = |e: std::io::Error| {
        CommandFailure::new(
            "models.unreadable",
            format!(
                "Le dossier de modèles « {} » n'a pas pu être lu : {e}.",
                folder.display()
            ),
        )
    };
    let mut ggufs = Vec::new();
    let mut declared = Vec::new();
    for entry in std::fs::read_dir(folder).map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_declaration(&name) {
            declared.push((name, read_declaration(&path)));
            continue;
        }
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
        ggufs.push(Scanned {
            file: name,
            size_bytes: meta.len(),
            declaration: None,
            missing: Vec::new(),
        });
    }
    let mut models = ggufs;
    for (name, declaration) in declared {
        let (mut size_bytes, mut missing) = (0, Vec::new());
        if let Ok(d) = &declaration {
            for f in d.files() {
                match std::fs::metadata(folder.join(f)) {
                    Ok(m) if m.is_file() => size_bytes += m.len(),
                    _ => missing.push(f.to_string()),
                }
            }
        }
        models.push(Scanned {
            file: name,
            size_bytes,
            declaration: Some(declaration),
            missing,
        });
    }
    models.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(models)
}

/// Retire les GGUF seuls nommés par une déclaration lisible : tous ceux qu'elle nomme si
/// `hides` la retient, sinon son seul adaptateur (un adaptateur n'est jamais un modèle seul).
fn hide_named(models: Vec<Scanned>, hides: impl Fn(&Scanned) -> bool) -> Vec<Scanned> {
    let named: Vec<String> = models
        .iter()
        .filter_map(|m| {
            let d = m.declaration.as_ref()?.as_ref().ok()?;
            Some(if hides(m) {
                d.files().map(str::to_string).collect::<Vec<_>>()
            } else {
                d.adapter.iter().cloned().collect()
            })
        })
        .flatten()
        .collect();
    models
        .into_iter()
        .filter(|m| {
            m.declaration.is_some() || !named.iter().any(|f| f.eq_ignore_ascii_case(&m.file))
        })
        .collect()
}

/// Choisit le dossier de modèles puis le parcourt (un seul niveau).
/// Les entrées sont des arguments pour tester sans toucher l'environnement.
fn discover(
    setting: Option<&str>,
    exe_dir: Option<&Path>,
) -> Result<ModelListOutput, CommandFailure> {
    let folder = choose_folder(setting, exe_dir)?;
    // Un GGUF nommé par une déclaration lisible n'est plus listé seul.
    let models = hide_named(scan(&folder)?, |_| true)
        .into_iter()
        // Une déclaration illisible est ignorée, pas la liste. Un GGUF nommé absent ne
        // compte pour rien : son chargement dira qu'il manque.
        .filter_map(|m| match m.declaration {
            Some(Err(_)) => None,
            declaration => Some(ModelEntry {
                file: m.file,
                size_bytes: m.size_bytes,
                declaration: declaration.and_then(Result::ok),
            }),
        })
        .collect();
    Ok(ModelListOutput {
        folder: folder.display().to_string(),
        models,
    })
}

/// Un modèle tel que le sélecteur le montre : nom, taille, et ce qui l'empêche de servir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosedModel {
    /// Nom du fichier seul, sans dossier : le GGUF, ou la déclaration.
    pub file: String,
    /// Taille du GGUF ; pour une déclaration, celle des GGUF nommés présents.
    pub size_bytes: u64,
    /// Ce qui empêche ce modèle de servir (déclaration illisible, fichier manquant), en
    /// français et sans chemin ; absent si rien.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

/// Le diagnostic du dossier de modèles : le dossier retenu, et chaque modèle avec son problème.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDiagnosis {
    /// Dossier de modèles retenu.
    pub folder: PathBuf,
    /// Modèles triés par nom ; une déclaration illisible y reste, avec son problème.
    pub models: Vec<DiagnosedModel>,
}

/// Diagnostic des modèles (ticket 5.14) : même choix de dossier et même parcours que
/// `model.list`, mais une déclaration illisible reste listée avec son problème, et un GGUF
/// nommé absent devient un problème. `model.list` ne s'en sert pas. Aucun GGUF n'est ouvert.
pub fn diagnose(
    setting: Option<&str>,
    exe_dir: Option<&Path>,
) -> Result<ModelDiagnosis, CommandFailure> {
    let folder = choose_folder(setting, exe_dir)?;
    // Un GGUF reste listé seul, sauf si une déclaration sans problème le nomme, ou si une
    // déclaration lisible le nomme comme adaptateur : la base d'une déclaration à laquelle il
    // manque un fichier reste choisissable, son adaptateur jamais.
    let models = hide_named(scan(&folder)?, |m| m.missing.is_empty())
        .into_iter()
        .map(|m| {
            let problem = match m.declaration {
                Some(Err(e)) => Some(e),
                _ if !m.missing.is_empty() => {
                    Some(format!("fichier manquant : {}", m.missing.join(", ")))
                }
                _ => None,
            };
            DiagnosedModel {
                file: m.file,
                size_bytes: m.size_bytes,
                problem,
            }
        })
        .collect();
    Ok(ModelDiagnosis { folder, models })
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
            "a_declaration_is_one_entry_and_hides_the_files_it_names",
            "an_unreadable_or_path_naming_declaration_is_ignored",
            "a_declaration_naming_a_missing_file_is_still_listed",
            "read_declaration_accepts_the_owner_example_and_defaults_to_the_full_prompt",
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

    #[test]
    fn a_declaration_is_one_entry_and_hides_the_files_it_names() {
        let dir = scratch("declared");
        std::fs::write(dir.join("base.gguf"), b"12345").unwrap();
        std::fs::write(dir.join("adapt.gguf"), b"12").unwrap();
        std::fs::write(dir.join("merged.gguf"), b"1234").unwrap();
        std::fs::write(dir.join("Qwen-9B.gguf"), b"1").unwrap();
        // Nommé avec une autre casse : caché aussi (Windows ignore la casse).
        std::fs::write(dir.join("autre-base.gguf"), b"1").unwrap();
        std::fs::write(
            dir.join("casse.modele.json"),
            r#"{"model":"Autre-Base.GGUF"}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("ajuste-adaptateur.modele.json"),
            r#"{"model":"base.gguf","adapter":"adapt.gguf","prompt":"short"}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("ajuste.modele.json"),
            r#"{"model":"merged.gguf","prompt":"short"}"#,
        )
        .unwrap();
        let out = discover(dir.to_str(), None).unwrap();
        let got: Vec<_> = out
            .models
            .iter()
            .map(|m| (m.file.as_str(), m.size_bytes))
            .collect();
        // La taille de « casse » dépend du système de fichiers : seul son nom est comparé.
        assert_eq!(
            got[..3],
            [
                ("Qwen-9B.gguf", 1),
                ("ajuste-adaptateur.modele.json", 7),
                ("ajuste.modele.json", 4),
            ]
        );
        assert_eq!(got.len(), 4, "{got:?}");
        assert_eq!(got[3].0, "casse.modele.json");
        assert!(out.models[0].declaration.is_none());
        let d = out.models[1].declaration.as_ref().unwrap();
        assert_eq!(d.adapter.as_deref(), Some("adapt.gguf"));
        assert_eq!(d.prompt, PromptKind::Short);
        // Le GGUF seul ne porte aucune déclaration dans la sortie machine.
        let json = serde_json::to_value(&out).unwrap();
        assert!(json["models"][0].get("declaration").is_none());
        assert_eq!(json["models"][1]["declaration"]["prompt"], "short");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unreadable_or_path_naming_declaration_is_ignored() {
        let dir = scratch("baddecl");
        std::fs::write(dir.join("m.gguf"), b"1").unwrap();
        for (name, text) in [
            ("pas-json.modele.json", "pas du json"),
            ("inconnu.modele.json", r#"{"model":"m.gguf","x":1}"#),
            ("chemin.modele.json", r#"{"model":"../m.gguf"}"#),
            ("sous-dossier.modele.json", r#"{"model":"a/m.gguf"}"#),
            ("windows.modele.json", r#"{"model":"C:\\m.gguf"}"#),
            ("pas-gguf.modele.json", r#"{"model":"m.bin"}"#),
            ("adaptateur.modele.json", r#"{"model":"m.gguf","adapter":"..\\a.gguf"}"#),
            ("prompt.modele.json", r#"{"model":"m.gguf","prompt":"moyen"}"#),
            ("barre-inverse.modele.json", r#"{"model":"a\\m.gguf"}"#),
        ] {
            std::fs::write(dir.join(name), text).unwrap();
        }
        let out = discover(dir.to_str(), None).unwrap();
        let got: Vec<_> = out.models.iter().map(|m| m.file.as_str()).collect();
        assert_eq!(got, vec!["m.gguf"], "seul le GGUF reste, et seul");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_declaration_naming_a_missing_file_is_still_listed() {
        let dir = scratch("missingdecl");
        std::fs::write(dir.join("d.modele.json"), r#"{"model":"absent.gguf"}"#).unwrap();
        let out = discover(dir.to_str(), None).unwrap();
        assert_eq!(out.models.len(), 1);
        assert_eq!(out.models[0].file, "d.modele.json");
        assert_eq!(out.models[0].size_bytes, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_declaration_accepts_the_owner_example_and_defaults_to_the_full_prompt() {
        let dir = scratch("readdecl");
        let f = dir.join("x.modele.json");
        std::fs::write(
            &f,
            r#"{"model":"qwen3.5-4b-base-Q4_K_M.gguf","adapter":"cassini-composition-adaptateur-f16.gguf","prompt":"short"}"#,
        )
        .unwrap();
        let d = read_declaration(&f).unwrap();
        assert_eq!(d.model, "qwen3.5-4b-base-Q4_K_M.gguf");
        assert_eq!(
            d.files().collect::<Vec<_>>(),
            vec![
                "qwen3.5-4b-base-Q4_K_M.gguf",
                "cassini-composition-adaptateur-f16.gguf"
            ]
        );
        std::fs::write(&f, r#"{"model":"m.gguf"}"#).unwrap();
        let d = read_declaration(&f).unwrap();
        assert_eq!((d.adapter, d.prompt), (None, PromptKind::Full));
        // Le message d'erreur ne porte pas le chemin.
        let e = read_declaration(&dir.join("absent.modele.json")).unwrap_err();
        assert!(!e.contains(dir.to_str().unwrap()), "{e}");
        assert!(is_declaration("a.MODELE.JSON") && !is_declaration(".modele.json"));
        assert!(!is_declaration("a.gguf"));
        let _ = std::fs::remove_dir_all(&dir);
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

    #[test]
    fn diagnose_keeps_a_broken_declaration_with_its_problem() {
        let dir = scratch("diagnose");
        std::fs::write(dir.join("a.modele.json"), "pas du json").unwrap();
        std::fs::write(
            dir.join("b.modele.json"),
            r#"{"model":"present.gguf","adapter":"absent.gguf"}"#,
        )
        .unwrap();
        std::fs::write(dir.join("present.gguf"), b"123").unwrap();
        std::fs::write(dir.join("c.gguf"), b"12").unwrap();
        let d = diagnose(dir.to_str(), None).unwrap();
        assert_eq!(d.folder, dir);
        let got: Vec<_> = d
            .models
            .iter()
            .map(|m| (m.file.as_str(), m.size_bytes, m.problem.as_deref()))
            .collect();
        // La base présente d'une déclaration à problème reste choisissable seule.
        assert_eq!(got[1..], [
            ("b.modele.json", 3, Some("fichier manquant : absent.gguf")),
            ("c.gguf", 2, None),
            ("present.gguf", 3, None),
        ]);
        assert_eq!((got.len(), got[0].0), (4, "a.modele.json"));
        let problem = got[0].2.unwrap();
        assert_eq!(
            problem,
            "la déclaration de modèle n'est pas lisible (ligne 1, colonne 1)"
        );
        // Le GGUF seul ne porte pas de champ `problem`.
        let json = serde_json::to_value(&d.models[2]).unwrap();
        assert_eq!(json, json!({"file": "c.gguf", "size_bytes": 2}));
        // `model.list` reste inchangé : la déclaration illisible n'y figure pas, et la
        // déclaration lisible cache les GGUF qu'elle nomme.
        let listed = discover(dir.to_str(), None).unwrap();
        let names: Vec<_> = listed.models.iter().map(|m| m.file.as_str()).collect();
        assert_eq!(names, vec!["b.modele.json", "c.gguf"]);
        std::fs::write(dir.join("ok.modele.json"), r#"{"model":"present.gguf"}"#).unwrap();
        let d = diagnose(dir.to_str(), None).unwrap();
        assert!(d.models.iter().all(|m| m.file != "present.gguf"), "{d:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_declaration_never_echoes_a_path_or_serde_text() {
        let dir = scratch("decl-path");
        let f = dir.join("x.modele.json");
        for text in [
            r#"{"model":"C:\\Users\\kev\\m.gguf"}"#,
            r#"{"model":"/home/kev/m.gguf"}"#,
            r#"{"model":"m.gguf","x":"/home/kev/secret"}"#,
            r#"{"model":"m.gguf","prompt":"/home/kev/secret"}"#,
        ] {
            std::fs::write(&f, text).unwrap();
            let e = read_declaration(&f).unwrap_err();
            assert!(!e.contains("kev") && !e.contains("Users"), "{e}");
        }
        std::fs::write(&f, r#"{"model":"m.bin"}"#).unwrap();
        assert!(read_declaration(&f).unwrap_err().contains("« m.bin »"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn diagnose_names_every_missing_file_and_shares_the_folder_errors() {
        let dir = scratch("diagnose-missing");
        std::fs::write(
            dir.join("d.modele.json"),
            r#"{"model":"x.gguf","adapter":"y.gguf"}"#,
        )
        .unwrap();
        let d = diagnose(dir.to_str(), None).unwrap();
        assert_eq!(
            d.models[0].problem.as_deref(),
            Some("fichier manquant : x.gguf, y.gguf")
        );
        // Un adaptateur présent reste caché, même quand sa déclaration a un problème.
        std::fs::write(dir.join("y.gguf"), b"12").unwrap();
        let d = diagnose(dir.to_str(), None).unwrap();
        let got: Vec<_> = d
            .models
            .iter()
            .map(|m| (m.file.as_str(), m.size_bytes, m.problem.as_deref()))
            .collect();
        assert_eq!(got, vec![("d.modele.json", 2, Some("fichier manquant : x.gguf"))]);
        let err = diagnose(dir.join("absent").to_str(), None).unwrap_err();
        assert_eq!(err.code, "models.no_folder");
        assert!(diagnose(None, None).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
