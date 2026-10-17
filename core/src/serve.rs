//! Canal local (AD-2, AD-3, AD-4, AD-12) : JSON-RPC 2.0 sur l'entrée et la sortie
//! standard, un message par ligne, dans les deux sens.
//!
//! Un fil ne fait que lire l'entrée et passer les lignes ; la boucle principale
//! est seule à écrire. `session.hello` ouvre la session du projet et rend le
//! catalogue ; `command.run` exécute une commande par `Session::call`. Une
//! commande `qgis` annoncée part en ordre `order.execute` vers l'extension : en
//! attendant sa réponse, le noyau continue de lire (annulation, requêtes
//! refusées `session.busy`). `request.cancel` annule la requête en vol.
//! `plan.propose`, `plan.execute` et `plan.reject` portent les plans de l'agent
//! (voir le sous-module `plans`).

mod plans;

pub use plans::Composer;

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use crate::error::ErrorReport;
use crate::exec::{Executor, Router};
use crate::journal::{Replay, Session};
use crate::registry::{Registry, RegistryError, RunsIn, PROTOCOL_VERSION};

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const APPLICATION_ERROR: i64 = -32000;

/// Temps laissé à l'extension, après une annulation, pour répondre à l'ordre
/// avant que le noyau nettoie les sorties de l'étape.
pub const CANCEL_GRACE: Duration = Duration::from_secs(5);

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

/// Ce que le fil de lecture transmet à la boucle.
enum Msg {
    Line(String),
    Eof,
    Failed(io::Error),
}

/// Le fil de lecture : il ne fait que lire des lignes et les passer.
fn read_lines(input: &mut impl BufRead, tx: Sender<Msg>) {
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match input.read_until(b'\n', &mut buf) {
            Ok(0) => {
                let _ = tx.send(Msg::Eof);
                return;
            }
            Ok(_) => {
                let text = String::from_utf8_lossy(&buf);
                let line = text.trim();
                if !line.is_empty() && tx.send(Msg::Line(line.to_string())).is_err() {
                    return;
                }
            }
            Err(e) => {
                let _ = tx.send(Msg::Failed(e));
                return;
            }
        }
    }
}

/// Les deux bouts du canal, partagés par la boucle et l'exécutant des ordres.
struct Link<'a> {
    registry: &'a Registry,
    rx: Receiver<Msg>,
    out: RefCell<&'a mut dyn Write>,
    grace: Duration,
    next_order: Cell<u64>,
    /// Identifiant du `command.run` en cours d'exécution (pour `request.cancel`).
    run_id: RefCell<Option<Value>>,
    /// Première panne d'entrée ou de sortie rencontrée pendant un ordre.
    failure: RefCell<Option<io::Error>>,
}

impl Link<'_> {
    fn send(&self, message: &Value) -> io::Result<()> {
        let mut out = self.out.borrow_mut();
        writeln!(out, "{message}")?;
        out.flush()
    }

    /// Écrit en gardant la panne pour la boucle ; vrai si l'écriture a réussi.
    fn send_or_fail(&self, message: &Value) -> bool {
        match self.send(message) {
            Ok(()) => true,
            Err(e) => {
                self.fail(e);
                false
            }
        }
    }

    fn fail(&self, e: io::Error) {
        self.failure.borrow_mut().get_or_insert(e);
    }
}

/// État de la connexion : la session n'existe qu'après un `session.hello` accepté.
struct State<'a> {
    registry: &'a Registry,
    link: Rc<Link<'a>>,
    session: Option<Session<'a>>,
    announced: HashSet<String>,
    /// Chemin du projet de la session, tel que reçu à `session.hello`.
    project: Option<String>,
    /// Compositeur de plans, absent si le noyau n'en a pas (`agent.unavailable`).
    composer: Option<&'a dyn Composer>,
    /// Le plan proposé qui attend sa validation ; un seul à la fois.
    pending: Option<plans::Pending>,
    /// Compteur des plans proposés, pour les identifiants `plan-<n>`.
    next_plan: u64,
}

/// Sert le canal jusqu'à la fermeture de l'entrée. Rend une erreur si la sortie
/// n'accepte plus d'écriture ou si l'entrée est illisible.
pub fn serve(
    registry: &Registry,
    input: impl BufRead + Send,
    output: impl Write,
) -> io::Result<()> {
    serve_with(registry, None, input, output, CANCEL_GRACE)
}

/// Comme `serve`, avec un compositeur de plans pour `plan.propose`.
pub fn serve_with_composer(
    registry: &Registry,
    composer: &dyn Composer,
    input: impl BufRead + Send,
    output: impl Write,
) -> io::Result<()> {
    serve_with(registry, Some(composer), input, output, CANCEL_GRACE)
}

fn serve_with<'a>(
    registry: &'a Registry,
    composer: Option<&'a dyn Composer>,
    mut input: impl BufRead + Send,
    mut output: impl Write,
    grace: Duration,
) -> io::Result<()> {
    let (tx, rx) = mpsc::channel();
    thread::scope(|scope| {
        let input = &mut input;
        scope.spawn(move || read_lines(input, tx));
        let link = Rc::new(Link {
            registry,
            rx,
            out: RefCell::new(&mut output),
            grace,
            next_order: Cell::new(0),
            run_id: RefCell::new(None),
            failure: RefCell::new(None),
        });
        let mut state = State {
            registry,
            link: link.clone(),
            session: None,
            announced: HashSet::new(),
            project: None,
            composer,
            pending: None,
            next_plan: 0,
        };
        loop {
            let line = match link.rx.recv() {
                Ok(Msg::Line(line)) => line,
                Ok(Msg::Failed(e)) => return Err(e),
                // Entrée fermée (la lecture a pu être consommée pendant un ordre).
                Ok(Msg::Eof) | Err(_) => return Ok(()),
            };
            if let Some(response) = handle_line(&mut state, &line) {
                link.send(&response)?;
            }
            if let Some(e) = link.failure.borrow_mut().take() {
                return Err(e);
            }
        }
    })
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

/// Un message reçu, une fois lu.
enum Incoming {
    /// Requête (avec `id`) ou notification (sans).
    Call {
        id: Option<Value>,
        method: String,
        params: Option<Value>,
    },
    /// Réponse à un ordre du noyau : `Ok(result)` ou `Err(objet error)`.
    Reply {
        id: Value,
        outcome: Result<Value, Value>,
    },
}

/// Lit une ligne. `Err` porte la réponse d'erreur à écrire telle quelle.
fn classify(line: &str) -> Result<Incoming, Value> {
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return Err(error_response(
            Value::Null,
            RpcError::protocol(PARSE_ERROR, "Message illisible : ce n'est pas du JSON."),
        ));
    };
    let Some(object) = message.as_object() else {
        return Err(error_response(
            Value::Null,
            RpcError::protocol(
                INVALID_REQUEST,
                "Requête invalide : un objet JSON est attendu.",
            ),
        ));
    };
    let id_value = match object.get("id") {
        None => None,
        Some(v) if v.is_string() || v.is_number() || v.is_null() => Some(v.clone()),
        Some(_) => {
            return Err(error_response(
                Value::Null,
                RpcError::protocol(INVALID_REQUEST, "Requête invalide : identifiant inattendu."),
            ))
        }
    };
    let jsonrpc_ok = object.get("jsonrpc") == Some(&json!("2.0"));
    let method = object.get("method").and_then(Value::as_str);
    if method.is_none() && jsonrpc_ok {
        if let Some(id) = &id_value {
            match (object.get("result"), object.get("error")) {
                (Some(result), None) => {
                    return Ok(Incoming::Reply {
                        id: id.clone(),
                        outcome: Ok(result.clone()),
                    })
                }
                (None, Some(error)) if error.is_object() => {
                    return Ok(Incoming::Reply {
                        id: id.clone(),
                        outcome: Err(error.clone()),
                    })
                }
                _ => {}
            }
        }
    }
    let well_formed =
        jsonrpc_ok && method.is_some() && object.get("params").is_none_or(|p| p.is_object());
    if !well_formed {
        // Une requête malformée sans identifiant reçoit une erreur à identifiant nul (JSON-RPC 2.0).
        return Err(error_response(
            id_value.unwrap_or(Value::Null),
            RpcError::protocol(
                INVALID_REQUEST,
                "Requête invalide : jsonrpc 2.0, method et params (objet) sont attendus.",
            ),
        ));
    }
    Ok(Incoming::Call {
        id: id_value,
        method: method.expect("vérifié ci-dessus").to_string(),
        params: object.get("params").cloned(),
    })
}

