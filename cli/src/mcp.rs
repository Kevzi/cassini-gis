//! `cassini mcp` : le serveur MCP sur l'entrée et la sortie standard (CAP-8, AD-3, AD-13).
//!
//! Le serveur n'a pas d'état propre : il traduit un appel MCP en appel du registre, et la
//! réponse du registre en résultat MCP. Chaque outil est une commande `runs_in: core` de
//! `Registry::standard()` ; aucune méthode du protocole Cassini n'est exposée (AD-6), aucun plan
//! n'est composé (AD-23, hors règle), et le serveur ne fait que répondre (AD-3). Seul transport :
//! les flux standard du processus (AD-14).

use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};

use cassini_core::error::ErrorReport;
use cassini_core::exec::Router;
use cassini_core::journal::Session;
use cassini_core::registry::{Registry, RunsIn};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::{MaybeSendFuture, QuitReason, RequestContext, ServerInitializeError};
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};
use serde_json::{json, Value};

/// Réponse du registre à un appel : le résultat, ou l'erreur de AD-21.
type Reply = Result<Value, ErrorReport>;

/// Un appel transmis au fil d'exécution.
struct Job {
    id: String,
    params: Value,
    reply: mpsc::Sender<Reply>,
}

/// Le catalogue d'outils : une entrée par commande `runs_in: core`, schémas du registre.
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

/// Le schéma du registre en objet JSON ; un schéma qui n'est pas un objet ne devrait pas exister
/// (schemars rend toujours un objet), il devient alors le schéma vide.
fn object(schema: &Value) -> JsonObject {
    schema.as_object().cloned().unwrap_or_default()
}

/// Le gestionnaire MCP : le catalogue, et l'émetteur vers le fil qui tient le registre (et la
/// session, qui n'est pas `Send`).
struct CassiniMcp {
    tools: Vec<Tool>,
    exposed: BTreeSet<String>,
    jobs: mpsc::Sender<Job>,
}

impl CassiniMcp {
    fn new(registry: &Registry, jobs: mpsc::Sender<Job>) -> Self {
        let tools = catalogue(registry);
        let exposed = tools.iter().map(|t| t.name.to_string()).collect();
        CassiniMcp {
            tools,
            exposed,
            jobs,
        }
    }

    /// Exécute un appel ; une demande hors catalogue est une erreur de protocole, une erreur de
    /// la commande est un résultat d'outil (`isError`), que l'IA externe peut lire et corriger.
    fn call(&self, request: CallToolRequestParams) -> Result<CallToolResult, McpError> {
        let name = request.name.to_string();
        if !self.exposed.contains(&name) {
            return Err(McpError::invalid_params(
                format!(
                    "Outil inconnu ou réservé à l'extension QGIS : « {name} » (seules les \
                     commandes du noyau sont exposées)."
                ),
                None,
            ));
        }
        let params = request.arguments.map(Value::Object).unwrap_or(json!({}));
        let (reply, answer) = mpsc::channel();
        let job = Job {
            id: name,
            params,
            reply,
        };
        let outcome = self
            .jobs
            .send(job)
            .ok()
            .and_then(|()| answer.recv().ok())
            .ok_or_else(|| McpError::internal_error("le fil d'exécution est arrêté", None))?;
        Ok(match outcome {
            Ok(value) => CallToolResult::structured(value),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(
                serde_json::to_string(&e).expect("une erreur se sérialise"),
            )]),
        })
    }
}

impl ServerHandler for CassiniMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("cassini", cassini_core::VERSION))
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + MaybeSendFuture + '_
    {
        std::future::ready(Ok(ListToolsResult::with_all_items(self.tools.clone())))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResponse, McpError>> + MaybeSendFuture + '_
    {
        // L'appel est servi sur place : le fil d'exécution ne dépend pas du runtime, l'attente
        // ne peut pas s'interbloquer, et les appels sont exécutés un à la fois, dans l'ordre.
        std::future::ready(self.call(request).map(CallToolResponse::from))
    }
}

/// Fil d'exécution : tient le registre, et la session sur le projet s'il y en a un, par le
/// même chemin que `cassini run` (journalisé avec `--project`, AD-9/AD-10). Annonce d'abord
/// l'ouverture sur `opened`, puis sert les appels jusqu'à la fermeture du canal.
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
        Some(project) => match Session::open(&registry, &project) {
            Err(e) => {
                let _ = opened.send(Err(ErrorReport::from(e)));
            }
            Ok(mut session) => {
                let _ = opened.send(Ok(()));
                for job in jobs {
                    let _ = job.reply.send(session.call(&job.id, job.params));
                }
            }
        },
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

/// `cassini mcp [--project <fichier>]` : sert le registre sur stdio jusqu'à la fin du flux.
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
    let handler = CassiniMcp::new(&Registry::standard(), jobs);
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
