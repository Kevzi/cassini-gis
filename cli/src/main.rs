//! Binaire `cassini` : premier point d'entrée du registre de commandes (AD-13).

mod mcp;

use std::io::{BufRead, Write};
use std::path::Path;
use std::process::ExitCode;

use cassini_core::error::ErrorReport;
use cassini_core::exec::Router;
use cassini_core::journal::{Replay, Session};
use cassini_core::registry::{Registry, RegistryError};
use serde_json::{json, Value};

const USAGE: &str = "Usage : cassini run <id> [json] [--project <fichier>]
       cassini replay <projet>
       cassini serve
       cassini mcp [--project <fichier>]
       cassini model-check --model <fichier.gguf> --prompt <phrase>
       cassini plan <phrase> --inventory <fichier.json> [--model <fichier.gguf|fichier.modele.json>]
";

/// Arguments de `run` : id, paramètres JSON facultatifs, projet facultatif.
struct RunArgs<'a> {
    id: &'a str,
    raw: Option<&'a str>,
    project: Option<&'a str>,
}

fn parse_run(args: &[String]) -> Option<RunArgs<'_>> {
    let (cmd, rest) = args.split_first()?;
    if cmd != "run" {
        return None;
    }
    let (mut project, mut positional) = (None, Vec::new());
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        if a == "--project" {
            if project.is_some() {
                return None;
            }
            project = Some(it.next()?.as_str());
        } else {
            positional.push(a.as_str());
        }
    }
    match positional[..] {
        [id] => Some(RunArgs {
            id,
            raw: None,
            project,
        }),
        [id, raw] => Some(RunArgs {
            id,
            raw: Some(raw),
            project,
        }),
        _ => None,
    }
}

/// Exécute la CLI ; renvoie le code de sortie. Résultat sur `out`, erreurs sur `err`.
pub fn run(
    args: &[String],
    input: &mut (dyn BufRead + Send),
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    if args.first().map(String::as_str) == Some("serve") {
        return serve(&args[1..], input, out, err);
    }
    if args.first().map(String::as_str) == Some("mcp") {
        let Some(project) = mcp::parse(&args[1..]) else {
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        };
        return mcp::serve(project, err);
    }
    if args.first().map(String::as_str) == Some("replay") {
        return replay(&args[1..], out, err);
    }
    if args.first().map(String::as_str) == Some("model-check") {
        return model_check(&args[1..], out, err);
    }
    if args.first().map(String::as_str) == Some("plan") {
        return plan(&args[1..], out, err);
    }
    let Some(RunArgs { id, raw, project }) = parse_run(args) else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let registry = Registry::standard();
    let params = match raw {
        None => Ok(json!({})),
        Some(text) => serde_json::from_str::<Value>(text)
            .map_err(|e| ErrorReport::from(RegistryError::InvalidParams(e.to_string()))),
    };
    let outcome = match (params, project) {
        (Err(e), _) => Err(e),
        (Ok(params), None) => Router::new(&registry)
            .call(id, params)
            .map_err(ErrorReport::from),
        (Ok(params), Some(project)) => Session::open(&registry, Path::new(project))
            .map_err(ErrorReport::from)
            .and_then(|mut session| session.call(id, params)),
    };
    match outcome {
        Ok(value) => {
            // Un résultat qu'on n'a pas pu écrire n'est pas un succès (AD-21).
            if writeln!(out, "{value}").is_err() {
                return 1;
            }
            0
        }
        Err(e) => {
            let _ = writeln!(
                err,
                "{}",
                serde_json::to_string(&e).expect("une erreur se sérialise")
            );
            1
        }
    }
}

/// `cassini serve` : le canal local JSON-RPC sur l'entrée et la sortie standard.
fn serve(
    args: &[String],
    input: &mut (dyn BufRead + Send),
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    if !args.is_empty() {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    }
    let registry = Registry::standard();
    match cassini_core::serve::serve_with_composer(
        &registry,
        &ModelComposer::from_env(),
        input,
        out,
    ) {
        Ok(()) => 0,
        Err(e) => {
            let report = ErrorReport {
                code: "serve.io".to_string(),
                message: format!("Le canal est interrompu (détail technique : {}).", e.kind()),
                step: None,
            };
            let _ = writeln!(
                err,
                "{}",
                serde_json::to_string(&report).expect("une erreur se sérialise")
            );
            1
        }
    }
}

/// Mention neutre qui remplace le dossier des modèles dans les messages (AD-9).
const FOLDER: &str = "(dossier des modèles)";

/// Compositeur du canal : le modèle local, cherché dans le dossier des modèles à chaque
/// proposition, puis `compose_plan` (AD-23). Ses messages ne portent aucun chemin absolu
/// (AD-9). Le noyau seul découvre les modèles ; écart assumé à AD-15 (2026-10-18) : il passe
/// par le diagnostic de `model_list` (le même parcours que `model.list`), hors registre.
struct ModelComposer {
    /// Valeur de `CASSINI_MODELS`, si elle est posée.
    setting: Option<String>,
    /// Dossier de l'exécutable, à côté duquel se cherche `models/`.
    exe_dir: Option<std::path::PathBuf>,
}

impl ModelComposer {
    fn from_env() -> Self {
        ModelComposer {
            setting: std::env::var(cassini_core::commands::model_list::MODELS_ENV).ok(),
            exe_dir: std::env::current_exe()
                .ok()
                .and_then(|e| e.parent().map(Path::to_path_buf)),
        }
    }

    /// Le diagnostic du dossier des modèles, et les chemins à masquer dans les messages : le
    /// dossier désigné, puis le dossier retenu. Une erreur de découverte est déjà masquée.
    fn discover(
        &self,
    ) -> Result<
        (
            cassini_core::commands::model_list::ModelDiagnosis,
            Vec<(String, String)>,
        ),
        ErrorReport,
    > {
        let mut hidden: Vec<(String, String)> = self
            .setting
            .clone()
            // Une valeur relative ou courte (« models ») n'est pas masquée : remplacée
            // partout dans le message, elle le défigurerait.
            .filter(|s| Path::new(s).is_absolute())
            .map(|s| (s, FOLDER.to_string()))
            .into_iter()
            .collect();
        let diagnosis = cassini_core::commands::model_list::diagnose(
            self.setting.as_deref(),
            self.exe_dir.as_deref(),
        )
        .map_err(|f| scrub(report(&f.code, f.message), &hidden))?;
        if diagnosis.folder.is_absolute() {
            hidden.push((diagnosis.folder.display().to_string(), FOLDER.to_string()));
        }
        Ok((diagnosis, hidden))
    }