/// Réponse à une requête ; une notification bien formée n'en reçoit aucune, même en erreur.
fn respond(id: Option<Value>, outcome: Result<Value, RpcError>) -> Option<Value> {
    let id = id?;
    Some(match outcome {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(e) => error_response(id, e),
    })
}

fn handle_line(state: &mut State<'_>, line: &str) -> Option<Value> {
    match classify(line) {
        Err(response) => Some(response),
        // Une réponse sans ordre en vol : id inconnu ou déjà soldé, ignorée.
        Ok(Incoming::Reply { .. }) => None,
        Ok(Incoming::Call { id, method, params }) => {
            // `plan.execute` comme `command.run` : `request.cancel` vise l'étape en vol.
            let is_run = method == "command.run" || method == "plan.execute";
            if is_run {
                *state.link.run_id.borrow_mut() = id.clone();
            }
            let outcome = dispatch(state, &method, params.as_ref());
            if is_run {
                *state.link.run_id.borrow_mut() = None;
            }
            respond(id, outcome)
        }
    }
}

fn dispatch(
    state: &mut State<'_>,
    method: &str,
    params: Option<&Value>,
) -> Result<Value, RpcError> {
    if method == "request.cancel" {
        // Aucun ordre en vol : rien à annuler.
        return request_cancel(&state.link, params, None);
    }
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
        "journal.replay" => journal_replay(state),
        "plan.propose" => plans::propose(state, params),
        "plan.execute" => plans::execute(state, params),
        "plan.reject" => plans::reject(state, params),
        _ => Err(RpcError::protocol(METHOD_NOT_FOUND, "Méthode inconnue.")),
    }
}

fn invalid_params(why: &str) -> RpcError {
    RpcError::protocol(INVALID_PARAMS, &format!("Paramètres invalides : {why}"))
}

/// Ordre en vol : sa commande, l'identifiant de son ordre et l'état de l'annulation.
struct InFlight<'c> {
    command: &'c str,
    order_id: &'c str,
    cancelling: &'c mut Option<Instant>,
}

/// `request.cancel` reçu comme requête ou comme notification. `in_flight` vaut
/// `None` hors d'un ordre : aucune requête n'est alors annulable.
fn request_cancel(
    link: &Link<'_>,
    params: Option<&Value>,
    in_flight: Option<InFlight<'_>>,
) -> Result<Value, RpcError> {
    let target = params
        .and_then(|p| p.get("id"))
        .filter(|v| v.is_string() || v.is_number())
        .ok_or_else(|| invalid_params("id est requis."))?;
    let unknown = || {
        RpcError::coded(
            "request.unknown",
            "Aucune requête en cours ne porte cet identifiant : rien n'est annulé.".to_string(),
        )
    };
    let Some(flight) = in_flight else {
        return Err(unknown());
    };
    if link.run_id.borrow().as_ref() != Some(target) {
        return Err(unknown());
    }
    let cancellable = link
        .registry
        .spec(flight.command)
        .is_some_and(|s| s.cancellable == Some(true));
    if !cancellable {
        return Err(RpcError::coded(
            "request.not_cancellable",
            format!(
                "La commande « {} » ne peut pas être annulée : rien n'est changé.",
                flight.command
            ),
        ));
    }
    if flight.cancelling.is_none() {
        link.send_or_fail(&cancel_notification(flight.order_id));
        *flight.cancelling = Some(Instant::now() + link.grace);
    }
    Ok(json!({"cancelled": true}))
}

fn cancel_notification(order_id: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": "request.cancel", "params": {"id": order_id}})
}

/// Exécutant des commandes `qgis` : un ordre `order.execute` vers l'extension,
/// puis l'attente de sa réponse en continuant d'écouter le canal.
struct Channel<'a> {
    link: Rc<Link<'a>>,
}

impl Channel<'_> {
    /// Une requête reçue pendant un ordre : une seule étape à la fois (AD-5).
    fn answer_during_order(
        &self,
        flight: InFlight<'_>,
        id: Option<Value>,
        method: &str,
        params: Option<&Value>,
    ) -> Option<Value> {
        let outcome = match method {
            "request.cancel" => request_cancel(&self.link, params, Some(flight)),
            "session.hello" | "command.run" | "journal.replay" | "plan.propose"
            | "plan.execute" | "plan.reject" => Err(RpcError::coded(
                "session.busy",
                "Une commande est en cours d'exécution : attendez sa fin ou annulez-la."
                    .to_string(),
            )),
            _ => Err(RpcError::protocol(METHOD_NOT_FOUND, "Méthode inconnue.")),
        };
        respond(id, outcome)
    }
}

