//! Une session QGIS simulée pour les tests : un vrai `cassini serve`, et le test dans le rôle de
//! l'extension sur l'entrée et la sortie standard. Le nom du canal d'IA externe est isolé par
//! `CASSINI_EXTERNAL_TAG`, le dossier de modèles est choisi par le test. Mécanique seule : ce
//! que la session doit faire se vérifie dans les tests qui l'emploient.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use cassini_core::registry::PROTOCOL_VERSION;
use serde_json::{json, Value};

const BIN: &str = env!("CARGO_BIN_EXE_cassini");

/// Délai d'attente d'un message : au-delà, le test échoue au lieu de se figer.
const PATIENCE: Duration = Duration::from_secs(30);

/// Un dossier de projet temporaire et le chemin de son projet `p.qgz`.
pub fn temp_project() -> (PathBuf, String) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "cassini-qgis-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    (dir.clone(), project_of(&dir))
}

fn project_of(dir: &std::path::Path) -> String {
    dir.join("p.qgz").to_string_lossy().replace('\\', "/")
}

pub struct QgisSession {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<Value>,
    /// Messages lus mais pas encore consommés par une attente.
    pub unread: VecDeque<Value>,
    next_id: u64,
    /// Nom du canal d'IA externe de cette session.
    pub tag: String,
    /// Dossier du projet.
    pub dir: PathBuf,
    /// Commandes QGIS annoncées à `session.hello`.
    commands: Vec<String>,
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
    /// Lance la session sur un projet neuf avec les modèles de `models`, ouvre le projet par
    /// `session.hello` en annonçant `commands`, et allume le canal d'IA externe si `external`.
    pub fn start(external: bool, models: &str, commands: &[&str]) -> QgisSession {
        let (dir, _) = temp_project();
        let commands = commands.iter().map(|c| c.to_string()).collect();
        let mut session = QgisSession::start_in(dir, models, commands);
        if external {
            session.switch(true);
        }
        session
    }

    /// Ferme cette session, puis en lance une seconde sur le même projet, avec les modèles de
    /// `models` et les mêmes commandes annoncées. Le dossier passe à la seconde.
    pub fn reopen(mut self, models: &str) -> QgisSession {
        self.end();
        let dir = std::mem::take(&mut self.dir);
        let commands = std::mem::take(&mut self.commands);
        QgisSession::start_in(dir, models, commands)
    }

    fn start_in(dir: PathBuf, models: &str, commands: Vec<String>) -> QgisSession {
        let project = project_of(&dir);
        let tag = format!(
            "relais-{}-{}",
            std::process::id(),
            dir.file_name().unwrap().to_string_lossy()
        );
        let mut child = Command::new(BIN)
            .arg("serve")
            .env("CASSINI_EXTERNAL_TAG", &tag)
            .env("CASSINI_MODELS", models)
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
            commands,
        };
        let hello = session.request(
            "session.hello",
            json!({"protocol_version": PROTOCOL_VERSION, "commands": session.commands,
                "project": project}),
        );
        assert!(hello["result"].is_object(), "{hello}");
        session
    }

    /// Le pid du processus `cassini serve`.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    pub fn start_request(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        id
    }

    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.start_request(method, params);
        self.wait_reply(method, id)
    }

    /// Attend la réponse à la requête `id`.
    pub fn wait_reply(&mut self, what: &str, id: u64) -> Value {
        self.wait_for(what, |m| m["id"] == json!(id) && m.get("method").is_none())
    }

    /// Lit ce qui arrive encore pendant 300 ms après le dernier message ; le garde à lire.
    pub fn drain(&mut self) {
        while let Ok(message) = self.lines.recv_timeout(Duration::from_millis(300)) {
            self.unread.push_back(message);
        }
    }

    pub fn wait_for(&mut self, what: &str, wanted: impl Fn(&Value) -> bool) -> Value {
        self.wait_for_within(what, PATIENCE, wanted)
    }

    /// Comme `wait_for`, avec une échéance choisie (une composition au vrai modèle est longue).
    pub fn wait_for_within(
        &mut self,
        what: &str,
        patience: Duration,
        wanted: impl Fn(&Value) -> bool,
    ) -> Value {
        if let Some(i) = self.unread.iter().position(&wanted) {
            return self.unread.remove(i).unwrap();
        }
        loop {
            let message = self
                .lines
                .recv_timeout(patience)
                .unwrap_or_else(|e| panic!("la session n'a rien envoyé ({e}) : {what}"));
            if wanted(&message) {
                return message;
            }
            self.unread.push_back(message);
        }
    }

    /// Allume ou éteint le canal d'IA externe (`session.external`).
    pub fn switch(&mut self, enabled: bool) {
        let reply = self.request(
            "session.external",
            json!({"enabled": enabled, "ask": false}),
        );
        assert_eq!(reply["result"]["enabled"], enabled, "{reply}");
    }

    /// Le prochain ordre `order.execute` dicté à l'extension.
    pub fn wait_order(&mut self) -> Value {
        self.wait_for("order.execute", |m| m["method"] == "order.execute")
    }

    pub fn answer(&mut self, order: &Value, outcome: Value) {
        let mut reply = json!({"jsonrpc": "2.0", "id": order["id"]});
        let key = if outcome.get("code").is_some() && outcome.get("data").is_some() {
            "error"
        } else {
            "result"
        };
        reply[key] = outcome;
        self.send(&reply);
    }

    pub fn journal_path(&self) -> PathBuf {
        self.dir.join("p.cassini/journal.jsonl")
    }

    pub fn journal(&self) -> Vec<Value> {
        std::fs::read_to_string(self.journal_path())
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// Ferme l'entrée et attend une sortie sans erreur du processus.
    fn end(&mut self) {
        drop(self.stdin.take());
        assert!(self.child.wait().unwrap().success());
    }

    /// Ferme la session et efface le projet (par `Drop`).
    pub fn close(mut self) {
        self.end();
    }
}

/// Une assertion qui échoue ne laisse ni `cassini serve` vivant (modèle chargé, mémoire de la
/// carte) ni le projet temporaire. Un dossier vide est celui que `reopen` a passé à la suite.
impl Drop for QgisSession {
    fn drop(&mut self) {
        drop(self.stdin.take());
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        if !self.dir.as_os_str().is_empty() {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}
