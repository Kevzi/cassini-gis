//! Client MCP minimal pour les tests : lance `cassini mcp`, fait la poignée de main de la
//! révision 2025-11-25, puis envoie une requête à la fois et lit sa réponse. Garde toutes les
//! lignes reçues, pour vérifier qu'aucune n'est une requête du serveur (AD-3).

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};

use serde_json::{json, Value};

pub const BIN: &str = env!("CARGO_BIN_EXE_cassini");

pub struct McpClient {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    /// Toutes les lignes reçues du serveur, relues en JSON.
    pub received: Vec<Value>,
}

impl McpClient {
    /// Lance `cassini mcp` avec `args` et fait la poignée de main.
    pub fn connect(args: &[&str], models: &str) -> Self {
        let mut client = Self::spawn(args, models);
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
        let mut child = Command::new(BIN)
            .arg("mcp")
            .args(args)
            .env("CASSINI_MODELS", models)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("le binaire se lance");
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        McpClient {
            child,
            stdin,
            stdout,
            next_id: 1,
            received: Vec::new(),
        }
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().expect("entrée ouverte");
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    pub fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    /// Envoie une requête et rend la réponse qui porte son `id`.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let mut line = String::new();
            let n = self.stdout.read_line(&mut line).unwrap();
            assert!(
                n > 0,
                "le serveur a fermé sa sortie sans répondre à {method}"
            );
            let message: Value = serde_json::from_str(&line).expect("une ligne JSON par message");
            self.received.push(message.clone());
            if message["id"] == json!(id) && message.get("method").is_none() {
                return message;
            }
        }
    }

    /// Appelle un outil et rend la réponse entière.
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Value {
        self.request("tools/call", json!({"name": name, "arguments": arguments}))
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
        let mut rest = String::new();
        let _ = std::io::Read::read_to_string(&mut self.stdout, &mut rest);
        for line in rest.lines().filter(|l| !l.trim().is_empty()) {
            self.received
                .push(serde_json::from_str(line).expect("une ligne JSON par message"));
        }
        let status = self.child.wait().unwrap();
        (status, self.received)
    }
}
