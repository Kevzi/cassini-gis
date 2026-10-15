//! Canal local (AD-2, AD-3, AD-7) : JSON-RPC 2.0 sur l'entrée et la sortie
//! standard, un message par ligne. `session.hello` ouvre la session du projet
//! et rend le catalogue ; `command.run` exécute une commande par `Session::call`.

use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::error::ErrorReport;
use crate::journal::Session;
use crate::registry::{Registry, RegistryError, RunsIn, PROTOCOL_VERSION};

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const APPLICATION_ERROR: i64 = -32000;

/// Une erreur prête à être mise dans une réponse.
struct RpcError {
    code: i64,
    message: String,
    data: Option<Value>,
}

impl RpcError {
    fn protocol(code: i64, message: &str) -> Self {
        RpcError {
            code,
            message: message.to_string(),
            data: None,
        }
    }

    fn business(report: ErrorReport) -> Self {
        RpcError {
            code: APPLICATION_ERROR,
            message: report.message.clone(),
            data: Some(serde_json::to_value(&report).expect("une erreur se sérialise")),
        }
    }

    fn coded(code: &str, message: String) -> Self {
        RpcError::business(ErrorReport {
            code: code.to_string(),
            message,
            step: None,
        })
    }
}

/// État de la connexion : la session n'existe qu'après un `session.hello` accepté.
struct State<'a> {
    registry: &'a Registry,
    session: Option<Session<'a>>,
    announced: HashSet<String>,
}

/// Sert le canal jusqu'à la fermeture de l'entrée. Rend une erreur si la sortie
/// n'accepte plus d'écriture ou si l'entrée est illisible.
pub fn serve(
    registry: &Registry,
    mut input: impl BufRead,
    mut output: impl Write,
) -> io::Result<()> {
    let mut state = State {
        registry,
        session: None,
        announced: HashSet::new(),
    };
    let mut buf = Vec::new();
    loop {
        buf.clear();
        if input.read_until(b'\n', &mut buf)? == 0 {
            return Ok(());
        }
        let text = String::from_utf8_lossy(&buf);
        let line = text.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(response) = handle_line(&mut state, line) {
            writeln!(output, "{response}")?;
            output.flush()?;
        }
    }
}

fn error_response(id: Value, e: RpcError) -> Value {
    let mut error = Map::new();
    error.insert("code".into(), json!(e.code));
    error.insert("message".into(), json!(e.message));
    if let Some(data) = e.data {
        error.insert("data".into(), data);
    }
    json!({"jsonrpc": "2.0", "id": id, "error": error})
}

fn handle_line(state: &mut State<'_>, line: &str) -> Option<Value> {
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return Some(error_response(
            Value::Null,
            RpcError::protocol(PARSE_ERROR, "Message illisible : ce n'est pas du JSON."),
        ));
    };
    let Some(object) = message.as_object() else {
        return Some(error_response(
            Value::Null,
            RpcError::protocol(
                INVALID_REQUEST,
                "Requête invalide : un objet JSON est attendu.",
            ),
        ));
    };
    let id = object.get("id");
    let id_value = match id {
        None => None,
        Some(v) if v.is_string() || v.is_number() || v.is_null() => Some(v.clone()),
        Some(_) => {
            return Some(error_response(
                Value::Null,
                RpcError::protocol(INVALID_REQUEST, "Requête invalide : identifiant inattendu."),
            ))
        }
    };
    let method = object.get("method").and_then(Value::as_str);
    let well_formed = object.get("jsonrpc") == Some(&json!("2.0"))
        && method.is_some()
        && object.get("params").is_none_or(|p| p.is_object());
    let outcome = if well_formed {
        dispatch(
            state,
            method.expect("vérifié ci-dessus"),
            object.get("params"),
        )
    } else {
        Err(RpcError::protocol(
            INVALID_REQUEST,
            "Requête invalide : jsonrpc 2.0, method et params (objet) sont attendus.",
        ))
    };
    // Une notification bien formée n'a pas d'identifiant : aucune réponse, même en erreur.
    // Une requête malformée sans identifiant reçoit une erreur à identifiant nul (JSON-RPC 2.0).
    let id = match id_value {
        Some(id) => id,
        None if !well_formed => Value::Null,
        None => return None,
    };
    Some(match outcome {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(e) => error_response(id, e),
    })
}

fn dispatch(
    state: &mut State<'_>,
    method: &str,
    params: Option<&Value>,
) -> Result<Value, RpcError> {
    if state.session.is_none() && method != "session.hello" {
        return Err(RpcError::coded(
            "session.not_started",
            "La session n'est pas ouverte : la première requête doit être session.hello."
                .to_string(),
        ));
    }
    match method {
        "session.hello" => hello(state, params),
        "command.run" => command_run(state, params),
        _ => Err(RpcError::protocol(METHOD_NOT_FOUND, "Méthode inconnue.")),
    }
}

fn invalid_params(why: &str) -> RpcError {
    RpcError::protocol(INVALID_PARAMS, &format!("Paramètres invalides : {why}"))
}

