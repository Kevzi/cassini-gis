//! `cassini mcp` : le serveur MCP sur l'entrée et la sortie standard (CAP-8, AD-3, AD-13).
//!
//! Le serveur a deux modes, choisis à chaque `tools/list` et `tools/call` (AD-13, ticket 5.25) :
//!
//! - **relais** quand le canal d'IA externe d'une session QGIS répond à `external.attach` : le
//!   catalogue et les schémas viennent de la session, commandes QGIS comprises, et chaque appel
//!   lui est transmis par `command.run` ; le serveur ne décide rien et n'exécute rien (AD-29) ;
//!   `--project` y est ignoré ;
//! - **autonome** sinon : chaque outil est une commande `runs_in: core` de `Registry::standard()`
//!   (ticket 5.7), et le projet de `--project` n'est ouvert que le temps d'un appel.
//!
//! Tout changement de mode ou de catalogue est annoncé par `notifications/tools/list_changed`.
//! Aucune méthode du protocole Cassini n'est exposée (AD-6), aucun plan n'est composé (AD-23, hors
//! règle), et le serveur ne fait que répondre (AD-3). Transports : les flux standard du processus
//! côté client, le canal d'IA externe côté session, rien d'autre (AD-14). Le runtime n'attend
//! jamais un appel de façon bloquante : `ping` et annulation sont servis pendant une commande.

mod relay;

use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::{mpsc, Arc, Weak};
use std::task::Poll;

use cassini_core::error::ErrorReport;
use cassini_core::exec::Router;
use cassini_core::external::ChannelName;
use cassini_core::journal::Session;
use cassini_core::registry::{Registry, RunsIn};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorCode,
    Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities,
    ServerConfig, Tool,
};
use rmcp::service::{QuitReason, RequestContext, ServerInitializeError};
use rmcp::{ErrorData as McpError, Peer, RoleServer, ServerHandler};
use serde_json::{json, Value};
use tokio::sync::{oneshot, Mutex};

use relay::{Answer, AttachError, Relay};

/// Réponse du registre à un appel : le résultat, ou l'erreur de AD-21.
type Reply = Result<Value, ErrorReport>;

/// Un appel transmis au fil d'exécution (mode autonome).
struct Job {
    id: String,
    params: Value,
    reply: oneshot::Sender<Reply>,
}

/// Le catalogue d'outils autonome : une entrée par commande `runs_in: core`, schémas du registre.
pub(crate) fn catalogue(registry: &Registry) -> Vec<Tool> {
    registry
        .specs()
        .filter(|s| s.runs_in == Some(RunsIn::Core))
        .map(|s| {
            let description = format!("{}\n{}", s.description_fr, s.description_en);
            Tool::new(
                s.id.clone(),
                description,
                Arc::new(object(&s.params_schema)),
            )
            .with_raw_output_schema(Arc::new(object(&s.output_schema)))
        })
        .collect()
}

/// Le catalogue d'outils du relais, tiré du catalogue d'`external.attach` : une entrée par
/// commande `available`, de même forme que le catalogue autonome (nom `id`, description en
/// français puis en anglais, schémas de la session). Le registre propre du relais n'est jamais lu.
pub(crate) fn catalogue_from_session(commands: &[Value]) -> Vec<Tool> {
    commands
        .iter()
        .filter(|entry| entry["available"] == json!(true))
        .filter_map(|entry| {
            let id = entry["id"].as_str()?;
            let text = |key: &str| entry[key].as_str().unwrap_or_default().to_string();
            let description = format!("{}\n{}", text("description_fr"), text("description_en"));
            let tool = Tool::new(
                id.to_string(),
                description,
                Arc::new(object(&entry["params_schema"])),
            );
            Some(match entry["output_schema"].as_object() {
                Some(output) => tool.with_raw_output_schema(Arc::new(output.clone())),
                None => tool,
            })
        })
        .collect()
}

/// Le schéma en objet JSON ; un schéma qui n'est pas un objet ne devrait pas exister (schemars
/// rend toujours un objet), il devient alors le schéma vide.
fn object(schema: &Value) -> JsonObject {
    schema.as_object().cloned().unwrap_or_default()
}