    /// Choisit le modèle : `model` (un nom de la liste, comparé tel quel), sinon le premier
    /// sans problème. Rend aussi les chemins à masquer, le modèle d'abord. Rien n'est chargé.
    fn choose(
        &self,
        model: Option<&str>,
    ) -> Result<(ChosenModel, Vec<(String, String)>), ErrorReport> {
        let (diagnosis, mut hidden) = self.discover()?;
        let entry = match model {
            None => diagnosis
                .models
                .iter()
                .find(|m| m.problem.is_none())
                .ok_or_else(|| {
                    report(
                        "ai.no_model",
                        "Aucun modèle utilisable n'a été trouvé dans le dossier des modèles."
                            .to_string(),
                    )
                })?,
            Some(name) => {
                let found = diagnosis.models.iter().find(|m| m.file == name);
                let Some(found) = found else {
                    return Err(scrub(
                        report(
                            "models.unknown",
                            format!("Le modèle « {name} » n'est pas dans le dossier des modèles."),
                        ),
                        &hidden,
                    ));
                };
                if let Some(problem) = &found.problem {
                    return Err(scrub(
                        report(
                            "models.unusable",
                            format!("Le modèle « {name} » ne peut pas servir : {problem}."),
                        ),
                        &hidden,
                    ));
                }
                found
            }
        };
        let path = diagnosis.folder.join(&entry.file);
        hidden.insert(0, (path.to_string_lossy().into_owned(), entry.file.clone()));
        let chosen = resolve_model(&path).map_err(|e| scrub(e, &hidden))?;
        // Chaque fichier du modèle (base, adaptateur) se masque par son seul nom, avant le dossier.
        for pair in chosen.hidden().into_iter().rev() {
            hidden.insert(0, pair);
        }
        Ok((chosen, hidden))
    }
}

impl cassini_core::serve::Composer for ModelComposer {
    fn compose(
        &self,
        registry: &Registry,
        request: &str,
        model: Option<&str>,
        inventory: &mut dyn FnMut() -> Result<
            cassini_core::agent_inventory::ModelInventory,
            ErrorReport,
        >,
    ) -> Result<cassini_core::plan::Plan, ErrorReport> {
        // La phrase vide est refusée avant toute recherche de modèle ni demande d'inventaire.
        if request.trim().is_empty() {
            let e = cassini_ai::AiError::EmptyPrompt;
            return Err(report(e.code(), e.to_string()));
        }
        // Modèle inconnu ou inutilisable : refusé avant tout inventaire.
        let (chosen, hidden) = self.choose(model)?;
        let view = inventory()?;
        let mut generator = measured_generator(chosen.files.clone());
        chosen
            .compose(
                registry,
                &view.inventory,
                Some(view.user_layers),
                request,
                &mut generator,
            )
            .map_err(|e| scrub(report(e.code(), e.to_string()), &hidden))
    }

    fn models(&self, _registry: &Registry) -> Result<Vec<Value>, ErrorReport> {
        let (diagnosis, hidden) = self.discover()?;
        Ok(diagnosis
            .models
            .into_iter()
            .map(|mut m| {
                m.problem = m.problem.map(|p| hide(&p, &hidden));
                serde_json::to_value(m).expect("un modèle se sérialise")
            })
            .collect())
    }
}

/// Générateur du canal, branché sur le modèle `files`. Deux variables d'environnement servent à
/// mesurer une session réelle (ticket 5.19), sans rien changer sans elles : `CASSINI_TIMINGS` non
/// vide fait écrire à chaque appel au modèle ses durées sur la sortie d'erreur (une ligne, sans
/// rien de la demande) ; `CASSINI_THREADS=<n>` fixe le nombre de fils du moteur, à la place du
/// défaut (`cassini_ai::default_threads` : les cœurs physiques autorisés par le masque d'affinité,
/// sous Windows et sous Linux).
fn measured_generator(
    files: cassini_ai::ModelFiles,
) -> impl FnMut(
    &cassini_ai::plan::GenerationRequest,
) -> Result<cassini_ai::Completion, cassini_ai::AiError> {
    let report = std::env::var_os("CASSINI_TIMINGS").is_some_and(|v| !v.is_empty());
    let threads = std::env::var("CASSINI_THREADS")
        .ok()
        .and_then(|v| v.parse::<std::num::NonZeroU32>().ok());
    move |request| {
        let n = threads.unwrap_or_else(cassini_ai::default_threads);
        let (completion, t) = cassini_ai::generate_files_measured(
            &files,
            request.turns,
            Some(request.grammar),
            request.max_new_tokens,
            Some(n),
        )?;
        if report {
            eprintln!(
                "cassini-timings chargement={:.2}s lecture={:.2}s jetons={} repris={} ecriture={:.2}s ecrits={} fils={}",
                t.load.as_secs_f64(),
                t.prompt.as_secs_f64(),
                t.prompt_tokens,
                t.cached_tokens,
                t.write.as_secs_f64(),
                t.written_tokens,
                n
            );
        }
        Ok(completion)
    }
}

/// Un modèle choisi : ses fichiers, et le prompt qu'il attend.
#[derive(Debug, PartialEq)]
struct ChosenModel {
    files: cassini_ai::ModelFiles,
    style: cassini_ai::plan::PromptStyle,
}

impl ChosenModel {
    /// Compose le plan de `request` dans le prompt que ce modèle attend. `user_layers` : le nombre
    /// de couches de l'utilisateur en tête de `inventory`, quand il est connu (ticket 5.19).
    fn compose(
        &self,
        registry: &Registry,
        inventory: &Value,
        user_layers: Option<usize>,
        request: &str,
        generate: &mut dyn FnMut(
            &cassini_ai::plan::GenerationRequest,
        ) -> Result<cassini_ai::Completion, cassini_ai::AiError>,
    ) -> Result<cassini_core::plan::Plan, cassini_ai::plan::AgentError> {
        cassini_ai::plan::compose_plan_anchored(
            self.style,
            registry,
            inventory,
            user_layers,
            request,
            generate,
        )
    }

    /// Chemin de chaque fichier du modèle, avec le seul nom de fichier qui le remplace.
    fn hidden(&self) -> Vec<(String, String)> {
        std::iter::once(&self.files.model)
            .chain(self.files.adapter.as_ref())
            .map(|p| {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                (p.to_string_lossy().into_owned(), name)
            })
            .collect()
    }
}

