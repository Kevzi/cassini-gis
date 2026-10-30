//! `cassini mcp` (CAP-8) : un client MCP lance le vrai binaire, liste les commandes du registre
//! et les exécute, sans QGIS et sans réseau.

#[path = "support/mcp_client.rs"]
mod mcp_client;

use std::path::PathBuf;
use std::process::Command;

use cassini_core::registry::{Registry, RunsIn};
use mcp_client::{McpClient, BIN};
use serde_json::{json, Value};

fn models_dir() -> String {
    let dir = std::env::temp_dir().join(format!("cassini-mcp-models-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.to_string_lossy().into_owned()
}

fn temp_project() -> (PathBuf, String) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "cassini-mcp-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let project = dir.join("p.qgz").to_string_lossy().replace('\\', "/");
    (dir, project)
}

#[test]
fn tools_list_gives_every_core_command_with_its_schemas() {
    let mut client = McpClient::connect(&[], &models_dir());
    let reply = client.request("tools/list", json!({}));
    let tools = reply["result"]["tools"]
        .as_array()
        .expect("une liste d'outils");
    let registry = Registry::standard();
    let core: Vec<_> = registry
        .specs()
        .filter(|s| s.runs_in == Some(RunsIn::Core))
        .collect();
    assert_eq!(tools.len(), core.len());
    for (tool, spec) in tools.iter().zip(core) {
        assert_eq!(tool["name"], spec.id.as_str());
        assert_eq!(
            tool["description"],
            format!("{}\n{}", spec.description_fr, spec.description_en)
        );
        assert_eq!(tool["inputSchema"], spec.params_schema);
        assert_eq!(tool["outputSchema"], spec.output_schema);
        // MCP exige un schéma d'objet pour l'entrée comme pour la sortie.
        assert_eq!(tool["inputSchema"]["type"], "object", "{}", spec.id);
        assert_eq!(tool["outputSchema"]["type"], "object", "{}", spec.id);
    }
    // Aucune commande déléguée à QGIS, aucune méthode du protocole Cassini.
    for spec in registry.specs().filter(|s| s.runs_in == Some(RunsIn::Qgis)) {
        assert!(tools.iter().all(|t| t["name"] != spec.id.as_str()));
    }
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        assert!(!name.starts_with("session.") && !name.starts_with("plan."));
    }
    client.assert_no_server_request();
    let (status, _) = client.close();
    assert_eq!(status.code(), Some(0));
}

#[test]
fn system_about_gives_the_same_result_as_cassini_run() {
    let mut client = McpClient::connect(&[], &models_dir());
    let reply = client.call_tool("system.about", json!({}));
    let result = &reply["result"];
    assert_eq!(result["isError"], false);
    let run = Command::new(BIN)
        .args(["run", "system.about"])
        .output()
        .unwrap();
    assert!(run.status.success());
    let expected: Value = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(result["structuredContent"], expected);
    let text: Value = serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text, expected);
    client.assert_no_server_request();
    assert_eq!(client.close().0.code(), Some(0));
}

#[test]
fn invalid_params_are_a_tool_error_carrying_the_error_report() {
    let mut client = McpClient::connect(&[], &models_dir());
    let reply = client.call_tool("system.about", json!({"x": 1}));
    assert!(reply.get("error").is_none(), "{reply}");
    let result = &reply["result"];
    assert_eq!(result["isError"], true);
    let report: Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(report["code"], "registry.invalid_params");
    assert!(report["message"].is_string());
    // Le serveur reste vivant.
    let again = client.call_tool("system.about", json!({}));
    assert_eq!(again["result"]["isError"], false);
    client.assert_no_server_request();
    assert_eq!(client.close().0.code(), Some(0));
}