/// Le résultat d'outil `isError` qui porte une erreur de AD-21, en JSON (comme en 5.7).
fn tool_error(report: &ErrorReport) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(
        serde_json::to_string(report).expect("une erreur se sérialise"),
    )])
}

/// L'appel commencé en relais dont la session a été perdue : jamais réexécuté en autonome.
fn session_lost() -> ErrorReport {
    ErrorReport {
        code: "external.session_lost".to_string(),
        message: "La session QGIS a été perdue pendant l'appel (canal d'IA externe fermé ou \
                  QGIS arrêté) : l'appel n'a pas été réexécuté ailleurs."
            .to_string(),
        step: None,
    }
}

/// Traduit la réponse de la session (AD-29) : un résultat devient un résultat structuré, une
/// erreur de AD-21 un résultat `isError`, une erreur de protocole une erreur de protocole, une
/// perte `external.session_lost`.
fn translate(answer: Answer) -> Result<CallToolResult, McpError> {
    match answer {
        Answer::Result(value) => Ok(CallToolResult::structured(value)),
        Answer::Error(error) => match relay::report(&error) {
            Some(report) => Ok(tool_error(&report)),
            None => {
                let code = error["code"]
                    .as_i64()
                    .and_then(|c| i32::try_from(c).ok())
                    .unwrap_or(-32603);
                let message = error["message"].as_str().unwrap_or_default().to_string();
                Err(McpError::new(ErrorCode(code), message, None))
            }
        },
        Answer::Lost => Ok(tool_error(&session_lost())),
    }
}

/// L'issue de `first` : le premier des deux futurs prêt.
enum Either<A, B> {
    Left(A),
    Right(B),
}

/// Attend le premier de deux futurs, sans la macro `select!` (pas de feature `macros`).
async fn first<A: Future, B: Future>(a: A, b: B) -> Either<A::Output, B::Output> {
    let mut a = pin!(a);
    let mut b = pin!(b);
    std::future::poll_fn(move |cx| {
        if let Poll::Ready(v) = a.as_mut().poll(cx) {
            return Poll::Ready(Either::Left(v));
        }
        if let Poll::Ready(v) = b.as_mut().poll(cx) {
            return Poll::Ready(Either::Right(v));
        }
        Poll::Pending
    })
    .await
}

/// Le mode choisi pour une requête.
enum Mode {
    Autonomous,
    Relay(Arc<Relay>),
    /// La session parle un autre protocole : catalogue autonome, et chaque appel refusé.
    Mismatch(ErrorReport),
}

/// Ce que le choix du mode garde d'une requête à l'autre.
#[derive(Default)]
struct Modes {
    /// La connexion à la session, vivante ou non.
    relay: Option<Arc<Relay>>,
    /// Le dernier catalogue servi ou annoncé, sérialisé ; aucun avant la première requête.
    announced: Option<Value>,
    /// Le client, pour lui annoncer un changement depuis le fil lecteur du relais.
    peer: Option<Peer<RoleServer>>,
}

/// Le gestionnaire MCP : le catalogue autonome, l'émetteur vers le fil qui tient le registre (et
/// la session, qui n'est pas `Send`), et le canal d'IA externe de l'utilisateur.
struct CassiniMcp {
    autonomous: Vec<Tool>,
    jobs: mpsc::Sender<Job>,
    /// `None` : le nom du canal n'a pas pu être résolu, le serveur reste autonome.
    channel: Option<ChannelName>,
    modes: Arc<Mutex<Modes>>,
}

fn fingerprint(tools: &[Tool]) -> Value {
    serde_json::to_value(tools).unwrap_or(Value::Null)
}

impl CassiniMcp {
    fn new(registry: &Registry, jobs: mpsc::Sender<Job>, channel: Option<ChannelName>) -> Self {
        CassiniMcp {
            autonomous: catalogue(registry),
            jobs,
            channel,
            modes: Default::default(),
        }
    }

