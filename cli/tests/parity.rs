//! Parité des points d'entrée (R3, AD-13) : une même commande du registre donne le
//! même résultat par `cassini run`, par `cassini serve` et par `cassini mcp`, sans QGIS.
//!
//! Le test lance le vrai binaire, pour chaque commande `core` du registre et chacun
//! de ses exemples déclarés, sur deux projets de test de même contenu.

#[path = "support/mcp_client.rs"]
mod mcp_client;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use cassini_core::registry::{Registry, RunsIn};
use serde_json::{json, Value};

const BIN: &str = env!("CARGO_BIN_EXE_cassini");

/// Forme canonique : JSON relu, clés triées à tous les niveaux, sans espace.
fn canonical(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let fields: Vec<String> = keys
                .into_iter()
                .map(|k| format!("{}:{}", json!(k), canonical(&map[k])))
                .collect();
            format!("{{{}}}", fields.join(","))
        }
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", items.join(","))
        }
        other => other.to_string(),
    }
}

/// Copie où un chemin sous le dossier du projet de test devient `<projet>/…` : chaque point
/// d'entrée a son projet, et une sortie du noyau (`file`) y est nommée en absolu.
fn in_project(value: &Value, project: &str) -> Value {
    let dir = project.rsplit_once('/').map_or(project, |(d, _)| d);
    match value {
        Value::String(s) => match s.replace('\\', "/").strip_prefix(&format!("{dir}/")) {
            Some(rest) => Value::String(format!("<projet>/{rest}")),
            None => value.clone(),
        },
        Value::Array(items) => Value::Array(items.iter().map(|v| in_project(v, project)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), in_project(v, project)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Dossier de modèles vide, commun aux deux points d'entrée : `model.list` rend le même `folder`.
fn models_dir() -> String {
    let dir = std::env::temp_dir().join(format!("cassini-parity-models-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.to_string_lossy().into_owned()
}

fn temp_project(label: &str) -> (PathBuf, String) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "cassini-parity-{label}-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    // Les MNT de la recette, pour les exemples des commandes de terrain qui en lisent un.
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures");
    for name in ["mnt-plan-incline-2154.tif", "mnt-colline-2154.tif"] {
        std::fs::copy(fixtures.join(name), dir.join(name)).unwrap();
    }
    let project = dir.join("p.qgz").to_string_lossy().replace('\\', "/");
    (dir, project)
}

/// Résultat canonique de `cassini run <id> <params> --project <projet>`.
fn via_run(id: &str, params: &Value, project: &str) -> String {
    let out = Command::new(BIN)
        .args(["run", id, &params.to_string(), "--project", project])
        .env("CASSINI_MODELS", models_dir())
        .output()
        .expect("le binaire se lance");
    assert!(
        out.status.success(),
        "run {id} a échoué : {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_slice(&out.stdout).expect("run rend du JSON");
    canonical(&in_project(&value, project))
}

/// Résultat canonique de la même commande par `cassini serve` : hello, puis `command.run`.
fn via_serve(id: &str, params: &Value, project: &str) -> String {
    let mut child = Command::new(BIN)
        .arg("serve")
        .env("CASSINI_MODELS", models_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("le binaire se lance");
    let hello = json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
        "params":{"protocol_version":2,"commands":[],"project":project}});
    let call = json!({"jsonrpc":"2.0","id":2,"method":"command.run",
        "params":{"command":id,"params":params}});
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{hello}").unwrap();
    writeln!(stdin, "{call}").unwrap();
    drop(stdin); // l'entrée fermée termine le serveur
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "serve a échoué");
    let reply: Value = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .nth(1)
        .map(|l| serde_json::from_str(l).expect("serve rend du JSON"))
        .expect("serve répond à command.run");
    assert!(reply.get("error").is_none(), "serve {id} : {reply}");
    canonical(&in_project(&reply["result"], project))
}

/// Résultat canonique de la même commande par `cassini mcp --project` : `tools/call`.
fn via_mcp(id: &str, params: &Value, project: &str) -> String {
    let mut client = mcp_client::McpClient::connect(&["--project", project], &models_dir());
    let reply = client.call_tool(id, params.clone());
    client.assert_no_server_request();
    let (status, _) = client.close();
    assert!(status.success(), "mcp a échoué");
    assert_eq!(reply["result"]["isError"], false, "mcp {id} : {reply}");
    canonical(&in_project(&reply["result"]["structuredContent"], project))
}

/// Toutes les paires (commande, paramètres) à comparer ; échoue si une commande
/// `core` n'a aucun exemple, pour qu'aucune n'échappe à la parité.
fn core_calls() -> Vec<(String, Value)> {
    let registry = Registry::standard();
    let mut calls = Vec::new();
    for spec in registry.specs() {
        // Une mise en forme demande les gestes de QGIS (entrée 11.20) : sans QGIS, elle est
        // refusée par `style.unavailable` partout, ce que couvrent les essais du noyau.
        if spec.runs_in != Some(RunsIn::Core) || !spec.required_gestures().is_empty() {
            continue;
        }
        assert!(
            !spec.examples.is_empty(),
            "La commande {} n'a pas d'exemple : la parité ne peut pas la vérifier.",
            spec.id
        );
        for example in &spec.examples {
            calls.push((spec.id.clone(), example.clone()));
        }
    }
    assert!(!calls.is_empty(), "aucune commande core dans le registre");
    calls
}

/// Compare deux résultats canoniques ; l'erreur nomme la commande et les deux versions.
fn compare(id: &str, params: &Value, run: &str, serve: &str) -> Result<(), String> {
    if run.as_bytes() == serve.as_bytes() {
        Ok(())
    } else {
        Err(format!(
            "{id} {params} : résultats différents, run={run} serve={serve}"
        ))
    }
}

fn cleanup(dirs: &[&Path]) {
    for dir in dirs {
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn every_core_command_gives_the_same_result_through_run_serve_and_mcp() {
    for (id, params) in core_calls() {
        let (dir_run, project_run) = temp_project("run");
        let (dir_serve, project_serve) = temp_project("serve");
        let (dir_mcp, project_mcp) = temp_project("mcp");
        let a = via_run(&id, &params, &project_run);
        let b = via_serve(&id, &params, &project_serve);
        let c = via_mcp(&id, &params, &project_mcp);
        cleanup(&[&dir_run, &dir_serve, &dir_mcp]);
        compare(&id, &params, &a, &b).unwrap();
        compare(&id, &params, &a, &c).unwrap();
        assert!(!a.is_empty());
    }
}

#[test]
fn a_result_altered_by_one_entry_point_is_detected() {
    let (id, params) = core_calls().remove(0);
    let (dir, project) = temp_project("tamper");
    let honest = via_run(&id, &params, &project);
    cleanup(&[&dir]);
    // Un point d'entrée qui altérerait le résultat d'un seul octet est refusé.
    let mut tampered = honest.clone();
    tampered.pop();
    tampered.push(' ');
    assert!(compare(&id, &params, &honest, &tampered).is_err());
    assert!(compare(&id, &params, &honest, &honest).is_ok());
    // Des clés dans un autre ordre ne sont pas une altération.
    let reordered: Value = serde_json::from_str(r#"{"b":1,"a":{"d":2,"c":3}}"#).unwrap();
    let sorted: Value = serde_json::from_str(r#"{"a":{"c":3,"d":2},"b":1}"#).unwrap();
    assert_eq!(canonical(&reordered), canonical(&sorted));
}
