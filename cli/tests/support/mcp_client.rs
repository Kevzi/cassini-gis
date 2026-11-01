//! Client MCP minimal pour les tests : lance `cassini mcp`, fait la poignée de main de la
//! révision 2025-11-25, puis envoie des requêtes et lit les messages. Un fil lit la sortie du
//! serveur : le client peut envoyer sans attendre (`start`), attendre une réponse par son `id`
//! (`wait`) ou une notification (`wait_notification`). Garde toutes les lignes reçues, pour
//! vérifier qu'aucune n'est une requête du serveur (AD-3).

#![allow(dead_code)]

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{json, Value};

pub const BIN: &str = env!("CARGO_BIN_EXE_cassini");

/// Délai d'attente d'un message : au-delà, le test échoue au lieu de se figer.
pub const PATIENCE: Duration = Duration::from_secs(30);

pub struct McpClient {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<Value>,
    /// Messages lus mais pas encore consommés par une attente.
    unread: VecDeque<Value>,
    next_id: u64,
    /// Toutes les lignes reçues du serveur, relues en JSON.
    pub received: Vec<Value>,
}

/// Un nom de canal d'IA externe propre au test, que personne n'ouvre : le serveur reste autonome
/// même si une vraie session QGIS de l'utilisateur tient le canal ordinaire.
pub fn unused_tag() -> String {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    format!(
        "aucun-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    )
}

impl McpClient {
    /// Lance `cassini mcp` avec `args` et fait la poignée de main, en mode autonome.
    pub fn connect(args: &[&str], models: &str) -> Self {
        Self::connect_tagged(args, models, &unused_tag())
    }

    /// Lance `cassini mcp` sur le canal d'IA externe isolé par `tag`, et fait la poignée de main.
    pub fn connect_tagged(args: &[&str], models: &str, tag: &str) -> Self {
        let mut client = Self::spawn_tagged(args, models, tag);
        let reply = client.request(
            "initialize",
            json!({"protocolVersion": "2025-11-25", "capabilities": {},
                "clientInfo": {"name": "test", "version": "0"}}),
        );
        assert!(reply.get("error").is_none(), "initialize : {reply}");
        assert_eq!(reply["result"]["protocolVersion"], "2025-11-25");
        assert!(reply["result"]["capabilities"]["tools"].is_object());
        client.notify("notifications/initialized", json!({}));
        client
    }

    /// Lance `cassini mcp` sans rien envoyer.
    pub fn spawn(args: &[&str], models: &str) -> Self {
        Self::spawn_tagged(args, models, &unused_tag())
    }

    fn spawn_tagged(args: &[&str], models: &str, tag: &str) -> Self {
        let mut child = Command::new(BIN)
            .arg("mcp")
            .args(args)
            .env("CASSINI_MODELS", models)
            .env("CASSINI_EXTERNAL_TAG", tag)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("le binaire se lance");
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in stdout.lines() {
                let Ok(line) = line else { return };
                if line.trim().is_empty() {
                    continue;
                }
                let message: Value =
                    serde_json::from_str(&line).expect("une ligne JSON par message");
                if tx.send(message).is_err() {
                    return;
                }
            }
        });
        McpClient {
            child,
            stdin,
            lines,
            unread: VecDeque::new(),
            next_id: 1,
            received: Vec::new(),
        }
    }

    /// Le pid du processus `cassini mcp`.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().expect("entrée ouverte");
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    pub fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    /// Envoie une requête sans attendre sa réponse ; rend son `id`.
    pub fn start(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        id
    }

    /// Le prochain message reçu (non encore consommé), au plus après `PATIENCE`.
    fn next(&mut self, waiting_for: &str) -> Value {
        let message = self
            .lines
            .recv_timeout(PATIENCE)
            .unwrap_or_else(|e| panic!("aucun message ({e}) en attendant {waiting_for}"));
        self.received.push(message.clone());
        message
    }

    /// Attend le premier message qui satisfait `wanted` ; les autres restent à lire.
    pub fn wait_for(&mut self, what: &str, wanted: impl Fn(&Value) -> bool) -> Value {
        if let Some(i) = self.unread.iter().position(&wanted) {
            return self.unread.remove(i).unwrap();
        }
        loop {
            let message = self.next(what);
            if wanted(&message) {
                return message;
            }
            self.unread.push_back(message);
        }
    }

    /// Attend la réponse à la requête `id`.
    pub fn wait(&mut self, id: u64) -> Value {
        self.wait_for(&format!("la réponse {id}"), |m| {
            m["id"] == json!(id) && m.get("method").is_none()
        })
    }

    /// Attend la notification `method`.
    pub fn wait_notification(&mut self, method: &str) -> Value {
        self.wait_for(method, |m| m["method"] == method && m.get("id").is_none())
    }

    /// Lit ce qui arrive encore pendant `quiet` après le dernier message ; le garde à lire.
    pub fn drain(&mut self, quiet: Duration) {
        while let Ok(message) = self.lines.recv_timeout(quiet) {
            self.received.push(message.clone());
            self.unread.push_back(message);
        }
    }

    /// Les notifications `method` reçues et pas encore consommées, retirées, après avoir lu ce
    /// qui arrive encore pendant 300 ms.
    pub fn take_notifications(&mut self, method: &str) -> usize {
        self.drain(Duration::from_millis(300));
        let before = self.unread.len();
        self.unread.retain(|m| m["method"] != method);
        before - self.unread.len()
    }

    /// Envoie une requête et rend la réponse qui porte son `id`.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.start(method, params);
        self.wait(id)
    }

    /// Appelle un outil et rend la réponse entière.
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Value {
        self.request("tools/call", json!({"name": name, "arguments": arguments}))
    }

    /// Commence un appel d'outil sans attendre ; rend son `id`.
    pub fn start_tool(&mut self, name: &str, arguments: Value) -> u64 {
        self.start("tools/call", json!({"name": name, "arguments": arguments}))
    }

    /// Aucun message reçu n'est une requête du serveur (pas de `method` avec `id`).
    pub fn assert_no_server_request(&self) {
        for m in &self.received {
            assert!(
                !(m.get("method").is_some() && m.get("id").is_some()),
                "le serveur a émis une requête : {m}"
            );
        }
    }

    /// Ferme l'entrée, lit la fin de la sortie et attend la sortie du processus.
    pub fn close(mut self) -> (ExitStatus, Vec<Value>) {
        drop(self.stdin.take());
        while let Ok(message) = self.lines.recv_timeout(PATIENCE) {
            self.received.push(message);
        }
        let status = self.child.wait().unwrap();
        (status, self.received)
    }
}
