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
