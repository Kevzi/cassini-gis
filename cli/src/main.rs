//! Binaire `cassini` : premier point d'entrée du registre de commandes (AD-13).

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use cassini_core::error::ErrorReport;
use cassini_core::exec::Router;
use cassini_core::journal::{Replay, Session};
use cassini_core::registry::{Registry, RegistryError};
use serde_json::{json, Value};

const USAGE: &str = "Usage : cassini run <id> [json] [--project <fichier>]
       cassini replay <projet>
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
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if args.first().map(String::as_str) == Some("replay") {
        return replay(&args[1..], out, err);
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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&args, &mut std::io::stdout(), &mut std::io::stderr());
    ExitCode::from(code as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(args: &[&str]) -> (i32, String, String) {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(&args, &mut out, &mut err);
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
}
