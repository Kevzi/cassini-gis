//! Binaire `cassini` : premier point d'entrée du registre de commandes (AD-13).

use std::io::Write;
use std::process::ExitCode;

use cassini_core::registry::{Registry, RegistryError};
use serde_json::{json, Value};

const USAGE: &str = "Usage : cassini run <id> [json]\n";

/// Exécute la CLI ; renvoie le code de sortie. Résultat sur `out`, erreurs sur `err`.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let (id, raw) = match args {
        [cmd, id] if cmd == "run" => (id, None),
        [cmd, id, raw] if cmd == "run" => (id, Some(raw)),
        _ => {
            let _ = err.write_all(USAGE.as_bytes());
            return 2;
        }
    };
    let registry = Registry::standard();
    let outcome = match raw {
        None => registry.call(id, json!({})),
        Some(text) => match serde_json::from_str::<Value>(text) {
            Ok(params) => registry.call(id, params),
            Err(e) => Err(RegistryError::InvalidParams(e.to_string())),
        },
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
                json!({"code": e.code(), "message": e.to_string()})
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
}