    /// Choisit le mode de la requête (AD-13) : connexion vivante, relais ; sinon tentative de
    /// branchement, relais ou autonome. Annonce `list_changed` si le catalogue servi diffère du
    /// dernier annoncé.
    async fn decide(&self, peer: &Peer<RoleServer>) -> (Mode, Vec<Tool>) {
        let mut modes = self.modes.lock().await;
        modes.peer = Some(peer.clone());
        let mode = match modes.relay.clone().filter(|r| r.alive()) {
            Some(relay) => Mode::Relay(relay),
            None => {
                modes.relay = None;
                match self.attach().await {
                    Ok(relay) => {
                        modes.relay = Some(relay.clone());
                        Mode::Relay(relay)
                    }
                    Err(AttachError::Mismatch(report)) => Mode::Mismatch(report),
                    Err(AttachError::Unavailable) => Mode::Autonomous,
                }
            }
        };
        let tools = match &mode {
            Mode::Relay(relay) => catalogue_from_session(relay.commands()),
            Mode::Autonomous | Mode::Mismatch(_) => self.autonomous.clone(),
        };
        let served = fingerprint(&tools);
        if modes.announced.as_ref().is_some_and(|a| *a != served) {
            let _ = peer.notify_tool_list_changed().await;
        }
        modes.announced = Some(served);
        (mode, tools)
    }

    /// Tente le branchement au canal, hors du runtime (la connexion et la poignée de main sont
    /// bloquantes). Branché, le relais annonce sa perte depuis son fil lecteur.
    async fn attach(&self) -> Result<Arc<Relay>, AttachError> {
        let Some(name) = self.channel.clone() else {
            return Err(AttachError::Unavailable);
        };
        let relay = tokio::task::spawn_blocking(move || Relay::attach(&name))
            .await
            .unwrap_or(Err(AttachError::Unavailable))?;
        let relay = Arc::new(relay);
        let handle = tokio::runtime::Handle::current();
        let modes = Arc::downgrade(&self.modes);
        let lost = Arc::downgrade(&relay);
        let autonomous = fingerprint(&self.autonomous);
        relay.on_lost(move || {
            handle.spawn(lost_relay(modes, lost, autonomous));
        });
        Ok(relay)
    }

    /// Exécute un appel ; une demande hors catalogue du mode courant est une erreur de protocole,
    /// une erreur de la commande est un résultat d'outil (`isError`), que l'IA externe peut lire.
    async fn call(
        &self,
        request: CallToolRequestParams,
        context: &RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let (mode, tools) = self.decide(&context.peer).await;
        // Versions différentes : chaque appel est refusé par le message qui nomme le binaire à
        // mettre à jour, quel que soit l'outil.
        if let Mode::Mismatch(report) = &mode {
            return Ok(tool_error(report));
        }
        let name = request.name.to_string();
        if !tools.iter().any(|t| t.name == name) {
            let scope = match mode {
                Mode::Relay(_) => "les commandes disponibles de la session QGIS",
                _ => {
                    "les commandes du noyau, aucune session QGIS n'ayant ouvert son canal d'IA \
                      externe"
                }
            };
            return Err(McpError::invalid_params(
                format!("Outil inconnu : « {name} » (seules {scope} sont exposées)."),
                None,
            ));
        }
        let params = request.arguments.map(Value::Object).unwrap_or(json!({}));
        match mode {
            Mode::Autonomous => self.call_autonomous(name, params, context).await,
            Mode::Relay(relay) => call_relay(&relay, &name, params, context).await,
            Mode::Mismatch(report) => Ok(tool_error(&report)),
        }
    }

