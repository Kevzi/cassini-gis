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
//! (voir le sous-module `plans`) ; `agent.models` rend les modèles que l'agent peut prendre.
//! `session.external` ouvre ou ferme le canal d'IA externe (AD-29, sous-module `external`) :
//! les fils lecteurs de ses relais versent dans la même file que l'entrée les seuls appels qui
//! exigent la boucle, exécutés par le sous-module `calls`. Un seul état d'occupation vaut pour
//! l'extension et les relais : qui trouve l'exécutant pris reçoit `session.busy` aussitôt.

mod calls;
mod external;
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

use crate::commands::layer_open_raster::OPEN_RASTER;
use crate::commands::layer_open_vector::OPEN_VECTOR;
use crate::error::ErrorReport;
use crate::exec::{Executor, Router};
pub use crate::external::ChannelName;
use crate::journal::{JournalError, Mode, Replay, Session};
use crate::registry::{Registry, RegistryError, RunsIn, PROTOCOL_VERSION};

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const APPLICATION_ERROR: i64 = -32000;

/// Temps laissé à l'extension, après une annulation, pour répondre à l'ordre
/// avant que le noyau nettoie les sorties de l'étape.
pub const CANCEL_GRACE: Duration = Duration::from_secs(5);

/// Temps laissé à l'utilisateur pour accorder ou refuser l'appel d'un relais, quand la case « Me
/// demander avant chaque appel » est cochée (AD-29) : au-delà, l'appel n'est pas exécuté.
pub const APPROVE_TIMEOUT: Duration = Duration::from_secs(120);

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

/// D'où vient une ligne : l'extension (entrée standard) ou un relais du canal d'IA externe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Extension,
    External(u64),
}

/// Ce que les fils de lecture transmettent à la boucle, par une file unique.
enum Msg {
    /// Une ligne de l'extension (entrée standard).
    Line(String),
    /// Entrée standard fermée.
    Eof,
    /// Entrée standard illisible.
    Failed(io::Error),
    /// Un relais s'est débranché.
    Detached(u64),
    /// Un appel d'un relais qui exige la boucle, versé par son fil lecteur.
    Relay(u64, RelayCall),
}

/// Ce qu'un fil lecteur de relais verse dans la file : le reste, il y répond lui-même.
enum RelayCall {
    /// Un `command.run` dont le fil lecteur a réservé l'occupation : la boucle doit la libérer.
    Run { id: Value, params: Option<Value> },
    /// Un `request.cancel`, requête ou notification.
    Cancel {
        id: Option<Value>,
        params: Option<Value>,
    },
}

/// L'appel en cours, que `request.cancel` peut viser : un `command.run` ou un `plan.execute` de
/// l'extension (par son `id`), ou un appel d'un relais (par son `id` depuis sa connexion, par son
/// `call_id` depuis l'extension).
struct Current {
    source: Source,
    id: Option<Value>,
    call_id: Option<String>,
}

impl Current {
    /// `target`, reçu de `requester`, désigne-t-il cet appel ?
    fn answers(&self, requester: Source, target: &Value) -> bool {
        match (requester, self.source) {
            (Source::Extension, Source::Extension) => self.id.as_ref() == Some(target),
            (Source::Extension, Source::External(_)) => {
                self.call_id.is_some() && target.as_str() == self.call_id.as_deref()
            }
            (Source::External(a), Source::External(b)) => {
                a == b && self.id.as_ref() == Some(target)
            }
            (Source::External(_), Source::Extension) => false,
        }
    }
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
    /// Délai de la demande d'accord `external.approve` (`APPROVE_TIMEOUT` hors des tests).
    approve_timeout: Duration,
    next_order: Cell<u64>,
    /// L'appel en cours d'exécution, et sa source (pour `request.cancel`).
    current: RefCell<Option<Current>>,
    /// Première panne d'entrée ou de sortie rencontrée pendant un ordre.
    failure: RefCell<Option<io::Error>>,
    /// L'entrée standard s'est fermée (la fin a pu être lue pendant un ordre). La file ne se
    /// ferme pas d'elle-même : le canal d'IA externe en tient une entrée.
    input_closed: Cell<bool>,
    /// Le canal d'IA externe et ses relais.
    external: external::Hub,
}

impl Link<'_> {
    /// Le relais de l'appel en cours a-t-il disparu (débranché, ou coupé par l'interrupteur) ?
    fn caller_lost(&self) -> bool {
        match self.current.borrow().as_ref().map(|c| c.source) {
            Some(Source::External(conn)) => !self.external.has_conn(conn),
            _ => false,
        }
    }

    /// Le `request.cancel` d'un relais : la réponse lui revient, par sa connexion.
    fn relay_cancel(
        &self,
        conn: u64,
        id: Option<Value>,
        params: Option<&Value>,
        in_flight: Option<InFlight<'_>>,
    ) {
        let outcome = request_cancel(self, Source::External(conn), params, in_flight);
        if let Some(response) = respond(id, outcome) {
            self.external.send(conn, &response);
        }
    }

    fn send(&self, message: &Value) -> io::Result<()> {
        let mut out = self.out.borrow_mut();
        out.write_all(&line_of(message))?;
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
    /// Formes de `StyleSpec` annoncées à `session.hello` (`style_kinds`), pour le rejeu.
    style_kinds: HashSet<String>,
    /// Chemin du projet de la session, tel que reçu à `session.hello`.
    project: Option<String>,
    /// Compositeur de plans, absent si le noyau n'en a pas (`agent.unavailable`).
    composer: Option<&'a dyn Composer>,
    /// Le plan proposé qui attend sa validation ; un seul à la fois.
    pending: Option<plans::Pending>,
    /// Compteur des plans proposés, pour les identifiants `plan-<n>`.
    next_plan: u64,
    /// Compteur des appels des relais, pour les identifiants `external-<n>`.
    next_call: u64,
    /// Ids des couches posées par les `journal.replay` de ce processus (ticket 5.36) : un rejeu
    /// suivant les laisse hors de sa carte. En mémoire seulement, perdue à la relance du noyau.
    replayed_layers: HashSet<String>,
}

/// Sert le canal jusqu'à la fermeture de l'entrée. Rend une erreur si la sortie
/// n'accepte plus d'écriture ou si l'entrée est illisible.
pub fn serve(
    registry: &Registry,
    input: impl BufRead + Send,
    output: impl Write,
) -> io::Result<()> {
    serve_with(
        registry,
        None,
        None,
        input,
        output,
        CANCEL_GRACE,
        APPROVE_TIMEOUT,
    )
}

/// Comme `serve`, avec un compositeur de plans pour `plan.propose`.
pub fn serve_with_composer(
    registry: &Registry,
    composer: &dyn Composer,
    input: impl BufRead + Send,
    output: impl Write,
) -> io::Result<()> {
    serve_with(
        registry,
        Some(composer),
        None,
        input,
        output,
        CANCEL_GRACE,
        APPROVE_TIMEOUT,
    )
}

/// Comme `serve_with_composer`, avec un nom de canal d'IA externe imposé (tests : un nom par
/// test, voir `ChannelName::tagged`) et le délai de la demande d'accord `external.approve`
/// (`APPROVE_TIMEOUT` hors des tests, plus court pour les tester).
pub fn serve_with_channel(
    registry: &Registry,
    composer: Option<&dyn Composer>,
    channel: ChannelName,
    approve_timeout: Duration,
    input: impl BufRead + Send,
    output: impl Write,
) -> io::Result<()> {
    serve_with(
        registry,
        composer,
        Some(channel),
        input,
        output,
        CANCEL_GRACE,
        approve_timeout,
    )
}

fn serve_with<'a>(
    registry: &'a Registry,
    composer: Option<&'a dyn Composer>,
    channel: Option<ChannelName>,
    mut input: impl BufRead + Send,
    mut output: impl Write,
    grace: Duration,
    approve_timeout: Duration,
) -> io::Result<()> {
    let (tx, rx) = mpsc::channel();
    let hub = external::Hub::new(channel, tx.clone());
    thread::scope(|scope| {
        let input = &mut input;
        scope.spawn(move || read_lines(input, tx));
        let link = Rc::new(Link {
            registry,
            rx,
            out: RefCell::new(&mut output),
            grace,
            approve_timeout,
            next_order: Cell::new(0),
            current: RefCell::new(None),
            failure: RefCell::new(None),
            input_closed: Cell::new(false),
            external: hub,
        });
        let mut state = State {
            registry,
            link: link.clone(),
            session: None,
            announced: HashSet::new(),
            style_kinds: HashSet::new(),
            project: None,
            composer,
            pending: None,
            next_plan: 0,
            next_call: 0,
            replayed_layers: HashSet::new(),
        };
        let outcome = loop {
            // Entrée fermée pendant un ordre : la fin a déjà été lue.
            if link.input_closed.get() {
                break Ok(());
            }
            let msg = match link.rx.recv() {
                Ok(msg) => msg,
                Err(_) => break Ok(()),
            };
            let line = match msg {
                Msg::Line(line) => line,
                Msg::Failed(e) => break Err(e),
                Msg::Eof => break Ok(()),
                Msg::Detached(conn) => {
                    link.external.detached(conn);
                    continue;
                }
                // Aucun appel en vol : rien à annuler.
                Msg::Relay(conn, RelayCall::Cancel { id, params }) => {
                    link.relay_cancel(conn, id, params.as_ref(), None);
                    continue;
                }
                Msg::Relay(conn, RelayCall::Run { id, params }) => {
                    calls::run(&mut state, conn, id, params);
                    if let Some(e) = link.failure.borrow_mut().take() {
                        break Err(e);
                    }
                    continue;
                }
            };
            if let Some(response) = handle_line(&mut state, &line) {
                if let Err(e) = link.send(&response) {
                    break Err(e);
                }
            }
            if let Some(e) = link.failure.borrow_mut().take() {
                break Err(e);
            }
        };
        // Fin de session, quelle qu'en soit la cause : les relais sont prévenus, le canal fermé.
        link.external.close(external::Lost::SessionEnded);
        outcome
    })
}

/// Un message du canal, sérialisé d'un bloc et suivi de sa fin de ligne (AD-3) : il part en une
/// seule écriture. Formaté sur la sortie, il partait en milliers de petits morceaux ; la sortie
/// standard les regroupe, mais un écrivain sans tampon les reçoit un à un, et l'écrivain des
/// tests y passait 3,4 s par réponse à `session.hello` (le catalogue entier, ticket 5.44).
fn line_of(message: &Value) -> Vec<u8> {
    let mut line = serde_json::to_vec(message).expect("une valeur JSON se sérialise toujours");
    line.push(b'\n');
    line
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
            // Un seul état d'occupation (AD-29) : un appel de relais réservé, même encore en file,
            // passe avant l'extension, qui reçoit `session.busy`.
            let occupies = matches!(
                method.as_str(),
                "command.run" | "plan.propose" | "plan.execute" | "journal.replay"
            );
            let link = state.link.clone();
            if occupies && !link.external.reserve(Source::Extension) {
                return respond(id, Err(busy()));
            }
            if method == "plan.reject" && link.external.is_busy() {
                return respond(id, Err(busy()));
            }
            // `plan.execute` comme `command.run` : `request.cancel` vise l'étape en vol.
            let is_run = method == "command.run" || method == "plan.execute";
            if is_run {
                *link.current.borrow_mut() = Some(Current {
                    source: Source::Extension,
                    id: id.clone(),
                    call_id: None,
                });
            }
            let outcome = dispatch(state, &method, params.as_ref());
            if is_run {
                *link.current.borrow_mut() = None;
            }
            // Un plan proposé non tranché occupe pour les relais seulement : il est noté avant
            // la libération, pour qu'aucun relais ne réserve entre les deux.
            link.external.set_plan_pending(state.pending.is_some());
            if occupies {
                link.external.release();
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
        return request_cancel(&state.link, Source::Extension, params, None);
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
        "agent.models" => plans::models(state),
        "session.external" => external::session_external(&state.link.external, params),
        _ => Err(RpcError::protocol(METHOD_NOT_FOUND, "Méthode inconnue.")),
    }
}

fn invalid_params(why: &str) -> RpcError {
    RpcError::protocol(INVALID_PARAMS, &format!("Paramètres invalides : {why}"))
}

/// L'exécutant est pris (AD-29) : refus immédiat, rien n'est mis en file.
fn busy() -> RpcError {
    RpcError::coded(
        "session.busy",
        "Une commande est en cours d'exécution : attendez sa fin ou annulez-la.".to_string(),
    )
}

/// La réponse à une méthode de l'extension pendant que l'exécutant est tenu (ordre en vol, demande
/// d'accord) : `session.busy` pour celles qui l'exigent, « méthode inconnue » pour les autres.
fn refused_while_held(method: &str) -> RpcError {
    match method {
        "session.hello" | "command.run" | "journal.replay" | "plan.propose" | "plan.execute"
        | "plan.reject" | "agent.models" => busy(),
        _ => RpcError::protocol(METHOD_NOT_FOUND, "Méthode inconnue."),
    }
}

fn is_cancellable(registry: &Registry, command: &str) -> bool {
    registry
        .spec(command)
        .is_some_and(|s| s.cancellable == Some(true))
}

/// Ordre en vol : sa commande, l'identifiant de son ordre et l'état de l'annulation.
struct InFlight<'c> {
    command: &'c str,
    order_id: &'c str,
    cancelling: &'c mut Option<Instant>,
}

