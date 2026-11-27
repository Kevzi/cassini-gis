//! Preuve par l'exemple (entrée 2.14) : `exemple.echo` passe par les quatre points d'entrée du
//! programme composé, avec le même résultat, journalisée et rejouée ; le programme officiel,
//! lui, nomme la commande manquante au lieu de la sauter ; l'annonce dit qui est branché.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

const EXEMPLE: &str = env!("CARGO_BIN_EXE_programme-exemple");

fn temp_project() -> (PathBuf, String) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "cassini-exemple-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let project = dir.join("p.qgz").to_string_lossy().replace('\\', "/");
    (dir, project)
}

fn echo_params() -> Value {
    json!({"text": "salut Annecy"})
}

fn expected() -> Value {
    json!({"text": "salut Annecy", "length": 12})
}

/// `programme-exemple run <id> <json>`, éventuellement sur un projet ; rend le résultat.
fn run(args: &[&str]) -> Value {
    let out = Command::new(EXEMPLE)
        .arg("run")
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "run {args:?} : {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// Lance `serve`, envoie `lines`, ferme l'entrée et rend les réponses.
fn serve(lines: &[Value]) -> Vec<Value> {
    let mut child = Command::new(EXEMPLE)
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for line in lines {
            writeln!(stdin, "{line}").unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn hello(project: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": 1, "method": "session.hello",
        "params": {"protocol_version": 2, "commands": [], "project": project}})
}

/// Une session MCP autonome : poignée de main, puis les appels, réponses lues une à une.
struct Mcp {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    lines: std::sync::mpsc::Receiver<Value>,
    next: u64,
}

impl Mcp {
    fn connect() -> Self {
        // Un canal d'IA externe que personne n'ouvre : le serveur reste autonome.
        let mut child = Command::new(EXEMPLE)
            .arg("mcp")
            .env(
                "CASSINI_EXTERNAL_TAG",
                format!("aucun-exemple-{}", std::process::id()),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let (tx, lines) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in stdout.lines().map_while(Result::ok) {
                if !line.trim().is_empty() {
                    let _ = tx.send(serde_json::from_str::<Value>(&line).unwrap());
                }
            }
        });
        let mut mcp = Mcp {
            child,
            stdin,
            lines,
            next: 1,
        };
        let reply = mcp.request(
            "initialize",
            json!({"protocolVersion": "2025-11-25", "capabilities": {},
                "clientInfo": {"name": "test", "version": "0"}}),
        );
        assert!(reply.get("error").is_none(), "initialize : {reply}");
        writeln!(
            mcp.stdin,
            "{}",
            json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
        )
        .unwrap();
        mcp
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        writeln!(
            self.stdin,
            "{}",
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
        )
        .unwrap();
        loop {
            let message = self
                .lines
                .recv_timeout(std::time::Duration::from_secs(30))
                .unwrap_or_else(|e| panic!("aucune réponse à {method} ({e})"));
            if message["id"] == json!(id) {
                return message;
            }
        }
    }

    fn close(self) {
        let Mcp {
            mut child, stdin, ..
        } = self;
        drop(stdin);
        let _ = child.wait();
    }
}

#[test]
fn run_serve_and_mcp_give_the_same_result_for_the_added_command() {
    let by_run = run(&["exemple.echo", &echo_params().to_string()]);
    assert_eq!(by_run, expected());

    let (dir, project) = temp_project();
    let replies = serve(&[
        hello(&project),
        json!({"jsonrpc": "2.0", "id": 2, "method": "command.run",
            "params": {"command": "exemple.echo", "params": echo_params()}}),
    ]);
    assert_eq!(replies[1]["result"], by_run, "{replies:?}");

    let mut mcp = Mcp::connect();
    let tools = mcp.request("tools/list", json!({}));
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"exemple_echo"), "{names:?}");
    let reply = mcp.request(
        "tools/call",
        json!({"name": "exemple_echo", "arguments": echo_params()}),
    );
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    assert_eq!(reply["result"]["structuredContent"], by_run);
    mcp.close();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_added_command_is_journaled_and_replayed_identically_by_the_example_program() {
    let (dir, project) = temp_project();
    run(&[
        "exemple.echo",
        &echo_params().to_string(),
        "--project",
        &project,
    ]);
    let journal = std::fs::read_to_string(dir.join("p.cassini/journal.jsonl")).unwrap();
    assert_eq!(journal.lines().count(), 1);
    let line: Value = serde_json::from_str(journal.lines().next().unwrap()).unwrap();
    assert_eq!(line["id"], "exemple.echo");

    let out = Command::new(EXEMPLE)
        .args(["replay", &project])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["identical"], true, "{report}");
    assert_eq!(report["steps"], 1, "{report}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_official_program_replaying_that_journal_names_the_missing_command_and_the_step() {
    let (dir, project) = temp_project();
    run(&[
        "exemple.echo",
        &echo_params().to_string(),
        "--project",
        &project,
    ]);
    // Le programme officiel (`cassini::run`) rejoue le journal du programme d'exemple.
    let args = vec!["replay".to_string(), project.clone()];
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = cassini::run(
        &args,
        &mut std::io::Cursor::new(Vec::new()),
        &mut out,
        &mut err,
    );
    let err = String::from_utf8(err).unwrap();
    assert_eq!(code, 1, "{err}");
    let report: Value = serde_json::from_str(err.trim()).unwrap();
    assert_eq!(report["code"], "registry.unknown_command", "{report}");
    assert!(
        report["message"].as_str().unwrap().contains("exemple.echo"),
        "{report}"
    );
    assert_eq!(report["step"], 1, "{report}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_announcement_names_the_program_and_its_added_commands() {
    let about = run(&["system.about"]);
    assert_eq!(about["program_name"], "programme-exemple");
    assert_eq!(about["program_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(about["core_version"], cassini_core::VERSION);
    assert_eq!(about["added_commands"], json!(["exemple.echo"]));

    let (dir, project) = temp_project();
    let replies = serve(&[hello(&project)]);
    let result = &replies[0]["result"];
    assert_eq!(result["program_name"], "programme-exemple");
    assert_eq!(result["added_commands"], json!(["exemple.echo"]));
    assert_eq!(result["core_version"], cassini_core::VERSION);
    let listed: Vec<&str> = result["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert!(listed.contains(&"exemple.echo"), "{listed:?}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_official_program_announces_cassini_and_no_added_command() {
    let args = vec!["run".to_string(), "system.about".to_string()];
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = cassini::run(
        &args,
        &mut std::io::Cursor::new(Vec::new()),
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let about: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(about["program_name"], "cassini");
    assert_eq!(about["added_commands"], json!([]));
}