    /// Mode autonome : l'appel part au fil d'exécution ; le runtime attend sa réponse sans
    /// bloquer, ou l'annulation du client (la réponse est alors abandonnée).
    async fn call_autonomous(
        &self,
        id: String,
        params: Value,
        context: &RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let stopped = || McpError::internal_error(WORKER_STOPPED, None);
        let (reply, answer) = oneshot::channel();
        self.jobs
            .send(Job { id, params, reply })
            .map_err(|_| stopped())?;
        match first(answer, context.ct.cancelled()).await {
            Either::Left(Ok(Ok(value))) => Ok(CallToolResult::structured(value)),
            Either::Left(Ok(Err(e))) => Ok(tool_error(&e)),
            Either::Left(Err(_)) => Err(stopped()),
            // Le client a annulé : il n'attend plus de réponse (le service l'abandonne).
            Either::Right(()) => Err(McpError::internal_error("appel annulé", None)),
        }
    }
}

/// Mode relais : `command.run` à la session, puis attente de sa réponse ou de l'annulation du
/// client, qui devient `request.cancel` ; la réponse est alors attendue jusqu'au bout.
async fn call_relay(
    relay: &Relay,
    name: &str,
    params: Value,
    context: &RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    let (id, mut answer) = relay.run(name, params);
    let outcome = match first(&mut answer, context.ct.cancelled()).await {
        Either::Left(outcome) => outcome,
        Either::Right(()) => {
            if let Some(id) = id {
                relay.cancel(id);
            }
            answer.await
        }
    };
    translate(outcome.unwrap_or(Answer::Lost))
}

/// La perte d'un relais, vue du runtime : s'il est encore le relais courant, il est oublié, le
/// catalogue autonome devient le dernier annoncé et le client reçoit `list_changed`.
async fn lost_relay(modes: Weak<Mutex<Modes>>, lost: Weak<Relay>, autonomous: Value) {
    let Some(modes) = modes.upgrade() else {
        return;
    };
    let mut modes = modes.lock().await;
    let current = modes
        .relay
        .as_ref()
        .is_some_and(|r| std::ptr::eq(Arc::as_ptr(r), lost.as_ptr()));
    if !current {
        return;
    }
    modes.relay = None;
    if modes.announced.as_ref() != Some(&autonomous) {
        modes.announced = Some(autonomous);
        if let Some(peer) = modes.peer.clone() {
            let _ = peer.notify_tool_list_changed().await;
        }
    }
}

impl ServerHandler for CassiniMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        )
        .with_server_info(Implementation::new("cassini", cassini_core::VERSION))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let (_, tools) = self.decide(&context.peer).await;
        Ok(ListToolsResult::with_all_items(tools))
    }

    /// Chaque requête est une tâche du runtime : l'attente de l'appel le laisse libre de servir
    /// `ping`, l'annulation et les autres requêtes.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        self.call(request, &context)
            .await
            .map(CallToolResponse::from)
    }
}

/// Fil d'exécution du mode autonome : tient le registre et, avec `--project`, ouvre la session
/// sur le projet le temps de chaque appel, par le même chemin que `cassini run` (journalisé,
/// AD-9/AD-10) : le verrou du projet n'est jamais tenu entre deux appels (AD-13). Annonce d'abord
/// sur `opened` si le projet s'ouvre, puis sert les appels jusqu'à la fermeture du canal.
fn worker(
    project: Option<PathBuf>,
    opened: mpsc::Sender<Result<(), ErrorReport>>,
    jobs: mpsc::Receiver<Job>,
) {
    let registry = Registry::standard();
    match project {
        None => {
            let _ = opened.send(Ok(()));
            let router = Router::new(&registry);
            for job in jobs {
                let _ = job
                    .reply
                    .send(router.call(&job.id, job.params).map_err(ErrorReport::from));
            }
        }
        Some(project) => {
            // Un projet introuvable est refusé au démarrage, avant tout échange.
            if let Err(e) = Session::open(&registry, &project) {
                let _ = opened.send(Err(ErrorReport::from(e)));
                return;
            }
            let _ = opened.send(Ok(()));
            for job in jobs {
                // La session est refermée, verrou libéré, avant que la réponse parte.
                let outcome = match Session::open(&registry, &project) {
                    Ok(mut session) => session.call(&job.id, job.params),
                    Err(e) => Err(ErrorReport::from(e)),
                };
                let _ = job.reply.send(outcome);
            }
        }
    }
}