/// Résout le fichier `path` d'un modèle par `cassini_ai::plan::resolve_model`, seule façon
/// de lire une déclaration ; la CLI n'en garde que la mise en forme de l'erreur.
fn resolve_model(path: &Path) -> Result<ChosenModel, ErrorReport> {
    let (files, style) =
        cassini_ai::plan::resolve_model(path).map_err(|e| report(e.code(), e.to_string()))?;
    Ok(ChosenModel { files, style })
}

fn report(code: &str, message: String) -> ErrorReport {
    ErrorReport {
        code: code.to_string(),
        message,
        step: None,
    }
}

/// Remplace dans le message chaque chemin de `hidden` par son substitut, le plus précis
/// d'abord : le modèle par son seul nom de fichier, le dossier par une mention neutre.
fn scrub(mut e: ErrorReport, hidden: &[(String, String)]) -> ErrorReport {
    e.message = hide(&e.message, hidden);
    e
}

/// Remplace dans `text` chaque chemin de `hidden` par son substitut, dans l'ordre. Un chemin
/// est aussi reconnu écrit autrement, comme le fait llama.cpp : séparateurs `\` au lieu de `/`
/// ou l'inverse, et préfixe Windows `\\?\` (forme longue) devant l'écriture en `\`.
fn hide(text: &str, hidden: &[(String, String)]) -> String {
    let mut text = text.to_string();
    for (path, instead) in hidden.iter().filter(|(p, _)| !p.is_empty()) {
        let back = path.replace('/', "\\");
        let forward = path.replace('\\', "/");
        // Un partage réseau `\\serveur\partage` a pour forme longue `\\?\UNC\serveur\partage`.
        let unc = back
            .strip_prefix(r"\\")
            .filter(|_| !back.starts_with(r"\\?\"))
            .map(|rest| format!(r"\\?\UNC\{rest}"));
        // Les formes longues d'abord : remplacer l'écriture en `\` avant elles laisserait le préfixe.
        for variant in unc
            .into_iter()
            .chain([format!(r"\\?\{back}"), path.clone(), back, forward])
        {
            text = text.replace(variant.as_str(), instead);
        }
    }
    text
}

/// `cassini replay <projet>` : rejoue le journal du projet et rend le bilan.
fn replay(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let [project] = args else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let registry = Registry::standard();
    let outcome = Replay::load(&registry, Path::new(project)).and_then(|replay| {
        // Avant la première étape : les entrées non portables sont signalées.
        for warning in replay.warnings() {
            let _ = writeln!(err, "Avertissement : {warning}");
        }
        replay.run(&Router::new(&registry), &mut Default::default())
    });
    match outcome {
        Ok(report) => {
            let json = serde_json::to_string(&report).expect("un bilan se sérialise");
            if writeln!(out, "{json}").is_err() {
                return 1;
            }
            0
        }
        Err(e) => {
            let _ = writeln!(
                err,
                "{}",
                serde_json::to_string(&e).expect("une erreur se sérialise")
            );
            1
        }
    }
}

/// `cassini model-check` : charge le modèle local et rend le texte de sa réponse.
fn model_check(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let (mut model, mut prompt) = (None, None);
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let slot = match flag.as_str() {
            "--model" => &mut model,
            "--prompt" => &mut prompt,
            _ => {
                let _ = err.write_all(USAGE.as_bytes());
                return 2;
            }
        };
        match it.next() {
            Some(v) if slot.is_none() => *slot = Some(v.as_str()),
            _ => {
                let _ = err.write_all(USAGE.as_bytes());
                return 2;
            }
        }
    }
    let (Some(model), Some(prompt)) = (model, prompt) else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    match cassini_ai::complete(Path::new(model), prompt) {
        Ok(done) => {
            if writeln!(out, "{}", done.text).is_err() {
                return 1;
            }
            if done.truncated {
                let report = ErrorReport {
                    code: "ai.truncated".to_string(),
                    message: "La réponse a été coupée avant la fin.".to_string(),
                    step: None,
                };
                let _ = writeln!(
                    err,
                    "{}",
                    serde_json::to_string(&report).expect("une erreur se sérialise")
                );
                return 1;
            }
            0
        }
        Err(e) => {
            let report = ErrorReport {
                code: e.code().to_string(),
                message: e.to_string(),
                step: None,
            };
            let _ = writeln!(
                err,
                "{}",
                serde_json::to_string(&report).expect("une erreur se sérialise")
            );
            1
        }
    }
}

/// Écrit une erreur typée sur `err` et rend le code de sortie 1.
fn fail(err: &mut dyn Write, code: &str, message: String) -> i32 {
    let report = ErrorReport {
        code: code.to_string(),
        message,
        step: None,
    };
    let _ = writeln!(
        err,
        "{}",
        serde_json::to_string(&report).expect("une erreur se sérialise")
    );
    1
}

/// Modèle de `cassini plan` : le fichier `model` s'il est donné, sinon la même règle que le
/// canal (`ModelComposer::choose`, le premier modèle sans problème du dossier des modèles).
/// Rend aussi les chemins à masquer ; une erreur est déjà masquée (AD-9).
fn plan_model(
    model: Option<&str>,
    composer: &ModelComposer,
) -> Result<(ChosenModel, Vec<(String, String)>), ErrorReport> {
    match model {
        Some(m) => {
            let chosen = resolve_model(Path::new(m))?;
            let hidden = chosen.hidden();
            Ok((chosen, hidden))
        }
        None => composer.choose(None),
    }
}

/// `cassini plan <phrase> --inventory <fichier.json> [--model <fichier.gguf|.modele.json>]` :
/// compose le plan d'une demande (AD-23) et l'affiche en JSON. `--model` prend un GGUF seul ou
/// une déclaration de modèle (base, adaptateur, prompt). Rien n'est exécuté. Une question du
/// modèle ou du garde-fou sur les coordonnées (`{"kind":"question",...}`, `"source":"guard"`
/// pour le second) est un résultat : affichée sur `out`, code 0. Un plan
/// invalide n'est jamais affiché : l'erreur `agent.plan_invalid` ou `agent.refused` sort sur
/// `err`.
fn plan(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let (mut inventory, mut model, mut request) = (None, None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let slot = match a.as_str() {
            "--inventory" => &mut inventory,
            "--model" => &mut model,
            _ if request.is_none() && !a.starts_with("--") => {
                request = Some(a.as_str());
                continue;
            }
            _ => {
                let _ = err.write_all(USAGE.as_bytes());
                return 2;
            }
        };
        match it.next() {
            Some(v) if slot.is_none() => *slot = Some(v.as_str()),
            _ => {
                let _ = err.write_all(USAGE.as_bytes());
                return 2;
            }
        }
    }
    let (Some(request), Some(inventory)) = (request, inventory) else {
        let _ = err.write_all(USAGE.as_bytes());
        return 2;
    };
    let registry = Registry::standard();
    let inventory: Value = match std::fs::read_to_string(inventory)
        .map_err(|e| format!("L'inventaire n'a pas pu être lu : {e}."))
        .and_then(|t| {
            serde_json::from_str(&t)
                .map_err(|e| format!("L'inventaire n'est pas du JSON lisible : {e}."))
        }) {
        Ok(v) => v,
        Err(message) => return fail(err, "agent.bad_inventory", message),
    };
    // La phrase vide est refusée avant toute recherche de modèle.
    if request.trim().is_empty() {
        let e = cassini_ai::AiError::EmptyPrompt;
        return fail(err, e.code(), e.to_string());
    }
    let (chosen, hidden) = match plan_model(model, &ModelComposer::from_env()) {
        Ok(c) => c,
        Err(e) => return fail(err, &e.code, e.message),
    };
    let mut generator = cassini_ai::plan::model_files_generator(chosen.files.clone());
    match chosen.compose(&registry, &inventory, None, request, &mut generator) {
        // Les fichiers du modèle ne sont nommés que par leur nom, le dossier par une mention
        // neutre, comme dans le canal.
        Err(e) => {
            let e = scrub(report(e.code(), e.to_string()), &hidden);
            fail(err, &e.code, e.message)
        }
        ok => print_composed(ok, out, err),
    }
}

/// Affiche le résultat d'une composition : un plan ou une question en JSON sur `out` (code 0),
/// une erreur typée sur `err`. La provenance d'une question n'est écrite que pour le garde-fou
/// (`"source":"guard"`) : une question du modèle garde sa forme d'avant.
fn print_composed(
    composed: Result<cassini_core::plan::Plan, cassini_ai::plan::AgentError>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match composed {
        Ok(plan) => {
            let json = serde_json::to_string(&plan).expect("un plan se sérialise");
            if writeln!(out, "{json}").is_err() {
                return 1;
            }
            0
        }
        Err(e) => fail(err, e.code(), e.to_string()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run(
        &args,
        &mut std::io::BufReader::new(std::io::stdin()),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    ExitCode::from(code as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(args: &[&str]) -> (i32, String, String) {
        call_with(args, "")
    }

    fn call_with(args: &[&str], input: &str) -> (i32, String, String) {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(&args, &mut input.as_bytes(), &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    fn error_code(err: &str) -> String {
        let v: Value = serde_json::from_str(err).unwrap();
        v["code"].as_str().unwrap().to_string()
    }

    #[test]
    fn about_prints_versions() {
        let (code, out, err) = call(&["run", "system.about"]);
        assert_eq!(code, 0);
        assert_eq!(
            out.trim(),
            format!(
                r#"{{"core_version":"{}","protocol_version":2}}"#,
                cassini_core::VERSION
            )
        );
        assert!(err.is_empty());
    }

    #[test]
    fn unknown_command() {
        let (code, out, err) = call(&["run", "foo.bar"]);
        assert_eq!((code, out.as_str()), (1, ""));
        assert_eq!(error_code(&err), "registry.unknown_command");
        assert!(err.contains("foo.bar"));
    }

    #[test]
    fn unknown_field() {
        let (code, out, err) = call(&["run", "system.about", r#"{"x":1}"#]);
        assert_eq!((code, out.as_str()), (1, ""));
        assert_eq!(error_code(&err), "registry.invalid_params");
    }

    #[test]
    fn unreadable_json() {
        let (code, out, err) = call(&["run", "system.about", "{"]);
        assert_eq!((code, out.as_str()), (1, ""));
        assert_eq!(error_code(&err), "registry.invalid_params");
    }

    #[test]
    fn no_argument_prints_usage() {
        let (code, out, err) = call(&["run"]);
        assert_eq!((code, out.as_str()), (2, ""));
        assert!(err.contains("Usage"));
    }

    fn temp_project() -> (std::path::PathBuf, String) {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cassini-cli-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let project = dir.join("p.qgz").to_string_lossy().into_owned();
        (dir, project)
    }

    #[test]
    fn three_runs_with_a_project_give_three_journal_lines() {
        let (dir, project) = temp_project();
        for _ in 0..3 {
            let (code, out, err) = call(&["run", "system.about", "--project", &project]);
            assert_eq!((code, err.as_str()), (0, ""));
            assert!(out.contains("core_version"));
        }
        let journal = std::fs::read_to_string(dir.join("p.cassini/journal.jsonl")).unwrap();
        assert_eq!(journal.lines().count(), 3);
        let last: Value = serde_json::from_str(journal.lines().last().unwrap()).unwrap();
        assert_eq!(last["step"], 3);
        assert!(!journal.contains("cassini-cli-"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_second_session_is_read_only() {
        let (dir, project) = temp_project();
        let registry = Registry::standard();
        let _holder = Session::open(&registry, Path::new(&project)).unwrap();
        let (code, out, err) = call(&["run", "system.about", "--project", &project]);
        assert_eq!((code, out.as_str()), (1, ""));
        assert_eq!(error_code(&err), "journal.read_only");
        assert!(!dir.join("p.cassini/journal.jsonl").exists());
        drop(_holder);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_params_with_a_project_write_nothing() {
        let (dir, project) = temp_project();
        let (code, _, err) = call(&["run", "system.about", r#"{"x":1}"#, "--project", &project]);
        assert_eq!(code, 1);
        assert_eq!(error_code(&err), "registry.invalid_params");
        assert!(!dir.join("p.cassini/journal.jsonl").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn replay_gives_an_identical_report_even_while_a_session_holds_the_lock() {
        let (dir, project) = temp_project();
        let registry = Registry::standard();
        let mut holder = Session::open(&registry, Path::new(&project)).unwrap();
        holder.call("system.about", json!({})).unwrap();
        holder.call("system.about", json!({})).unwrap();
        let before = std::fs::read_to_string(dir.join("p.cassini/journal.jsonl")).unwrap();
        let (code, out, err) = call(&["replay", &project]);
        assert_eq!((code, err.as_str()), (0, ""));
        assert_eq!(out.trim(), r#"{"steps":2,"identical":true}"#);
        let after = std::fs::read_to_string(dir.join("p.cassini/journal.jsonl")).unwrap();
        assert_eq!(before, after);
        drop(holder);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn terrain_slope_runs_without_qgis_and_replays_identically() {
        let (dir, project) = temp_project();
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/mnt-plan-incline-2154.tif");
        std::fs::create_dir_all(dir.join("dem")).unwrap();
        std::fs::copy(&fixture, dir.join("dem/plan-incline.tif")).unwrap();
        let (code, out, err) = call(&[
            "run",
            "terrain.slope",
            r#"{"dem":"dem/plan-incline.tif","unit":"percent"}"#,
            "--project",
            &project,
        ]);
        assert_eq!((code, err.as_str()), (0, ""));
        let result: Value = serde_json::from_str(out.trim()).unwrap();
        // La réponse porte le chemin absolu, comme `command.run` à l'extension ; le journal le
        // garde relatif au dossier de travail.
        let file = Path::new(result["file"].as_str().unwrap());
        assert!(file.is_absolute(), "{}", file.display());
        assert_eq!(file, dir.join("p.cassini/1-terrain.slope.tif"));
        assert_eq!(result["nodata_cells"], 136);
        // Le journal garde la sortie relative au dossier de travail.
        let journal = std::fs::read_to_string(dir.join("p.cassini/journal.jsonl")).unwrap();
        let line: Value = serde_json::from_str(journal.trim()).unwrap();
        assert_eq!(line["result"]["file"], "1-terrain.slope.tif");
        assert_eq!(line["params"]["dem"], "dem/plan-incline.tif");

        let written = dir.join("p.cassini/1-terrain.slope.tif");
        let grid = cassini_core::dem::Dem::open(&written).unwrap();
        let v = grid.value(10, 10).unwrap();
        assert!((f64::from(v) - 10.0).abs() < 1e-4, "{v}");
        let before = std::fs::read(&written).unwrap();

        let (code, out, err) = call(&["replay", &project]);
        assert_eq!((code, err.as_str()), (0, ""));
        assert_eq!(out.trim(), r#"{"steps":1,"identical":true}"#);
        let replayed = dir.join("p.cassini/replay/1-terrain.slope.tif");
        assert_eq!(std::fs::read(&replayed).unwrap(), before);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn replay_without_a_journal_is_refused() {
        let (dir, project) = temp_project();
        let (code, out, err) = call(&["replay", &project]);
        assert_eq!((code, out.as_str()), (1, ""));
        assert_eq!(error_code(&err), "replay.no_journal");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn replay_with_an_unknown_command_names_it() {
        let (dir, project) = temp_project();
        std::fs::create_dir_all(dir.join("p.cassini")).unwrap();
        std::fs::write(
            dir.join("p.cassini/journal.jsonl"),
            "{\"step\":1,\"id\":\"no.such\",\"params\":{},\"inputs\":[],\"result\":{}}\n",
        )
        .unwrap();
        let (code, out, err) = call(&["replay", &project]);
        assert_eq!((code, out.as_str()), (1, ""));
        assert_eq!(error_code(&err), "registry.unknown_command");
        assert!(err.contains("no.such"));
        assert!(!dir.join("p.cassini/replay").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn replay_needs_exactly_one_project() {
        for args in [&["replay"][..], &["replay", "a", "b"][..]] {
            let (code, _, err) = call(args);
            assert_eq!(code, 2);
            assert!(err.contains("cassini replay"));
        }
    }

    #[test]
    fn a_dangling_project_flag_prints_usage() {
        let (code, _, err) = call(&["run", "system.about", "--project"]);
        assert_eq!(code, 2);
        assert!(err.contains("Usage"));
    }

    #[test]
    fn serve_answers_hello_and_a_call_then_frees_the_lock() {
        let (dir, project) = temp_project();
        let project = project.replace('\\', "/");
        let input = format!(
            "{}
{}
",
            json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
                "params":{"protocol_version":2,"commands":[],"project":project}}),
            json!({"jsonrpc":"2.0","id":2,"method":"command.run",
                "params":{"command":"system.about"}}),
        );
        let (code, out, err) = call_with(&["serve"], &input);
        assert_eq!((code, err.as_str()), (0, ""));
        let lines: Vec<Value> = out
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0]["result"]["commands"].is_array());
        assert_eq!(lines[1]["result"]["protocol_version"], 2);
        let journal = std::fs::read_to_string(dir.join("p.cassini/journal.jsonl")).unwrap();
        assert_eq!(journal.lines().count(), 1);
        let registry = Registry::standard();
        let session = Session::open(&registry, Path::new(&project)).unwrap();
        assert_eq!(session.mode(), cassini_core::journal::Mode::Write);
        drop(session);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn serve_survives_an_unreadable_line_and_takes_no_argument() {
        let (code, out, _) = call_with(
            &["serve"],
            "xx
",
        );
        assert_eq!(code, 0);
        assert!(out.contains("-32700"));
        let (code, _, err) = call(&["serve", "extra"]);
        assert_eq!(code, 2);
        assert!(err.contains("cassini serve"));
    }

    #[test]
    fn mcp_with_a_bad_argument_prints_usage() {
        for args in [
            &["mcp", "extra"][..],
            &["mcp", "--project"][..],
            &["mcp", "--project", "a", "b"][..],
        ] {
            let (code, out, err) = call(args);
            assert_eq!((code, out.as_str()), (2, ""), "{args:?}");
            assert!(err.contains("cassini mcp"), "{args:?}");
        }
    }

    #[test]
    fn mcp_with_a_project_that_cannot_open_fails_before_any_exchange() {
        let (dir, _) = temp_project();
        let project = dir.join("absent").join("manque.qgz");
        let (code, out, err) = call(&["mcp", "--project", &project.to_string_lossy()]);
        assert_eq!((code, out.as_str()), (1, ""));
        assert!(error_code(err.trim()).starts_with("journal."), "{err}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn model_check_missing_model_is_a_clear_error() {
        let (code, out, err) =
            call(&["model-check", "--model", "absent.gguf", "--prompt", "Salut"]);
        assert_eq!((code, out.as_str()), (1, ""));
        assert_eq!(error_code(&err), "ai.model_not_found");
    }

    #[test]
    fn model_check_empty_prompt_is_refused_before_loading() {
        let (code, _, err) = call(&["model-check", "--model", "absent.gguf", "--prompt", ""]);
        assert_eq!(code, 1);
        assert_eq!(error_code(&err), "ai.empty_prompt");
    }

    #[test]
    fn model_check_without_model_prints_usage() {
        for args in [
            &["model-check", "--prompt", "x"][..],
            &["model-check"][..],
            &["model-check", "--model"][..],
            &[
                "model-check",
                "--model",
                "a",
                "--model",
                "b",
                "--prompt",
                "x",
            ][..],
            &["model-check", "--bogus", "a"][..],
        ] {
            let (code, _, err) = call(args);
            assert_eq!(code, 2, "{args:?}");
            assert!(err.contains("cassini model-check"));
        }
    }

    fn inventory_file(dir: &std::path::Path) -> String {
        let f = dir.join("inventaire.json");
        std::fs::write(&f, r#"{"layers":[]}"#).unwrap();
        f.to_string_lossy().into_owned()
    }

    #[test]
    fn plan_with_a_missing_model_is_a_typed_error() {
        let (dir, _) = temp_project();
        let inv = inventory_file(&dir);
        let (code, out, err) = call(&[
            "plan",
            "Ouvre les gares.",
            "--inventory",
            &inv,
            "--model",
            "absent.gguf",
        ]);
        assert_eq!((code, out.as_str()), (1, ""));
        assert_eq!(error_code(&err), "ai.model_not_found");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_declaration_resolves_to_its_files_in_its_folder_with_its_prompt() {
        use cassini_ai::plan::PromptStyle;
        use cassini_ai::ModelFiles;
        let (dir, _) = temp_project();
        let decl = dir.join("ajuste.modele.json");
        std::fs::write(
            &decl,
            r#"{"model":"base.gguf","adapter":"lora.gguf","prompt":"short"}"#,
        )
        .unwrap();
        let chosen = resolve_model(&decl).unwrap();
        assert_eq!(
            chosen,
            ChosenModel {
                files: ModelFiles::with_adapter(dir.join("base.gguf"), dir.join("lora.gguf")),
                style: PromptStyle::Short,
            }
        );
        let hidden = chosen.hidden();
        assert_eq!(hidden[0].1, "base.gguf");
        assert_eq!(hidden[1].1, "lora.gguf");
        assert_eq!(hidden[1].0, dir.join("lora.gguf").to_string_lossy());
        // Un GGUF seul garde le long prompt.
        let gguf = dir.join("Qwen3.5-9B-Q4_K_M.gguf");
        assert_eq!(
            resolve_model(&gguf).unwrap(),
            ChosenModel {
                files: ModelFiles::single(&gguf),
                style: PromptStyle::Full,
            }
        );
        // Une déclaration absente est un modèle introuvable, nommé par son seul fichier.
        let absent = dir.join("absente.modele.json");
        let e = resolve_model(&absent).unwrap_err();
        assert_eq!(e.code, "ai.model_not_found");
        assert!(e.message.contains("absente.modele.json"), "{}", e.message);
        assert!(
            !e.message.contains(&*dir.to_string_lossy()),
            "{}",
            e.message
        );
        // Une déclaration qui nomme un chemin est refusée, sans recopier le dossier.
        std::fs::write(&decl, r#"{"model":"../base.gguf"}"#).unwrap();
        let e = resolve_model(&decl).unwrap_err();
        assert_eq!(e.code, "models.bad_declaration");
        assert!(
            !e.message.contains(&*dir.to_string_lossy()),
            "{}",
            e.message
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn plan_with_a_declaration_naming_a_missing_file_is_model_not_found() {
        let (dir, _) = temp_project();
        let inv = inventory_file(&dir);
        let decl = dir.join("ajuste.modele.json");
        std::fs::write(&decl, r#"{"model":"absent-base.gguf","prompt":"short"}"#).unwrap();
        let (code, out, err) = call(&[
            "plan",
            "Ouvre les gares.",
            "--inventory",
            &inv,
            "--model",
            &decl.to_string_lossy(),
        ]);
        assert_eq!((code, out.as_str()), (1, ""));
        assert_eq!(error_code(&err), "ai.model_not_found");
        assert!(err.contains("absent-base.gguf"), "{err}");
        let folder = dir.to_string_lossy();
        assert!(
            !err.contains(&*folder)
                && !err.contains(&folder.replace('\\', "/"))
                && !err.contains(&folder.replace('\\', "\\\\")),
            "{err}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_declared_short_prompt_reaches_the_model_and_a_bare_gguf_gets_the_long_one() {
        let (dir, _) = temp_project();
        let decl = dir.join("ajuste.modele.json");
        std::fs::write(&decl, r#"{"model":"base.gguf","prompt":"short"}"#).unwrap();
        let registry = Registry::standard();
        let inventory = json!({"layers": []});
        let first_system = |chosen: &ChosenModel| {
            let mut seen = None;
            let _ = chosen.compose(&registry, &inventory, None, "Ouvre les gares.", &mut |r| {
                seen.get_or_insert_with(|| r.turns[0].cached_head.clone());
                Ok(cassini_ai::Completion {
                    text: r#"{"kind":"question","question":"Laquelle ?"}"#.to_string(),
                    truncated: false,
                })
            });
            seen.expect("le générateur est appelé")
        };
        let short = first_system(&resolve_model(&decl).unwrap());
        assert!(short.starts_with("Tu es l'agent"), "{short}");
        assert!(!short.contains("CATALOGUE"), "{short}");
        let long = first_system(&resolve_model(&dir.join("Qwen.gguf")).unwrap());
        assert!(long.contains("CATALOGUE"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_question_is_printed_as_json_on_out_with_code_0() {
        let question = cassini_core::plan::Plan::Question {
            question: "Aucune couche de pharmacies : laquelle utiliser ?".to_string(),
            source: cassini_core::plan::QuestionSource::Model,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = print_composed(Ok(question), &mut out, &mut err);
        let out = String::from_utf8(out).unwrap();
        assert_eq!(code, 0);
        assert!(err.is_empty());
        let shown: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(
            shown,
            json!({"kind": "question", "question": "Aucune couche de pharmacies : laquelle utiliser ?"})
        );
    }

    #[test]
    fn a_guard_question_is_printed_with_its_source() {
        let question = cassini_core::plan::Plan::Question {
            question: cassini_core::point::GUARD_QUESTION.to_string(),
            source: cassini_core::plan::QuestionSource::Guard,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(print_composed(Ok(question), &mut out, &mut err), 0);
        assert!(err.is_empty());
        let shown: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(
            shown,
            json!({"kind": "question", "question": cassini_core::point::GUARD_QUESTION,
                "source": "guard"})
        );
    }

    #[test]
    fn a_plan_is_printed_on_out_and_a_refusal_is_an_error_on_err() {
        let plan =
            json!({"kind": "calls", "calls": [{"command": "project.inventory", "params": {}}]});
        let calls: cassini_core::plan::Plan = serde_json::from_value(plan.clone()).unwrap();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = print_composed(Ok(calls), &mut out, &mut err);
        assert_eq!(code, 0);
        assert!(err.is_empty());
        let shown: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(shown, plan);

        let refused = cassini_ai::plan::AgentError::Refused("Hors catalogue.".to_string());
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = print_composed(Err(refused), &mut out, &mut err);
        assert_eq!(code, 1);
        assert!(out.is_empty());
        assert_eq!(
            error_code(&String::from_utf8(err).unwrap()),
            "agent.refused"
        );
    }

    #[test]
    fn plan_with_an_empty_sentence_is_refused_before_loading() {
        let (dir, _) = temp_project();
        let inv = inventory_file(&dir);
        let (code, _, err) = call(&["plan", " ", "--inventory", &inv, "--model", "absent.gguf"]);
        assert_eq!(code, 1);
        assert_eq!(error_code(&err), "ai.empty_prompt");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn plan_with_an_unreadable_inventory_is_a_typed_error() {
        let (code, out, err) = call(&[
            "plan",
            "x",
            "--inventory",
            "absent.json",
            "--model",
            "absent.gguf",
        ]);
        assert_eq!((code, out.as_str()), (1, ""));
        assert_eq!(error_code(&err), "agent.bad_inventory");
        let (dir, _) = temp_project();
        let bad = dir.join("mauvais.json");
        std::fs::write(&bad, "{").unwrap();
        let (code, _, err) = call(&[
            "plan",
            "x",
            "--inventory",
            &bad.to_string_lossy(),
            "--model",
            "absent.gguf",
        ]);
        assert_eq!(code, 1);
        assert_eq!(error_code(&err), "agent.bad_inventory");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn plan_without_sentence_or_inventory_prints_usage() {
        for args in [
            &["plan"][..],
            &["plan", "x"][..],
            &["plan", "--inventory", "a.json"][..],
            &["plan", "x", "y", "--inventory", "a.json"][..],
            &["plan", "x", "--inventory"][..],
            &["plan", "x", "--bogus", "a"][..],
            &["plan", "x", "--inventory", "a", "--inventory", "b"][..],
        ] {
            let (code, _, err) = call(args);
            assert_eq!(code, 2, "{args:?}");
            assert!(err.contains("cassini plan"));
        }
    }

    /// Dossier de modèles de test : deux GGUF (`a.gguf`, `b.gguf`), une déclaration illisible
    /// et une déclaration dont un GGUF manque.
    fn models_folder() -> (std::path::PathBuf, ModelComposer) {
        let (dir, _) = temp_project();
        std::fs::write(dir.join("a.gguf"), b"12345").unwrap();
        std::fs::write(dir.join("b.gguf"), b"123").unwrap();
        std::fs::write(dir.join("casse.modele.json"), "pas du json").unwrap();
        std::fs::write(dir.join("manque.modele.json"), r#"{"model":"absent.gguf"}"#).unwrap();
        let composer = ModelComposer {
            setting: Some(dir.to_string_lossy().into_owned()),
            exe_dir: None,
        };
        (dir, composer)
    }

    fn shows_no_folder(text: &str, dir: &std::path::Path) -> bool {
        let folder = dir.to_string_lossy();
        !text.contains(&*folder)
            && !text.contains(&folder.replace('\\', "/"))
            && !text.contains(&folder.replace('\\', "\\\\"))
    }

    #[test]
    fn the_composer_lists_its_models_by_name_with_their_problems() {
        use cassini_core::serve::Composer;
        let (dir, composer) = models_folder();
        let models = composer.models(&Registry::standard()).unwrap();
        let text = Value::Array(models.clone()).to_string();
        assert!(shows_no_folder(&text, &dir), "{text}");
        assert_eq!(models[0], json!({"file": "a.gguf", "size_bytes": 5}));
        assert_eq!(models[1], json!({"file": "b.gguf", "size_bytes": 3}));
        assert_eq!(models[2]["file"], "casse.modele.json");
        assert!(models[2]["problem"].is_string());
        assert_eq!(models[3]["problem"], "fichier manquant : absent.gguf");
        assert_eq!(models.len(), 4);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_composer_takes_the_chosen_model_or_the_first_usable_one() {
        let (dir, composer) = models_folder();
        let (chosen, _) = composer.choose(Some("b.gguf")).unwrap();
        assert_eq!(
            chosen.files,
            cassini_ai::ModelFiles::single(dir.join("b.gguf"))
        );
        let (first, _) = composer.choose(None).unwrap();
        assert_eq!(
            first.files,
            cassini_ai::ModelFiles::single(dir.join("a.gguf"))
        );
        // Sans GGUF utilisable, le premier sans problème manque : `ai.no_model`.
        for f in ["a.gguf", "b.gguf"] {
            std::fs::remove_file(dir.join(f)).unwrap();
        }
        assert_eq!(composer.choose(None).unwrap_err().code, "ai.no_model");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Ticket 5.19 : l'inventaire du noyau arrive au modèle coupé une fois par couche, avec
    /// l'ancrage sur la dernière couche de l'utilisateur. Une forme de sérialisation changée
    /// couperait l'inventaire d'un tenant, sans erreur : ce test le verrait.
    #[test]
    fn the_core_inventory_reaches_the_model_cut_per_layer_with_the_user_anchor() {
        use cassini_core::agent_inventory::{
            model_inventory, WorkFile, INVENTORY_BUDGET_CHARS, INVENTORY_LOW_CHARS,
        };
        let (dir, _) = temp_project();
        let decl = dir.join("ajuste.modele.json");
        std::fs::write(&decl, r#"{"model":"base.gguf","prompt":"short"}"#).unwrap();
        let layer = |id: &str, source: &str| {
            json!({"layer_id": id, "name": id, "source": source, "crs": "EPSG:2154",
                "kind": "vector", "geometry": "MultiPolygon", "feature_count": 3,
                "fields": [{"name": "typezone", "type": "String", "values": ["U", "N"]}]})
        };
        // Comme QGIS les pose : le résultat en tête de légende.
        let raw = json!({"project_name": "p", "project_crs": "EPSG:2154", "layers": [
            layer("1_analysis_buffer_e63fa196_c4d5_4255_9a08_203c73df1f96", "1-analysis.buffer.gpkg"),
            layer("plu_zones_5a071b18_e012_4e1f_9c1c_250bbfb7550c", "plu.gpkg"),
            layer("gares_83d935aa_ca33_431d_966f_dbbf0fb7ac0f", "gares.gpkg")
        ]});
        let files = [WorkFile {
            name: "1-analysis.buffer.gpkg".to_string(),
            modified: std::time::SystemTime::now(),
        }];
        let (view, _) = model_inventory(
            &raw,
            &files,
            &std::collections::HashSet::new(),
            INVENTORY_BUDGET_CHARS,
            INVENTORY_LOW_CHARS,
        );
        assert_eq!(view.user_layers, 2);
        let registry = Registry::standard();
        let mut seen = None;
        let _ = resolve_model(&decl).unwrap().compose(
            &registry,
            &view.inventory,
            Some(view.user_layers),
            "Ouvre les gares.",
            &mut |r| {
                seen.get_or_insert_with(|| r.turns[0].clone());
                Ok(cassini_ai::Completion {
                    text: r#"{"kind":"question","question":"Laquelle ?"}"#.to_string(),
                    truncated: false,
                })
            },
        );
        let system = seen.expect("le générateur est appelé");
        assert_eq!(system.cached_parts.len(), 3);
        assert_eq!(system.anchor, Some(view.user_layers - 1));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_unknown_or_unusable_model_is_refused_before_any_inventory() {
        use cassini_core::serve::Composer;
        let (dir, composer) = models_folder();
        let registry = Registry::standard();
        for (model, code) in [
            ("x.gguf", "models.unknown"),
            // Comparé tel quel : une autre casse n'est pas le même nom.
            ("A.gguf", "models.unknown"),
            ("manque.modele.json", "models.unusable"),
            ("casse.modele.json", "models.unusable"),
        ] {
            let mut asked = false;
            let e = composer
                .compose(&registry, "Ouvre les gares.", Some(model), &mut || {
                    asked = true;
                    Ok(cassini_core::agent_inventory::ModelInventory {
                        inventory: json!({}),
                        user_layers: 0,
                        left_out: Vec::new(),
                    })
                })
                .unwrap_err();
            assert_eq!(e.code, code, "{model}");
            assert!(e.message.contains(model), "{}", e.message);
            assert!(shows_no_folder(&e.message, &dir), "{}", e.message);
            assert!(!asked, "aucun inventaire pour {model}");
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_missing_models_folder_is_masked_and_an_empty_one_lists_nothing() {
        use cassini_core::serve::Composer;
        let (dir, _) = temp_project();
        let absent = ModelComposer {
            setting: Some(dir.join("absent").to_string_lossy().into_owned()),
            exe_dir: None,
        };
        let e = absent.models(&Registry::standard()).unwrap_err();
        assert_eq!(e.code, "models.no_folder");
        assert!(e.message.contains(FOLDER), "{}", e.message);
        assert!(shows_no_folder(&e.message, &dir), "{}", e.message);
        let empty = ModelComposer {
            setting: Some(dir.to_string_lossy().into_owned()),
            exe_dir: None,
        };
        assert!(empty.models(&Registry::standard()).unwrap().is_empty());
        assert_eq!(empty.choose(None).unwrap_err().code, "ai.no_model");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn hide_recognises_a_path_written_with_other_separators_or_the_long_prefix() {
        let hidden = vec![
            (
                r"C:\Cassini\models\base.gguf".to_string(),
                "base.gguf".to_string(),
            ),
            (r"C:\Cassini\models".to_string(), FOLDER.to_string()),
        ];
        for text in [
            r"échec : C:\Cassini\models\base.gguf",
            "échec : C:/Cassini/models/base.gguf",
            r"échec : \\?\C:\Cassini\models\base.gguf",
        ] {
            assert_eq!(hide(text, &hidden), "échec : base.gguf", "{text}");
        }
        assert_eq!(
            hide(r"rien dans \\?\C:\Cassini\models", &hidden),
            format!("rien dans {FOLDER}")
        );
        let unc = vec![(r"\\serveur\partage\models".to_string(), FOLDER.to_string())];
        for text in [
            r"rien dans \\?\UNC\serveur\partage\models",
            r"rien dans \\serveur\partage\models",
            "rien dans //serveur/partage/models",
        ] {
            assert_eq!(hide(text, &unc), format!("rien dans {FOLDER}"), "{text}");
        }
        let posix = vec![(
            "/opt/cassini/models/a.gguf".to_string(),
            "a.gguf".to_string(),
        )];
        assert_eq!(
            hide(
                r"vu \opt\cassini\models\a.gguf et /opt/cassini/models/a.gguf",
                &posix
            ),
            "vu a.gguf et a.gguf"
        );
    }

    #[test]
    fn plan_without_a_model_follows_the_channel_rule() {
        // Le premier modèle a un problème, le second est sain : le second est choisi.
        let (dir, _) = temp_project();
        std::fs::write(dir.join("a.modele.json"), "pas du json").unwrap();
        std::fs::write(dir.join("b.gguf"), b"123").unwrap();
        let composer = ModelComposer {
            setting: Some(dir.to_string_lossy().into_owned()),
            exe_dir: None,
        };
        let (chosen, hidden) = plan_model(None, &composer).unwrap();
        assert_eq!(chosen.files.model, dir.join("b.gguf"));
        assert_eq!(hidden[0].1, "b.gguf");
        assert!(hidden.iter().any(|(_, h)| h == FOLDER));
        // Un dossier vide : `ai.no_model`, sans le chemin du dossier.
        std::fs::remove_file(dir.join("b.gguf")).unwrap();
        std::fs::remove_file(dir.join("a.modele.json")).unwrap();
        let e = plan_model(None, &composer).unwrap_err();
        assert_eq!(e.code, "ai.no_model");
        assert!(shows_no_folder(&e.message, &dir), "{}", e.message);
        // Un dossier absent : erreur typée, sans le chemin du dossier.
        let absent = ModelComposer {
            setting: Some(dir.join("absent").to_string_lossy().into_owned()),
            exe_dir: None,
        };
        let e = plan_model(None, &absent).unwrap_err();
        assert_eq!(e.code, "models.no_folder");
        assert!(shows_no_folder(&e.message, &dir), "{}", e.message);
        // `--model` donné : le fichier tel quel, sans passer par le dossier.
        let (chosen, _) = plan_model(Some("ailleurs.gguf"), &absent).unwrap();
        assert_eq!(chosen.files.model, Path::new("ailleurs.gguf"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