fn hello(state: &mut State<'_>, params: Option<&Value>) -> Result<Value, RpcError> {
    if state.session.is_some() {
        return Err(RpcError::coded(
            "session.already_started",
            "La session est déjà ouverte : session.hello ne s'envoie qu'une fois.".to_string(),
        ));
    }
    let params = params
        .ok_or_else(|| invalid_params("protocol_version, commands et project sont requis."))?;
    let version = params
        .get("protocol_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_params("protocol_version doit être un entier."))?;
    let expected = u64::from(PROTOCOL_VERSION);
    if version < expected {
        return Err(RpcError::coded(
            "session.protocol_mismatch",
            format!(
                "L'extension parle le protocole {version}, le noyau le {expected} : mettez à jour l'extension Cassini."
            ),
        ));
    }
    if version > expected {
        return Err(RpcError::coded(
            "session.protocol_mismatch",
            format!(
                "L'extension parle le protocole {version}, le noyau le {expected} : mettez à jour le paquet du noyau Cassini."
            ),
        ));
    }
    let commands = params
        .get("commands")
        .and_then(Value::as_array)
        .and_then(|a| {
            a.iter()
                .map(|v| v.as_str().map(str::to_string))
                .collect::<Option<HashSet<String>>>()
        })
        .ok_or_else(|| invalid_params("commands doit être une liste d'identifiants."))?;
    let project = params
        .get("project")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("project doit être un chemin."))?;
    if !Path::new(project).is_absolute() {
        return Err(invalid_params("project doit être un chemin absolu."));
    }
    let session = Session::open(state.registry, Path::new(project))
        .map_err(|e| RpcError::business(ErrorReport::from(e)))?;
    state.session = Some(session);
    state.announced = commands;
    Ok(json!({
        "protocol_version": PROTOCOL_VERSION,
        "core_version": crate::VERSION,
        "commands": catalogue(state),
    }))
}

fn is_available(state: &State<'_>, id: &str, runs_in: Option<RunsIn>) -> bool {
    runs_in != Some(RunsIn::Qgis) || state.announced.contains(id)
}

fn catalogue(state: &State<'_>) -> Vec<Value> {
    state
        .registry
        .specs()
        .map(|spec| {
            let mut entry = serde_json::to_value(spec).expect("une description se sérialise");
            entry["available"] = json!(is_available(state, &spec.id, spec.runs_in));
            entry
        })
        .collect()
}

