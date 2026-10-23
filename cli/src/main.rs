//! Binaire `cassini` : premier point d'entrée du registre de commandes (AD-13).

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
    match cassini_core::serve::serve_with_composer(&registry, &ModelComposer, input, out) {
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

/// Compositeur du canal : le modèle local, cherché par `model.list` à chaque proposition
/// (AD-15), puis `compose_plan` (AD-23). Ses messages ne portent aucun chemin absolu (AD-9).
struct ModelComposer;

impl cassini_core::serve::Composer for ModelComposer {
    fn compose(
        &self,
        registry: &Registry,
        request: &str,
        inventory: &mut dyn FnMut() -> Result<Value, ErrorReport>,
    ) -> Result<cassini_core::plan::Plan, ErrorReport> {
        // La phrase vide est refusée avant toute recherche de modèle ni demande d'inventaire.
        if request.trim().is_empty() {
            let e = cassini_ai::AiError::EmptyPrompt;
            return Err(report(e.code(), e.to_string()));
        }
        // Chemins à masquer dans les messages : le dossier désigné, puis le dossier retenu.
        const FOLDER: &str = "(dossier des modèles)";
        let mut hidden: Vec<(String, String)> =
            std::env::var(cassini_core::commands::model_list::MODELS_ENV)
                .ok()
                // Une valeur relative ou courte (« models ») n'est pas masquée : remplacée
                // partout dans le message, elle le défigurerait.
                .filter(|s| Path::new(s).is_absolute())
                .map(|s| (s, FOLDER.to_string()))
                .into_iter()
                .collect();
        let listed = registry
            .call("model.list", json!({}))
            .map_err(|e| scrub(ErrorReport::from(e), &hidden))?;
        let folder = listed["folder"].as_str().unwrap_or_default().to_string();
        if Path::new(&folder).is_absolute() {
            hidden.push((folder.clone(), FOLDER.to_string()));
        }
        let Some(file) = listed["models"][0]["file"].as_str() else {
            return Err(report(
                "ai.no_model",
                "Aucun modèle GGUF n'a été trouvé dans le dossier des modèles.".to_string(),
            ));
        };
        let entry = Path::new(&folder).join(file);
        hidden.insert(0, (entry.to_string_lossy().into_owned(), file.to_string()));
        let chosen = resolve_model(&entry).map_err(|e| scrub(e, &hidden))?;
        // Chaque fichier du modèle (base, adaptateur) se masque par son seul nom, avant le dossier.
        for pair in chosen.hidden().into_iter().rev() {
            hidden.insert(0, pair);
        }
        let inventory = inventory()?;
        let mut generator = cassini_ai::plan::model_files_generator(chosen.files.clone());
        chosen
            .compose(registry, &inventory, request, &mut generator)
            .map_err(|e| scrub(report(e.code(), e.to_string()), &hidden))
    }
}

/// Un modèle choisi : ses fichiers, et le prompt qu'il attend.
#[derive(Debug, PartialEq)]
struct ChosenModel {
    files: cassini_ai::ModelFiles,
    style: cassini_ai::plan::PromptStyle,
}

impl ChosenModel {
    /// Compose le plan de `request` dans le prompt que ce modèle attend.
    fn compose(
        &self,
        registry: &Registry,
        inventory: &Value,
        request: &str,
        generate: &mut dyn FnMut(
            &cassini_ai::plan::GenerationRequest,
        ) -> Result<cassini_ai::Completion, cassini_ai::AiError>,
    ) -> Result<cassini_core::plan::Plan, cassini_ai::plan::AgentError> {
        cassini_ai::plan::compose_plan_with(self.style, registry, inventory, request, generate)
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
    for (path, instead) in hidden.iter().filter(|(p, _)| !p.is_empty()) {
        e.message = e.message.replace(path.as_str(), instead);
    }
    e
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
        replay.run(&Router::new(&registry))
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

/// Modèle choisi quand `--model` manque : le premier de `model.list` (AD-15, le noyau seul
/// découvre les modèles).
fn default_model(registry: &Registry) -> Result<std::path::PathBuf, ErrorReport> {
    let listed = registry
        .call("model.list", json!({}))
        .map_err(ErrorReport::from)?;
    let folder = listed["folder"].as_str().unwrap_or_default();
    match listed["models"][0]["file"].as_str() {
        Some(file) => Ok(Path::new(folder).join(file)),
        None => Err(ErrorReport {
            code: "ai.no_model".to_string(),
            message: format!("Aucun modèle GGUF n'a été trouvé dans « {folder} »."),
            step: None,
        }),
    }
}

/// `cassini plan <phrase> --inventory <fichier.json> [--model <fichier.gguf|.modele.json>]` :
/// compose le plan d'une demande (AD-23) et l'affiche en JSON. `--model` prend un GGUF seul ou
/// une déclaration de modèle (base, adaptateur, prompt). Rien n'est exécuté. Une question du
/// modèle (`{"kind":"question",...}`) est un résultat : affichée sur `out`, code 0. Un plan
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
    let model = match model {
        Some(m) => std::path::PathBuf::from(m),
        None => match default_model(&registry) {
            Ok(m) => m,
            Err(e) => return fail(err, &e.code, e.message),
        },
    };
    let chosen = match resolve_model(&model) {
        Ok(c) => c,
        Err(e) => return fail(err, &e.code, e.message),
    };
    let mut generator = cassini_ai::plan::model_files_generator(chosen.files.clone());
    match chosen.compose(&registry, &inventory, request, &mut generator) {
        // Les fichiers du modèle ne sont nommés que par leur nom, comme dans le canal.
        Err(e) => {
            let e = scrub(report(e.code(), e.to_string()), &chosen.hidden());
            fail(err, &e.code, e.message)
        }
        ok => print_composed(ok, out, err),
    }
}

/// Affiche le résultat d'une composition : un plan ou une question en JSON sur `out` (code 0),
/// une erreur typée sur `err`.
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
                r#"{{"core_version":"{}","protocol_version":1}}"#,
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
                "params":{"protocol_version":1,"commands":[],"project":project}}),
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
        assert_eq!(lines[1]["result"]["protocol_version"], 1);
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
            let _ = chosen.compose(&registry, &inventory, "Ouvre les gares.", &mut |r| {
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
}