/// Arguments de `mcp` : rien, ou `--project <fichier>`.
pub(crate) fn parse(args: &[String]) -> Option<Option<PathBuf>> {
    match args {
        [] => Some(None),
        [flag, project] if flag == "--project" => Some(Some(PathBuf::from(project))),
        _ => None,
    }
}

/// `cassini mcp [--project <fichier>]` : sert le relais ou le registre sur stdio jusqu'à la fin
/// du flux.
/// Rend 0 quand le client ferme l'entrée, 1 si le projet ne s'ouvre pas (avant tout échange) ou
/// si le canal casse.
pub(crate) fn serve(project: Option<PathBuf>, err: &mut dyn Write) -> i32 {
    let (jobs, inbox) = mpsc::channel();
    let (opened, opening) = mpsc::channel();
    let thread = std::thread::spawn(move || worker(project, opened, inbox));
    match opening.recv() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            let _ = thread.join();
            let _ = writeln!(
                err,
                "{}",
                serde_json::to_string(&e).expect("une erreur se sérialise")
            );
            return 1;
        }
        Err(_) => {
            let _ = thread.join();
            return io_failure(err, WORKER_STOPPED);
        }
    }
    // Sans nom de canal résolu, le serveur reste autonome.
    let channel = ChannelName::for_user().ok();
    let handler = CassiniMcp::new(&Registry::standard(), jobs, channel);
    let modes = handler.modes.clone();
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => return io_failure(err, &e.to_string()),
    };
    let outcome = runtime.block_on(async {
        match rmcp::serve_server(handler, (tokio::io::stdin(), tokio::io::stdout())).await {
            Ok(running) => match running.waiting().await {
                // La tâche du service a paniqué : ce n'est pas une fin normale.
                Ok(QuitReason::JoinError(e)) | Err(e) => Err(e.to_string()),
                Ok(_) => Ok(()),
            },
            // Le client a fermé l'entrée avant la poignée de main : fin normale.
            Err(ServerInitializeError::ConnectionClosed(_)) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    });
    // Le relais branché est fermé, son fil lecteur attendu.
    if let Ok(mut modes) = modes.try_lock() {
        if let Some(relay) = modes.relay.take() {
            relay.shutdown();
        }
    }
    // La lecture de l'entrée standard tient un fil bloquant : on ne l'attend pas. Le
    // gestionnaire, donc l'émetteur, part avec le runtime ; le fil d'exécution s'arrête alors
    // et libère le verrou du projet avant la sortie.
    runtime.shutdown_background();
    // Un fil d'exécution qui a paniqué n'est pas une fin normale, même si le service a fini.
    let worker_failed = thread.join().is_err();
    match outcome {
        Err(detail) => io_failure(err, &detail),
        Ok(()) if worker_failed => io_failure(err, WORKER_STOPPED),
        Ok(()) => 0,
    }
}

/// Détail rendu quand le fil d'exécution s'est arrêté de lui-même.
const WORKER_STOPPED: &str = "le fil d'exécution s'est arrêté";