fn command_run(state: &mut State<'_>, params: Option<&Value>) -> Result<Value, RpcError> {
    let params = params.ok_or_else(|| invalid_params("command est requis."))?;
    let id = params
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("command doit être un identifiant."))?;
    let args = match params.get("params") {
        None => json!({}),
        Some(p) if p.is_object() => p.clone(),
        Some(_) => return Err(invalid_params("params doit être un objet.")),
    };
    if let Some(spec) = state.registry.spec(id) {
        if !is_available(state, id, spec.runs_in) {
            return Err(RpcError::business(ErrorReport::from(
                RegistryError::Unavailable(id.to_string()),
            )));
        }
    }
    let session = state.session.as_mut().expect("session vérifiée");
    session.call(id, args).map_err(RpcError::business)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::PathBuf;

    use super::*;

    fn temp_dir() -> PathBuf {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cassini-serve-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn project_in(dir: &Path) -> String {
        dir.join("p.qgz").to_string_lossy().replace('\\', "/")
    }

    fn hello_line(version: u64, commands: &[&str], project: &str) -> String {
        json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
            "params":{"protocol_version":version,"commands":commands,"project":project}})
        .to_string()
    }

    fn run_line(id: u64, command: &str, params: Value) -> String {
        json!({"jsonrpc":"2.0","id":id,"method":"command.run",
            "params":{"command":command,"params":params}})
        .to_string()
    }

    fn exchange(lines: &[String]) -> Vec<Value> {
        let registry = Registry::standard();
        let input = Cursor::new(lines.join("\n") + "\n");
        let mut out = Vec::new();
        serve(&registry, input, &mut out).unwrap();
        String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn code_of(response: &Value) -> &str {
        response["error"]["data"]["code"].as_str().unwrap()
    }

    #[test]
    fn hello_then_about_journals_one_line() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let r = exchange(&[
            hello_line(1, &[], &project),
            run_line(2, "system.about", json!({})),
        ]);
        assert_eq!(r.len(), 2);
        let commands = r[0]["result"]["commands"].as_array().unwrap();
        let about = commands.iter().find(|c| c["id"] == "system.about").unwrap();
        assert_eq!(about["available"], true);
        assert!(about["params_schema"].is_object() && about["output_schema"].is_object());
        assert_eq!(about["examples"], json!([{}]));
        assert_eq!(about["runs_in"], "core");
        let probe = commands
            .iter()
            .find(|c| c["id"] == "test.qgis_probe")
            .unwrap();
        assert_eq!(probe["available"], false);
        assert_eq!(r[1]["id"], 2);
        assert_eq!(r[1]["result"]["protocol_version"], 1);
        assert_eq!(r[1]["result"]["core_version"], crate::VERSION);
        let journal = std::fs::read_to_string(dir.join("p.cassini/journal.jsonl")).unwrap();
        assert_eq!(journal.lines().count(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_different_version_is_refused_naming_the_update() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let r = exchange(&[
            hello_line(99, &[], &project),
            run_line(2, "system.about", json!({})),
        ]);
        assert_eq!(code_of(&r[0]), "session.protocol_mismatch");
        assert!(r[0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("paquet du noyau"));
        assert_eq!(code_of(&r[1]), "session.not_started");
        assert!(!dir.join("p.cassini/journal.jsonl").exists());
        let r = exchange(&[hello_line(0, &[], &project)]);
        assert!(r[0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("l'extension"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_newer_hello_with_other_params_still_gets_the_mismatch() {
        let r = exchange(&[json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
            "params":{"protocol_version":99,"project":"p.qgz"}})
        .to_string()]);
        assert_eq!(code_of(&r[0]), "session.protocol_mismatch");
    }

    #[test]
    fn a_malformed_message_without_id_gets_a_null_id_error() {
        let r = exchange(&[json!({"foo": 1}).to_string()]);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["id"], Value::Null);
        assert_eq!(r[0]["error"]["code"], -32600);
    }

    #[test]
    fn a_call_before_hello_is_refused() {
        let r = exchange(&[run_line(1, "system.about", json!({}))]);
        assert_eq!(code_of(&r[0]), "session.not_started");
        assert_eq!(r[0]["error"]["code"], -32000);
    }

    #[test]
    fn a_second_hello_is_refused() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let r = exchange(&[hello_line(1, &[], &project), hello_line(1, &[], &project)]);
        assert!(r[0].get("result").is_some());
        assert_eq!(code_of(&r[1]), "session.already_started");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_unannounced_qgis_command_is_refused_and_not_journaled() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let r = exchange(&[
            hello_line(1, &[], &project),
            run_line(2, "test.qgis_probe", json!({"layer": "l"})),
        ]);
        assert_eq!(code_of(&r[1]), "registry.unavailable");
        assert!(!dir.join("p.cassini/journal.jsonl").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_announced_qgis_command_is_answered_by_the_stub() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let r = exchange(&[
            hello_line(1, &["test.qgis_probe"], &project),
            run_line(2, "test.qgis_probe", json!({"layer": "l"})),
        ]);
        let commands = r[0]["result"]["commands"].as_array().unwrap();
        let probe = commands
            .iter()
            .find(|c| c["id"] == "test.qgis_probe")
            .unwrap();
        assert_eq!(probe["available"], true);
        assert_eq!(code_of(&r[1]), "registry.unavailable");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_unreadable_line_gets_a_null_id_and_the_server_goes_on() {
        let r = exchange(&["pas du json".to_string(), "[]".to_string()]);
        assert_eq!(r[0]["id"], Value::Null);
        assert_eq!(r[0]["error"]["code"], -32700);
        assert_eq!(r[1]["error"]["code"], -32600);
    }

    #[test]
    fn a_relative_project_is_refused() {
        let r = exchange(&[hello_line(1, &[], "p.qgz")]);
        assert_eq!(r[0]["error"]["code"], -32602);
    }

    #[test]
    fn unknown_methods_and_notifications() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let notification = json!({"jsonrpc":"2.0","method":"command.run","params":{}}).to_string();
        let r = exchange(&[
            hello_line(1, &[], &project),
            notification,
            json!({"jsonrpc":"2.0","id":5,"method":"nope"}).to_string(),
            json!({"jsonrpc":"2.0","id":6,"method":"command.run","params":{}}).to_string(),
        ]);
        assert_eq!(r.len(), 3);
        assert_eq!(r[1]["error"]["code"], -32601);
        assert_eq!(r[2]["error"]["code"], -32602);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_domain_error_carries_the_report_and_no_absolute_path() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let r = exchange(&[
            hello_line(1, &[], &project),
            run_line(2, "system.about", json!({"x": 1})),
            run_line(3, "foo.bar", json!({})),
        ]);
        assert_eq!(code_of(&r[1]), "registry.invalid_params");
        assert_eq!(code_of(&r[2]), "registry.unknown_command");
        assert!(!r[1].to_string().contains("cassini-serve-"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn closing_the_input_frees_the_project_lock() {
        let dir = temp_dir();
        let project = project_in(&dir);
        exchange(&[hello_line(1, &[], &project)]);
        let registry = Registry::standard();
        let session = Session::open(&registry, Path::new(&project)).unwrap();
        assert_eq!(session.mode(), crate::journal::Mode::Write);
        drop(session);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_closed_output_ends_the_server_in_failure() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let registry = Registry::standard();
        let input = Cursor::new("pas du json\n");
        assert!(serve(&registry, input, Broken).is_err());
    }
}