/// `request.cancel` reçu de `requester`, comme requête ou comme notification. `in_flight` vaut
/// `None` hors d'un ordre : aucune requête n'est alors annulable. Un relais ne vise que l'appel de
/// sa connexion, par son `id` ; l'extension vise le sien par son `id`, celui d'un relais par son
/// `call_id`.
fn request_cancel(
    link: &Link<'_>,
    requester: Source,
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
    let aimed = link
        .current
        .borrow()
        .as_ref()
        .is_some_and(|c| c.answers(requester, target));
    if !aimed {
        return Err(unknown());
    }
    if !is_cancellable(link.registry, flight.command) {
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
            "request.cancel" => request_cancel(&self.link, Source::Extension, params, Some(flight)),
            // L'interrupteur n'exige pas l'exécutant : il est servi pendant l'ordre.
            "session.external" => external::session_external(&self.link.external, params),
            other => Err(refused_while_held(other)),
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
        let mut lost = false;
        loop {
            // Le relais de l'appel en cours a disparu (débranché, interrupteur coupé) : l'ordre est
            // annulé s'il est annulable, sinon il finit et l'étape est journalisée (AD-29, AD-12).
            if !lost && link.caller_lost() {
                lost = true;
                if cancelling.is_none() && is_cancellable(link.registry, id) {
                    link.send_or_fail(&cancel_notification(&order_id));
                    cancelling = Some(Instant::now() + link.grace);
                }
            }
            // La file ne se ferme pas avec l'entrée (les relais en tiennent une entrée) : une
            // entrée déjà fermée vaut une file fermée.
            let received = match cancelling {
                _ if link.input_closed.get() => Err(RecvTimeoutError::Disconnected),
                None => link.rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                Some(deadline) => link
                    .rx
                    .recv_timeout(deadline.saturating_duration_since(Instant::now())),
            };
            // Les relais sont servis pendant l'ordre ; seule l'extension peut y répondre.
            let line = match received {
                Ok(Msg::Line(line)) => line,
                Ok(Msg::Detached(conn)) => {
                    link.external.detached(conn);
                    continue;
                }
                Ok(Msg::Relay(
                    conn,
                    RelayCall::Cancel {
                        id: cancel_id,
                        params,
                    },
                )) => {
                    let flight = InFlight {
                        command: id,
                        order_id: &order_id,
                        cancelling: &mut cancelling,
                    };
                    link.relay_cancel(conn, cancel_id, params.as_ref(), Some(flight));
                    continue;
                }
                // Impossible tant que l'ordre tient l'exécutant ; refusé par prudence. Un appel en
                // file tient sa propre réservation : elle est libérée, sans toucher à celle d'un
                // autre.
                Ok(Msg::Relay(conn, RelayCall::Run { id: run_id, .. })) => {
                    link.external.release_for(Source::External(conn));
                    if let Some(response) = respond(Some(run_id), Err(busy())) {
                        link.external.send(conn, &response);
                    }
                    continue;
                }
                Err(RecvTimeoutError::Timeout) => return Err(cancelled()),
                // Entrée fermée ou illisible : plus rien à attendre, l'étape est annulée.
                other => {
                    match other {
                        Ok(Msg::Failed(e)) => link.fail(e),
                        _ => link.input_closed.set(true),
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

/// Nombre d'étapes passées que `session.hello` rend au panneau.
const HISTORY_LIMIT: usize = 200;

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
    // Formes de style que l'extension sait appliquer : un ajout compatible, absent d'une
    // extension plus ancienne (décision du propriétaire, 2026-11-09).
    let style_kinds = match params.get("style_kinds") {
        None | Some(Value::Null) => HashSet::new(),
        Some(v) => v
            .as_array()
            .and_then(|a| {
                a.iter()
                    .map(|v| v.as_str().map(str::to_string))
                    .collect::<Option<HashSet<String>>>()
            })
            .ok_or_else(|| invalid_params("style_kinds doit être une liste de noms de formes."))?,
    };
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
        })
        // L'extension sait ouvrir un raster du noyau : la session le lui dicte (ticket 4.9).
        .with_raster_opening(commands.contains(OPEN_RASTER))
        // De même pour un GeoJSON du noyau (AD-30, ticket 11.8).
        .with_vector_opening(commands.contains(OPEN_VECTOR))
        // Une mise en forme n'est exécutée que si l'extension annonce ses gestes (entrée 11.20).
        .with_gestures(commands.iter().cloned())
        // Le style d'un raster de résultat demande la forme `result_ramp` (2026-11-09).
        .with_style_kinds(style_kinds.iter().cloned());
    // La session dit dès la poignée de main si elle est en lecture seule : l'extension
    // n'a plus à lancer une commande pour le découvrir (ticket 5.32).
    let read_only = session.mode() == Mode::ReadOnly;
    // Les dernières étapes du journal, pour le panneau à la réouverture du projet : un ajout
    // compatible de la réponse, comme `read_only` ; rien n'est exécuté ni journalisé.
    let history = session.history(HISTORY_LIMIT);
    state.session = Some(session);
    state.project = Some(project.to_string());
    state.announced = commands;
    state.style_kinds = style_kinds;
    // Les relais ne voient que les commandes disponibles, sans aucun chemin (AD-25, AD-9).
    // Le système de calcul en mètres est choisi par le noyau (entrée 9.16) : l'outil du relais ne
    // le montre pas, comme le modèle et le formulaire.
    let available = catalogue(state)
        .into_iter()
        .filter(|entry| entry["available"] == json!(true))
        .map(|entry| without_core_filled(state.registry, entry))
        .collect();
    state.link.external.set_catalogue(available);
    let mut result = json!({
        "protocol_version": PROTOCOL_VERSION,
        "core_version": crate::VERSION,
        "read_only": read_only,
        "commands": catalogue(state),
        "history": history,
    });
    if read_only {
        result["message"] = json!(JournalError::ReadOnly.to_string());
    }
    Ok(result)
}

/// Une commande est offerte si elle s'exécute dans le noyau ou si l'extension l'annonce ; une
/// commande dictée par le noyau seul (ticket 4.9) ne l'est jamais : ni au catalogue des relais, ni
/// à un plan, ni à `command.run`.
fn is_available(state: &State<'_>, id: &str, runs_in: Option<RunsIn>) -> bool {
    if state.registry.spec(id).is_some_and(|s| s.dictated_by_core) {
        return false;
    }
    // Une mise en forme du noyau n'est offerte que si l'extension annonce tous ses gestes
    // (`project.inventory`, `layer.set_style`, `layer.arrange` ; AD-25, entrée 11.20).
    if state.registry.spec(id).is_some_and(|s| {
        !s.required_gestures()
            .iter()
            .all(|g| state.announced.contains(*g))
    }) {
        return false;
    }
    runs_in != Some(RunsIn::Qgis) || state.announced.contains(id)
}

/// Une entrée du catalogue des relais sans le paramètre que le noyau remplit seul du système de
/// calcul (`CommandSpec::work_crs_param`) : ni dans les propriétés, ni parmi les requis.
fn without_core_filled(registry: &Registry, mut entry: Value) -> Value {
    let hidden = entry["id"]
        .as_str()
        .and_then(|id| registry.spec(id))
        .and_then(|spec| spec.work_crs_param.as_ref())
        .map(|(name, _)| name.clone());
    if let Some(name) = hidden {
        let schema = &mut entry["params_schema"];
        if let Some(props) = schema.get_mut("properties").and_then(Value::as_object_mut) {
            props.remove(&name);
        }
        if let Some(required) = schema.get_mut("required").and_then(Value::as_array_mut) {
            required.retain(|r| r.as_str() != Some(name.as_str()));
        }
    }
    entry
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

/// Les paramètres d'un `command.run` : l'identifiant de la commande et ses paramètres (objet).
fn run_params(params: Option<&Value>) -> Result<(String, Value), RpcError> {
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
    Ok((id.to_string(), args))
}

/// Une commande du registre que l'extension n'a pas annoncée est indisponible (AD-7).
fn check_available(state: &State<'_>, id: &str) -> Result<(), RpcError> {
    if let Some(spec) = state.registry.spec(id) {
        if spec.dictated_by_core {
            return Err(RpcError::coded(
                "registry.dictated",
                format!(
                    "La commande « {id} » n'est appelée que par le noyau, après l'étape qui la \
                     demande : rien n'est exécuté."
                ),
            ));
        }
        if !is_available(state, id, spec.runs_in) {
            // Une mise en forme refusée l'est par `style.unavailable` (entrée 11.20).
            let error = if spec.required_gestures().is_empty() {
                RegistryError::Unavailable(id.to_string())
            } else {
                crate::style::unavailable()
            };
            return Err(RpcError::business(ErrorReport::from(error)));
        }
    }
    Ok(())
}

fn command_run(state: &mut State<'_>, params: Option<&Value>) -> Result<Value, RpcError> {
    let (id, args) = run_params(params)?;
    check_available(state, &id)?;
    let session = state.session.as_mut().expect("session vérifiée");
    session.call(&id, args).map_err(RpcError::business)
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
/// appel en cours n'est posé, donc `request.cancel` répond `request.unknown`.
fn journal_replay(state: &mut State<'_>) -> Result<Value, RpcError> {
    let project = state.project.clone().expect("session vérifiée");
    let replay = Replay::load(state.registry, Path::new(&project))
        .map_err(RpcError::business)?
        .with_gestures(state.announced.iter().cloned())
        .with_style_kinds(state.style_kinds.iter().cloned());
    let router = Router::new(state.registry).with_qgis(Announced {
        announced: state.announced.clone(),
        inner: Channel {
            link: state.link.clone(),
        },
    });
    let report = replay
        .run(&router, &mut state.replayed_layers)
        .map_err(RpcError::business)?;
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

    /// Garde chaque appel à `write` à part, pour compter les morceaux d'un message.
    struct Calls(Vec<Vec<u8>>);

    impl Write for Calls {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.push(bytes.to_vec());
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn each_message_leaves_in_a_single_write_with_its_end_of_line() {
        // Ticket 5.44 : la réponse à session.hello porte tout le catalogue ; écrite en morceaux,
        // elle coûtait 3,4 s de processeur à chaque poignée de main dans les tests.
        let dir = temp_dir();
        let project = project_in(&dir);
        let registry = Registry::standard();
        let input = Cursor::new(
            [
                hello_line(2, &[], &project),
                run_line(2, "system.about", json!({})),
            ]
            .join("\n")
                + "\n",
        );
        let mut out = Calls(Vec::new());
        serve(&registry, input, &mut out).unwrap();
        assert_eq!(out.0.len(), 2, "deux messages, deux écritures");
        for call in &out.0 {
            assert_eq!(call.iter().filter(|b| **b == b'\n').count(), 1);
            assert!(call.ends_with(b"\n"));
            let message: Value = serde_json::from_slice(call).unwrap();
            assert!(message["result"].is_object());
        }
        let hello: Value = serde_json::from_slice(&out.0[0]).unwrap();
        assert_eq!(
            hello["result"]["commands"].as_array().unwrap().len(),
            registry.specs().count(),
            "le catalogue entier part dans la seule écriture"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hello_then_about_journals_one_line() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let r = exchange(&[
            hello_line(2, &[], &project),
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
        assert_eq!(r[1]["result"]["protocol_version"], 2);
        assert_eq!(r[1]["result"]["core_version"], crate::VERSION);
        let journal = std::fs::read_to_string(dir.join("p.cassini/journal.jsonl")).unwrap();
        assert_eq!(journal.lines().count(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_reopened_project_gets_its_past_steps_in_the_hello_answer() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let first = exchange(&[hello_line(2, &[], &project)]);
        assert_eq!(first[0]["result"]["history"], json!([]));
        exchange(&[
            hello_line(2, &[], &project),
            run_line(2, "system.about", json!({})),
            run_line(3, "command.unknown_for_history", json!({})),
        ]);
        // Une ligne illisible est sautée, sans cacher les autres.
        let journal = dir.join("p.cassini/journal.jsonl");
        let mut text = std::fs::read_to_string(&journal).unwrap();
        text.push_str("pas du json\n");
        std::fs::write(&journal, text).unwrap();
        let again = exchange(&[hello_line(2, &[], &project)]);
        let history = again[0]["result"]["history"].as_array().unwrap();
        assert_eq!(history.len(), 1, "{history:?}");
        assert_eq!(history[0]["step"], 1);
        assert_eq!(history[0]["command"], "system.about");
        assert!(history[0]["result"].is_object());
        assert!(history[0]["at"]
            .as_u64()
            .is_some_and(|ms| ms > 1_700_000_000_000));
        assert!(history[0].get("versions").is_none() && history[0].get("inputs").is_none());
        // La réouverture n'allonge pas le journal.
        let lines = std::fs::read_to_string(&journal).unwrap();
        assert_eq!(lines.lines().count(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_history_keeps_only_the_last_steps_and_their_outcome() {
        let dir = temp_dir();
        let path = dir.join("journal.jsonl");
        let mut text = String::new();
        for step in 1..=5 {
            text.push_str(
                &json!({"step": step, "id": "analysis.buffer", "params": {"layer": "l"},
                    "error": "analysis.crs_not_metric", "plan_step": 1,
                    "versions": {"core": "x"}})
                .to_string(),
            );
            text.push('\n');
        }
        text.push_str(
            &json!({"step": 6, "id": "terrain.slope", "params": {"dem": "d"},
                "cancelled": true, "origin": "external", "at": 1_762_600_000_000_u64})
            .to_string(),
        );
        text.push('\n');
        text.push_str(
            &json!({"step": 7, "id": "layer.add", "params": {"points": "x".repeat(500)},
                "result": {"layer_id": "c_1"}, "layer_id": "c_1"})
            .to_string(),
        );
        text.push('\n');
        std::fs::write(&path, text).unwrap();
        let history = crate::journal::history_of(&path, 3);
        assert_eq!(history.len(), 3);
        assert_eq!(history[0]["step"], 5);
        assert_eq!(history[0]["error"], "analysis.crs_not_metric");
        assert_eq!(history[0]["plan_step"], 1);
        assert!(history[0].get("versions").is_none());
        assert_eq!(history[1]["cancelled"], true);
        assert_eq!(history[1]["origin"], "external");
        assert_eq!(history[1]["at"], 1_762_600_000_000_u64);
        assert_eq!(history[2]["layer_id"], "c_1");
        // Des paramètres longs sont résumés, comme un résultat.
        assert_eq!(history[2]["params"]["truncated"], true);
        assert_eq!(
            crate::journal::history_of(&dir.join("absent.jsonl"), 3),
            Vec::<Value>::new()
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn hello_alone_writes_no_journal_and_says_the_session_writes() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let journal = dir.join("p.cassini/journal.jsonl");
        let r = exchange(&[hello_line(2, &[], &project)]);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["result"]["read_only"], false);
        assert!(r[0]["result"].get("message").is_none());
        assert_eq!(r[0]["result"]["core_version"], crate::VERSION);
        assert_eq!(r[0]["result"]["protocol_version"], 2);
        // La poignée de main seule n'écrit aucune étape : le fichier n'existe pas.
        assert!(!journal.exists());
        let r = exchange(&[
            hello_line(2, &[], &project),
            run_line(2, "system.about", json!({})),
        ]);
        assert!(r[1]["result"].is_object());
        assert_eq!(
            std::fs::read_to_string(&journal).unwrap().lines().count(),
            1
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn replaying_twice_leaves_the_journal_byte_for_byte() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let journal = dir.join("p.cassini/journal.jsonl");
        // Un ancien journal : le system.about d'accueil, puis une commande.
        exchange(&[
            hello_line(2, &[], &project),
            run_line(2, "system.about", json!({})),
            run_line(3, "system.about", json!({})),
        ]);
        let before = std::fs::read(&journal).unwrap();
        assert_eq!(String::from_utf8_lossy(&before).lines().count(), 2);
        for _ in 0..2 {
            let r = exchange(&[
                hello_line(2, &[], &project),
                json!({"jsonrpc":"2.0","id":2,"method":"journal.replay"}).to_string(),
            ]);
            assert_eq!(r[1]["result"], json!({"steps": 2, "identical": true}));
            assert_eq!(std::fs::read(&journal).unwrap(), before);
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_second_session_says_read_only_at_hello() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let registry = Registry::standard();
        let first = Session::open(&registry, Path::new(&project)).unwrap();
        assert_eq!(first.mode(), Mode::Write);
        let r = exchange(&[hello_line(2, &[], &project)]);
        assert_eq!(r[0]["result"]["read_only"], true);
        // Le texte même que rend toute commande de cette session (`journal.read_only`).
        let refusal = ErrorReport::from(JournalError::ReadOnly);
        assert_eq!(refusal.code, "journal.read_only");
        assert_eq!(r[0]["result"]["message"], json!(refusal.message));
        drop(first);
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
        let r = exchange(&[hello_line(2, &[], &project), hello_line(2, &[], &project)]);
        assert!(r[0].get("result").is_some());
        assert_eq!(code_of(&r[1]), "session.already_started");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_unannounced_qgis_command_is_refused_and_not_journaled() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let r = exchange(&[
            hello_line(2, &[], &project),
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
            hello_line(2, &["test.qgis_probe"], &project),
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
        let r = exchange(&[hello_line(2, &[], "p.qgz")]);
        assert_eq!(r[0]["error"]["code"], -32602);
    }

    #[test]
    fn unknown_methods_and_notifications() {
        let dir = temp_dir();
        let project = project_in(&dir);
        let notification = json!({"jsonrpc":"2.0","method":"command.run","params":{}}).to_string();
        let r = exchange(&[
            hello_line(2, &[], &project),
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
            hello_line(2, &[], &project),
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
        exchange(&[hello_line(2, &[], &project)]);
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
        /// Ne cherche la fin de ligne que dans les octets reçus, jamais dans tout le tampon : le
        /// coût reste linéaire quelle que soit la façon dont le message arrive (5.44).
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            for piece in bytes.split_inclusive(|b| *b == b'\n') {
                self.buf.extend_from_slice(piece);
                if piece.ends_with(b"\n") {
                    let line = std::mem::take(&mut self.buf);
                    let text = String::from_utf8_lossy(&line).trim().to_string();
                    let _ = self.tx.send(text);
                }
            }
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn the_test_writer_rebuilds_lines_whatever_the_pieces() {
        let (tx, rx) = mpsc::channel();
        let mut w = ChanWriter {
            tx,
            buf: Vec::new(),
        };
        // Une ligne en trois morceaux, deux lignes d'un coup, un appel vide, une fin sans retour.
        for piece in [&b"{\"a\""[..], b":1", b"}\n", b"x\ny\n", b"", b"z"] {
            assert_eq!(w.write(piece).unwrap(), piece.len());
        }
        let lines: Vec<String> = rx.try_iter().collect();
        assert_eq!(lines, ["{\"a\":1}", "x", "y"]);
        assert_eq!(w.buf, b"z");
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

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct ChainParams {
        layer: String,
        #[serde(default)]
        out: String,
    }

    /// Commande qgis qui rend une couche écrite dans un fichier de sortie, comme une analyse
    /// (ticket 5.34 : les intermédiaires se reconnaissent par ce fichier).
    struct Chain;
    impl Delegated for Chain {
        type Params = ChainParams;
        type Output = Made;
        fn spec() -> crate::registry::CommandSpec {
            crate::registry::CommandSpec::new::<ChainParams, Made>(
                "test.chain",
                "Enchaîne.",
                "Chains.",
            )
            .runs_in(RunsIn::Qgis)
            .cancellable(true)
            .output_params(&["out"])
            .output_ext(".gpkg")
            .tests(&["t"])
        }
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct PointsParams {
        #[serde(default)]
        out: String,
    }

    /// Commande du noyau qui écrit les points de la fixture en GeoJSON (AD-30, ticket 11.8).
    struct WritePoints;
    impl crate::registry::Command for WritePoints {
        type Params = PointsParams;
        type Output = Done;
        fn spec() -> crate::registry::CommandSpec {
            crate::registry::CommandSpec::new::<PointsParams, Done>(
                "test.write_points",
                "Écrit des points.",
                "Writes points.",
            )
            .runs_in(RunsIn::Core)
            .cancellable(false)
            .output_params(&["out"])
            .output_ext(".geojson")
            .tests(&["t"])
        }
        fn run(&self, p: PointsParams) -> Result<Done, crate::registry::CommandFailure> {
            crate::geojson::write(Path::new(&p.out), 2154, &crate::geojson::fixture_points())
                .map_err(|e| crate::registry::CommandFailure {
                    code: e.code.to_string(),
                    message: e.message,
                })?;
            Ok(Done { ok: true })
        }
    }

    fn scripted_registry() -> Registry {
        let mut r = Registry::standard();
        r.register(WritePoints);
        r.register_delegated::<Render>();
        r.register_delegated::<Fixed>();
        r.register_delegated::<Make>();
        r.register_delegated::<Chain>();
        r
    }

    /// Compositeur de test : demande l'inventaire s'il le doit, puis rend d'abord les plans de
    /// `first` (un par composition, dans l'ordre), ensuite son issue fixe.
    struct FixedComposer {
        inventory: bool,
        outcome: Result<Value, ErrorReport>,
        first: std::cell::RefCell<std::collections::VecDeque<Value>>,
        /// Inventaires reçus par le compositeur, dans l'ordre.
        seen: std::sync::Arc<std::sync::Mutex<Vec<Value>>>,
        /// Modèle demandé à chaque composition, dans l'ordre.
        chosen: std::sync::Arc<std::sync::Mutex<Vec<Option<String>>>>,
        /// Applique le garde-fou des attributs au plan rendu, sur la vue reçue, comme le
        /// compositeur réel (ticket 5.57).
        guard_attributes: bool,
    }

    impl Composer for FixedComposer {
        fn compose(
            &self,
            registry: &Registry,
            _request: &str,
            model: Option<&str>,
            inventory: &mut dyn FnMut() -> Result<
                crate::agent_inventory::ModelInventory,
                ErrorReport,
            >,
        ) -> Result<crate::plan::Plan, ErrorReport> {
            self.chosen.lock().unwrap().push(model.map(str::to_string));
            let mut shown = Value::Null;
            if self.inventory {
                let view = inventory()?;
                assert!(view.inventory["layers"].is_array());
                // Les couches de l'utilisateur sont en tête de l'index d'une vue triée (5.48),
                // sinon en tête de `layers`.
                let cut = view
                    .inventory
                    .get("index")
                    .unwrap_or(&view.inventory["layers"]);
                let users = cut.as_array().unwrap().len();
                assert!(view.user_layers <= users);
                shown = view.inventory.clone();
                self.seen.lock().unwrap().push(view.inventory);
            }
            let next = self.first.borrow_mut().pop_front();
            next.map(Ok)
                .unwrap_or_else(|| self.outcome.clone())
                .map(|v| {
                    let guard = v.get("source").and_then(Value::as_str) == Some("guard");
                    let plan = serde_json::from_value(v).expect("plan de test lisible");
                    // La provenance ne se lit pas d'un JSON : le test la pose à la main.
                    match plan {
                        crate::plan::Plan::Question { question, .. } if guard => {
                            crate::plan::Plan::Question {
                                question,
                                source: crate::plan::QuestionSource::Guard,
                            }
                        }
                        other => other,
                    }
                })
                .map(|plan| {
                    if self.guard_attributes {
                        crate::attribute_guard::guard_attributes(registry, plan, &shown)
                    } else {
                        plan
                    }
                })
        }

        fn models(&self, _registry: &Registry) -> Result<Vec<Value>, ErrorReport> {
            Ok(vec![
                json!({"file": "a.gguf", "size_bytes": 5}),
                json!({"file": "b.modele.json", "size_bytes": 0,
                    "problem": "fichier manquant : absent.gguf"}),
            ])
        }
    }

    fn composer(plan: Value) -> Option<FixedComposer> {
        Some(FixedComposer {
            inventory: true,
            outcome: Ok(plan),
            first: Default::default(),
            seen: Default::default(),
            chosen: Default::default(),
            guard_attributes: false,
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
            Client::start_in(temp_dir(), grace, composer)
        }

        /// Ferme cette session, puis en ouvre une seconde sur **le même projet** (le verrou du
        /// journal est relâché à la fin de la première). Le dossier passe à la seconde.
        fn reopen(mut self, composer: Option<FixedComposer>) -> Client {
            if self.server.is_some() {
                self.close().1.unwrap();
            }
            let dir = std::mem::take(&mut self.dir);
            Client::start_in(dir, CANCEL_GRACE, composer)
        }

        fn start_in(dir: PathBuf, grace: Duration, composer: Option<FixedComposer>) -> Client {
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
                serve_with(
                    &registry,
                    composer,
                    None,
                    reader,
                    writer,
                    grace,
                    APPROVE_TIMEOUT,
                )
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
                "params":{"protocol_version":2,"commands":["test.render","test.fixed"],
                "project":self.project}}));
            assert!(self.recv()["result"].is_object());
        }

        /// Ouvre la session en annonçant `commands`.
        fn hello_with(&self, commands: &[&str]) {
            self.send(json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
                "params":{"protocol_version":2,"commands":commands,"project":self.project}}));
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

        /// Comme `expect_step`, pour une étape dont le noyau lit d'abord l'inventaire (le système
        /// de calcul d'un tampon, entrée 9.16) : répond `inventory` à cette lecture.
        fn expect_step_reading(
            &self,
            plan_id: &str,
            step: u64,
            command: &str,
            inventory: Value,
        ) -> Value {
            let notice = self.recv();
            assert_eq!(notice["method"], "plan.step", "{notice}");
            assert_eq!(
                notice["params"],
                json!({"plan_id": plan_id, "step": step, "command": command})
            );
            let read = self.recv();
            assert_eq!(read["method"], "order.execute", "{read}");
            assert_eq!(read["params"]["command"], "project.inventory", "{read}");
            self.send(reply(&read["id"], inventory));
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
            if self.dir.as_os_str().is_empty() {
                return;
            }
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
            "params":{"protocol_version":2,"commands":[],"project":c.project}}));
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
        assert_eq!(c.recv()["result"]["protocol_version"], 2);
        c.close().1.unwrap();
        let registry = scripted_registry();
        let replay = crate::journal::Replay::load(&registry, Path::new(&c.project)).unwrap();
        let report = replay
            .run(&crate::exec::Router::new(&registry), &mut HashSet::new())
            .unwrap();
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

    /// Répond à l'ordre d'export comme le geste : écrit le PDF, rend un résultat fixe.
    fn answer_export(c: &Client, order: &Value) {
        assert_eq!(order["params"]["command"], "layout.export_pdf");
        let out = order["params"]["params"]["out"].as_str().unwrap();
        std::fs::write(out, "pdf").unwrap();
        c.send(reply(
            &order["id"],
            json!({"file": out, "page_count": 1, "layer_count": 1,
                "sources": ["pts.csv"], "truncated": false}),
        ));
    }

    #[test]
    fn a_second_journal_replay_leaves_the_first_one_s_layers_out_of_the_export() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello_with(&["test.make", "layout.export_pdf"]);
        // Session d'origine : une couche, puis l'export, sans `leave_out`.
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"command.run",
            "params":{"command":"test.make","params":{"layer":"src"}}}));
        let order = c.recv();
        c.send(reply(&order["id"], json!({"layer_id": "orig"})));
        assert!(c.recv()["result"].is_object());
        c.send(json!({"jsonrpc":"2.0","id":3,"method":"command.run",
            "params":{"command":"layout.export_pdf","params":{"title":"Carte"}}}));
        let order = c.recv();
        assert!(order["params"]["params"].get("leave_out").is_none());
        answer_export(&c, &order);
        assert!(c.recv()["result"].is_object());

        // Deux rejeux de suite : le second reçoit aussi l'id posé par le premier.
        let expected = [json!(["orig"]), json!(["orig", "replay-1"])];
        for (n, leave_out) in expected.iter().enumerate() {
            let id = 4 + n as u64;
            c.send(json!({"jsonrpc":"2.0","id":id,"method":"journal.replay"}));
            let order = c.recv();
            assert_eq!(order["params"]["command"], "test.make");
            assert!(order["params"]["params"].get("leave_out").is_none());
            c.send(reply(
                &order["id"],
                json!({"layer_id": format!("replay-{}", n + 1)}),
            ));
            let order = c.recv();
            assert_eq!(order["params"]["params"]["leave_out"], *leave_out);
            answer_export(&c, &order);
            let done = c.recv();
            assert_eq!(done["id"], id);
            assert_eq!(done["result"], json!({"steps": 2, "identical": true}));
        }
        assert_eq!(c.journal().len(), 2);
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
            "params":{"protocol_version":2,"commands":[],"project":c.project}}));
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

    // ---- Parcours de la phrase de démo : plan, exécution, rejeu sans l'IA ----

    /// Commandes de la phrase de démo, telles que l'extension les annonce.
    const DEMO: [&str; 5] = [
        "analysis.select_by_attribute",
        "analysis.buffer",
        "analysis.intersect",
        "layout.export_pdf",
        "project.inventory",
    ];

    /// Les 5 étapes de la phrase de démo (même forme que `demo_plan` d'`ai/tests/ecart_reel.rs`).
    fn demo_plan() -> Value {
        json!({"kind":"calls","calls":[
            {"command":"analysis.select_by_attribute","params":{"layer_id":"plu","field":"typezone","operator":"eq","value":"U"}},
            {"command":"analysis.buffer","params":{"layer_id":"gares","distance":{"value":500,"unit":"m"}}},
            {"command":"analysis.intersect","params":{"layer_id":"parcelles","overlay_layer_id":"step:1.layer_id"}},
            {"command":"analysis.intersect","params":{"layer_id":"step:3.layer_id","overlay_layer_id":"step:2.layer_id"}},
            {"command":"layout.export_pdf","params":{"title":"Parcelles constructibles"}}
        ]})
    }

    /// L'inventaire que la fausse extension rend au noyau avant le tampon : les gares en degrés,
    /// le projet en Lambert-93, comme le projet d'Annecy.
    fn demo_inventory() -> Value {
        json!({"layers": [
            {"layer_id": "plu", "name": "plu", "crs": "EPSG:2154", "crs_unit": "m"},
            {"layer_id": "gares", "name": "gares", "crs": "EPSG:4326", "crs_unit": "deg"},
            {"layer_id": "parcelles", "name": "parcelles", "crs": "EPSG:2154", "crs_unit": "m"}
        ], "project_crs": "EPSG:2154", "project_crs_unit": "m", "extent": null})
    }

    /// Fausse extension : écrit la sortie que le noyau a nommée, puis rend le résultat typé de
    /// l'étape. `prefix` distingue les ids de couche d'une session à l'autre ; `count` est le
    /// nombre d'entités rendu par chaque analyse.
    fn demo_answer(order: &Value, step: u64, prefix: &str, count: u64) -> Value {
        let out = order["params"]["params"]["out"].as_str().unwrap();
        std::fs::write(out, b"sortie de la fausse extension").unwrap();
        let result = match order["params"]["command"].as_str().unwrap() {
            "layout.export_pdf" => json!({"file": out, "page_count": 1, "layer_count": 4,
                "sources": ["plu.gpkg", "gares.gpkg", "parcelles.gpkg"], "truncated": false}),
            _ => json!({"layer_id": format!("{prefix}_{step}"), "name": format!("Étape {step}"),
                "feature_count": count, "crs": "EPSG:2154"}),
        };
        reply(&order["id"], result)
    }

    /// Rejoue le journal par le canal ; l'étape `odd` (si donnée) rend 99 entités au lieu de 7.
    /// Rend aussi les paramètres de chaque ordre rejoué, dans l'ordre.
    fn replay_demo(c: &Client, id: u64, odd: Option<u64>) -> (Value, Vec<Value>) {
        c.send(json!({"jsonrpc":"2.0","id":id,"method":"journal.replay"}));
        let mut params = Vec::new();
        for step in 1..=5 {
            let order = c.recv();
            assert_eq!(order["method"], "order.execute", "{order}");
            params.push(order["params"]["params"].clone());
            let count = if odd == Some(step) { 99 } else { 7 };
            c.send(demo_answer(&order, step, "r", count));
            if odd == Some(step) {
                break;
            }
        }
        (c.recv(), params)
    }

    /// Exécute le plan de démo dans une première session, et rend le client fermé.
    fn run_demo_plan() -> Client {
        let mut c = Client::start_with(CANCEL_GRACE, composer(demo_plan()));
        c.hello_with(&DEMO);
        let proposed = c.propose(2);
        assert_eq!(proposed["result"]["plan"], demo_plan(), "{proposed}");
        let plan_id = proposed["result"]["plan_id"].as_str().unwrap().to_string();
        assert!(c.journal().is_empty());
        c.execute(3, &plan_id);
        let calls = demo_plan()["calls"].as_array().unwrap().clone();
        for (i, call) in calls.iter().enumerate() {
            let step = i as u64 + 1;
            let command = call["command"].as_str().unwrap();
            let order = if command == "analysis.buffer" {
                // Les gares sont en degrés, le projet en Lambert-93 : le tampon est calculé en
                // EPSG:2154, sans étape ajoutée au plan (entrée 9.16).
                let order = c.expect_step_reading(&plan_id, step, command, demo_inventory());
                assert_eq!(order["params"]["params"]["work_epsg"], 2154, "{order}");
                order
            } else {
                c.expect_step(&plan_id, step, command)
            };
            // Un seul ordre à la fois : rien ne part avant la réponse.
            c.assert_silent();
            c.send(demo_answer(&order, step, "l", 7));
        }
        let done = c.recv();
        assert_eq!(done["id"], 3);
        assert_eq!(
            done["result"]["steps"].as_array().unwrap().len(),
            5,
            "{done}"
        );
        let journal = c.journal();
        assert_eq!(journal.len(), 5);
        // Le système de calcul est journalisé avec les paramètres du tampon (AD-11).
        assert_eq!(journal[1]["params"]["work_epsg"], 2154, "{}", journal[1]);
        // Les références `step:N` sont résolues au journal.
        assert_eq!(
            journal[2]["params"]["overlay_layer_id"], "l_1",
            "{}",
            journal[2]
        );
        assert_eq!(journal[3]["params"]["layer_id"], "l_3");
        assert_eq!(journal[3]["params"]["overlay_layer_id"], "l_2");
        assert!(journal.iter().all(|l| !l.to_string().contains("step:")));
        c.close().1.unwrap();
        c
    }

    #[test]
    fn the_demo_plan_runs_then_replays_in_a_session_without_composer() {
        let c = run_demo_plan();
        let journal = c.dir.join("p.cassini/journal.jsonl");
        let before = std::fs::read(&journal).unwrap();
        // Seconde session du même projet, sans compositeur : aucun modèle ne peut y être chargé.
        let mut again = c.reopen(None);
        again.hello_with(&DEMO);
        // La preuve que l'IA n'y est pas : composer est refusé.
        again.send(json!({"jsonrpc":"2.0","id":2,"method":"plan.propose",
            "params":{"request":"Fais-le."}}));
        assert_eq!(code_of(&again.recv()), "agent.unavailable");
        let (replayed, params) = replay_demo(&again, 3, None);
        // Les couches sources restent ; chaque référence est liée à l'id neuf de son étape.
        assert_eq!(params[0]["layer_id"], "plu");
        assert_eq!(params[1]["layer_id"], "gares");
        // Le système de calcul est relu dans le journal, sans relire l'inventaire (AD-11).
        assert_eq!(params[1]["work_epsg"], 2154);
        assert_eq!(params[2]["layer_id"], "parcelles");
        assert_eq!(params[2]["overlay_layer_id"], "r_1");
        assert_eq!(params[3]["layer_id"], "r_3");
        assert_eq!(params[3]["overlay_layer_id"], "r_2");
        assert_eq!(replayed["id"], 3);
        assert_eq!(replayed["result"], json!({"steps": 5, "identical": true}));
        // Le rejeu n'écrit pas le journal : identique octet pour octet.
        assert_eq!(std::fs::read(&journal).unwrap(), before);
        again.close().1.unwrap();
    }

    #[test]
    fn a_demo_replay_that_differs_names_the_step() {
        let c = run_demo_plan();
        let journal = c.dir.join("p.cassini/journal.jsonl");
        let before = std::fs::read(&journal).unwrap();
        let mut again = c.reopen(None);
        again.hello_with(&DEMO);
        // Un autre id de couche ne compte pas (valeur de session) ; un autre résultat, si.
        let (replayed, _) = replay_demo(&again, 2, Some(3));
        // Le rejeu arrêté n'a pas écrit le journal.
        assert_eq!(std::fs::read(&journal).unwrap(), before);
        assert_eq!(code_of(&replayed), "replay.diverged", "{replayed}");
        assert_eq!(replayed["error"]["data"]["step"], 3);
        assert!(replayed["error"]["data"]["message"]
            .as_str()
            .unwrap()
            .contains("analysis.intersect"));
        // La session reste debout.
        again.send(json!({"jsonrpc":"2.0","id":4,"method":"command.run",
            "params":{"command":"system.about","params":{}}}));
        assert_eq!(again.recv()["result"]["protocol_version"], 2);
        again.close().1.unwrap();
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
        assert_eq!(c.recv()["result"]["protocol_version"], 2);
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
                seen: Default::default(),
                chosen: Default::default(),
                guard_attributes: false,
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
        assert_eq!(
            r["result"],
            json!({"question": question, "source": "model"})
        );
        assert!(r["result"].get("plan_id").is_none());
        c.execute(3, "plan-1");
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        c.assert_silent();
        assert!(c.journal().is_empty());
        c.close().1.unwrap();
    }

    #[test]
    fn a_guard_question_carries_its_source_and_is_never_executed() {
        let plan = json!({"kind":"question","question":"Quel point ?","source":"guard"});
        let mut c = Client::start_with(CANCEL_GRACE, composer(plan));
        c.hello_with(&ALL);
        let r = c.propose(2);
        assert_eq!(
            r["result"],
            json!({"question": "Quel point ?", "source": "guard"})
        );
        c.execute(3, "plan-1");
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        c.assert_silent();
        assert!(c.journal().is_empty());
        c.close().1.unwrap();
    }

    /// Ticket 5.57 : une valeur absente des valeurs listées de l'inventaire rend la question du
    /// garde-fou, sans `plan_id`, jamais exécutée ni journalisée (AD-11).
    #[test]
    fn an_attribute_guard_question_has_no_plan_id_and_writes_no_journal_line() {
        let plan = json!({"kind":"calls","calls":[{"command":"analysis.select_by_attribute",
            "params":{"layer_id":"zonage","field":"typezone","operator":"eq","value":"AU"}}]});
        let mut composer = composer(plan).unwrap();
        composer.guard_attributes = true;
        let mut c = Client::start_with(CANCEL_GRACE, Some(composer));
        c.hello_with(&ALL);
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"plan.propose",
            "params":{"request":"Les zones AU."}}));
        let order = c.recv();
        assert_eq!(order["params"]["command"], "project.inventory");
        c.send(reply(
            &order["id"],
            json!({"project_crs": "EPSG:2154", "extent": null, "layers": [
                {"layer_id": "zonage", "name": "Zonage PLU", "source": "plu.gpkg",
                 "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiPolygon",
                 "feature_count": 4,
                 "fields": [{"name": "typezone", "type": "String", "values": ["U", "N"]}]}
            ]}),
        ));
        let r = c.recv();
        assert_eq!(
            r["result"],
            json!({"question": "Le champ typezone de la couche « Zonage PLU » n'a pas de valeur « AU » (valeurs : N, U) : laquelle faut-il prendre ?",
                "source": "guard"}),
            "{r}"
        );
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
        assert_eq!(
            r["result"],
            json!({"question": "Quelle couche utiliser ?", "source": "model"})
        );
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

    /// Dix couches de résultat dont les fichiers sont au dossier de travail, datées de l'étape 1
    /// (la plus ancienne) à la 10, et trois couches de l'utilisateur : `plan.propose` (id `id`)
    /// reçoit cet inventaire, qui dépasse le budget ; rend la réponse du noyau.
    fn propose_over_the_budget(c: &mut Client, id: u64) -> Value {
        use std::time::{Duration, SystemTime};
        let work = c.dir.join("p.cassini");
        let base = SystemTime::now() - Duration::from_secs(3_600);
        for k in 1..=10u64 {
            let file = work.join(format!("{k}-analysis.intersect.gpkg"));
            std::fs::write(&file, b"gpkg").unwrap();
            std::fs::File::options()
                .write(true)
                .open(&file)
                .unwrap()
                .set_modified(base + Duration::from_secs(k * 60))
                .unwrap();
        }
        let fields: Vec<Value> = (0..34)
            .map(|i| json!({"name": format!("champ_assez_long_{i:02}"), "type": "Integer64"}))
            .collect();
        let user = |id: &str| {
            json!({"layer_id": id, "name": id, "source": "villeneuve.gpkg", "crs": "EPSG:2154",
                "kind": "vector", "geometry": "MultiPolygon", "feature_count": 3,
                "fields": [{"name": "typezone", "type": "String", "values": ["U", "N"]}]})
        };
        // Comme QGIS les pose : les nouveaux résultats en tête de légende.
        let mut layers: Vec<Value> = (1..=10)
            .rev()
            .map(|k| {
                json!({"layer_id": format!("Intersection_{k}"), "name": "Intersection",
                    "source": format!("{k}-analysis.intersect.gpkg"), "crs": "EPSG:2154",
                    "kind": "vector", "geometry": "MultiPolygon", "feature_count": 9,
                    "fields": fields})
            })
            .collect();
        layers.extend([user("plu"), user("gares"), user("parcelles")]);
        c.send(json!({"jsonrpc":"2.0","id":id,"method":"plan.propose","params":{"request":"x"}}));
        let order = c.recv();
        assert_eq!(order["params"]["command"], "project.inventory");
        c.send(reply(
            &order["id"],
            json!({"project_name": "p", "project_crs": "EPSG:2154", "extent": null,
                "layers": layers}),
        ));
        let answer = c.recv();
        assert_eq!(answer["id"], id);
        answer
    }

    /// Ticket 5.33 : les couches retirées par le budget, telles que rendues avec la réponse.
    fn left_out_one_to_five() -> Value {
        Value::Array(
            (1..=5)
                .map(|k| json!({"layer_id": format!("Intersection_{k}"), "name": "Intersection"}))
                .collect(),
        )
    }

    /// Tickets 5.18 et 5.19 : dix couches de résultat dont les fichiers sont au dossier de travail
    /// dépassent le budget d'inventaire ; le compositeur reçoit toutes les couches de
    /// l'utilisateur, puis les résultats les plus récents gardés par paquets, les plus anciens absents.
    /// Ticket 5.33 : la réponse les signale, du plus ancien au plus récent, aux vrais identifiants.
    #[test]
    fn the_composer_sees_an_inventory_reduced_to_the_budget() {
        use crate::agent_inventory::INVENTORY_BUDGET_CHARS;
        let composer = composer(three_steps()).unwrap();
        let seen = composer.seen.clone();
        let mut c = Client::start_with(CANCEL_GRACE, Some(composer));
        c.hello_with(&ALL);
        let answer = propose_over_the_budget(&mut c, 2);
        assert!(answer["result"]["plan_id"].is_string(), "{answer}");
        assert_eq!(answer["result"]["left_out"], left_out_one_to_five());
        let seen = seen.lock().unwrap();
        let inventory = &seen[0];
        let ids: Vec<&str> = inventory["layers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["layer_id"].as_str().unwrap())
            .collect();
        // Ordre stable (ticket 5.19) : l'utilisateur, puis les résultats du plus ancien au plus
        // récent ; retrait par paquets (5.19) : rejoués du plus ancien au plus récent, les
        // résultats dépassent le budget et redescendent sous le seuil bas, puis s'ajoutent
        // jusqu'à l'étape 10 sans redépasser : les étapes 6 à 10 restent.
        assert_eq!(
            ids,
            [
                "plu",
                "gares",
                "parcelles",
                "Intersection_6",
                "Intersection_7",
                "Intersection_8",
                "Intersection_9",
                "Intersection_10"
            ]
        );
        assert!(inventory.to_string().chars().count() <= INVENTORY_BUDGET_CHARS);
        // Une couche gardée garde sa forme apprise, champs compris.
        assert_eq!(
            inventory["layers"][3]["fields"].as_array().unwrap().len(),
            34
        );
        // Le modèle ne voit rien du constat : l'inventaire n'a pas de clé `left_out`.
        assert!(inventory.get("left_out").is_none());
        drop(seen);
        c.close().1.unwrap();
    }

    /// Ticket 5.48 : un gros projet (couches de l'utilisateur au-delà du seuil du tri), dont aucune
    /// couche n'est désignée par la demande « Fais-le. » : `zonage` n'est montrée que dans l'index.
    fn big_project_inventory() -> Value {
        let fields: Vec<Value> = (0..12)
            .map(|i| {
                json!({"name": format!("attribut_{i:02}"), "type": "String",
                "values": ["valeur_a", "valeur_b", "valeur_c"]})
            })
            .collect();
        let mut layers: Vec<Value> = (0..30)
            .map(|k| {
                json!({"layer_id": format!("objet_{k:02}"), "name": format!("objet_{k:02}"),
                    "source": "fond.gpkg", "crs": "EPSG:2154", "kind": "vector",
                    "geometry": "MultiPolygon", "feature_count": 3, "fields": fields})
            })
            .collect();
        layers.push(
            json!({"layer_id": "zonage", "name": "zonage", "source": "plu.shp",
            "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiPolygon",
            "feature_count": 3,
            "fields": [{"name": "typezone", "type": "String", "values": ["U", "N"]}]}),
        );
        json!({"project_name": "p", "project_crs": "EPSG:2154", "extent": null, "layers": layers})
    }

    /// `plan.propose` sur le gros projet ; le compositeur rend d'abord les plans de `first`.
    fn propose_big(first: Vec<Value>, then: Value) -> (Value, Vec<Value>, Vec<Value>) {
        let composer = composer(then).unwrap();
        composer.first.borrow_mut().extend(first);
        let seen = composer.seen.clone();
        let mut c = Client::start_with(CANCEL_GRACE, Some(composer));
        c.hello_with(&ALL);
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"plan.propose",
            "params":{"request":"Fais-le."}}));
        let order = c.recv();
        assert_eq!(order["params"]["command"], "project.inventory");
        c.send(reply(&order["id"], big_project_inventory()));
        let answer = c.recv();
        // La redemande ne redemande pas l'inventaire à l'extension.
        c.assert_silent();
        let journal = c.journal();
        c.close().1.unwrap();
        let seen = seen.lock().unwrap().clone();
        (answer, seen, journal)
    }

    fn detailed(view: &Value) -> Vec<&str> {
        view["layers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["layer_id"].as_str().unwrap())
            .collect()
    }

    #[test]
    fn a_field_on_a_reduced_layer_recomposes_once_with_that_layer_detailed() {
        let field_plan = json!({"kind":"calls","calls":[
            {"command":"analysis.select_by_attribute",
             "params":{"layer_id":"zonage","field":"typezone","operator":"eq","value":"U"}}]});
        let question = json!({"kind":"question","question":"Laquelle ?"});
        // Le second plan prend encore un champ : il est gardé tel quel, sans troisième composition.
        let (answer, seen, journal) = propose_big(vec![field_plan.clone(), field_plan], question);
        assert_eq!(seen.len(), 2, "une seule redemande");
        assert!(seen[0].get("index").is_some());
        assert!(!detailed(&seen[0]).contains(&"zonage"));
        assert!(detailed(&seen[1]).contains(&"zonage"));
        // Même index, même table : seule la partie détaillée change.
        assert_eq!(seen[0]["index"], seen[1]["index"]);
        // Le second plan, jugé comme un premier : ici il ne vérifie pas (commande non annoncée),
        // et c'est son erreur qui sort, jamais une troisième composition.
        assert_eq!(code_of(&answer), "agent.plan_invalid", "{answer}");
        // Ni l'inventaire ni la redemande ne sont journalisés.
        assert!(journal.is_empty());
    }

    /// Ticket 5.48 : le dernier plan exécuté est lu dans le journal du projet à chaque proposition ;
    /// la couche qu'il cite est détaillée, même si la demande ne la désigne pas.
    #[test]
    fn the_layer_cited_by_the_last_plan_of_the_journal_is_detailed() {
        let composer = composer(json!({"kind":"question","question":"?"})).unwrap();
        let seen = composer.seen.clone();
        let mut c = Client::start_with(CANCEL_GRACE, Some(composer));
        c.hello_with(&ALL);
        let line = json!({"id": "analysis.select_by_attribute", "plan_step": 1,
            "params": {"layer_id": "zonage", "field": "typezone", "operator": "eq", "value": "U"},
            "layer_id": "r1", "result": {}});
        std::fs::write(c.dir.join("p.cassini/journal.jsonl"), format!("{line}\n")).unwrap();
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"plan.propose",
            "params":{"request":"Fais-le."}}));
        let order = c.recv();
        c.send(reply(&order["id"], big_project_inventory()));
        let answer = c.recv();
        assert_eq!(answer["result"]["question"], "?", "{answer}");
        c.close().1.unwrap();
        let seen = seen.lock().unwrap();
        assert!(seen[0].get("index").is_some());
        assert_eq!(detailed(&seen[0]), ["zonage"]);
    }

    #[test]
    fn the_plan_of_the_recomposition_is_kept_and_shown_like_a_first_one() {
        let field_plan = json!({"kind":"calls","calls":[
            {"command":"analysis.select_by_attribute",
             "params":{"layer_id":"zonage","field":"typezone","operator":"eq","value":"U"}}]});
        let valid = json!({"kind":"calls","calls":[
            {"command":"test.fixed","params":{"layer":"zonage"}}]});
        let (answer, seen, journal) = propose_big(vec![field_plan], valid);
        assert_eq!(seen.len(), 2);
        assert_eq!(answer["result"]["plan_id"], "plan-1", "{answer}");
        assert_eq!(
            answer["result"]["plan"]["calls"][0]["command"],
            "test.fixed"
        );
        assert!(journal.is_empty());
    }

    #[test]
    fn a_field_reached_through_a_step_reference_recomposes_too() {
        let plan = json!({"kind":"calls","calls":[
            {"command":"analysis.buffer","params":{"layer_id":"zonage",
                "distance":{"value":10,"unit":"m"}}},
            {"command":"analysis.select_by_attribute",
             "params":{"layer_id":"step:1.layer_id","field":"typezone","operator":"eq","value":"U"}}]});
        let question = json!({"kind":"question","question":"Laquelle ?"});
        let (answer, seen, _) = propose_big(vec![plan], question);
        assert_eq!(seen.len(), 2);
        assert!(detailed(&seen[1]).contains(&"zonage"));
        assert_eq!(answer["result"]["question"], "Laquelle ?", "{answer}");
    }

    #[test]
    fn a_reduced_layer_taken_as_geometry_only_does_not_recompose() {
        let question = json!({"kind":"question","question":"Laquelle ?"});
        let plan = json!({"kind":"calls","calls":[
            {"command":"test.fixed","params":{"layer_id":"zonage"}}]});
        let (_, seen, _) = propose_big(vec![plan], question.clone());
        assert_eq!(seen.len(), 1);
        // Une question ou un champ sur une couche détaillée ne recomposent pas non plus.
        let (_, seen, _) = propose_big(vec![], question);
        assert_eq!(seen.len(), 1);
        let on_detailed = json!({"kind":"calls","calls":[
            {"command":"analysis.select_by_attribute",
             "params":{"layer_id":"step:9.layer_id","field":"typezone","operator":"eq","value":"U"}}]});
        let (_, seen, _) =
            propose_big(vec![on_detailed], json!({"kind":"question","question":"?"}));
        assert_eq!(seen.len(), 1, "référence sans étape : aucune couche");
    }

    #[test]
    fn under_the_threshold_a_field_never_recomposes() {
        let plan = json!({"kind":"calls","calls":[
            {"command":"analysis.select_by_attribute",
             "params":{"layer_id":"plu","field":"typezone","operator":"eq","value":"U"}}]});
        let composer = composer(json!({"kind":"question","question":"?"})).unwrap();
        composer.first.borrow_mut().push_back(plan);
        let seen = composer.seen.clone();
        let mut c = Client::start_with(CANCEL_GRACE, Some(composer));
        c.hello_with(&ALL);
        propose_over_the_budget(&mut c, 2);
        c.close().1.unwrap();
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].get("index").is_none());
    }

    /// Ticket 5.48 (AD-29) : l'IA externe et `command.run` lisent `project.inventory` brut, même
    /// pour un gros projet ; la vue triée ne sert qu'à `plan.propose`.
    #[test]
    fn command_run_returns_the_raw_inventory_of_a_big_project() {
        let mut c = Client::start_with(CANCEL_GRACE, composer(three_steps()));
        c.hello_with(&ALL);
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"command.run",
            "params":{"command":"project.inventory","params":{}}}));
        let order = c.recv();
        assert_eq!(order["params"]["command"], "project.inventory");
        let raw = big_project_inventory();
        c.send(reply(&order["id"], raw.clone()));
        let answer = c.recv();
        assert_eq!(answer["result"], raw, "{answer}");
        assert!(answer["result"].get("index").is_none());
        c.close().1.unwrap();
    }

    /// Ticket 5.33 : au-delà du budget, une question porte aussi les couches retirées ; le
    /// journal n'en garde rien.
    #[test]
    fn a_question_over_the_budget_carries_the_layers_left_out() {
        let plan = json!({"kind":"question","question":"Quelle zone tampon ?"});
        let mut c = Client::start_with(CANCEL_GRACE, composer(plan));
        c.hello_with(&ALL);
        let answer = propose_over_the_budget(&mut c, 2);
        assert_eq!(
            answer["result"],
            json!({"question": "Quelle zone tampon ?", "source": "model",
                "left_out": left_out_one_to_five()})
        );
        assert!(c.journal().is_empty());
        c.close().1.unwrap();
    }

    /// Ticket 5.33 : sous le budget, la réponse d'un plan est celle d'avant, sans clé `left_out`.
    #[test]
    fn under_the_budget_a_plan_carries_no_left_out_key() {
        let mut c = Client::start_with(CANCEL_GRACE, composer(three_steps()));
        c.hello_with(&ALL);
        let r = c.propose(2);
        assert!(r["result"]["plan_id"].is_string(), "{r}");
        assert!(r["result"].get("left_out").is_none(), "{r}");
        c.close().1.unwrap();
    }

    // ---- Couches intermédiaires déduites du journal (tickets 5.22 et 5.34) ----

    const CHAIN: [&str; 4] = ["test.chain", "test.fixed", "test.make", "project.inventory"];

    fn chain(layer: &str) -> Value {
        json!({"command":"test.chain","params":{"layer":layer}})
    }

    /// Répond à l'étape `step` de `plan_id` (`test.chain`) comme QGIS : écrit le fichier de sortie
    /// demandé, puis rend la couche `layer_id`.
    fn answer_chain(c: &Client, plan_id: &str, step: u64, layer_id: &str) {
        let order = c.expect_step(plan_id, step, "test.chain");
        write_output(&order);
        c.send(reply(&order["id"], json!({"layer_id": layer_id})));
    }

    fn write_output(order: &Value) {
        let out = order["params"]["params"]["out"].as_str().unwrap();
        std::fs::write(out, b"gpkg").unwrap();
    }

    /// Une couche de l'inventaire de QGIS ; `source` est son fichier, sans dossier.
    fn inventory_layer(id: &str, source: &str) -> Value {
        json!({"layer_id": id, "name": source, "source": source, "crs": "EPSG:2154",
            "kind": "vector", "geometry": "MultiPolygon", "feature_count": 3,
            "fields": [{"name": "typezone", "type": "String", "values": ["U", "N"]}]})
    }

    /// `plan.propose` sur un inventaire de `layers` ; rend l'inventaire vu par le compositeur.
    fn proposed_view(c: &Client, id: u64, layers: Vec<Value>, seen: &SeenInventories) -> Value {
        c.send(json!({"jsonrpc":"2.0","id":id,"method":"plan.propose","params":{"request":"x"}}));
        let order = c.recv();
        assert_eq!(order["params"]["command"], "project.inventory");
        c.send(reply(
            &order["id"],
            json!({"project_name": "p", "project_crs": "EPSG:2154", "layers": layers}),
        ));
        let answer = c.recv();
        assert_eq!(answer["id"], id, "{answer}");
        assert!(answer["result"]["plan_id"].is_string(), "{answer}");
        seen.lock().unwrap().last().unwrap().clone()
    }

    type SeenInventories = std::sync::Arc<std::sync::Mutex<Vec<Value>>>;

    fn view_ids(view: &Value) -> Vec<String> {
        view["layers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["layer_id"].as_str().unwrap().to_string())
            .collect()
    }

    /// La vue sans les identifiants, que QGIS renouvelle à chaque ouverture.
    fn without_ids(view: &Value) -> Value {
        let mut view = view.clone();
        for layer in view["layers"].as_array_mut().unwrap() {
            layer.as_object_mut().unwrap().remove("layer_id");
        }
        view
    }

    /// Un compositeur qui rend `plans` dans l'ordre, puis un plan d'une étape sans couche.
    fn composer_of(plans: Vec<Value>) -> (FixedComposer, SeenInventories) {
        let last = json!({"kind":"calls","calls":[
            {"command":"test.fixed","params":{"layer":"plu"}}]});
        let composer = composer(last).unwrap();
        composer.first.borrow_mut().extend(plans);
        let seen = composer.seen.clone();
        (composer, seen)
    }

    /// Ticket 5.22 : un plan à résultats chaînés exécuté, puis un plan arrêté en cours ; à la
    /// proposition suivante, le compositeur ne voit que les sorties qu'aucune étape exécutée n'a
    /// consommées, sans valeurs, et les couches de l'utilisateur intactes. Ticket 5.34 : après une
    /// relance du noyau, les mêmes fichiers rechargés sous de nouveaux ids donnent la même vue.
    #[test]
    fn the_intermediates_of_an_executed_plan_stay_hidden_after_a_restart() {
        let chained = json!({"kind":"calls","calls":[
            chain("src"), chain("step:1.layer_id"), chain("step:2.layer_id"),
            {"command":"test.fixed","params":{"layer":"step:3.layer_id"}}
        ]});
        let stopped = json!({"kind":"calls","calls":[chain("src"), chain("step:1.layer_id")]});
        let (composer, seen) = composer_of(vec![chained, stopped]);
        let c = Client::start_with(CANCEL_GRACE, Some(composer));
        c.hello_with(&CHAIN);
        // Plan à trois résultats chaînés (journal 1 à 3) : r1 et r2 consommés, r3 final, cité
        // ensuite par une étape qui ne produit aucune couche (comme une description) : r3 reste vu.
        assert_eq!(c.propose(2)["result"]["plan_id"], "plan-1");
        c.execute(3, "plan-1");
        for (step, id) in [(1, "r1"), (2, "r2"), (3, "r3")] {
            answer_chain(&c, "plan-1", step, id);
        }
        let order = c.expect_step("plan-1", 4, "test.fixed");
        c.send(reply(&order["id"], json!({"ok": true})));
        assert_eq!(c.recv()["id"], 3);
        // Plan arrêté à l'étape 2 (journal 5 et 6) : r4 n'a été consommé par aucune étape exécutée.
        assert_eq!(c.propose(4)["result"]["plan_id"], "plan-2");
        c.execute(5, "plan-2");
        answer_chain(&c, "plan-2", 1, "r4");
        let order = c.expect_step("plan-2", 2, "test.chain");
        c.send(
            json!({"jsonrpc":"2.0","id":order["id"],"error":{"code":-32000,
            "message":"Raté.","data":{"code":"qgis.failed","message":"Raté."}}}),
        );
        assert!(c.recv()["error"].is_object());
        let journal = c.journal();
        let steps: Vec<&Value> = journal.iter().map(|l| &l["plan_step"]).collect();
        assert_eq!(
            steps,
            [
                &json!(1),
                &json!(2),
                &json!(3),
                &json!(4),
                &json!(1),
                &json!(2)
            ]
        );
        let files = [
            "1-test.chain.gpkg",
            "2-test.chain.gpkg",
            "3-test.chain.gpkg",
            "5-test.chain.gpkg",
        ];
        let layers = |ids: [&str; 4]| -> Vec<Value> {
            let mut layers: Vec<Value> = ids
                .iter()
                .zip(files)
                .rev()
                .map(|(id, f)| inventory_layer(id, f))
                .collect();
            layers.push(inventory_layer("plu", "plu.gpkg"));
            layers
        };
        let before = proposed_view(&c, 6, layers(["r1", "r2", "r3", "r4"]), &seen);
        // Ordre stable : l'utilisateur, puis les résultats du plus ancien au plus récent.
        assert_eq!(view_ids(&before), ["plu", "r3", "r4"]);
        assert_eq!(
            before["layers"][0]["fields"][0]["values"],
            json!(["N", "U"])
        );
        for l in &before["layers"].as_array().unwrap()[1..] {
            assert_eq!(l["fields"], json!([{"name": "typezone", "type": "string"}]));
        }
        // Relance : nouveau noyau sur le même projet, QGIS rouvert avec de nouveaux ids.
        let (composer, seen) = composer_of(Vec::new());
        let mut c = c.reopen(Some(composer));
        c.hello_with(&CHAIN);
        let after = proposed_view(&c, 2, layers(["n1", "n2", "n3", "n4"]), &seen);
        assert_eq!(view_ids(&after), ["plu", "n3", "n4"]);
        assert_eq!(without_ids(&after), without_ids(&before));
        c.close().1.unwrap();
    }

    /// Ticket 5.34 : la sortie finale d'un plan reprise par la phrase suivante reste vue, et les
    /// copies des intermédiaires posées par `journal.replay` sous `replay/` sont cachées comme
    /// leurs originaux, dans la même session comme après une relance.
    #[test]
    fn the_next_sentence_keeps_its_input_and_replayed_intermediates_stay_hidden() {
        let first = json!({"kind":"calls","calls":[chain("src"), chain("step:1.layer_id")]});
        // La phrase suivante reprend la sortie finale du plan 1 par son id.
        let next = json!({"kind":"calls","calls":[chain("a2"), chain("step:1.layer_id")]});
        let (composer, seen) = composer_of(vec![first, next]);
        let c = Client::start_with(CANCEL_GRACE, Some(composer));
        c.hello_with(&CHAIN);
        assert_eq!(c.propose(2)["result"]["plan_id"], "plan-1");
        c.execute(3, "plan-1");
        answer_chain(&c, "plan-1", 1, "a1");
        answer_chain(&c, "plan-1", 2, "a2");
        assert_eq!(c.recv()["id"], 3);
        assert_eq!(c.propose(4)["result"]["plan_id"], "plan-2");
        c.execute(5, "plan-2");
        answer_chain(&c, "plan-2", 1, "b1");
        answer_chain(&c, "plan-2", 2, "b2");
        assert_eq!(c.recv()["id"], 5);
        // Rejeu : les quatre étapes repassent par QGIS, leurs sorties vont sous `replay/`.
        c.send(json!({"jsonrpc":"2.0","id":6,"method":"journal.replay"}));
        for id in ["p1", "p2", "p3", "p4"] {
            let order = c.recv();
            assert_eq!(order["params"]["command"], "test.chain", "{order}");
            let out = order["params"]["params"]["out"].as_str().unwrap();
            assert!(out.replace('\\', "/").contains("/replay/"), "{out}");
            write_output(&order);
            c.send(reply(&order["id"], json!({"layer_id": id})));
        }
        let report = c.recv();
        assert_eq!(report["result"]["identical"], true, "{report}");
        let files = [
            "1-test.chain.gpkg",
            "2-test.chain.gpkg",
            "3-test.chain.gpkg",
            "4-test.chain.gpkg",
        ];
        // Originaux et copies du rejeu (même nom de fichier), puis la couche de l'utilisateur.
        let layers = |ids: [&str; 8]| -> Vec<Value> {
            let mut layers: Vec<Value> = ids
                .iter()
                .zip(files.iter().chain(files.iter()))
                .map(|(id, f)| inventory_layer(id, f))
                .collect();
            layers.reverse();
            layers.push(inventory_layer("plu", "plu.gpkg"));
            layers
        };
        let sorted = |view: &Value| {
            let mut ids = view_ids(view);
            ids.sort();
            ids
        };
        let view = proposed_view(
            &c,
            7,
            layers(["a1", "a2", "b1", "b2", "p1", "p2", "p3", "p4"]),
            &seen,
        );
        assert_eq!(sorted(&view), ["a2", "b2", "p2", "p4", "plu"]);
        // Après une relance : mêmes fichiers, nouveaux ids, même règle.
        let (composer, seen) = composer_of(Vec::new());
        let mut c = c.reopen(Some(composer));
        c.hello_with(&CHAIN);
        let view = proposed_view(
            &c,
            2,
            layers(["c1", "c2", "c3", "c4", "q1", "q2", "q3", "q4"]),
            &seen,
        );
        assert_eq!(sorted(&view), ["c2", "c4", "plu", "q2", "q4"]);
        c.close().1.unwrap();
    }

    #[test]
    fn the_composer_sees_the_inventory_in_the_learned_shape() {
        let composer = composer(three_steps()).unwrap();
        let seen = composer.seen.clone();
        let mut c = Client::start_with(CANCEL_GRACE, Some(composer));
        c.hello_with(&ALL);
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"plan.propose","params":{"request":"x"}}));
        let order = c.recv();
        assert_eq!(order["params"]["command"], "project.inventory");
        c.send(reply(
            &order["id"],
            json!({
                "project_name": "projet-test",
                "project_crs": "EPSG:2154",
                "extent": null,
                "layers": [
                    {"layer_id": "plu_1", "name": "Zonage du PLU", "source": "plu.gpkg",
                     "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiPolygon",
                     "feature_count": 3, "fields": [
                        {"name": "typezone", "type": "String", "values": ["U", "N", "Nh"]},
                        {"name": "surface", "type": "Real"}]},
                    {"layer_id": "mnt_1", "name": "MNT", "source": "mnt.tif",
                     "crs": "IGNF:LAMB93", "kind": "raster", "geometry": null, "fields": []}
                ]
            }),
        ));
        assert!(c.recv()["result"]["plan_id"].is_string());
        assert_eq!(
            *seen.lock().unwrap(),
            vec![json!({
                "project": {"name": "projet-test", "epsg": 2154},
                "layers": [
                    {"layer_id": "plu_1", "name": "Zonage du PLU", "source": "plu.gpkg",
                     "epsg": 2154, "kind": "vector", "geometry": "MultiPolygon",
                     "feature_count": 3, "fields": [
                        {"name": "typezone", "type": "string", "values": ["N", "Nh", "U"]},
                        {"name": "surface", "type": "number"}]},
                    {"layer_id": "mnt_1", "name": "MNT", "source": "mnt.tif",
                     "kind": "raster", "geometry": null, "fields": []}
                ],
                "unloaded_files": []
            })]
        );
        c.close().1.unwrap();
    }

    /// Ticket 5.19 : le compositeur voit des identifiants courts ; le plan qu'il rend en courts est
    /// retraduit vers les identifiants de QGIS avant d'être vérifié, gardé, montré et exécuté.
    #[test]
    fn the_composer_sees_short_ids_and_its_plan_is_translated_back() {
        const PARCELLES: &str = "parcelles_e9ba733c_8a43_4717_ac73_04344c1b6bb5";
        const GARES: &str = "gares_83d935aa_ca33_431d_966f_dbbf0fb7ac0f";
        let composer = composer(json!({"kind": "calls", "calls": [
            {"command": "test.make", "params": {"layer": "gares_83d9"}},
            {"command": "test.fixed", "params": {"layer": "parcelles_e9ba"}},
            {"command": "test.fixed", "params": {"layer": "step:1.layer_id"}}
        ]}))
        .unwrap();
        let seen = composer.seen.clone();
        let mut c = Client::start_with(CANCEL_GRACE, Some(composer));
        c.hello_with(&ALL);
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"plan.propose","params":{"request":"x"}}));
        let order = c.recv();
        assert_eq!(order["params"]["command"], "project.inventory");
        let layer = |id: &str| json!({"layer_id": id, "name": id, "crs": "EPSG:2154"});
        c.send(reply(
            &order["id"],
            json!({"project_crs": "EPSG:2154", "layers": [layer(PARCELLES), layer(GARES)]}),
        ));
        let proposed = c.recv();
        let seen_ids: Vec<String> = seen.lock().unwrap()[0]["layers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["layer_id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(seen_ids, ["parcelles_e9ba", "gares_83d9"]);
        // Le plan montré porte les identifiants de QGIS.
        assert_eq!(
            proposed["result"]["plan"]["calls"],
            json!([
                {"command": "test.make", "params": {"layer": GARES}},
                {"command": "test.fixed", "params": {"layer": PARCELLES}},
                {"command": "test.fixed", "params": {"layer": "step:1.layer_id"}}
            ])
        );
        // Le plan gardé aussi : l'extension reçoit les vrais identifiants, le journal les garde.
        c.execute(3, "plan-1");
        let order = c.expect_step("plan-1", 1, "test.make");
        assert_eq!(order["params"]["params"], json!({"layer": GARES}));
        c.send(reply(&order["id"], json!({"layer_id": "l_1"})));
        let order = c.expect_step("plan-1", 2, "test.fixed");
        assert_eq!(order["params"]["params"], json!({"layer": PARCELLES}));
        c.send(reply(&order["id"], json!({"ok": true})));
        let order = c.expect_step("plan-1", 3, "test.fixed");
        assert_eq!(order["params"]["params"], json!({"layer": "l_1"}));
        c.send(reply(&order["id"], json!({"ok": true})));
        assert_eq!(c.recv()["id"], 3);
        let journal = c.journal();
        assert_eq!(journal[0]["params"], json!({"layer": GARES}));
        assert_eq!(journal[1]["params"], json!({"layer": PARCELLES}));
        c.close().1.unwrap();
    }

    /// Ticket 5.19 : une question ou un refus du modèle qui cite un identifiant court le montre
    /// avec l'identifiant de QGIS.
    #[test]
    fn a_question_or_a_refusal_quoting_a_short_id_shows_the_qgis_id() {
        const GARES: &str = "gares_83d935aa_ca33_431d_966f_dbbf0fb7ac0f";
        let composer = FixedComposer {
            inventory: true,
            outcome: Ok(json!({"kind": "refusal", "reason": "gares_83d9 n'a pas de champ."})),
            first: std::cell::RefCell::new(
                [json!({"kind": "question", "question": "Quelle gare de gares_83d9 ?"})].into(),
            ),
            seen: Default::default(),
            chosen: Default::default(),
            guard_attributes: false,
        };
        let mut c = Client::start_with(CANCEL_GRACE, Some(composer));
        c.hello_with(&ALL);
        let inventory = json!({"project_crs": "EPSG:2154",
            "layers": [{"layer_id": GARES, "name": "Gares", "crs": "EPSG:2154"}]});
        for id in [2, 3] {
            c.send(
                json!({"jsonrpc":"2.0","id":id,"method":"plan.propose","params":{"request":"x"}}),
            );
            let order = c.recv();
            assert_eq!(order["params"]["command"], "project.inventory");
            c.send(reply(&order["id"], inventory.clone()));
            let answer = c.recv();
            if id == 2 {
                assert_eq!(
                    answer["result"]["question"],
                    format!("Quelle gare de {GARES} ?")
                );
            } else {
                assert_eq!(code_of(&answer), "agent.refused");
                let message = answer["error"]["message"].as_str().unwrap();
                assert!(
                    message.contains(&format!("{GARES} n'a pas de champ.")),
                    "{message}"
                );
            }
        }
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
    fn a_plan_result_keeps_the_absolute_output_path_for_the_extension() {
        // Décision du 2026-11-04 (AD-9) : la réponse de `plan.execute` porte les chemins de
        // sortie absolus, comme `command.run` ; le journal les garde relatifs.
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
            json!({"ok": true, "file": file})
        );
        assert_eq!(c.journal()[0]["result"]["file"], "1-carte.pdf");
        c.close().1.unwrap();
    }

    #[test]
    fn a_slope_in_a_plan_returns_its_absolute_existing_file() {
        let plan = json!({"kind":"calls","calls":[
            {"command":"test.fixed","params":{"layer":"x"}},
            {"command":"terrain.slope","params":{"dem":"dem/plan-incline.tif","unit":"degrees"}}]});
        let mut c = Client::start_with(CANCEL_GRACE, composer(plan));
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/mnt-plan-incline-2154.tif");
        std::fs::create_dir_all(c.dir.join("dem")).unwrap();
        std::fs::copy(&fixture, c.dir.join("dem/plan-incline.tif")).unwrap();
        c.hello_with(&ALL);
        assert_eq!(c.propose(2)["result"]["plan_id"], "plan-1");
        c.execute(3, "plan-1");
        let order = c.expect_step("plan-1", 1, "test.fixed");
        c.send(reply(&order["id"], json!({"ok": true})));
        let notice = c.recv();
        assert_eq!(notice["method"], "plan.step", "{notice}");
        assert_eq!(notice["params"]["command"], "terrain.slope");
        let done = c.recv();
        let step = &done["result"]["steps"][1];
        assert_eq!(step["command"], "terrain.slope", "{done}");
        let file = Path::new(step["result"]["file"].as_str().unwrap());
        assert!(file.is_absolute(), "{}", file.display());
        assert_eq!(file, c.dir.join("p.cassini").join("2-terrain.slope.tif"));
        assert!(file.is_file());
        let journal = c.journal();
        assert_eq!(journal[1]["result"]["file"], "2-terrain.slope.tif");
        // L'extension n'annonce pas `layer.open_raster` : rien n'est dicté (ticket 4.9).
        assert_eq!(journal.len(), 2);
        c.close().1.unwrap();
    }

    // ---- Ticket 4.9 : l'ouverture d'un raster du noyau, dictée par le noyau ----

    /// L'extension annonce l'ouverture d'un raster, l'export et l'inventaire.
    const RASTER: [&str; 3] = [
        "layer.open_raster",
        "layout.export_pdf",
        "project.inventory",
    ];

    /// Copie le MNT de test sous `dem/plan.tif` dans le dossier du projet de `c`.
    fn with_dem(c: &Client) {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/mnt-plan-incline-2154.tif");
        std::fs::create_dir_all(c.dir.join("dem")).unwrap();
        std::fs::copy(&fixture, c.dir.join("dem/plan.tif")).unwrap();
    }

    /// Attend l'ordre `layer.open_raster` sur la sortie `file` du dossier `dir`, y répond comme
    /// le geste (couche `layer_id`, nommée du radical du fichier).
    fn answer_open(c: &Client, dir: &Path, file: &str, layer_id: &str) {
        let order = c.recv();
        assert_eq!(order["method"], "order.execute", "{order}");
        assert_eq!(order["params"]["command"], "layer.open_raster");
        let path = Path::new(order["params"]["params"]["path"].as_str().unwrap());
        assert_eq!(path, dir.join(file));
        assert!(path.is_file(), "{}", path.display());
        let stem = file.trim_end_matches(".tif");
        c.send(reply(
            &order["id"],
            json!({"layer_id": layer_id, "name": stem}),
        ));
    }

    #[test]
    fn the_dictated_command_is_never_offered_and_command_run_refuses_it() {
        let mut c = Client::start(CANCEL_GRACE);
        c.send(json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
            "params":{"protocol_version":2,"commands":RASTER,"project":c.project}}));
        let hello = c.recv();
        let commands = hello["result"]["commands"].as_array().unwrap();
        let entry = commands
            .iter()
            .find(|e| e["id"] == "layer.open_raster")
            .unwrap();
        // Annoncée, mais jamais offerte : le panneau la cache, les relais ne la voient pas.
        assert_eq!(entry["dictated_by_core"], json!(true));
        assert_eq!(entry["available"], json!(false));
        assert!(commands
            .iter()
            .filter(|e| ![
                "layer.open_raster",
                "layer.open_vector",
                "layer.set_style",
                "layer.arrange"
            ]
            .contains(&e["id"].as_str().unwrap()))
            .all(|e| e.get("dictated_by_core").is_none()));
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"command.run",
            "params":{"command":"layer.open_raster","params":{"path":"out:1.1"}}}));
        let refused = c.recv();
        assert_eq!(code_of(&refused), "registry.dictated");
        assert!(c.journal().is_empty());
        c.close().1.unwrap();
    }

    #[test]
    fn the_dictated_vector_opening_is_never_offered_and_command_run_refuses_it() {
        // AD-30, ticket 11.8 : `layer.open_vector`, annoncée par l'extension, reste dictée.
        let mut c = Client::start(CANCEL_GRACE);
        c.send(json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
            "params":{"protocol_version":2,
                "commands":["layer.open_raster", "layer.open_vector", "layout.export_pdf"],
                "project":c.project}}));
        let hello = c.recv();
        let commands = hello["result"]["commands"].as_array().unwrap();
        let entry = commands
            .iter()
            .find(|e| e["id"] == "layer.open_vector")
            .unwrap();
        assert_eq!(entry["dictated_by_core"], json!(true));
        assert_eq!(entry["available"], json!(false));
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"command.run",
            "params":{"command":"layer.open_vector","params":{"path":"out:1.1"}}}));
        let refused = c.recv();
        assert_eq!(code_of(&refused), "registry.dictated");
        assert!(c.journal().is_empty());
        c.close().1.unwrap();
    }

    #[test]
    fn a_slope_run_by_hand_is_followed_by_the_dictated_opening() {
        let mut c = Client::start(CANCEL_GRACE);
        with_dem(&c);
        c.hello_with(&RASTER);
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"command.run",
            "params":{"command":"terrain.slope","params":{"dem":"dem/plan.tif","unit":"degrees"}}}));
        let work = c.dir.join("p.cassini");
        answer_open(&c, &work, "1-terrain.slope.tif", "raster_1");
        let done = c.recv();
        assert_eq!(done["id"], 2);
        let file = done["result"]["file"].as_str().unwrap();
        assert_eq!(Path::new(file), work.join("1-terrain.slope.tif"));
        let journal = c.journal();
        assert_eq!(journal.len(), 2);
        assert_eq!(journal[1]["id"], "layer.open_raster");
        assert_eq!(journal[1]["params"], json!({"path": "out:1.1"}));
        assert_eq!(journal[1]["layer_id"], "raster_1");
        c.close().1.unwrap();
    }

    #[test]
    fn the_result_style_is_dictated_only_when_the_extension_announces_result_ramp() {
        // Décision du propriétaire du 2026-11-09 : `layer.set_style` annoncé ne suffit pas,
        // l'extension doit annoncer savoir appliquer `result_ramp` (`style_kinds`).
        for kinds in [
            None,
            Some(json!(["hillshade"])),
            Some(json!(["result_ramp"])),
        ] {
            let mut c = Client::start(CANCEL_GRACE);
            with_dem(&c);
            let mut params = json!({"protocol_version":2,
                "commands":["layer.open_raster", "layer.set_style"], "project":c.project});
            if let Some(kinds) = &kinds {
                params["style_kinds"] = kinds.clone();
            }
            c.send(json!({"jsonrpc":"2.0","id":1,"method":"session.hello","params":params}));
            assert!(c.recv()["result"].is_object());
            c.send(json!({"jsonrpc":"2.0","id":2,"method":"command.run",
                "params":{"command":"terrain.slope","params":{"dem":"dem/plan.tif","unit":"degrees"}}}));
            let work = c.dir.join("p.cassini");
            answer_open(&c, &work, "1-terrain.slope.tif", "raster_1");
            let styled = kinds == Some(json!(["result_ramp"]));
            if styled {
                let order = c.recv();
                assert_eq!(order["params"]["command"], "layer.set_style", "{order}");
                assert_eq!(order["params"]["params"]["layer_id"], "raster_1");
                assert_eq!(order["params"]["params"]["style"]["kind"], "result_ramp");
                c.send(reply(&order["id"], json!({"layer_id": "raster_1"})));
            }
            let done = c.recv();
            assert_eq!(done["id"], 2, "{done}");
            let ids: Vec<String> = c
                .journal()
                .iter()
                .map(|l| l["id"].as_str().unwrap().to_string())
                .collect();
            let expected: &[&str] = if styled {
                &["terrain.slope", "layer.open_raster", "layer.set_style"]
            } else {
                &["terrain.slope", "layer.open_raster"]
            };
            assert_eq!(ids, expected, "{kinds:?}");
            c.assert_silent();
            c.close().1.unwrap();
        }
    }

    #[test]
    fn malformed_style_kinds_are_invalid_params() {
        let mut c = Client::start(CANCEL_GRACE);
        c.send(json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
            "params":{"protocol_version":2,"commands":[],"style_kinds":"result_ramp",
                "project":c.project}}));
        assert_eq!(c.recv()["error"]["code"], -32602);
        c.close().1.unwrap();
    }

    #[test]
    fn a_geojson_run_by_hand_is_followed_by_the_dictated_vector_opening() {
        // AD-30, ticket 11.8 : l'annonce de `layer.open_vector` à `session.hello` fait dicter
        // l'ouverture après une sortie GeoJSON du noyau.
        let mut c = Client::start(CANCEL_GRACE);
        c.hello_with(&["layer.open_vector", "layout.export_pdf"]);
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"command.run",
            "params":{"command":"test.write_points","params":{}}}));
        let work = c.dir.join("p.cassini");
        let order = c.recv();
        assert_eq!(order["method"], "order.execute", "{order}");
        assert_eq!(order["params"]["command"], "layer.open_vector");
        let path = Path::new(order["params"]["params"]["path"].as_str().unwrap());
        assert_eq!(path, work.join("1-test.write_points.geojson"));
        assert!(path.is_file(), "{}", path.display());
        c.send(reply(
            &order["id"],
            json!({"layer_id": "vector_1", "name": "1-test.write_points", "crs": "EPSG:2154"}),
        ));
        let done = c.recv();
        assert_eq!(done["id"], 2);
        assert_eq!(done["result"], json!({"ok": true}), "{done}");
        let journal = c.journal();
        assert_eq!(journal.len(), 2);
        assert_eq!(journal[1]["id"], "layer.open_vector");
        assert_eq!(journal[1]["params"], json!({"path": "out:1.1"}));
        assert_eq!(journal[1]["layer_id"], "vector_1");
        c.close().1.unwrap();
    }

    #[test]
    fn a_slope_then_an_export_plan_replays_identically_in_the_same_and_a_new_session() {
        let plan = json!({"kind":"calls","calls":[
            {"command":"terrain.slope","params":{"dem":"dem/plan.tif","unit":"degrees"}},
            {"command":"layout.export_pdf","params":{"title":"Pente"}}]});
        let c = Client::start_with(CANCEL_GRACE, composer(plan));
        with_dem(&c);
        c.hello_with(&RASTER);
        assert_eq!(c.propose(2)["result"]["plan_id"], "plan-1");
        c.execute(3, "plan-1");
        let notice = c.recv();
        assert_eq!(notice["params"]["command"], "terrain.slope", "{notice}");
        let work = c.dir.join("p.cassini");
        // L'ouverture suit la pente, sans notification d'étape : elle n'est pas une étape du plan.
        answer_open(&c, &work, "1-terrain.slope.tif", "orig");
        let order = c.expect_step("plan-1", 2, "layout.export_pdf");
        assert!(order["params"]["params"].get("leave_out").is_none());
        answer_export(&c, &order);
        let done = c.recv();
        assert_eq!(done["id"], 3, "{done}");
        assert_eq!(done["result"]["steps"].as_array().unwrap().len(), 2);
        let journal = c.journal();
        let ids: Vec<&str> = journal.iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(
            ids,
            ["terrain.slope", "layer.open_raster", "layout.export_pdf"]
        );
        let steps: Vec<&Value> = journal.iter().map(|l| &l["plan_step"]).collect();
        assert_eq!(steps, [&json!(1), &json!(1), &json!(2)]);

        // Deux rejeux dans la même session : le raster d'origine, puis celui du premier rejeu,
        // sont laissés hors de la carte ; chaque rejeu rouvre le sien avant l'export.
        let expected = [json!(["orig"]), json!(["orig", "replay-1"])];
        for (n, leave_out) in expected.iter().enumerate() {
            let id = 4 + n as u64;
            c.send(json!({"jsonrpc":"2.0","id":id,"method":"journal.replay"}));
            let layer = format!("replay-{}", n + 1);
            let order = c.recv();
            assert_eq!(order["params"]["command"], "layer.open_raster", "{order}");
            let path = Path::new(order["params"]["params"]["path"].as_str().unwrap());
            assert_eq!(path.file_name().unwrap(), "1-terrain.slope.tif");
            assert!(path.starts_with(&work) && path != work.join("1-terrain.slope.tif"));
            assert!(path.is_file());
            c.send(reply(
                &order["id"],
                json!({"layer_id": layer, "name": "1-terrain.slope"}),
            ));
            let order = c.recv();
            assert_eq!(order["params"]["params"]["leave_out"], *leave_out);
            answer_export(&c, &order);
            let done = c.recv();
            assert_eq!(done["id"], id);
            assert_eq!(done["result"], json!({"steps": 3, "identical": true}));
        }
        assert_eq!(c.journal().len(), 3);

        // Session neuve : le projet rouvert n'a plus le raster ; le rejeu le recalcule et le rouvre.
        let c = c.reopen(None);
        c.hello_with(&RASTER);
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"journal.replay"}));
        let order = c.recv();
        assert_eq!(order["params"]["command"], "layer.open_raster", "{order}");
        c.send(reply(
            &order["id"],
            json!({"layer_id": "fresh", "name": "1-terrain.slope"}),
        ));
        let order = c.recv();
        assert_eq!(order["params"]["params"]["leave_out"], json!(["orig"]));
        answer_export(&c, &order);
        let done = c.recv();
        assert_eq!(done["result"], json!({"steps": 3, "identical": true}));
    }

    /// Un point au centre du MNT de test, dans son système.
    fn dem_center() -> Value {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/mnt-plan-incline-2154.tif");
        let e = crate::dem::Dem::open(&fixture).unwrap().extent();
        json!({"x": (e.min_x + e.max_x) / 2.0, "y": (e.min_y + e.max_y) / 2.0, "epsg": 2154})
    }

    /// Plan pente puis champ de vision sur la pente, le raster cité par `step:1`.
    fn slope_then_viewshed() -> Value {
        json!({"kind":"calls","calls":[
            {"command":"terrain.slope","params":{"dem":"dem/plan.tif","unit":"degrees"}},
            {"command":"terrain.viewshed","params":{"dem":"step:1","observer":dem_center()}}]})
    }

    // ---- Ticket 11.22 : conversions de points ajoutées au plan par le noyau ----

    /// L'extension annonce la conversion d'un point et l'inventaire.
    const CONVERSION: [&str; 2] = ["data.transform_point", "project.inventory"];

    /// Deux points WGS84 ; le MNT de test est en Lambert-93.
    fn wgs_a() -> Value {
        json!({"x": 2.40, "y": 48.73, "epsg": 4326})
    }

    fn wgs_b() -> Value {
        json!({"x": 2.41, "y": 48.74, "epsg": 4326})
    }

    /// Ligne de vue entre deux points WGS84 sur `dem/plan.tif`, tel que le modèle l'écrit.
    fn line_of_sight_in_wgs84() -> Value {
        json!({"kind":"calls","calls":[
            {"command":"terrain.line_of_sight","params":{"dem":"dem/plan.tif",
                "observer":wgs_a(),"target":wgs_b()}}]})
    }

    /// Répond à l'ordre de conversion comme le geste de QGIS, par le point `to`.
    fn answer_conversion(c: &Client, order: &Value, from: Value, to: &Value) {
        assert_eq!(
            order["params"]["params"],
            json!({"point": from, "target_epsg": 2154})
        );
        c.send(reply(&order["id"], json!({"point": to})));
    }

    #[test]
    fn a_wgs84_point_on_a_lambert_dem_is_converted_by_a_step_the_core_adds() {
        let mut c = Client::start_with(CANCEL_GRACE, composer(line_of_sight_in_wgs84()));
        with_dem(&c);
        c.hello_with(&CONVERSION);
        let proposed = c.propose(2);
        assert_eq!(proposed["result"]["plan_id"], "plan-1", "{proposed}");
        assert_eq!(
            proposed["result"]["plan"],
            json!({"kind":"calls","calls":[
                {"command":"data.transform_point","params":{"point":wgs_a(),"target_epsg":2154}},
                {"command":"data.transform_point","params":{"point":wgs_b(),"target_epsg":2154}},
                {"command":"terrain.line_of_sight","params":{"dem":"dem/plan.tif",
                    "observer":"step:1.point","target":"step:2.point"}}]})
        );
        let (a, b) = (
            json!({"x": 652100.0, "y": 6850100.0, "epsg": 2154}),
            json!({"x": 652300.0, "y": 6850200.0, "epsg": 2154}),
        );
        c.execute(3, "plan-1");
        let order = c.expect_step("plan-1", 1, "data.transform_point");
        answer_conversion(&c, &order, wgs_a(), &a);
        let order = c.expect_step("plan-1", 2, "data.transform_point");
        answer_conversion(&c, &order, wgs_b(), &b);
        let notice = c.recv();
        assert_eq!(notice["params"]["command"], "terrain.line_of_sight");
        let done = c.recv();
        assert_eq!(done["id"], 3, "{done}");
        let steps = done["result"]["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 3);
        assert!(steps[2]["result"]["visible"].is_boolean(), "{done}");
        // Les conversions sont des lignes ordinaires, avec leur place dans le plan ; la ligne de
        // vue garde les points résolus.
        let journal = c.journal();
        let ids: Vec<&str> = journal.iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(
            ids,
            [
                "data.transform_point",
                "data.transform_point",
                "terrain.line_of_sight"
            ]
        );
        let steps: Vec<&Value> = journal.iter().map(|l| &l["plan_step"]).collect();
        assert_eq!(steps, [&json!(1), &json!(2), &json!(3)]);
        assert_eq!(journal[2]["params"]["observer"], a);
        assert_eq!(journal[2]["params"]["target"], b);
        // Le rejeu redemande les deux conversions et redonne la même ligne de vue.
        c.send(json!({"jsonrpc":"2.0","id":4,"method":"journal.replay"}));
        let order = c.recv();
        answer_conversion(&c, &order, wgs_a(), &a);
        let order = c.recv();
        answer_conversion(&c, &order, wgs_b(), &b);
        let replayed = c.recv();
        assert_eq!(
            replayed["result"],
            json!({"steps": 3, "identical": true}),
            "{replayed}"
        );
        c.close().1.unwrap();
    }

    #[test]
    fn a_list_of_wgs84_observers_is_completed_point_by_point() {
        let plan = json!({"kind":"calls","calls":[
            {"command":"terrain.viewshed_cumulative","params":{"dem":"dem/plan.tif",
                "observers":[wgs_a(), dem_center(), wgs_b()]}}]});
        let mut c = Client::start_with(CANCEL_GRACE, composer(plan));
        with_dem(&c);
        c.hello_with(&CONVERSION);
        let proposed = c.propose(2);
        assert_eq!(
            proposed["result"]["plan"]["calls"][2]["params"]["observers"],
            json!(["step:1.point", dem_center(), "step:2.point"]),
            "{proposed}"
        );
        c.close().1.unwrap();
    }

    #[test]
    fn without_the_announced_gesture_or_with_an_unreadable_dem_the_plan_is_unchanged() {
        // Geste non annoncé : le plan reste celui du modèle.
        let mut c = Client::start_with(CANCEL_GRACE, composer(line_of_sight_in_wgs84()));
        with_dem(&c);
        c.hello_with(&["project.inventory"]);
        let proposed = c.propose(2);
        assert_eq!(proposed["result"]["plan"], line_of_sight_in_wgs84());
        c.close().1.unwrap();
        // MNT introuvable : rien n'est ajouté, l'exécution rendra son erreur habituelle.
        let mut c = Client::start_with(CANCEL_GRACE, composer(line_of_sight_in_wgs84()));
        c.hello_with(&CONVERSION);
        let proposed = c.propose(2);
        assert_eq!(proposed["result"]["plan"], line_of_sight_in_wgs84());
        c.close().1.unwrap();
    }

    #[test]
    fn a_direct_call_is_never_completed_and_stays_refused() {
        let mut c = Client::start(CANCEL_GRACE);
        with_dem(&c);
        c.hello_with(&CONVERSION);
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"command.run",
            "params":{"command":"terrain.line_of_sight","params":{"dem":"dem/plan.tif",
                "observer":wgs_a(),"target":wgs_b()}}}));
        let refused = c.recv();
        assert_eq!(code_of(&refused), "terrain.crs_mismatch", "{refused}");
        // Aucune conversion n'a été demandée à l'extension.
        c.assert_silent();
        c.close().1.unwrap();
    }

    #[test]
    fn a_raster_cited_by_step_is_journaled_as_a_marker_and_replays_on_the_new_raster() {
        let mut c = Client::start_with(CANCEL_GRACE, composer(slope_then_viewshed()));
        with_dem(&c);
        c.hello_with(&RASTER);
        let work = c.dir.join("p.cassini");
        // Un appel avant le plan : la pente du plan est la ligne 3 du journal, pas la 1.
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"command.run",
            "params":{"command":"terrain.slope","params":{"dem":"dem/plan.tif","unit":"degrees"}}}));
        answer_open(&c, &work, "1-terrain.slope.tif", "hand");
        assert_eq!(c.recv()["id"], 2);
        assert_eq!(c.propose(3)["result"]["plan_id"], "plan-1");
        c.execute(4, "plan-1");
        assert_eq!(c.recv()["params"]["command"], "terrain.slope");
        answer_open(&c, &work, "3-terrain.slope.tif", "slope");
        assert_eq!(c.recv()["params"]["command"], "terrain.viewshed");
        answer_open(&c, &work, "5-terrain.viewshed.tif", "view");
        let done = c.recv();
        assert_eq!(done["id"], 4, "{done}");
        let journal = c.journal();
        let ids: Vec<&str> = journal.iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(
            ids,
            [
                "terrain.slope",
                "layer.open_raster",
                "terrain.slope",
                "layer.open_raster",
                "terrain.viewshed",
                "layer.open_raster"
            ]
        );
        // Le raster cité est le repère de la ligne 3 : aucun chemin du dossier de travail.
        assert_eq!(journal[4]["params"]["dem"], json!("out:3.1"));
        assert_eq!(journal[4]["inputs"], json!([]));
        assert_eq!(journal[5]["params"], json!({"path": "out:5.1"}));
        let visible = journal[4]["result"]["visible_cells"].clone();
        assert!(visible.as_u64().unwrap() > 0, "{}", journal[4]);

        // Le rejeu résout `out:3.1` vers la pente qu'il vient de recalculer sous son dossier
        // (`Resolver::resolve_reference`) ; la pente d'origine reste, ses octets étant comparés.
        c.send(json!({"jsonrpc":"2.0","id":5,"method":"journal.replay"}));
        for (k, file) in [
            (1, "1-terrain.slope.tif"),
            (3, "3-terrain.slope.tif"),
            (5, "5-terrain.viewshed.tif"),
        ] {
            let order = c.recv();
            assert_eq!(order["params"]["command"], "layer.open_raster", "{order}");
            let path = Path::new(order["params"]["params"]["path"].as_str().unwrap());
            assert_eq!(path.file_name().unwrap(), file);
            assert!(path.starts_with(&work) && path.parent() != Some(work.as_path()));
            c.send(reply(
                &order["id"],
                json!({"layer_id": format!("replay-{k}"), "name": file.trim_end_matches(".tif")}),
            ));
        }
        let done = c.recv();
        assert_eq!(done["id"], 5, "{done}");
        assert_eq!(done["result"], json!({"steps": 6, "identical": true}));
        c.close().1.unwrap();
    }

    #[test]
    fn a_raster_cited_by_step_is_a_marker_without_the_dictated_opening() {
        let mut c = Client::start_with(CANCEL_GRACE, composer(slope_then_viewshed()));
        with_dem(&c);
        c.hello_with(&["project.inventory"]);
        assert_eq!(c.propose(2)["result"]["plan_id"], "plan-1");
        c.execute(3, "plan-1");
        assert_eq!(c.recv()["params"]["command"], "terrain.slope");
        assert_eq!(c.recv()["params"]["command"], "terrain.viewshed");
        let done = c.recv();
        assert_eq!(done["id"], 3, "{done}");
        let journal = c.journal();
        assert_eq!(journal.len(), 2);
        assert_eq!(journal[1]["params"]["dem"], json!("out:1.1"));
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

    // ---- agent.models et le choix du modèle ----

    #[test]
    fn agent_models_lists_the_composer_models_without_journal_or_pending_change() {
        let mut c = Client::start_with(CANCEL_GRACE, composer(three_steps()));
        c.hello_with(&ALL);
        assert_eq!(c.propose(2)["result"]["plan_id"], "plan-1");
        c.send(json!({"jsonrpc":"2.0","id":3,"method":"agent.models"}));
        let r = c.recv();
        assert_eq!(r["id"], 3);
        assert_eq!(
            r["result"]["models"][0],
            json!({"file": "a.gguf", "size_bytes": 5})
        );
        assert_eq!(
            r["result"]["models"][1]["problem"],
            "fichier manquant : absent.gguf"
        );
        // Aucun ordre, rien au journal, et le plan en attente reste exécutable.
        c.assert_silent();
        assert!(c.journal().is_empty());
        c.send(
            json!({"jsonrpc":"2.0","id":4,"method":"plan.reject","params":{"plan_id":"plan-1"}}),
        );
        assert_eq!(c.recv()["result"]["rejected"], true);
        c.close().1.unwrap();
    }

    #[test]
    fn agent_models_without_a_composer_is_unavailable_and_needs_a_session() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello_with(&ALL);
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"agent.models"}));
        assert_eq!(code_of(&c.recv()), "agent.unavailable");
        c.close().1.unwrap();
        let r = exchange(&[json!({"jsonrpc":"2.0","id":1,"method":"agent.models"}).to_string()]);
        assert_eq!(code_of(&r[0]), "session.not_started");
    }

    #[test]
    fn agent_models_during_an_order_is_busy() {
        let mut c = Client::start_with(CANCEL_GRACE, composer(three_steps()));
        c.hello_with(&ALL);
        c.send(json!({"jsonrpc":"2.0","id":2,"method":"plan.propose","params":{"request":"x"}}));
        let order = c.recv();
        assert_eq!(order["params"]["command"], "project.inventory");
        c.send(json!({"jsonrpc":"2.0","id":3,"method":"agent.models"}));
        let busy = c.recv();
        assert_eq!(busy["id"], 3);
        assert_eq!(code_of(&busy), "session.busy");
        c.send(reply(&order["id"], json!({"layers": []})));
        assert_eq!(c.recv()["result"]["plan_id"], "plan-1");
        c.close().1.unwrap();
    }

    #[test]
    fn propose_passes_the_model_to_the_composer_and_refuses_a_bad_type() {
        let composer = composer(three_steps()).unwrap();
        let chosen = composer.chosen.clone();
        let mut c = Client::start_with(CANCEL_GRACE, Some(composer));
        c.hello_with(&ALL);
        assert_eq!(c.propose(2)["result"]["plan_id"], "plan-1");
        c.send(json!({"jsonrpc":"2.0","id":3,"method":"plan.propose",
            "params":{"request":"x","model":"b.gguf"}}));
        let order = c.recv();
        c.send(reply(&order["id"], json!({"layers": []})));
        assert_eq!(c.recv()["result"]["plan_id"], "plan-2");
        // `model: null` vaut absent.
        c.send(json!({"jsonrpc":"2.0","id":6,"method":"plan.propose",
            "params":{"request":"x","model":null}}));
        let order = c.recv();
        c.send(reply(&order["id"], json!({"layers": []})));
        assert_eq!(c.recv()["result"]["plan_id"], "plan-3");
        c.send(json!({"jsonrpc":"2.0","id":4,"method":"plan.propose",
            "params":{"request":"x","model":3}}));
        assert_eq!(c.recv()["error"]["code"], -32602);
        // La proposition refusée a effacé le plan 3, sans rien demander au compositeur.
        c.execute(5, "plan-3");
        assert_eq!(code_of(&c.recv()), "plan.unknown");
        assert_eq!(
            *chosen.lock().unwrap(),
            vec![None, Some("b.gguf".to_string()), None]
        );
        c.close().1.unwrap();
    }

    // --- Mise en forme topographique et dégradé (entrée 11.20). ---

    const STYLE_GESTURES: [&str; 3] = ["project.inventory", "layer.set_style", "layer.arrange"];

    fn available(hello: &Value, id: &str) -> Value {
        hello["result"]["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == id)
            .unwrap()["available"]
            .clone()
    }

    #[test]
    fn the_styling_commands_are_offered_only_with_their_gestures() {
        for (announced, topographic, graduated) in [
            (vec![], false, false),
            (vec!["layer.set_style"], false, true),
            (vec!["project.inventory", "layer.set_style"], false, true),
            (STYLE_GESTURES.to_vec(), true, true),
        ] {
            let mut c = Client::start(CANCEL_GRACE);
            c.send(json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
                "params":{"protocol_version":2,"commands":announced,"project":c.project}}));
            let hello = c.recv();
            assert_eq!(
                available(&hello, "map.topographic_style"),
                json!(topographic)
            );
            assert_eq!(available(&hello, "map.graduated_style"), json!(graduated));
            assert_eq!(available(&hello, "layer.set_style"), json!(false));
            assert_eq!(available(&hello, "layer.arrange"), json!(false));
            if !topographic {
                c.send(
                    run_line(2, "map.topographic_style", json!({}))
                        .parse()
                        .unwrap(),
                );
                let refused = c.recv();
                assert_eq!(code_of(&refused), "style.unavailable", "{refused}");
                c.assert_silent();
                assert!(c.journal().is_empty());
            }
            c.close().1.unwrap();
        }
    }

    #[test]
    fn a_topographic_style_reads_the_inventory_then_dictates_styles_and_the_arrangement() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello_with(&STYLE_GESTURES);
        c.send(
            run_line(2, "map.topographic_style", json!({}))
                .parse()
                .unwrap(),
        );
        let order = c.recv();
        assert_eq!(order["params"]["command"], "project.inventory");
        c.send(reply(&order["id"], json!({"project_crs": "EPSG:2154", "extent": null, "layers": [
            {"layer_id": "routes_1", "name": "routes", "kind": "vector", "geometry": "LineString",
             "crs": "EPSG:2154", "fields": []},
            {"layer_id": "mnt_2", "name": "mnt", "kind": "raster", "geometry": null,
             "crs": "EPSG:2154", "fields": []}
        ]})));
        for layer in ["routes_1", "mnt_2"] {
            let order = c.recv();
            assert_eq!(order["params"]["command"], "layer.set_style", "{order}");
            assert_eq!(order["params"]["params"]["layer_id"], layer);
            c.send(reply(&order["id"], json!({"layer_id": layer})));
        }
        let order = c.recv();
        assert_eq!(order["params"]["command"], "layer.arrange");
        assert_eq!(
            order["params"]["params"]["groups"],
            json!([{"name": "Transport", "layers": ["routes_1"]},
                   {"name": "Relief", "layers": ["mnt_2"]}])
        );
        c.send(reply(
            &order["id"],
            json!({"group_count": 2, "layer_count": 2}),
        ));
        let done = c.recv();
        assert_eq!(done["id"], 2);
        assert!(done["result"].get("orders").is_none(), "{done}");
        assert_eq!(done["result"]["styled"].as_array().unwrap().len(), 2);
        let journal = c.journal();
        let ids: Vec<&str> = journal.iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(
            ids,
            [
                "map.topographic_style",
                "layer.set_style",
                "layer.set_style",
                "layer.arrange"
            ]
        );
        c.close().1.unwrap();
    }

    #[test]
    fn a_graduated_style_dictates_one_set_style_over_the_channel() {
        let mut c = Client::start(CANCEL_GRACE);
        c.hello_with(&["layer.set_style"]);
        c.send(
            run_line(
                2,
                "map.graduated_style",
                json!({"layer_id": "communes_1", "field": "population", "ramp": "blues"}),
            )
            .parse()
            .unwrap(),
        );
        let order = c.recv();
        assert_eq!(order["method"], "order.execute", "{order}");
        assert_eq!(order["params"]["command"], "layer.set_style");
        assert_eq!(order["params"]["params"]["layer_id"], "communes_1");
        assert_eq!(
            order["params"]["params"]["style"],
            json!({"kind": "graduated", "field": "population", "method": "jenks",
                   "class_count": 5, "ramp": {"from": "#f7fbff", "to": "#08306b"}})
        );
        c.send(reply(&order["id"], json!({"layer_id": "communes_1"})));
        let done = c.recv();
        assert_eq!(done["id"], 2);
        assert_eq!(
            done["result"],
            json!({"layer_id": "communes_1", "field": "population", "class_count": 5})
        );
        c.assert_silent();
        let journal = c.journal();
        let ids: Vec<&str> = journal.iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["map.graduated_style", "layer.set_style"]);
        assert!(journal.iter().all(|l| l.get("layer_id").is_none()));
        assert!(!journal[0].to_string().contains("orders"));
        c.close().1.unwrap();
    }
}