fn io_failure(err: &mut dyn Write, detail: &str) -> i32 {
    let report = ErrorReport {
        code: "mcp.io".to_string(),
        message: format!("Le canal MCP est interrompu (détail technique : {detail})."),
        step: None,
    };
    let _ = writeln!(
        err,
        "{}",
        serde_json::to_string(&report).expect("une erreur se sérialise")
    );
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_holds_every_core_command_and_no_qgis_one() {
        let registry = Registry::standard();
        let tools = catalogue(&registry);
        let core: Vec<&str> = registry
            .specs()
            .filter(|s| s.runs_in == Some(RunsIn::Core))
            .map(|s| s.id.as_str())
            .collect();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert_eq!(names, core);
        for tool in &tools {
            let spec = registry.spec(&tool.name).unwrap();
            assert_eq!(
                Value::Object((*tool.input_schema).clone()),
                spec.params_schema
            );
            assert_eq!(
                tool.output_schema.as_deref().cloned().map(Value::Object),
                Some(spec.output_schema.clone())
            );
            assert_eq!(
                tool.description.as_deref(),
                Some(format!("{}\n{}", spec.description_fr, spec.description_en).as_str())
            );
        }
        assert!(registry
            .specs()
            .any(|s| s.runs_in == Some(RunsIn::Qgis) && !names.contains(&s.id.as_str())));
    }

    /// Le catalogue d'un `external.attach`, tel que la session le rend : chaque entrée
    /// disponible devient un outil de même forme que le catalogue autonome ; une entrée
    /// indisponible n'en devient aucun.
    #[test]
    fn the_relay_catalogue_comes_from_the_attach_entries() {
        let registry = Registry::standard();
        let mut commands: Vec<Value> = registry
            .specs()
            .map(|s| {
                let mut entry = serde_json::to_value(s).unwrap();
                entry["available"] = json!(s.id != "analysis.intersect");
                entry
            })
            .collect();
        commands.push(json!({"id": "x.unavailable", "available": false}));
        let tools = catalogue_from_session(&commands);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        let expected: Vec<&str> = registry
            .specs()
            .map(|s| s.id.as_str())
            .filter(|id| *id != "analysis.intersect")
            .collect();
        assert_eq!(names, expected);
        for tool in &tools {
            let spec = registry.spec(&tool.name).unwrap();
            assert_eq!(
                Value::Object((*tool.input_schema).clone()),
                spec.params_schema
            );
            assert_eq!(
                tool.output_schema.as_deref().cloned().map(Value::Object),
                Some(spec.output_schema.clone())
            );
            assert_eq!(
                tool.description.as_deref(),
                Some(format!("{}\n{}", spec.description_fr, spec.description_en).as_str())
            );
        }
        // Le catalogue autonome est le sous-ensemble `core` de la même forme.
        let core: Vec<Value> = catalogue(&registry)
            .iter()
            .map(|t| serde_json::to_value(t).unwrap())
            .collect();
        let relayed: Vec<Value> = tools
            .iter()
            .filter(|t| registry.spec(&t.name).unwrap().runs_in == Some(RunsIn::Core))
            .map(|t| serde_json::to_value(t).unwrap())
            .collect();
        assert_eq!(core, relayed);
    }

    #[test]
    fn a_session_answer_is_translated_for_the_mcp_client() {
        let ok = translate(Answer::Result(json!({"a": 1}))).unwrap();
        assert_eq!(ok.is_error, Some(false));
        assert_eq!(ok.structured_content, Some(json!({"a": 1})));

        let error = json!({"code": -32000, "message": "m",
            "data": {"code": "session.busy", "message": "La session est occupée."}});
        let busy = translate(Answer::Error(error)).unwrap();
        assert_eq!(busy.is_error, Some(true));
        let text = serde_json::to_value(&busy.content[0]).unwrap();
        let report: Value = serde_json::from_str(text["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            report,
            json!({"code": "session.busy", "message": "La session est occupée."})
        );

        let protocol = translate(Answer::Error(json!({"code": -32602, "message": "x"})));
        assert_eq!(protocol.unwrap_err().code, ErrorCode(-32602));

        let lost = translate(Answer::Lost).unwrap();
        assert_eq!(lost.is_error, Some(true));
        let text = serde_json::to_value(&lost.content[0]).unwrap();
        assert!(text["text"]
            .as_str()
            .unwrap()
            .contains("external.session_lost"));
    }

    #[test]
    fn mcp_takes_nothing_or_one_project() {
        let s = |v: &[&str]| v.iter().map(|a| a.to_string()).collect::<Vec<_>>();
        assert_eq!(parse(&s(&[])), Some(None));
        assert_eq!(
            parse(&s(&["--project", "p.qgz"])),
            Some(Some(PathBuf::from("p.qgz")))
        );
        for bad in [
            &["--project"][..],
            &["x"][..],
            &["--project", "a", "--project", "b"][..],
            &["--bogus", "a"][..],
        ] {
            assert_eq!(parse(&s(bad)), None, "{bad:?}");
        }
    }
}