#[test]
fn an_unknown_tool_or_a_qgis_command_is_an_invalid_params_protocol_error() {
    let registry = Registry::standard();
    let qgis = registry
        .specs()
        .find(|s| s.runs_in == Some(RunsIn::Qgis))
        .expect("au moins une commande déléguée")
        .id
        .clone();
    let mut client = McpClient::connect(&[], &models_dir());
    for name in ["foo.bar", qgis.as_str(), "session.hello"] {
        let reply = client.call_tool(name, json!({}));
        assert_eq!(reply["error"]["code"], -32602, "{name} : {reply}");
        assert!(reply.get("result").is_none());
    }
    let alive = client.call_tool("system.about", json!({}));
    assert_eq!(alive["result"]["isError"], false);
    client.assert_no_server_request();
    assert_eq!(client.close().0.code(), Some(0));
}

#[test]
fn a_call_with_a_project_writes_the_journal_and_frees_the_lock() {
    let (dir, project) = temp_project();
    let mut client = McpClient::connect(&["--project", &project], &models_dir());
    for _ in 0..2 {
        let reply = client.call_tool("system.about", json!({}));
        assert_eq!(reply["result"]["isError"], false, "{reply}");
    }
    // Paramètres invalides : rien n'est écrit.
    let bad = client.call_tool("system.about", json!({"x": 1}));
    assert_eq!(bad["result"]["isError"], true);
    let report: Value =
        serde_json::from_str(bad["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(report["code"], "registry.invalid_params");
    client.assert_no_server_request();
    assert_eq!(client.close().0.code(), Some(0));
    let journal = std::fs::read_to_string(dir.join("p.cassini/journal.jsonl")).unwrap();
    assert_eq!(journal.lines().count(), 2);
    let last: Value = serde_json::from_str(journal.lines().last().unwrap()).unwrap();
    assert_eq!(last["id"], "system.about");
    // Le verrou est libéré : une nouvelle session écrit.
    let registry = Registry::standard();
    let session =
        cassini_core::journal::Session::open(&registry, std::path::Path::new(&project)).unwrap();
    assert_eq!(session.mode(), cassini_core::journal::Mode::Write);
    drop(session);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_project_that_cannot_open_fails_with_code_1_before_any_exchange() {
    let (dir, _) = temp_project();
    let project = dir.join("absent").join("manque.qgz");
    let out = Command::new(BIN)
        .args(["mcp", "--project", &project.to_string_lossy()])
        .env("CASSINI_MODELS", models_dir())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let report: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert!(report["code"].as_str().unwrap().starts_with("journal."));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn closing_the_input_ends_the_server_with_code_0() {
    // Avant toute poignée de main.
    let (status, received) = McpClient::spawn(&[], &models_dir()).close();
    assert_eq!(status.code(), Some(0));
    assert!(received.is_empty());
    // Après.
    let client = McpClient::connect(&[], &models_dir());
    let (status, received) = client.close();
    assert_eq!(status.code(), Some(0));
    assert!(received
        .iter()
        .all(|m| !(m.get("method").is_some() && m.get("id").is_some())));
}

// --- Relais (ticket 5.25) : un vrai `cassini serve` derrière le canal d'IA externe, dont le
// test joue l'extension sur l'entrée et la sortie standard, nom isolé par
// `CASSINI_EXTERNAL_TAG`.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{self, Receiver};

use cassini_core::registry::PROTOCOL_VERSION;
use mcp_client::PATIENCE;

/// Les commandes QGIS que la session simulée annonce à `session.hello`.
const ANNOUNCED: [&str; 2] = ["analysis.buffer", "layer.describe"];

/// Paramètres d'un `analysis.buffer` (commande annulable) ; `out` est donné par le noyau.
fn buffer_params() -> Value {
    json!({"layer_id": "gares", "distance": {"value": 500, "unit": "m"}})
}

fn buffered() -> Value {
    json!({"layer_id": "tampon", "name": "Tampon", "feature_count": 3, "crs": "EPSG:2154"})
}

/// Une session QGIS simulée : `cassini serve`, et le test dans le rôle de l'extension.
struct QgisSession {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<Value>,
    unread: VecDeque<Value>,
    next_id: u64,
    tag: String,
    dir: PathBuf,
}

/// Lit les lignes JSON de `stdout` dans un fil, pour attendre avec une échéance.
fn spawn_lines(stdout: impl std::io::Read + Send + 'static) -> Receiver<Value> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { return };
            if line.trim().is_empty() {
                continue;
            }
            if tx.send(serde_json::from_str(&line).unwrap()).is_err() {
                return;
            }
        }
    });
    rx
}

impl QgisSession {
    /// Lance la session, ouvre le projet par `session.hello`, et allume le canal d'IA externe
    /// si `external` est vrai.
    fn start(external: bool) -> QgisSession {
        let (dir, project) = temp_project();
        let tag = format!(
            "relais-{}-{}",
            std::process::id(),
            dir.file_name().unwrap().to_string_lossy()
        );
        let mut child = Command::new(BIN)
            .arg("serve")
            .env("CASSINI_EXTERNAL_TAG", &tag)
            .env("CASSINI_MODELS", models_dir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("le binaire se lance");
        let stdin = child.stdin.take();
        let lines = spawn_lines(child.stdout.take().unwrap());
        let mut session = QgisSession {
            child,
            stdin,
            lines,
            unread: VecDeque::new(),
            next_id: 1,
            tag,
            dir,
        };
        let hello = session.request(
            "session.hello",
            json!({"protocol_version": PROTOCOL_VERSION, "commands": ANNOUNCED, "project": project}),
        );
        assert!(hello["result"].is_object(), "{hello}");
        if external {
            session.switch(true);
        }
        session
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    fn start_request(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        id
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.start_request(method, params);
        self.wait_for(method, |m| {
            m["id"] == json!(id) && m.get("method").is_none()
        })
    }

    /// Lit ce qui arrive encore pendant 300 ms après le dernier message ; le garde à lire.
    fn drain(&mut self) {
        while let Ok(message) = self
            .lines
            .recv_timeout(std::time::Duration::from_millis(300))
        {
            self.unread.push_back(message);
        }
    }

    fn wait_for(&mut self, what: &str, wanted: impl Fn(&Value) -> bool) -> Value {
        if let Some(i) = self.unread.iter().position(&wanted) {
            return self.unread.remove(i).unwrap();
        }
        loop {
            let message = self
                .lines
                .recv_timeout(PATIENCE)
                .unwrap_or_else(|e| panic!("la session n'a rien envoyé ({e}) : {what}"));
            if wanted(&message) {
                return message;
            }
            self.unread.push_back(message);
        }
    }

    /// Allume ou éteint le canal d'IA externe (`session.external`).
    fn switch(&mut self, enabled: bool) {
        let reply = self.request(
            "session.external",
            json!({"enabled": enabled, "ask": false}),
        );
        assert_eq!(reply["result"]["enabled"], enabled, "{reply}");
    }

    /// Le prochain ordre `order.execute` dicté à l'extension.
    fn wait_order(&mut self) -> Value {
        self.wait_for("order.execute", |m| m["method"] == "order.execute")
    }

    fn answer(&mut self, order: &Value, outcome: Value) {
        let mut reply = json!({"jsonrpc": "2.0", "id": order["id"]});
        let key = if outcome.get("code").is_some() && outcome.get("data").is_some() {
            "error"
        } else {
            "result"
        };
        reply[key] = outcome;
        self.send(&reply);
    }

    fn journal(&self) -> Vec<Value> {
        std::fs::read_to_string(self.dir.join("p.cassini/journal.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn close(mut self) {
        drop(self.stdin.take());
        assert!(self.child.wait().unwrap().success());
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn described() -> Value {
    json!({"name": "Gares", "fields": [{"name": "nom", "type": "String"}], "crs": "EPSG:2154",
        "extent": null, "feature_count": 3, "geometry": "Point"})
}

fn tool_names(reply: &Value) -> Vec<String> {
    reply["result"]["tools"]
        .as_array()
        .expect("une liste d'outils")
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect()
}

fn core_names() -> Vec<String> {
    Registry::standard()
        .specs()
        .filter(|s| s.runs_in == Some(RunsIn::Core))
        .map(|s| s.id.clone())
        .collect()
}

/// Le rapport d'erreur porté par un résultat d'outil `isError`.
fn tool_report(reply: &Value) -> Value {
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn the_relay_serves_the_catalogue_of_the_session_and_runs_a_qgis_command() {
    let mut session = QgisSession::start(false);
    let mut client = McpClient::connect_tagged(&[], &models_dir(), &session.tag);
    // Canal éteint : autonome.
    let reply = client.request("tools/list", json!({}));
    assert_eq!(tool_names(&reply), core_names());

    // Canal allumé : le catalogue de la session, commandes QGIS annoncées comprises.
    session.switch(true);
    let reply = client.request("tools/list", json!({}));
    let mut expected = core_names();
    expected.extend(ANNOUNCED.iter().map(|s| s.to_string()));
    expected.sort();
    assert_eq!(tool_names(&reply), expected);
    client.wait_notification("notifications/tools/list_changed");
    let registry = Registry::standard();
    let spec = registry.spec("layer.describe").unwrap();
    let tool = reply["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "layer.describe")
        .unwrap();
    assert_eq!(
        tool["description"],
        format!("{}\n{}", spec.description_fr, spec.description_en)
    );
    assert_eq!(tool["inputSchema"], spec.params_schema);
    assert_eq!(tool["outputSchema"], spec.output_schema);

    // Un appel QGIS : l'ordre est dicté à l'extension ; `ping` est servi pendant l'appel.
    let call = client.start_tool("layer.describe", json!({"layer_id": "l1"}));
    let order = session.wait_order();
    assert_eq!(order["params"]["command"], "layer.describe");
    assert_eq!(order["params"]["params"], json!({"layer_id": "l1"}));
    let ping = client.request("ping", json!({}));
    assert!(ping["result"].is_object(), "{ping}");
    session.answer(&order, described());
    let reply = client.wait(call);
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    assert_eq!(reply["result"]["structuredContent"], described());
    let journal = session.journal();
    let last = journal.last().expect("l'étape est journalisée");
    assert_eq!(last["id"], "layer.describe");
    assert_eq!(last["origin"], "external");

    // Une erreur de l'extension devient un résultat `isError`.
    let call = client.start_tool("layer.describe", json!({"layer_id": "absente"}));
    let order = session.wait_order();
    session.answer(
        &order,
        json!({"code": -32000, "message": "Couche absente.",
            "data": {"code": "qgis.layer_missing", "message": "La couche est absente."}}),
    );
    let report = tool_report(&client.wait(call));
    assert_eq!(report["code"], "qgis.layer_missing");

    // Une commande QGIS non annoncée reste hors catalogue : erreur de protocole.
    let reply = client.call_tool("analysis.intersect", json!({}));
    assert_eq!(reply["error"]["code"], -32602, "{reply}");
    // Aucun autre changement de catalogue.
    assert_eq!(
        client.take_notifications("notifications/tools/list_changed"),
        0
    );
    client.assert_no_server_request();
    assert_eq!(client.close().0.code(), Some(0));
    session.close();
}

#[test]
fn a_busy_session_refuses_the_relay_at_once() {
    let mut session = QgisSession::start(true);
    let mut client = McpClient::connect_tagged(&[], &models_dir(), &session.tag);
    // L'extension tient l'exécutant : un ordre en vol.
    let run = session.start_request(
        "command.run",
        json!({"command": "analysis.buffer", "params": buffer_params()}),
    );
    let order = session.wait_order();
    let report = tool_report(&client.call_tool("layer.describe", json!({"layer_id": "l1"})));
    assert_eq!(report["code"], "session.busy");
    session.answer(&order, buffered());
    session.wait_for("command.run", |m| m["id"] == json!(run));
    client.assert_no_server_request();
    assert_eq!(client.close().0.code(), Some(0));
    session.close();
}

#[test]
fn a_cancelled_mcp_call_becomes_a_request_cancel_and_a_cancelled_step() {
    let mut session = QgisSession::start(true);
    let mut client = McpClient::connect_tagged(&[], &models_dir(), &session.tag);
    let call = client.start_tool("analysis.buffer", buffer_params());
    let order = session.wait_order();
    client.notify(
        "notifications/cancelled",
        json!({"requestId": call, "reason": "test"}),
    );
    // La session transmet l'annulation à l'extension, qui répond à l'ordre.
    let cancel = session.wait_for("request.cancel", |m| m["method"] == "request.cancel");
    assert_eq!(cancel["params"]["id"], order["id"]);
    session.answer(&order, buffered());
    // Le serveur reste vivant, et l'étape est journalisée annulée.
    let ping = client.request("ping", json!({}));
    assert!(ping["result"].is_object(), "{ping}");
    let deadline = std::time::Instant::now() + PATIENCE;
    let last = loop {
        if let Some(last) = session.journal().pop() {
            if last["id"] == "analysis.buffer" {
                break last;
            }
        }
        assert!(std::time::Instant::now() < deadline, "rien au journal");
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    assert_eq!(last["cancelled"], true, "{last}");
    assert_eq!(last["origin"], "external");
    client.assert_no_server_request();
    assert_eq!(client.close().0.code(), Some(0));
    session.close();
}

#[test]
fn switching_the_channel_off_during_a_call_ends_it_and_falls_back_to_autonomous() {
    let mut session = QgisSession::start(true);
    let mut client = McpClient::connect_tagged(&[], &models_dir(), &session.tag);
    let call = client.start_tool("layer.describe", json!({"layer_id": "l1"}));
    let order = session.wait_order();
    // L'utilisateur coupe l'interrupteur pendant l'ordre.
    session.switch(false);
    let report = tool_report(&client.wait(call));
    assert_eq!(report["code"], "external.session_lost");
    client.wait_notification("notifications/tools/list_changed");
    session.answer(&order, described());
    // La requête suivante est servie en autonome.
    let reply = client.call_tool("system.about", json!({}));
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    let reply = client.request("tools/list", json!({}));
    assert_eq!(tool_names(&reply), core_names());
    // Un outil QGIS n'est plus au catalogue.
    let reply = client.call_tool("layer.describe", json!({"layer_id": "l1"}));
    assert_eq!(reply["error"]["code"], -32602, "{reply}");
    assert_eq!(
        client.take_notifications("notifications/tools/list_changed"),
        0
    );
    client.assert_no_server_request();
    assert_eq!(client.close().0.code(), Some(0));
    // Jamais réexécuté : la session n'a reçu qu'un ordre.
    session.drain();
    assert!(session
        .unread
        .iter()
        .all(|m| m["method"] != "order.execute"));
    session.close();
}

#[test]
fn autonomous_mcp_does_not_hold_the_project_lock_between_calls() {
    let (dir, project) = temp_project();
    let mut client = McpClient::connect(&["--project", &project], &models_dir());
    let reply = client.call_tool("system.about", json!({}));
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    // `mcp` vit encore : `cassini run` écrit le journal du même projet.
    let run = Command::new(BIN)
        .args(["run", "system.about", "--project", &project])
        .env("CASSINI_EXTERNAL_TAG", mcp_client::unused_tag())
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let reply = client.call_tool("system.about", json!({}));
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    assert_eq!(client.close().0.code(), Some(0));
    let journal = std::fs::read_to_string(dir.join("p.cassini/journal.jsonl")).unwrap();
    assert_eq!(journal.lines().count(), 3);
    let _ = std::fs::remove_dir_all(dir);
}

/// Une autre session tient le verrou du projet : l'appel autonome répond `journal.read_only` sans
/// rien journaliser ; le verrou libéré, l'appel suivant écrit le journal.
#[test]
fn an_autonomous_call_while_another_session_holds_the_lock_is_read_only() {
    let (dir, project) = temp_project();
    let journal = dir.join("p.cassini/journal.jsonl");
    let lines = || {
        std::fs::read_to_string(&journal)
            .unwrap_or_default()
            .lines()
            .count()
    };
    let registry = Registry::standard();
    let holder =
        cassini_core::journal::Session::open(&registry, std::path::Path::new(&project)).unwrap();
    assert_eq!(holder.mode(), cassini_core::journal::Mode::Write);
    let mut client = McpClient::connect(&["--project", &project], &models_dir());
    let before = lines();
    let report = tool_report(&client.call_tool("system.about", json!({})));
    assert_eq!(report["code"], "journal.read_only");
    assert_eq!(lines(), before);
    drop(holder);
    let reply = client.call_tool("system.about", json!({}));
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    assert_eq!(lines(), before + 1);
    assert_eq!(client.close().0.code(), Some(0));
    let _ = std::fs::remove_dir_all(dir);
}

/// Versions différentes : aucune vraie session ne parle un autre protocole que le relais, le
/// test tient donc lui-même le canal et répond `session.protocol_mismatch` à chaque poignée de
/// main.
#[test]
fn a_protocol_mismatch_keeps_the_core_catalogue_and_refuses_every_call() {
    use cassini_core::external::{ChannelName, Listener};

    let tag = format!("mismatch-{}", std::process::id());
    let name = ChannelName::tagged(&tag).unwrap();
    let mut listener = Listener::open(&name).unwrap();
    let message =
        "Le relais parle le protocole 2, la session QGIS le 3 : mettez à jour le binaire \
                   cassini du relais (cassini mcp).";
    // Trois poignées de main : `tools/list` puis deux `tools/call`.
    let fake = std::thread::spawn(move || {
        for _ in 0..3 {
            let stream = listener.accept().unwrap();
            let mut out = stream.try_clone().unwrap();
            let mut input = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            input.read_line(&mut line).unwrap();
            let attach: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(attach["method"], "external.attach");
            let reply = json!({"jsonrpc": "2.0", "id": attach["id"], "error": {"code": -32000,
                "message": message, "data": {"code": "session.protocol_mismatch",
                "message": message}}});
            writeln!(out, "{reply}").unwrap();
            let mut rest = String::new();
            let _ = std::io::Read::read_to_string(&mut input, &mut rest);
        }
    });
    let mut client = McpClient::connect_tagged(&[], &models_dir(), &tag);
    let reply = client.request("tools/list", json!({}));
    assert_eq!(tool_names(&reply), core_names());
    let report = tool_report(&client.call_tool("system.about", json!({})));
    assert_eq!(report["code"], "session.protocol_mismatch");
    assert_eq!(report["message"], message);
    // Un outil hors du catalogue autonome est refusé de même, avec le message de mise à jour.
    let report = tool_report(&client.call_tool("layer.describe", json!({"layer_id": "l1"})));
    assert_eq!(report["code"], "session.protocol_mismatch");
    assert_eq!(report["message"], message);
    fake.join().unwrap();
    client.assert_no_server_request();
    assert_eq!(client.close().0.code(), Some(0));
}

#[test]
fn get_info_announces_tool_list_changed() {
    let mut client = McpClient::spawn(&[], &models_dir());
    let reply = client.request(
        "initialize",
        json!({"protocolVersion": "2025-11-25", "capabilities": {},
            "clientInfo": {"name": "test", "version": "0"}}),
    );
    assert_eq!(
        reply["result"]["capabilities"]["tools"]["listChanged"],
        true
    );
    assert_eq!(client.close().0.code(), Some(0));
}