/// L'erreur d'une réponse d'extension, avec son code et son message gardés.
fn delegated_error(error: &Value) -> RegistryError {
    let data = error.get("data");
    let code = data
        .and_then(|d| d.get("code"))
        .and_then(Value::as_str)
        .unwrap_or("order.failed")
        .to_string();
    let message = data
        .and_then(|d| d.get("message"))
        .or_else(|| error.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("L'extension a répondu à l'ordre par une erreur.")
        .to_string();
    RegistryError::Delegated { code, message }
}

impl Executor for Channel<'_> {
    fn execute(&self, id: &str, params: Value) -> Result<Value, RegistryError> {
        let link = &*self.link;
        let n = link.next_order.get() + 1;
        link.next_order.set(n);
        let order_id = format!("core-{n}");
        let order = json!({
            "jsonrpc": "2.0", "id": order_id, "method": "order.execute",
            "params": {"command": id, "params": params},
        });
        if !link.send_or_fail(&order) {
            return Err(RegistryError::Unavailable(id.to_string()));
        }
        let cancelled = || RegistryError::Cancelled(id.to_string());
        let mut cancelling: Option<Instant> = None;
        loop {
            let received = match cancelling {
                None => link.rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                Some(deadline) => link
                    .rx
                    .recv_timeout(deadline.saturating_duration_since(Instant::now())),
            };
            let line = match received {
                Ok(Msg::Line(line)) => line,
                Err(RecvTimeoutError::Timeout) => return Err(cancelled()),
                // Entrée fermée ou illisible : plus rien à attendre, l'étape est annulée.
                other => {
                    if let Ok(Msg::Failed(e)) = other {
                        link.fail(e);
                    }
                    if cancelling.is_none() {
                        link.send_or_fail(&cancel_notification(&order_id));
                    }
                    return Err(cancelled());
                }
            };
            match classify(&line) {
                Err(response) => {
                    if !link.send_or_fail(&response) {
                        return Err(cancelled());
                    }
                }
                Ok(Incoming::Reply {
                    id: reply_id,
                    outcome,
                }) => {
                    if reply_id == json!(order_id) {
                        // Une réponse arrivée après l'annulation ne change pas l'issue.
                        if cancelling.is_some() {
                            return Err(cancelled());
                        }
                        return outcome.map_err(|error| delegated_error(&error));
                    }
                }
                Ok(Incoming::Call {
                    id: call_id,
                    method,
                    params,
                }) => {
                    let flight = InFlight {
                        command: id,
                        order_id: &order_id,
                        cancelling: &mut cancelling,
                    };
                    if let Some(response) =
                        self.answer_during_order(flight, call_id, &method, params.as_ref())
                    {
                        if !link.send_or_fail(&response) {
                            return Err(cancelled());
                        }
                    }
                }
            }
        }
    }
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
        .map_err(|e| RpcError::business(ErrorReport::from(e)))?
        .with_qgis(Channel {
            link: state.link.clone(),
        });
    state.session = Some(session);
    state.project = Some(project.to_string());
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

/// Exécutant QGIS du rejeu : refuse, en la nommant, une commande que l'extension n'a pas annoncée.
struct Announced<E> {
    announced: HashSet<String>,
    inner: E,
}

impl<E: Executor> Executor for Announced<E> {
    fn execute(&self, id: &str, params: Value) -> Result<Value, RegistryError> {
        if !self.announced.contains(id) {
            return Err(RegistryError::Unavailable(id.to_string()));
        }
        self.inner.execute(id, params)
    }
}

/// `journal.replay` : le noyau rejoue le journal du projet par le même routeur et
/// le même canal que l'exécution d'origine (AD-11). Il n'est pas annulable : aucun
/// `run_id` n'est posé, donc `request.cancel` répond `request.unknown`.
fn journal_replay(state: &mut State<'_>) -> Result<Value, RpcError> {
    let project = state.project.clone().expect("session vérifiée");
    let replay = Replay::load(state.registry, Path::new(&project)).map_err(RpcError::business)?;
    let router = Router::new(state.registry).with_qgis(Announced {
        announced: state.announced.clone(),
        inner: Channel {
            link: state.link.clone(),
        },
    });
    let report = replay.run(&router).map_err(RpcError::business)?;
    Ok(serde_json::to_value(report).expect("un bilan se sérialise"))
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
    fn an_announced_qgis_command_becomes_an_order_cancelled_by_a_closed_input() {
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
        assert_eq!(r[1]["method"], "order.execute");
        assert_eq!(r[1]["params"]["command"], "test.qgis_probe");
        assert_eq!(r[2]["method"], "request.cancel");
        assert_eq!(code_of(r.last().unwrap()), "registry.cancelled");
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

    // ---- Client scripté : deux canaux `mpsc`, aucun processus ----

    use std::io::Read;
    use std::sync::mpsc;
    use std::thread::JoinHandle;
    use std::time::Duration;

    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use crate::registry::Delegated;

    struct ChanReader {
        rx: mpsc::Receiver<String>,
        buf: Vec<u8>,
        pos: usize,
    }

    impl Read for ChanReader {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let n = {
                let available = self.fill_buf()?;
                let n = available.len().min(out.len());
                out[..n].copy_from_slice(&available[..n]);
                n
            };
            self.consume(n);
            Ok(n)
        }
    }

    impl BufRead for ChanReader {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            if self.pos >= self.buf.len() {
                self.pos = 0;
                self.buf = match self.rx.recv() {
                    Ok(line) => format!("{line}\n").into_bytes(),
                    Err(_) => Vec::new(),
                };
            }
            Ok(&self.buf[self.pos..])
        }

        fn consume(&mut self, n: usize) {
            self.pos += n;
        }
    }

    struct ChanWriter {
        tx: mpsc::Sender<String>,
        buf: Vec<u8>,
    }

    impl Write for ChanWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.buf.extend_from_slice(bytes);
            while let Some(i) = self.buf.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = self.buf.drain(..=i).collect();
                let text = String::from_utf8_lossy(&line).trim().to_string();
                let _ = self.tx.send(text);
            }
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct RenderParams {
        layer: String,
        #[serde(default)]
        out: String,
    }

    #[derive(Debug, Serialize, JsonSchema)]
    struct Done {
        ok: bool,
    }

    /// Commande qgis annulable qui écrit un fichier de sortie.
    struct Render;
    impl Delegated for Render {
        type Params = RenderParams;
        type Output = Done;
        fn spec() -> crate::registry::CommandSpec {
            crate::registry::CommandSpec::new::<RenderParams, Done>(
                "test.render",
                "Rend une couche.",
                "Renders a layer.",
            )
            .runs_in(RunsIn::Qgis)
            .cancellable(true)
            .output_params(&["out"])
            .tests(&["t"])
        }
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct FixedParams {
        layer: String,
    }

    /// Commande qgis non annulable.
    struct Fixed;
    impl Delegated for Fixed {
        type Params = FixedParams;
        type Output = Done;
        fn spec() -> crate::registry::CommandSpec {
            crate::registry::CommandSpec::new::<FixedParams, Done>("test.fixed", "Fixe.", "Fixed.")
                .runs_in(RunsIn::Qgis)
                .cancellable(false)
                .tests(&["t"])
        }
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct MakeParams {
        layer: String,
    }

    #[derive(Debug, Serialize, JsonSchema)]
    struct Made {
        layer_id: String,
    }

    /// Commande qgis annulable qui rend une couche, sans fichier de sortie.
    struct Make;
    impl Delegated for Make {
        type Params = MakeParams;
        type Output = Made;
        fn spec() -> crate::registry::CommandSpec {
            crate::registry::CommandSpec::new::<MakeParams, Made>("test.make", "Fait.", "Makes.")
                .runs_in(RunsIn::Qgis)
                .cancellable(true)
                .tests(&["t"])
        }
    }

    fn scripted_registry() -> Registry {
        let mut r = Registry::standard();
        r.register_delegated::<Render>();
        r.register_delegated::<Fixed>();
        r.register_delegated::<Make>();
        r
    }

    /// Compositeur de test : demande l'inventaire s'il le doit, puis rend d'abord les plans de
    /// `first` (un par composition, dans l'ordre), ensuite son issue fixe.
    struct FixedComposer {
        inventory: bool,
        outcome: Result<Value, ErrorReport>,
        first: std::cell::RefCell<std::collections::VecDeque<Value>>,
    }

    impl Composer for FixedComposer {
        fn compose(
            &self,
            _registry: &Registry,
            _request: &str,
            inventory: &mut dyn FnMut() -> Result<Value, ErrorReport>,
        ) -> Result<crate::plan::Plan, ErrorReport> {
            if self.inventory {
                let inv = inventory()?;
                assert!(inv["layers"].is_array());
            }
            let next = self.first.borrow_mut().pop_front();
            next.map(Ok)
                .unwrap_or_else(|| self.outcome.clone())
                .map(|v| serde_json::from_value(v).expect("plan de test lisible"))
        }
    }

    fn composer(plan: Value) -> Option<FixedComposer> {
        Some(FixedComposer {
            inventory: true,
            outcome: Ok(plan),
            first: Default::default(),
        })
    }

    struct Client {
        to: Option<mpsc::Sender<String>>,
        from: mpsc::Receiver<String>,
        server: Option<JoinHandle<io::Result<()>>>,
        dir: PathBuf,
        project: String,
    }

    const WAIT: Duration = Duration::from_secs(10);

    impl Client {
        fn start(grace: Duration) -> Client {
            Client::start_with(grace, None)
        }

        /// Démarre le noyau avec un compositeur de test (qui rend un plan fixe).
        fn start_with(grace: Duration, composer: Option<FixedComposer>) -> Client {
            let dir = temp_dir();
            let project = project_in(&dir);
            let (to, in_rx) = mpsc::channel();
            let (out_tx, from) = mpsc::channel();
            let server = std::thread::spawn(move || {
                let registry = scripted_registry();
                let reader = ChanReader {
                    rx: in_rx,
                    buf: Vec::new(),
                    pos: 0,
                };
                let writer = ChanWriter {
                    tx: out_tx,
                    buf: Vec::new(),
                };
                let composer = composer.as_ref().map(|c| c as &dyn Composer);
                serve_with(&registry, composer, reader, writer, grace)
            });
            Client {
                to: Some(to),
                from,
                server: Some(server),
                dir,
                project,
            }
        }

        fn send(&self, message: Value) {
            self.to.as_ref().unwrap().send(message.to_string()).unwrap();
        }

        fn recv(&self) -> Value {
            let line = self
                .from
                .recv_timeout(WAIT)
                .expect("le noyau devait écrire");
            serde_json::from_str(&line).unwrap()
        }

        fn assert_silent(&self) {
            assert!(self
                .from
                .recv_timeout(Duration::from_millis(200))
                .is_err_and(|e| e == mpsc::RecvTimeoutError::Timeout));
        }

        /// Ouvre la session, annonce les deux commandes qgis.
        fn hello(&self) {
            self.send(json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
                "params":{"protocol_version":1,"commands":["test.render","test.fixed"],
                "project":self.project}}));
            assert!(self.recv()["result"].is_object());
        }

        /// Ouvre la session en annonçant `commands`.
        fn hello_with(&self, commands: &[&str]) {
            self.send(json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
                "params":{"protocol_version":1,"commands":commands,"project":self.project}}));
            assert!(self.recv()["result"].is_object());
        }

        /// `plan.propose` : répond à l'ordre d'inventaire, rend la réponse du noyau.
        fn propose(&self, id: u64) -> Value {
            self.send(json!({"jsonrpc":"2.0","id":id,"method":"plan.propose",
                "params":{"request":"Fais-le."}}));
            let first = self.recv();
            if first["method"] != "order.execute" {
                return first;
            }
            assert_eq!(first["params"]["command"], "project.inventory");
            self.send(json!({"jsonrpc":"2.0","id":first["id"],
                "result":{"layers":[],"project_crs":"EPSG:2154","extent":null}}));
            self.recv()
        }

        /// Attend la notification `plan.step`, puis l'ordre de l'étape ; rend l'ordre.
        fn expect_step(&self, plan_id: &str, step: u64, command: &str) -> Value {
            let notice = self.recv();
            assert_eq!(notice["method"], "plan.step", "{notice}");
            assert!(notice.get("id").is_none());
            assert_eq!(
                notice["params"],
                json!({"plan_id": plan_id, "step": step, "command": command})
            );
            let order = self.recv();
            assert_eq!(order["method"], "order.execute");
            assert_eq!(order["params"]["command"], command);
            order
        }

        fn execute(&self, id: u64, plan_id: &str) {
            self.send(json!({"jsonrpc":"2.0","id":id,"method":"plan.execute",
                "params":{"plan_id":plan_id}}));
        }

        fn run(&self, id: u64, command: &str) -> Value {
            self.send(json!({"jsonrpc":"2.0","id":id,"method":"command.run",
                "params":{"command":command,"params":{"layer":"l"}}}));
            let order = self.recv();
            assert_eq!(order["method"], "order.execute");
            assert_eq!(order["params"]["command"], command);
            order
        }

        /// Ferme l'entrée : rend ce que le noyau écrit encore, puis son issue.
        fn close(&mut self) -> (Vec<Value>, io::Result<()>) {
            self.to = None;
            let result = self.server.take().unwrap().join().unwrap();
            let rest = self
                .from
                .try_iter()
                .map(|l| serde_json::from_str(&l).unwrap())
                .collect();
            (rest, result)
        }

        fn journal(&self) -> Vec<Value> {
            std::fs::read_to_string(self.dir.join("p.cassini/journal.jsonl"))
                .unwrap_or_default()
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect()
        }
    }

    impl Drop for Client {
        fn drop(&mut self) {
            self.to = None;
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn reply_ok(id: &str) -> Value {
        json!({"jsonrpc":"2.0","id":id,"result":{"ok":true}})
    }

    #[test]
    fn an_order_is_sent_and_its_answer_becomes_the_result() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello();
        let order = c.run(2, "test.render");
        assert_eq!(order["id"], "core-1");
        assert_eq!(order["params"]["params"]["layer"], "l");
        let out = order["params"]["params"]["out"].as_str().unwrap();
        assert!(Path::new(out).is_absolute() && out.ends_with("1-test.render.out"));
        // Une réponse à un id inconnu est ignorée ; le client répond hors d'ordre.
        c.send(reply_ok("core-99"));
        c.send(json!({"jsonrpc":"2.0","id":9,"method":"nope"}));
        let other = c.recv();
        assert_eq!(other["id"], 9);
        assert_eq!(other["error"]["code"], -32601);
        c.send(reply_ok("core-1"));
        let done = c.recv();
        assert_eq!(done["id"], 2);
        assert_eq!(done["result"], json!({"ok": true}));
        let journal = c.journal();
        assert_eq!(journal.len(), 1);
        assert_eq!(journal[0]["result"], json!({"ok": true}));
        // Un second ordre porte un nouvel identifiant.
        let order = c.run(3, "test.fixed");
        assert_eq!(order["id"], "core-2");
        c.send(reply_ok("core-2"));
        assert_eq!(c.recv()["result"], json!({"ok": true}));
        let (rest, result) = c.close();
        assert!(rest.is_empty() && result.is_ok());
    }

    #[test]
    fn an_error_answer_becomes_the_error_of_the_run() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello();
        c.run(2, "test.render");
        c.send(json!({"jsonrpc":"2.0","id":"core-1","error":{"code":-32000,
            "message":"Couche absente.",
            "data":{"code":"qgis.layer_missing","message":"La couche est absente."}}}));
        let done = c.recv();
        assert_eq!(done["id"], 2);
        assert_eq!(code_of(&done), "qgis.layer_missing");
        assert_eq!(done["error"]["data"]["message"], "La couche est absente.");
        let journal = c.journal();
        assert_eq!(journal[0]["error"], "qgis.layer_missing");
        assert!(journal[0].get("cancelled").is_none());
        c.close().1.unwrap();
    }

    #[test]
    fn cancelling_a_run_in_flight_cleans_its_outputs_and_journals_it() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello();
        let order = c.run(2, "test.render");
        let out = order["params"]["params"]["out"]
            .as_str()
            .unwrap()
            .to_string();
        std::fs::write(&out, "partiel").unwrap();
        c.send(json!({"jsonrpc":"2.0","id":3,"method":"request.cancel","params":{"id":2}}));
        let first = c.recv();
        let second = c.recv();
        let (notification, response) = if first.get("method").is_some() {
            (first, second)
        } else {
            (second, first)
        };
        assert_eq!(notification["method"], "request.cancel");
        assert!(notification.get("id").is_none());
        assert_eq!(notification["params"]["id"], "core-1");
        assert_eq!(response["id"], 3);
        assert_eq!(response["result"], json!({"cancelled": true}));
        // L'extension acquitte l'annulation en répondant à l'ordre.
        c.send(json!({"jsonrpc":"2.0","id":"core-1","error":{"code":-32000,
            "message":"Annulé.","data":{"code":"qgis.cancelled","message":"Annulé."}}}));
        let done = c.recv();
        assert_eq!(done["id"], 2);
        assert_eq!(code_of(&done), "registry.cancelled");
        assert!(!Path::new(&out).exists());
        let journal = c.journal();
        assert_eq!(journal.len(), 1);
        assert_eq!(journal[0]["cancelled"], true);
        assert!(journal[0].get("result").is_none() && journal[0].get("error").is_none());
        assert_eq!(journal[0]["step"], 1);
        // La session continue : l'étape suivante est la 2.
        c.run(4, "test.fixed");
        c.send(reply_ok("core-2"));
        assert_eq!(c.recv()["result"], json!({"ok": true}));
        assert_eq!(c.journal()[1]["step"], 2);
        c.close().1.unwrap();
    }

    #[test]
    fn an_answer_that_never_comes_is_given_up_after_the_grace() {
        let grace = Duration::from_millis(150);
        let mut c = Client::start(grace);
        c.hello();
        let order = c.run(2, "test.render");
        let out = order["params"]["params"]["out"]
            .as_str()
            .unwrap()
            .to_string();
        std::fs::write(&out, "partiel").unwrap();
        let start = Instant::now();
        // Cancel en notification : aucune réponse à l'annulation elle-même.
        c.send(json!({"jsonrpc":"2.0","method":"request.cancel","params":{"id":2}}));
        let notification = c.recv();
        assert_eq!(notification["method"], "request.cancel");
        let done = c.recv();
        assert!(start.elapsed() >= grace);
        assert_eq!(done["id"], 2);
        assert_eq!(code_of(&done), "registry.cancelled");
        assert!(!Path::new(&out).exists());
        assert_eq!(c.journal()[0]["cancelled"], true);
        c.assert_silent();
        c.close().1.unwrap();
    }

    #[test]
    fn a_non_cancellable_command_and_an_unknown_id_change_nothing() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello();
        c.run(2, "test.fixed");
        c.send(json!({"jsonrpc":"2.0","id":3,"method":"request.cancel","params":{"id":2}}));
        let refused = c.recv();
        assert_eq!(refused["id"], 3);
        assert_eq!(code_of(&refused), "request.not_cancellable");
        c.send(json!({"jsonrpc":"2.0","id":4,"method":"request.cancel","params":{"id":77}}));
        assert_eq!(code_of(&c.recv()), "request.unknown");
        // L'ordre n'a pas été touché : sa réponse solde la requête.
        c.send(reply_ok("core-1"));
        let done = c.recv();
        assert_eq!(done["result"], json!({"ok": true}));
        // Requête soldée : son id est désormais inconnu.
        c.send(json!({"jsonrpc":"2.0","id":5,"method":"request.cancel","params":{"id":2}}));
        assert_eq!(code_of(&c.recv()), "request.unknown");
        assert_eq!(c.journal().len(), 1);
        assert_eq!(c.journal()[0]["result"], json!({"ok": true}));
        c.close().1.unwrap();
    }

    #[test]
    fn a_run_or_a_hello_during_an_order_is_busy_at_once() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello();
        c.run(2, "test.render");
        c.send(json!({"jsonrpc":"2.0","id":3,"method":"command.run",
            "params":{"command":"system.about","params":{}}}));
        let busy = c.recv();
        assert_eq!(busy["id"], 3);
        assert_eq!(code_of(&busy), "session.busy");
        c.send(json!({"jsonrpc":"2.0","id":4,"method":"session.hello",
            "params":{"protocol_version":1,"commands":[],"project":c.project}}));
        assert_eq!(code_of(&c.recv()), "session.busy");
        c.send(reply_ok("core-1"));
        assert_eq!(c.recv()["id"], 2);
        // Le refus n'a rien journalisé : une seule étape.
        assert_eq!(c.journal().len(), 1);
        c.close().1.unwrap();
    }

    #[test]
    fn a_closed_input_during_an_order_cancels_the_step_and_frees_the_lock() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello();
        let order = c.run(2, "test.render");
        let out = order["params"]["params"]["out"]
            .as_str()
            .unwrap()
            .to_string();
        std::fs::write(&out, "partiel").unwrap();
        // Le client ne répond ni ne lit plus rien : il ferme l'entrée.
        let (rest, result) = c.close();
        assert!(result.is_ok());
        assert!(rest.iter().any(|m| m["method"] == "request.cancel"));
        let done = rest.last().unwrap();
        assert_eq!(done["id"], 2);
        assert_eq!(code_of(done), "registry.cancelled");
        assert!(!Path::new(&out).exists());
        assert_eq!(c.journal()[0]["cancelled"], true);
        let registry = Registry::standard();
        let session = Session::open(&registry, Path::new(&c.project)).unwrap();
        assert_eq!(session.mode(), crate::journal::Mode::Write);
    }

    #[test]
    fn a_cancelled_step_is_skipped_by_the_replay() {
        let mut c = Client::start(Duration::from_millis(50));
        c.hello();
        c.run(2, "test.render");
        c.send(json!({"jsonrpc":"2.0","method":"request.cancel","params":{"id":2}}));
        c.recv();
        assert_eq!(code_of(&c.recv()), "registry.cancelled");
        c.send(json!({"jsonrpc":"2.0","id":3,"method":"command.run",
            "params":{"command":"system.about","params":{}}}));
        assert_eq!(c.recv()["result"]["protocol_version"], 1);
        c.close().1.unwrap();
        let registry = scripted_registry();
        let replay = crate::journal::Replay::load(&registry, Path::new(&c.project)).unwrap();
        let report = replay.run(&crate::exec::Router::new(&registry)).unwrap();
        assert_eq!(report.steps, 1);
        assert!(report.identical);
    }

    #[test]
    fn journal_replay_runs_the_qgis_steps_through_the_channel() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello();
        c.run(2, "test.fixed");
        c.send(reply_ok("core-1"));
        assert_eq!(c.recv()["result"], json!({"ok": true}));
        c.send(json!({"jsonrpc":"2.0","id":3,"method":"journal.replay"}));
        let order = c.recv();
        assert_eq!(order["method"], "order.execute");
        assert_eq!(order["id"], "core-2");
        assert_eq!(order["params"]["command"], "test.fixed");
        // Pendant le rejeu : une autre requête est refusée, et rien n'est annulable.
        for (id, method) in [
            (4, "command.run"),
            (5, "journal.replay"),
            (6, "session.hello"),
        ] {
            c.send(json!({"jsonrpc":"2.0","id":id,"method":method,
                "params":{"command":"system.about","params":{}}}));
            assert_eq!(code_of(&c.recv()), "session.busy");
        }
        c.send(json!({"jsonrpc":"2.0","id":7,"method":"request.cancel","params":{"id":3}}));
        assert_eq!(code_of(&c.recv()), "request.unknown");
        c.send(reply_ok("core-2"));
        let done = c.recv();
        assert_eq!(done["id"], 3);
        assert_eq!(done["result"], json!({"steps": 1, "identical": true}));
        // Le rejeu n'écrit pas le journal.
        assert_eq!(c.journal().len(), 1);
        c.close().1.unwrap();
    }

    #[test]
    fn journal_replay_stops_at_the_first_difference() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello();
        c.run(2, "test.fixed");
        c.send(reply_ok("core-1"));
        c.recv();
        c.send(json!({"jsonrpc":"2.0","id":3,"method":"journal.replay"}));
        c.recv();
        c.send(json!({"jsonrpc":"2.0","id":"core-2","result":{"ok":false}}));
        let done = c.recv();
        assert_eq!(code_of(&done), "replay.diverged");
        assert_eq!(done["error"]["data"]["step"], 1);
        c.close().1.unwrap();
    }

    #[test]
    fn journal_replay_without_the_extension_names_the_command() {
        let mut c = Client::start(CANCEL_GRACE);
        let work = c.dir.join("p.cassini");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(
            work.join("journal.jsonl"),
            json!({"step":1,"id":"test.fixed","params":{"layer":"l"},"inputs":[],"result":{"ok":true}})
                .to_string()
                + "
",
        )
        .unwrap();
        c.send(json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
            "params":{"protocol_version":1,"commands":[],"project":c.project}}));
        assert!(c.recv()["result"].is_object());
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"journal.replay"}));
        let done = c.recv();
        assert_eq!(code_of(&done), "registry.unavailable");
        assert!(done["error"]["data"]["message"]
            .as_str()
            .unwrap()
            .contains("test.fixed"));
        c.assert_silent();
        c.close().1.unwrap();
    }

    #[test]
    fn journal_replay_before_hello_is_refused() {
        let r = exchange(&[json!({"jsonrpc":"2.0","id":1,"method":"journal.replay"}).to_string()]);
        assert_eq!(code_of(&r[0]), "session.not_started");
    }

    // ---- Plans : plan.propose, plan.execute, plan.reject ----

    const ALL: [&str; 4] = [
        "test.make",
        "test.render",
        "test.fixed",
        "project.inventory",
    ];

    fn three_steps() -> Value {
        json!({"kind":"calls","calls":[
            {"command":"test.make","params":{"layer":"src"}},
            {"command":"test.make","params":{"layer":"step:1.layer_id"}},
            {"command":"test.fixed","params":{"layer":"step:2.layer_id"}}
        ]})
    }

    fn reply(id: &Value, result: Value) -> Value {
        json!({"jsonrpc":"2.0","id":id,"result":result})
    }

    #[test]
    fn a_three_step_plan_runs_one_order_at_a_time_and_replays_identically() {
        let mut c = Client::start_with(CANCEL_GRACE, composer(three_steps()));
        c.hello_with(&ALL);
        let proposed = c.propose(2);
        assert_eq!(proposed["id"], 2);
        assert_eq!(proposed["result"]["plan_id"], "plan-1");
        assert_eq!(proposed["result"]["plan"], three_steps());
        // L'inventaire demandé pour composer n'est pas journalisé.
        assert!(c.journal().is_empty());
        c.execute(3, "plan-1");
        let order = c.expect_step("plan-1", 1, "test.make");
        assert_eq!(order["params"]["params"], json!({"layer": "src"}));
        // Rien ne part avant la réponse à l'ordre en cours.
        c.assert_silent();
        c.send(reply(&order["id"], json!({"layer_id": "l_1"})));
        let order = c.expect_step("plan-1", 2, "test.make");
        assert_eq!(order["params"]["params"], json!({"layer": "l_1"}));
        c.assert_silent();
        c.send(reply(&order["id"], json!({"layer_id": "l_2"})));
        let order = c.expect_step("plan-1", 3, "test.fixed");
        assert_eq!(order["params"]["params"], json!({"layer": "l_2"}));
        c.send(reply(&order["id"], json!({"ok": true})));
        let done = c.recv();
        assert_eq!(done["id"], 3);
        assert_eq!(
            done["result"],
            json!({"plan_id": "plan-1", "steps": [
                {"step": 1, "command": "test.make", "result": {"layer_id": "l_1"}},
                {"step": 2, "command": "test.make", "result": {"layer_id": "l_2"}},
                {"step": 3, "command": "test.fixed", "result": {"ok": true}}
            ]})
        );
        // Le journal garde les valeurs résolues, pas `step:k`.
        let journal = c.journal();
        assert_eq!(journal.len(), 3);
        assert_eq!(journal[1]["params"], json!({"layer": "l_1"}));
        assert_eq!(journal[2]["params"], json!({"layer": "l_2"}));
        // Un plan exécuté est consommé : rien n'est envoyé.
        c.execute(4, "plan-1");
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        c.assert_silent();
        // Le rejeu lie les nouveaux `layer_id` et redonne les mêmes résultats.
        c.send(json!({"jsonrpc":"2.0","id":5,"method":"journal.replay"}));
        let order = c.recv();
        assert_eq!(order["params"]["params"], json!({"layer": "src"}));
        c.send(reply(&order["id"], json!({"layer_id": "r_1"})));
        let order = c.recv();
        assert_eq!(order["params"]["params"], json!({"layer": "r_1"}));
        c.send(reply(&order["id"], json!({"layer_id": "r_2"})));
        let order = c.recv();
        assert_eq!(order["params"]["params"], json!({"layer": "r_2"}));
        c.send(reply(&order["id"], json!({"ok": true})));
        let replayed = c.recv();
        assert_eq!(replayed["result"], json!({"steps": 3, "identical": true}));
        c.close().1.unwrap();
    }

    #[test]
    fn a_failing_step_stops_the_plan_with_its_step_number() {
        let mut c = Client::start_with(CANCEL_GRACE, composer(three_steps()));
        c.hello_with(&ALL);
        c.propose(2);
        c.execute(3, "plan-1");
        let order = c.expect_step("plan-1", 1, "test.make");
        c.send(reply(&order["id"], json!({"layer_id": "l_1"})));
        let order = c.expect_step("plan-1", 2, "test.make");
        c.send(
            json!({"jsonrpc":"2.0","id":order["id"],"error":{"code":-32000,
            "message":"Couche absente.",
            "data":{"code":"qgis.layer_missing","message":"La couche est absente."}}}),
        );
        let done = c.recv();
        assert_eq!(done["id"], 3);
        assert_eq!(code_of(&done), "qgis.layer_missing");
        assert_eq!(done["error"]["data"]["step"], 2);
        // L'étape 3 ne part jamais.
        c.assert_silent();
        let journal = c.journal();
        assert_eq!(journal.len(), 2);
        assert_eq!(journal[1]["error"], "qgis.layer_missing");
        c.close().1.unwrap();
    }

    #[test]
    fn cancelling_a_plan_stops_at_the_step_in_flight() {
        let plan = json!({"kind":"calls","calls":[
            {"command":"test.make","params":{"layer":"src"}},
            {"command":"test.render","params":{"layer":"step:1.layer_id"}},
            {"command":"test.fixed","params":{"layer":"x"}}
        ]});
        let mut c = Client::start_with(CANCEL_GRACE, composer(plan));
        c.hello_with(&ALL);
        c.propose(2);
        c.execute(3, "plan-1");
        let order = c.expect_step("plan-1", 1, "test.make");
        c.send(reply(&order["id"], json!({"layer_id": "l_1"})));
        let order = c.expect_step("plan-1", 2, "test.render");
        let out = order["params"]["params"]["out"]
            .as_str()
            .unwrap()
            .to_string();
        std::fs::write(&out, "partiel").unwrap();
        c.send(json!({"jsonrpc":"2.0","id":4,"method":"request.cancel","params":{"id":3}}));
        let mut answers = [c.recv(), c.recv()];
        answers.sort_by_key(|m| m.get("method").is_none());
        assert_eq!(answers[0]["method"], "request.cancel");
        assert_eq!(answers[0]["params"]["id"], order["id"]);
        assert_eq!(answers[1]["result"], json!({"cancelled": true}));
        c.send(
            json!({"jsonrpc":"2.0","id":order["id"],"error":{"code":-32000,
            "message":"Annulé.","data":{"code":"qgis.cancelled","message":"Annulé."}}}),
        );
        let done = c.recv();
        assert_eq!(done["id"], 3);
        assert_eq!(code_of(&done), "registry.cancelled");
        assert_eq!(done["error"]["data"]["step"], 2);
        assert!(!Path::new(&out).exists());
        c.assert_silent();
        let journal = c.journal();
        assert_eq!(journal.len(), 2);
        assert_eq!(journal[1]["cancelled"], true);
        c.close().1.unwrap();
    }

    #[test]
    fn a_non_cancellable_step_of_a_plan_refuses_the_cancel() {
        let plan =
            json!({"kind":"calls","calls":[{"command":"test.fixed","params":{"layer":"x"}}]});
        let mut c = Client::start_with(CANCEL_GRACE, composer(plan));
        c.hello_with(&ALL);
        c.propose(2);
        c.execute(3, "plan-1");
        let order = c.expect_step("plan-1", 1, "test.fixed");
        c.send(json!({"jsonrpc":"2.0","id":4,"method":"request.cancel","params":{"id":3}}));
        assert_eq!(code_of(&c.recv()), "request.not_cancellable");
        c.send(reply(&order["id"], json!({"ok": true})));
        assert_eq!(
            c.recv()["result"]["steps"][0]["result"],
            json!({"ok": true})
        );
        c.close().1.unwrap();
    }

    #[test]
    fn an_unknown_executed_or_rejected_plan_id_runs_nothing() {
        let plan =
            json!({"kind":"calls","calls":[{"command":"test.fixed","params":{"layer":"x"}}]});
        let mut c = Client::start_with(CANCEL_GRACE, composer(plan));
        c.hello_with(&ALL);
        c.execute(2, "plan-9");
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        assert_eq!(c.propose(3)["result"]["plan_id"], "plan-1");
        // Un identifiant faux ne consomme pas le plan en attente.
        c.send(
            json!({"jsonrpc":"2.0","id":4,"method":"plan.reject","params":{"plan_id":"plan-7"}}),
        );
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        c.send(
            json!({"jsonrpc":"2.0","id":5,"method":"plan.reject","params":{"plan_id":"plan-1"}}),
        );
        assert_eq!(
            c.recv()["result"],
            json!({"plan_id": "plan-1", "rejected": true})
        );
        c.execute(6, "plan-1");
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        c.send(
            json!({"jsonrpc":"2.0","id":7,"method":"plan.reject","params":{"plan_id":"plan-1"}}),
        );
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        // Une nouvelle proposition remplace l'ancienne.
        assert_eq!(c.propose(8)["result"]["plan_id"], "plan-2");
        assert_eq!(c.propose(9)["result"]["plan_id"], "plan-3");
        c.execute(10, "plan-2");
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        c.send(json!({"jsonrpc":"2.0","id":11,"method":"plan.execute","params":{}}));
        assert_eq!(c.recv()["error"]["code"], -32602);
        c.assert_silent();
        assert!(c.journal().is_empty());
        c.close().1.unwrap();
    }

    #[test]
    fn without_a_composer_propose_is_unavailable_and_the_channel_still_works() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello_with(&ALL);
        let r = c.propose(2);
        assert_eq!(code_of(&r), "agent.unavailable");
        assert!(!r.to_string().contains(&*c.dir.to_string_lossy()));
        c.execute(3, "plan-1");
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        c.send(json!({"jsonrpc":"2.0","id":4,"method":"command.run",
            "params":{"command":"system.about","params":{}}}));
        assert_eq!(c.recv()["result"]["protocol_version"], 1);
        c.close().1.unwrap();
    }

    #[test]
    fn a_composer_error_is_passed_on_and_nothing_is_kept() {
        for (code, inventory) in [("ai.no_model", false), ("agent.plan_invalid", true)] {
            let failing = FixedComposer {
                inventory,
                outcome: Err(ErrorReport {
                    code: code.to_string(),
                    message: "Raté.".to_string(),
                    step: None,
                }),
                first: Default::default(),
            };
            let mut c = Client::start_with(CANCEL_GRACE, Some(failing));
            c.hello_with(&ALL);
            assert_eq!(code_of(&c.propose(2)), code);
            c.execute(3, "plan-1");
            assert_eq!(code_of(&c.recv()), "plan.unknown");
            c.close().1.unwrap();
        }
    }

    #[test]
    fn a_refusal_or_an_invalid_plan_is_refused_and_not_kept() {
        let cases = [
            (
                json!({"kind":"refusal","reason":"Hors du catalogue."}),
                "agent.refused",
                None,
            ),
            (
                json!({"kind":"calls","calls":[{"command":"test.fixed","params":{"layer":"x"}},
                    {"command":"no.such","params":{}}]}),
                "agent.plan_invalid",
                Some(2),
            ),
            (
                json!({"kind":"calls","calls":[{"command":"test.fixed","params":{"layer":"step:1"}}]}),
                "agent.plan_invalid",
                Some(1),
            ),
        ];
        for (plan, code, step) in cases {
            let mut c = Client::start_with(CANCEL_GRACE, composer(plan));
            c.hello_with(&ALL);
            let r = c.propose(2);
            assert_eq!(code_of(&r), code, "{r}");
            assert_eq!(r["error"]["data"].get("step").and_then(Value::as_u64), step);
            c.execute(3, "plan-1");
            assert_eq!(code_of(&c.recv()), "plan.unknown");
            c.assert_silent();
            c.close().1.unwrap();
        }
    }

    #[test]
    fn a_question_is_a_result_without_plan_id_and_is_never_executed() {
        let question = "Aucune couche de pharmacies dans le projet : laquelle faut-il utiliser ?";
        let plan = json!({"kind":"question","question":question});
        let mut c = Client::start_with(CANCEL_GRACE, composer(plan));
        c.hello_with(&ALL);
        let r = c.propose(2);
        assert!(r.get("error").is_none(), "{r}");
        assert_eq!(r["result"], json!({"question": question}));
        assert!(r["result"].get("plan_id").is_none());
        c.execute(3, "plan-1");
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        c.assert_silent();
        assert!(c.journal().is_empty());
        c.close().1.unwrap();
    }

    #[test]
    fn a_question_erases_the_plan_in_waiting() {
        let question = json!({"kind":"question","question":"Quelle couche utiliser ?"});
        let plan =
            json!({"kind":"calls","calls":[{"command":"test.fixed","params":{"layer":"x"}}]});
        let mut composer = composer(question).unwrap();
        composer.first.get_mut().push_back(plan);
        let mut c = Client::start_with(CANCEL_GRACE, Some(composer));
        c.hello_with(&ALL);
        assert_eq!(c.propose(2)["result"]["plan_id"], "plan-1");
        let r = c.propose(3);
        assert_eq!(r["result"], json!({"question": "Quelle couche utiliser ?"}));
        c.execute(4, "plan-1");
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        c.assert_silent();
        assert!(c.journal().is_empty());
        c.close().1.unwrap();
    }

    #[test]
    fn a_plan_naming_an_unannounced_command_is_refused_naming_the_step() {
        let plan = json!({"kind":"calls","calls":[
            {"command":"test.fixed","params":{"layer":"x"}},
            {"command":"test.render","params":{"layer":"x"}}
        ]});
        let mut c = Client::start_with(CANCEL_GRACE, composer(plan));
        c.hello_with(&["test.fixed", "project.inventory"]);
        let r = c.propose(2);
        assert_eq!(code_of(&r), "agent.plan_invalid");
        assert_eq!(r["error"]["data"]["step"], 2);
        assert!(r["error"]["data"]["message"]
            .as_str()
            .unwrap()
            .contains("test.render"));
        c.execute(3, "plan-1");
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        c.close().1.unwrap();
    }

    #[test]
    fn an_unannounced_inventory_is_the_error_of_the_command() {
        let mut c = Client::start_with(CANCEL_GRACE, composer(three_steps()));
        c.hello_with(&["test.make", "test.fixed"]);
        let r = c.propose(2);
        assert_eq!(code_of(&r), "registry.unavailable");
        assert!(r["error"]["data"]["message"]
            .as_str()
            .unwrap()
            .contains("project.inventory"));
        c.assert_silent();
        c.close().1.unwrap();
    }

    #[test]
    fn requests_during_a_plan_step_are_busy() {
        let mut c = Client::start_with(CANCEL_GRACE, composer(three_steps()));
        c.hello_with(&ALL);
        c.propose(2);
        c.execute(3, "plan-1");
        let order = c.expect_step("plan-1", 1, "test.make");
        for (id, method) in [
            (4, "plan.propose"),
            (5, "plan.execute"),
            (6, "plan.reject"),
            (7, "command.run"),
            (8, "session.hello"),
            (9, "journal.replay"),
        ] {
            c.send(json!({"jsonrpc":"2.0","id":id,"method":method,
                "params":{"request":"x","plan_id":"plan-1"}}));
            let busy = c.recv();
            assert_eq!(busy["id"], id);
            assert_eq!(code_of(&busy), "session.busy", "{method}");
        }
        // Les refus n'ont rien changé : le plan va à son terme.
        c.send(reply(&order["id"], json!({"layer_id": "l_1"})));
        let order = c.expect_step("plan-1", 2, "test.make");
        c.send(reply(&order["id"], json!({"layer_id": "l_2"})));
        let order = c.expect_step("plan-1", 3, "test.fixed");
        c.send(reply(&order["id"], json!({"ok": true})));
        assert_eq!(c.recv()["id"], 3);
        assert_eq!(c.journal().len(), 3);
        c.close().1.unwrap();
    }

    #[test]
    fn requests_during_the_inventory_of_a_proposal_are_busy() {
        let mut c = Client::start_with(CANCEL_GRACE, composer(three_steps()));
        c.hello_with(&ALL);
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"plan.propose","params":{"request":"x"}}));
        let order = c.recv();
        assert_eq!(order["params"]["command"], "project.inventory");
        c.send(
            json!({"jsonrpc":"2.0","id":3,"method":"plan.execute","params":{"plan_id":"plan-1"}}),
        );
        assert_eq!(code_of(&c.recv()), "session.busy");
        // La composition n'est pas annulable.
        c.send(json!({"jsonrpc":"2.0","id":4,"method":"request.cancel","params":{"id":2}}));
        assert_eq!(code_of(&c.recv()), "request.unknown");
        c.send(reply(&order["id"], json!({"layers": []})));
        assert_eq!(c.recv()["result"]["plan_id"], "plan-1");
        c.close().1.unwrap();
    }

    #[test]
    fn plan_methods_before_hello_are_refused() {
        for method in ["plan.propose", "plan.execute", "plan.reject"] {
            let r = exchange(&[json!({"jsonrpc":"2.0","id":1,"method":method,
                "params":{"request":"x","plan_id":"plan-1"}})
            .to_string()]);
            assert_eq!(code_of(&r[0]), "session.not_started");
        }
    }

    #[test]
    fn a_plan_result_makes_an_output_path_relative_to_the_work_folder() {
        let plan =
            json!({"kind":"calls","calls":[{"command":"test.fixed","params":{"layer":"x"}}]});
        let mut c = Client::start_with(CANCEL_GRACE, composer(plan));
        c.hello_with(&ALL);
        c.propose(2);
        c.execute(3, "plan-1");
        let order = c.expect_step("plan-1", 1, "test.fixed");
        let work = c.dir.join("p.cassini");
        let file = work.join("1-carte.pdf").to_string_lossy().into_owned();
        c.send(reply(&order["id"], json!({"ok": true, "file": file})));
        let done = c.recv();
        assert_eq!(
            done["result"]["steps"][0]["result"],
            json!({"ok": true, "file": "1-carte.pdf"})
        );
        let text = done.to_string();
        let work_text = work.to_string_lossy();
        assert!(!text.contains(&*work_text) && !text.contains(&work_text.replace('\\', "/")));
        c.close().1.unwrap();
    }

    #[test]
    fn a_failed_proposal_still_erases_the_pending_plan() {
        let plan =
            json!({"kind":"calls","calls":[{"command":"test.fixed","params":{"layer":"x"}}]});
        let mut c = Client::start_with(CANCEL_GRACE, composer(plan));
        c.hello_with(&ALL);
        assert_eq!(c.propose(2)["result"]["plan_id"], "plan-1");
        // Proposition sans `request` : refusée, mais l'ancien plan est effacé.
        c.send(json!({"jsonrpc":"2.0","id":3,"method":"plan.propose","params":{}}));
        assert_eq!(c.recv()["error"]["code"], -32602);
        c.execute(4, "plan-1");
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        c.assert_silent();
        assert!(c.journal().is_empty());
        c.close().1.unwrap();
    }
}
