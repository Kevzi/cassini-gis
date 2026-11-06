//! Canal d'IA externe côté session (AD-29, tickets 5.24, 5.28 et 5.29) : une extension simulée par des
//! flux en mémoire, un relais brut branché au vrai canal (socket Unix ou tube nommé), un nom de
//! canal propre à chaque test. Chaque ligne des matrices de ces tickets a son test.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cassini_core::agent_inventory::ModelInventory;
use cassini_core::error::ErrorReport;
use cassini_core::external::{self, ChannelName};
use cassini_core::plan::Plan;
use cassini_core::registry::{Registry, PROTOCOL_VERSION};
use cassini_core::serve::{serve_with_channel, Composer, APPROVE_TIMEOUT};
use serde_json::{json, Value};

const WAIT: Duration = Duration::from_secs(10);

// ---- Extension simulée : deux files en mémoire, aucun processus ----

struct ChanReader {
    rx: Receiver<String>,
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
    tx: Sender<String>,
    buf: Vec<u8>,
}

impl Write for ChanWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(bytes);
        while let Some(i) = self.buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=i).collect();
            let _ = self
                .tx
                .send(String::from_utf8_lossy(&line).trim().to_string());
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Un nom de canal propre au test, unique entre tests et entre exécutions parallèles.
fn unique_name(label: &str) -> ChannelName {
    static N: AtomicUsize = AtomicUsize::new(0);
    ChannelName::tagged(&format!(
        "{label}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ))
    .unwrap()
}

fn temp_dir() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "cassini-canal-externe-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Une session `cassini serve` en mémoire, sur le canal `name`.
struct Session {
    to: Option<Sender<String>>,
    from: Receiver<String>,
    server: Option<JoinHandle<io::Result<()>>>,
    dir: PathBuf,
    project: String,
    next_id: u64,
}

/// Un compositeur factice : il rend un plan d'une étape, `system.about` sans commande imposée.
/// Avec une barrière, il signale qu'il compose, puis attend que le test le libère (composition
/// lente).
struct FakeComposer {
    gate: Option<Gate>,
    /// L'étape du plan rendu (`{"command", "params"}`) ; `None` : `system.about`.
    step: Option<Value>,
}

/// La barrière d'une composition lente : « je compose » vers le test, « vas-y » depuis le test.
struct Gate {
    entered: Mutex<Sender<()>>,
    release: Mutex<Receiver<()>>,
}

impl Composer for FakeComposer {
    fn compose(
        &self,
        _registry: &Registry,
        _request: &str,
        _model: Option<&str>,
        _inventory: &mut dyn FnMut() -> Result<ModelInventory, ErrorReport>,
    ) -> Result<Plan, ErrorReport> {
        if let Some(gate) = &self.gate {
            let _ = gate.entered.lock().unwrap().send(());
            let _ = gate.release.lock().unwrap().recv_timeout(WAIT);
        }
        let step = self
            .step
            .clone()
            .unwrap_or_else(|| json!({"command": "system.about", "params": {}}));
        Ok(serde_json::from_value(json!({"kind": "calls", "calls": [step]})).unwrap())
    }

    fn models(&self, _registry: &Registry) -> Result<Vec<Value>, ErrorReport> {
        Ok(Vec::new())
    }
}

impl Session {
    fn start(name: &ChannelName) -> Session {
        Session::start_with(name, None)
    }

    fn start_with(name: &ChannelName, composer: Option<FakeComposer>) -> Session {
        Session::start_full(name, composer, APPROVE_TIMEOUT)
    }

    /// Une session dont la demande d'accord `external.approve` expire après `timeout`.
    fn start_quick(name: &ChannelName, timeout: Duration) -> Session {
        Session::start_full(name, None, timeout)
    }

    fn start_full(
        name: &ChannelName,
        composer: Option<FakeComposer>,
        approve_timeout: Duration,
    ) -> Session {
        let dir = temp_dir();
        let project = dir.join("p.qgz").to_string_lossy().replace('\\', "/");
        let (to, in_rx) = mpsc::channel();
        let (out_tx, from) = mpsc::channel();
        let name = name.clone();
        let server = thread::spawn(move || {
            let registry = Registry::standard();
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
            serve_with_channel(&registry, composer, name, approve_timeout, reader, writer)
        });
        Session {
            to: Some(to),
            from,
            server: Some(server),
            dir,
            project,
            next_id: 0,
        }
    }

    fn send(&self, message: Value) {
        self.to.as_ref().unwrap().send(message.to_string()).unwrap();
    }

    fn recv(&self) -> Value {
        let line = self
            .from
            .recv_timeout(WAIT)
            .expect("la session devait écrire");
        serde_json::from_str(&line).unwrap()
    }

    /// Envoie une requête et rend la réponse suivante de la session.
    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        self.send(json!({"jsonrpc":"2.0","id":self.next_id,"method":method,"params":params}));
        let r = self.recv();
        assert_eq!(r["id"], self.next_id, "{r}");
        r
    }

    fn hello(&mut self, commands: &[&str]) {
        let project = self.project.clone();
        let r = self.request(
            "session.hello",
            json!({"protocol_version": PROTOCOL_VERSION, "commands": commands, "project": project}),
        );
        assert!(r["result"].is_object(), "{r}");
    }

    fn switch(&mut self, params: Value) -> Value {
        self.request("session.external", params)
    }

    fn switch_on(&mut self) {
        let r = self.switch(json!({"enabled": true, "ask": false}));
        assert_eq!(r["result"], json!({"enabled": true, "ask": false}), "{r}");
    }

    /// La prochaine notification `external.call`, dont l'événement est vérifié.
    fn expect_call(&self, event: &str) -> Value {
        let notice = self.recv();
        assert_eq!(notice["method"], "external.call", "{notice}");
        assert!(notice.get("id").is_none());
        assert_eq!(notice["params"]["event"], event, "{notice}");
        notice["params"].clone()
    }

    /// Le prochain ordre `order.execute`, dont la commande est vérifiée.
    fn expect_order(&self, command: &str) -> Value {
        let order = self.recv();
        assert_eq!(order["method"], "order.execute", "{order}");
        assert_eq!(order["params"]["command"], command, "{order}");
        order
    }

    /// La prochaine demande d'accord `external.approve`, dont la commande est vérifiée.
    fn expect_approve(&self, command: &str) -> Value {
        let request = self.recv();
        assert_eq!(request["method"], "external.approve", "{request}");
        assert!(
            request["id"].as_str().unwrap().starts_with("core-"),
            "{request}"
        );
        assert_eq!(request["params"]["command"], command, "{request}");
        request
    }

    /// La prochaine notification `request.cancel`, qui doit viser `request`.
    fn expect_withdrawn(&self, request: &Value) {
        let notice = self.recv();
        assert_eq!(notice["method"], "request.cancel", "{notice}");
        assert!(notice.get("id").is_none(), "{notice}");
        assert_eq!(notice["params"]["id"], request["id"], "{notice}");
    }

    /// Allume le canal, case « Me demander avant chaque appel » cochée.
    fn switch_on_asking(&mut self) {
        let r = self.switch(json!({"enabled": true, "ask": true}));
        assert_eq!(r["result"], json!({"enabled": true, "ask": true}), "{r}");
    }

    fn reply(&self, order: &Value, result: Value) {
        self.send(json!({"jsonrpc":"2.0","id":order["id"],"result":result}));
    }

    fn assert_silent(&self) {
        assert_eq!(
            self.from.recv_timeout(Duration::from_millis(300)),
            Err(RecvTimeoutError::Timeout)
        );
    }

    fn journal(&self) -> Vec<Value> {
        std::fs::read_to_string(self.dir.join("p.cassini/journal.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// Ferme l'entrée standard (QGIS fermé) : rend l'issue du serveur.
    fn close(&mut self) -> io::Result<()> {
        self.to = None;
        self.server.take().unwrap().join().unwrap()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.to = None;
        if let Some(server) = self.server.take() {
            let _ = server.join();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Un relais brut : il parle JSON-RPC sur le canal, sans `cassini mcp`.
struct Relay {
    out: external::Stream,
    lines: Receiver<Option<String>>,
}

impl Relay {
    fn connect(name: &ChannelName) -> Relay {
        let stream = external::connect(name).expect("le canal devait être ouvert");
        let input = stream.try_clone().unwrap();
        let (tx, lines) = mpsc::channel();
        thread::spawn(move || {
            let mut input = BufReader::new(input);
            loop {
                let mut line = String::new();
                match input.read_line(&mut line) {
                    Ok(0) | Err(_) => {
                        let _ = tx.send(None);
                        return;
                    }
                    Ok(_) => {
                        if tx.send(Some(line.trim().to_string())).is_err() {
                            return;
                        }
                    }
                }
            }
        });
        Relay { out: stream, lines }
    }

    fn send(&mut self, message: Value) {
        writeln!(self.out, "{message}").unwrap();
    }

    fn recv(&self) -> Value {
        let line = self
            .lines
            .recv_timeout(WAIT)
            .expect("la session devait écrire au relais")
            .expect("la connexion s'est fermée");
        serde_json::from_str(&line).unwrap()
    }

    fn assert_closed(&self) {
        assert_eq!(
            self.lines.recv_timeout(WAIT),
            Ok(None),
            "fin de flux attendue"
        );
    }

    fn assert_silent(&self) {
        assert_eq!(
            self.lines.recv_timeout(Duration::from_millis(300)),
            Err(RecvTimeoutError::Timeout)
        );
    }

    fn attach(&mut self, version: u64) -> Value {
        self.send(json!({"jsonrpc":"2.0","id":1,"method":"external.attach",
            "params":{"protocol_version":version}}));
        self.recv()
    }

    /// Branche un relais et fait la poignée de main.
    fn attached(name: &ChannelName) -> Relay {
        let mut relay = Relay::connect(name);
        assert!(relay.attach(u64::from(PROTOCOL_VERSION))["result"].is_object());
        relay
    }

    fn run(&mut self, id: u64, command: &str, params: Value) {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":"command.run",
            "params":{"command":command,"params":params}}));
    }

    fn cancel(&mut self, id: u64, target: Value) -> Value {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":"request.cancel",
            "params":{"id":target}}));
        self.recv()
    }
}

fn code_of(response: &Value) -> &str {
    response["error"]["data"]["code"].as_str().unwrap_or("")
}

fn assert_absent(name: &ChannelName) {
    assert!(
        external::connect(name).is_err(),
        "le canal ne devait pas exister"
    );
    #[cfg(unix)]
    assert!(!name.socket_path().exists());
}

#[test]
fn without_the_switch_no_channel_exists() {
    let name = unique_name("defaut");
    let mut s = Session::start(&name);
    s.hello(&[]);
    assert_absent(&name);
    // La session sert l'extension comme avant.
    let r = s.request("command.run", json!({"command": "system.about"}));
    assert_eq!(r["result"]["protocol_version"], PROTOCOL_VERSION);
    assert_absent(&name);
}

#[test]
fn the_switch_needs_a_session_and_its_params() {
    let name = unique_name("params");
    let mut s = Session::start(&name);
    let r = s.switch(json!({"enabled": true, "ask": false}));
    assert_eq!(code_of(&r), "session.not_started");
    s.hello(&[]);
    for params in [
        json!({}),
        json!({"enabled": "oui"}),
        json!({"enabled": true}),
        json!({"enabled": true, "ask": 1}),
    ] {
        let r = s.switch(params);
        assert_eq!(r["error"]["code"], -32602, "{r}");
    }
    assert_absent(&name);
}

#[test]
fn switching_on_then_off_attaches_then_warns_the_relay() {
    let name = unique_name("cycle");
    let mut s = Session::start(&name);
    s.hello(&[]);
    s.switch_on();
    let mut relay = Relay::connect(&name);
    let r = relay.attach(u64::from(PROTOCOL_VERSION));
    let result = &r["result"];
    assert_eq!(result["protocol_version"], PROTOCOL_VERSION);
    assert_eq!(result["core_version"], cassini_core::VERSION);
    let commands = result["commands"].as_array().unwrap();
    assert!(commands.iter().any(|c| c["id"] == "system.about"));
    // Seules les commandes disponibles : aucune commande QGIS n'a été annoncée (AD-25).
    assert!(commands.iter().all(|c| c["available"] == true));
    assert!(!commands.iter().any(|c| c["id"] == "layer.describe"));
    // Aucun chemin du poste (AD-9).
    let text = r.to_string();
    assert!(!text.contains("cassini-canal-externe"), "{text}");
    // Allumer deux fois ne change rien, sauf la case.
    let again = s.switch(json!({"enabled": true, "ask": true}));
    assert_eq!(again["result"], json!({"enabled": true, "ask": true}));

    let off = s.switch(json!({"enabled": false}));
    assert_eq!(off["result"], json!({"enabled": false}));
    let lost = relay.recv();
    assert_eq!(lost["method"], "external.session_lost");
    assert!(lost.get("id").is_none());
    assert_eq!(lost["params"]["reason"], "switched_off");
    relay.assert_closed();
    assert_absent(&name);
    // Déjà éteint : même réponse.
    let off = s.switch(json!({"enabled": false}));
    assert_eq!(off["result"], json!({"enabled": false}));

    // Le canal se rouvre aussitôt, et se referme.
    s.switch_on();
    let mut relay = Relay::connect(&name);
    assert!(relay.attach(u64::from(PROTOCOL_VERSION))["result"].is_object());
    s.switch(json!({"enabled": false}));
    assert_eq!(relay.recv()["method"], "external.session_lost");
    relay.assert_closed();
    assert_absent(&name);
}

#[test]
fn a_second_session_finds_the_channel_taken() {
    let name = unique_name("pris");
    let mut first = Session::start(&name);
    first.hello(&[]);
    first.switch_on();
    let mut second = Session::start(&name);
    second.hello(&[]);
    let r = second.switch(json!({"enabled": true, "ask": false}));
    assert_eq!(code_of(&r), "external.channel_taken", "{r}");
    assert!(!r.to_string().contains("pipe") && !r.to_string().contains(".sock"));
    // Le canal reste celui de la première session.
    let mut relay = Relay::connect(&name);
    assert!(relay.attach(u64::from(PROTOCOL_VERSION))["result"].is_object());
    // La seconde n'a rien ouvert : s'éteindre ne coupe pas le canal de la première.
    let off = second.switch(json!({"enabled": false}));
    assert_eq!(off["result"], json!({"enabled": false}));
    relay.assert_silent();
}

#[test]
fn an_attach_in_another_version_names_the_binary_to_update() {
    let name = unique_name("version");
    let mut s = Session::start(&name);
    s.hello(&[]);
    s.switch_on();
    let mut relay = Relay::connect(&name);
    let older = relay.attach(1);
    assert_eq!(code_of(&older), "session.protocol_mismatch");
    assert!(older["error"]["message"]
        .as_str()
        .unwrap()
        .contains("relais"));
    let newer = relay.attach(u64::from(PROTOCOL_VERSION) + 1);
    assert_eq!(code_of(&newer), "session.protocol_mismatch");
    assert!(newer["error"]["message"]
        .as_str()
        .unwrap()
        .contains("session QGIS"));
    relay.send(json!({"jsonrpc":"2.0","id":2,"method":"external.attach"}));
    assert_eq!(relay.recv()["error"]["code"], -32602);
}

#[test]
fn an_attach_during_an_order_is_answered_at_once() {
    let name = unique_name("ordre");
    let mut s = Session::start(&name);
    s.hello(&["layer.describe"]);
    s.switch_on();
    let mut relay = Relay::connect(&name);
    s.send(json!({"jsonrpc":"2.0","id":50,"method":"command.run",
        "params":{"command":"layer.describe","params":{"layer_id":"l"}}}));
    let order = s.recv();
    assert_eq!(order["method"], "order.execute", "{order}");
    // L'ordre est en vol : le relais est servi sans attendre.
    let r = relay.attach(u64::from(PROTOCOL_VERSION));
    let commands = r["result"]["commands"].as_array().unwrap();
    assert!(commands.iter().any(|c| c["id"] == "layer.describe"));
    // L'interrupteur aussi est servi pendant l'ordre.
    s.send(json!({"jsonrpc":"2.0","id":51,"method":"session.external",
        "params":{"enabled":true,"ask":false}}));
    assert_eq!(s.recv()["id"], 51);
    s.send(
        json!({"jsonrpc":"2.0","id":order["id"],"error":{"code":-32000,"message":"absente",
        "data":{"code":"qgis.layer_missing","message":"La couche est absente."}}}),
    );
    let done = s.recv();
    assert_eq!(done["id"], 50);
    assert_eq!(code_of(&done), "qgis.layer_missing");
}

#[test]
fn another_method_from_a_relay_runs_nothing() {
    let name = unique_name("autre");
    let mut s = Session::start(&name);
    s.hello(&["layer.describe"]);
    s.switch_on();
    let mut relay = Relay::connect(&name);
    for method in ["plan.propose", "journal.replay", "session.external"] {
        relay.send(json!({"jsonrpc":"2.0","id":7,"method":method,"params":{}}));
        let r = relay.recv();
        assert_eq!(
            (r["id"].clone(), r["error"]["code"].clone()),
            (json!(7), json!(-32601))
        );
    }
    relay.send(json!({"jsonrpc":"2.0","id":8,"method":"session.hello",
        "params":{"protocol_version":PROTOCOL_VERSION,"commands":[],"project":"/x/p.qgz"}}));
    assert_eq!(relay.recv()["error"]["code"], -32601);
    // Un `command.run` sans identifiant n'est ni répondu ni exécuté ; une ligne illisible reçoit
    // une erreur à identifiant nul.
    relay.send(json!({"jsonrpc":"2.0","method":"command.run",
        "params":{"command":"system.about"}}));
    writeln!(relay.out, "pas du json").unwrap();
    let r = relay.recv();
    assert_eq!(
        (r["id"].clone(), r["error"]["code"].clone()),
        (Value::Null, json!(-32700))
    );
    // Rien n'a été envoyé à l'extension, rien n'est journalisé.
    assert!(s.from.recv_timeout(Duration::from_millis(300)).is_err());
    assert!(!s.dir.join("p.cassini/journal.jsonl").exists());
}

#[test]
fn closing_the_input_warns_the_relays_and_removes_the_channel() {
    let name = unique_name("fin");
    let mut s = Session::start(&name);
    s.hello(&[]);
    s.switch_on();
    let mut relay = Relay::connect(&name);
    assert!(relay.attach(u64::from(PROTOCOL_VERSION))["result"].is_object());
    let mut quiet = Relay::connect(&name);
    // QGIS se ferme : le serveur s'arrête proprement (code 0 du binaire).
    s.close().unwrap();
    for r in [&mut relay, &mut quiet] {
        let lost = r.recv();
        assert_eq!(lost["method"], "external.session_lost");
        assert_eq!(lost["params"]["reason"], "session_ended");
        r.assert_closed();
    }
    assert_absent(&name);
}

#[test]
fn a_relay_that_stops_reading_does_not_hold_the_switch_off() {
    let name = unique_name("muet");
    let mut s = Session::start(&name);
    s.hello(&[]);
    s.switch_on();
    // Des relais qui demandent sans jamais lire : l'écrivain de chaque connexion finit bloqué,
    // le tampon du système plein de catalogues. L'échéance des adieux est commune : quatre
    // relais muets retiennent l'extinction une demi-seconde, pas deux. Les lignes qu'un lecteur
    // n'a pas encore traitées à l'extinction ne le sont plus (5.43). Le seuil d'une seconde passe
    // entre l'échéance commune (≈ 0,5 s) et une échéance par connexion (≈ 2 s) ; il casse aussi,
    // sous charge, si les lignes en file sont de nouveau traitées (1,2 à 1,5 s mesurées).
    let attach = json!({"jsonrpc":"2.0","id":1,"method":"external.attach",
        "params":{"protocol_version":PROTOCOL_VERSION}});
    let mutes: Vec<external::Stream> = (0..4)
        .map(|_| {
            let mut mute = external::connect(&name).unwrap();
            for _ in 0..300 {
                writeln!(mute, "{attach}").unwrap();
            }
            mute
        })
        .collect();
    // Malgré ces lignes, l'extension reçoit sa réponse.
    let r = s.request("command.run", json!({"command": "system.about"}));
    assert!(r["result"].is_object(), "{r}");
    let started = std::time::Instant::now();
    let off = s.switch(json!({"enabled": false}));
    assert_eq!(off["result"], json!({"enabled": false}));
    assert!(
        started.elapsed() < Duration::from_millis(1000),
        "l'extinction a pris {:?}",
        started.elapsed()
    );
    assert_absent(&name);
    drop(mutes);
}

#[test]
fn a_relay_that_leaves_changes_nothing_for_the_others() {
    let name = unique_name("depart");
    let mut s = Session::start(&name);
    s.hello(&[]);
    s.switch_on();
    let leaving = Relay::connect(&name);
    leaving.out.shutdown();
    leaving.assert_closed();
    let mut relay = Relay::connect(&name);
    assert!(relay.attach(u64::from(PROTOCOL_VERSION))["result"].is_object());
    let r = s.request("command.run", json!({"command": "system.about"}));
    assert!(r["result"].is_object());
}

#[cfg(unix)]
#[test]
fn an_orphan_socket_is_removed_and_the_channel_opens() {
    use std::os::unix::fs::DirBuilderExt;
    use std::os::unix::net::UnixListener;
    let name = unique_name("orpheline");
    let socket = name.socket_path().to_path_buf();
    let dir = socket.parent().unwrap();
    if !dir.exists() {
        std::fs::DirBuilder::new().mode(0o700).create(dir).unwrap();
    }
    // Un noyau mort laisse sa socket : liée puis abandonnée, personne n'écoute.
    drop(UnixListener::bind(&socket).unwrap());
    assert!(socket.exists());
    let mut s = Session::start(&name);
    s.hello(&[]);
    s.switch_on();
    let mut relay = Relay::connect(&name);
    assert!(relay.attach(u64::from(PROTOCOL_VERSION))["result"].is_object());
    s.switch(json!({"enabled": false}));
    assert!(!socket.exists());
}

#[cfg(windows)]
#[test]
fn a_remote_client_is_refused() {
    let name = unique_name("distant");
    let mut s = Session::start(&name);
    s.hello(&[]);
    s.switch_on();
    let local = name.pipe_name();
    let remote = local.replacen(r"\\.\", r"\\localhost\", 1);
    assert_ne!(local, remote);
    let opened = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&remote);
    assert!(
        opened.is_err(),
        "un client par \\\\localhost devait être refusé"
    );
    // Le client local passe.
    let mut relay = Relay::connect(&name);
    assert!(relay.attach(u64::from(PROTOCOL_VERSION))["result"].is_object());
}

// ---- Ticket 5.28 : exécution des appels du relais ----

#[test]
fn a_core_call_is_announced_answered_journaled_and_replayed() {
    let name = unique_name("noyau");
    let mut s = Session::start(&name);
    s.hello(&[]);
    s.switch_on();
    let mut relay = Relay::attached(&name);
    relay.run(7, "system.about", json!({}));
    let started = s.expect_call("started");
    assert_eq!(started["call_id"], "external-1");
    assert_eq!(started["command"], "system.about");
    assert_eq!(started["params"], json!({}));
    let finished = s.expect_call("finished");
    assert_eq!(finished["call_id"], "external-1");
    assert_eq!(finished["result"]["protocol_version"], PROTOCOL_VERSION);
    assert!(finished["duration_ms"].is_u64());
    assert!(finished.get("error").is_none());
    let r = relay.recv();
    assert_eq!(r["id"], 7);
    assert_eq!(r["result"], finished["result"]);
    // Une commande inconnue ou des paramètres mal formés : une erreur, rien d'exécuté.
    relay.run(8, "nope.nothing", json!({}));
    assert_eq!(code_of(&relay.recv()), "registry.unknown_command");
    relay.send(json!({"jsonrpc":"2.0","id":9,"method":"command.run",
        "params":{"command":"system.about","params":[]}}));
    assert_eq!(relay.recv()["error"]["code"], -32602);
    s.assert_silent();
    // Le journal porte l'origine de l'étape ; l'étape de l'extension n'en porte aucune.
    let r = s.request("command.run", json!({"command": "system.about"}));
    assert!(r["result"].is_object(), "{r}");
    let journal = s.journal();
    assert_eq!(journal.len(), 2);
    assert_eq!(journal[0]["origin"], "external");
    assert!(journal[1].get("origin").is_none());
    // Le rejeu ignore l'origine ; il n'écrit rien, les lignes la gardent.
    let r = s.request("journal.replay", json!({}));
    assert_eq!(r["result"], json!({"steps": 2, "identical": true}), "{r}");
    assert_eq!(s.journal(), journal);
}

#[test]
fn a_qgis_call_is_an_order_and_the_extension_is_busy_meanwhile() {
    let name = unique_name("qgis");
    let mut s = Session::start(&name);
    s.hello(&["layer.describe"]);
    s.switch_on();
    let mut relay = Relay::attached(&name);
    // Une commande QGIS non annoncée est indisponible : rien ne part vers l'extension.
    relay.run(6, "layer.add", json!({"path": "x.gpkg"}));
    assert_eq!(code_of(&relay.recv()), "registry.unavailable");
    s.assert_silent();

    relay.run(7, "layer.describe", json!({"layer_id": "l1"}));
    let started = s.expect_call("started");
    assert_eq!(started["params"], json!({"layer_id": "l1"}));
    let order = s.expect_order("layer.describe");
    // L'appel du relais tient l'exécutant : l'extension reçoit `session.busy` aussitôt.
    for (method, params) in [
        ("command.run", json!({"command": "system.about"})),
        ("journal.replay", json!({})),
        ("plan.propose", json!({"request": "x"})),
        ("plan.execute", json!({"plan_id": "plan-1"})),
        ("plan.reject", json!({"plan_id": "plan-1"})),
    ] {
        let r = s.request(method, params);
        assert_eq!(code_of(&r), "session.busy", "{method} : {r}");
    }
    // Un second relais aussi.
    let mut other = Relay::attached(&name);
    other.run(1, "system.about", json!({}));
    assert_eq!(code_of(&other.recv()), "session.busy");
    let described = json!({"name": "Gares", "fields": [], "crs": "EPSG:2154",
        "feature_count": 3, "geometry": "point"});
    s.reply(&order, described.clone());
    let finished = s.expect_call("finished");
    assert_eq!(finished["result"], described);
    assert_eq!(relay.recv()["result"], described);
    let journal = s.journal();
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0]["id"], "layer.describe");
    assert_eq!(journal[0]["origin"], "external");
    // L'exécutant est libéré : le rejeu repasse par un ordre, à l'identique.
    s.send(json!({"jsonrpc":"2.0","id":90,"method":"journal.replay"}));
    let order = s.expect_order("layer.describe");
    s.reply(&order, described);
    let r = s.recv();
    assert_eq!(r["id"], 90);
    assert_eq!(r["result"], json!({"steps": 1, "identical": true}), "{r}");
}

#[test]
fn a_call_of_the_extension_makes_the_relay_busy() {
    let name = unique_name("occupe");
    let mut s = Session::start(&name);
    s.hello(&["layer.describe"]);
    s.switch_on();
    let mut relay = Relay::attached(&name);
    s.send(json!({"jsonrpc":"2.0","id":50,"method":"command.run",
        "params":{"command":"layer.describe","params":{"layer_id":"l"}}}));
    let order = s.expect_order("layer.describe");
    relay.run(7, "system.about", json!({}));
    let r = relay.recv();
    assert_eq!((r["id"].clone(), code_of(&r)), (json!(7), "session.busy"));
    // La poignée de main reste servie pendant l'ordre.
    assert!(relay.attach(u64::from(PROTOCOL_VERSION))["result"].is_object());
    s.reply(
        &order,
        json!({"name": "l", "fields": [], "crs": "EPSG:2154",
        "feature_count": 0, "geometry": "point"}),
    );
    assert_eq!(s.recv()["id"], 50);
    // Libéré : l'appel du relais passe.
    relay.run(8, "system.about", json!({}));
    s.expect_call("started");
    s.expect_call("finished");
    assert!(relay.recv()["result"].is_object());
}

#[test]
fn a_composition_or_a_pending_plan_makes_the_relay_busy() {
    let name = unique_name("plan");
    let (release, gate) = mpsc::channel();
    let (entered_tx, entered) = mpsc::channel();
    let composer = FakeComposer {
        gate: Some(Gate {
            entered: Mutex::new(entered_tx),
            release: Mutex::new(gate),
        }),
        step: None,
    };
    let mut s = Session::start_with(&name, Some(composer));
    s.hello(&[]);
    s.switch_on();
    let mut relay = Relay::attached(&name);
    s.send(json!({"jsonrpc":"2.0","id":60,"method":"plan.propose",
        "params":{"request":"Qui es-tu ?"}}));
    // Le modèle compose : le fil lecteur du relais répond lui-même, sans attendre.
    entered.recv_timeout(WAIT).unwrap();
    relay.run(7, "system.about", json!({}));
    let r = relay.recv();
    assert_eq!((r["id"].clone(), code_of(&r)), (json!(7), "session.busy"));
    release.send(()).unwrap();
    let proposed = s.recv();
    assert_eq!(proposed["id"], 60);
    let plan_id = proposed["result"]["plan_id"].clone();
    assert!(plan_id.is_string(), "{proposed}");
    // Un plan non tranché occupe pour le relais seulement : l'extension garde son `command.run`.
    relay.run(8, "system.about", json!({}));
    let r = relay.recv();
    assert_eq!(code_of(&r), "session.busy");
    assert!(
        r["error"]["message"].as_str().unwrap().contains("plan"),
        "{r}"
    );
    let r = s.request("command.run", json!({"command": "system.about"}));
    assert!(r["result"].is_object(), "{r}");
    let r = s.request("plan.reject", json!({"plan_id": plan_id}));
    assert_eq!(r["result"]["rejected"], true, "{r}");
    relay.run(9, "system.about", json!({}));
    s.expect_call("started");
    s.expect_call("finished");
    assert!(relay.recv()["result"].is_object());
}

#[test]
fn with_the_ask_box_on_an_approved_call_runs_and_off_none_is_asked() {
    let name = unique_name("accord");
    let mut s = Session::start(&name);
    s.hello(&["layer.describe"]);
    s.switch_on_asking();
    let mut relay = Relay::attached(&name);
    relay.run(7, "layer.describe", json!({"layer_id": "l1"}));
    // La demande précède toute annonce : rien n'est encore parti au panneau ni exécuté.
    let request = s.expect_approve("layer.describe");
    assert_eq!(request["params"]["call_id"], "external-1");
    assert_eq!(request["params"]["params"], json!({"layer_id": "l1"}));
    s.assert_silent();
    s.reply(&request, json!({"approved": true}));
    let started = s.expect_call("started");
    assert_eq!(started["call_id"], "external-1");
    let order = s.expect_order("layer.describe");
    s.reply(&order, described());
    assert_eq!(s.expect_call("finished")["result"], described());
    let r = relay.recv();
    assert_eq!(
        (r["id"].clone(), r["result"].clone()),
        (json!(7), described())
    );
    let journal = s.journal();
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0]["origin"], "external");

    // La case éteinte : aucune demande, le chemin de 5.28.
    s.switch_on();
    relay.run(8, "system.about", json!({}));
    s.expect_call("started");
    s.expect_call("finished");
    assert!(relay.recv()["result"].is_object());
}

#[test]
fn a_refused_call_runs_nothing_and_only_the_approved_one_is_journaled() {
    let name = unique_name("refus");
    let mut s = Session::start(&name);
    s.hello(&["layer.add"]);
    s.switch_on_asking();
    let mut relay = Relay::attached(&name);
    relay.run(7, "system.about", json!({}));
    let request = s.expect_approve("system.about");
    s.reply(&request, json!({"approved": false}));
    let r = relay.recv();
    assert_eq!(
        (r["id"].clone(), code_of(&r)),
        (json!(7), "external.not_approved")
    );
    assert!(
        r["error"]["message"].as_str().unwrap().contains("refusé"),
        "{r}"
    );
    // Aucun `external.call`, rien au journal.
    s.assert_silent();
    assert!(s.journal().is_empty());
    // Refusés d'office, sans déranger l'utilisateur : commande inconnue, chemin qui sort du
    // projet, commande QGIS non annoncée.
    relay.run(8, "nope.nothing", json!({}));
    assert_eq!(code_of(&relay.recv()), "registry.unknown_command");
    relay.run(81, "layer.add", json!({"path": "/abs/x.gpkg"}));
    assert_eq!(code_of(&relay.recv()), "external.path_refused");
    relay.run(82, "layer.describe", json!({"layer_id": "l"}));
    assert_eq!(code_of(&relay.recv()), "registry.unavailable");
    s.assert_silent();

    relay.run(9, "system.about", json!({}));
    let request = s.expect_approve("system.about");
    s.reply(&request, json!({"approved": true}));
    s.expect_call("started");
    s.expect_call("finished");
    assert!(relay.recv()["result"].is_object());
    let journal = s.journal();
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0]["id"], "system.about");
    assert_eq!(journal[0]["origin"], "external");
}

#[test]
fn an_error_or_a_malformed_answer_is_not_an_approval() {
    let name = unique_name("erreur");
    let mut s = Session::start(&name);
    s.hello(&[]);
    s.switch_on_asking();
    let mut relay = Relay::attached(&name);
    // L'extension d'avant 5.26 ne connaît pas la méthode.
    relay.run(7, "system.about", json!({}));
    let request = s.expect_approve("system.about");
    s.send(json!({"jsonrpc":"2.0","id":request["id"],
        "error":{"code":-32601,"message":"Méthode inconnue : external.approve"}}));
    let r = relay.recv();
    assert_eq!(code_of(&r), "external.not_approved");
    assert!(
        r["error"]["message"]
            .as_str()
            .unwrap()
            .contains("mettez-la à jour"),
        "{r}"
    );
    // Une autre erreur : une cause neutre.
    relay.run(5, "system.about", json!({}));
    let request = s.expect_approve("system.about");
    s.send(json!({"jsonrpc":"2.0","id":request["id"],
        "error":{"code":-32000,"message":"panne"}}));
    let r = relay.recv();
    assert_eq!(code_of(&r), "external.not_approved");
    assert!(
        !r["error"]["message"]
            .as_str()
            .unwrap()
            .contains("mettez-la à jour"),
        "{r}"
    );
    // Une réponse que la lecture rejette, mais qui porte l'id de la demande : retirée, refusée.
    relay.run(6, "system.about", json!({}));
    let request = s.expect_approve("system.about");
    s.send(json!({"jsonrpc":"2.0","id":request["id"]}));
    s.expect_withdrawn(&request);
    let r = relay.recv();
    assert_eq!(
        (r["id"].clone(), code_of(&r)),
        (json!(6), "external.not_approved")
    );
    for (id, answer) in [
        (8, json!({"approved": "oui"})),
        (9, json!({})),
        (10, json!(true)),
    ] {
        relay.run(id, "system.about", json!({}));
        let request = s.expect_approve("system.about");
        s.reply(&request, answer);
        let r = relay.recv();
        assert_eq!(
            (r["id"].clone(), code_of(&r)),
            (json!(id), "external.not_approved")
        );
    }
    s.assert_silent();
    assert!(s.journal().is_empty());
}

#[test]
fn an_unanswered_request_expires_and_a_late_answer_is_ignored() {
    let name = unique_name("delai");
    let mut s = Session::start_quick(&name, Duration::from_millis(300));
    s.hello(&[]);
    s.switch_on_asking();
    let mut relay = Relay::attached(&name);
    relay.run(7, "system.about", json!({}));
    let request = s.expect_approve("system.about");
    // Pas de réponse : la demande est retirée auprès de l'extension.
    s.expect_withdrawn(&request);
    let r = relay.recv();
    assert_eq!(
        (r["id"].clone(), code_of(&r)),
        (json!(7), "external.not_approved")
    );
    assert!(s.journal().is_empty());
    // La réponse tardive est ignorée ; la session sert l'appel suivant.
    s.reply(&request, json!({"approved": true}));
    s.assert_silent();
    relay.run(8, "system.about", json!({}));
    let next = s.expect_approve("system.about");
    assert_ne!(next["id"], request["id"]);
    s.reply(&next, json!({"approved": true}));
    s.expect_call("started");
    s.expect_call("finished");
    assert!(relay.recv()["result"].is_object());
    assert_eq!(s.journal().len(), 1);
}

#[test]
fn the_relay_or_the_extension_cancels_a_pending_request() {
    let name = unique_name("annule-demande");
    let mut s = Session::start(&name);
    s.hello(&["layer.describe"]);
    s.switch_on_asking();
    let mut relay = Relay::attached(&name);
    let mut other = Relay::attached(&name);
    // Par le relais, avec l'`id` de son `command.run` ; une commande non annulable aussi.
    relay.run(7, "layer.describe", json!({"layer_id": "l"}));
    let request = s.expect_approve("layer.describe");
    assert_eq!(code_of(&relay.cancel(8, json!(99))), "request.unknown");
    assert_eq!(code_of(&other.cancel(8, json!(7))), "request.unknown");
    let r = relay.cancel(9, json!(7));
    assert_eq!(r["result"], json!({"cancelled": true}), "{r}");
    s.expect_withdrawn(&request);
    let r = relay.recv();
    assert_eq!(
        (r["id"].clone(), code_of(&r)),
        (json!(7), "external.not_approved")
    );
    // La réponse tardive est ignorée.
    s.reply(&request, json!({"approved": true}));
    s.assert_silent();

    // Par l'extension, avec le `call_id`.
    relay.run(10, "layer.describe", json!({"layer_id": "l"}));
    let request = s.expect_approve("layer.describe");
    let call_id = request["params"]["call_id"].clone();
    assert_eq!(call_id, "external-2");
    assert_eq!(
        code_of(&s.request("request.cancel", json!({"id": 10}))),
        "request.unknown"
    );
    let r = s.request("request.cancel", json!({"id": call_id}));
    assert_eq!(r["result"], json!({"cancelled": true}), "{r}");
    s.expect_withdrawn(&request);
    assert_eq!(code_of(&relay.recv()), "external.not_approved");
    s.assert_silent();
    assert!(s.journal().is_empty());

    // La session sert l'appel suivant.
    relay.run(11, "system.about", json!({}));
    let request = s.expect_approve("system.about");
    s.reply(&request, json!({"approved": true}));
    s.expect_call("started");
    s.expect_call("finished");
    assert!(relay.recv()["result"].is_object());
}

#[test]
fn a_pending_request_makes_everyone_else_busy() {
    let name = unique_name("attente");
    let mut s = Session::start(&name);
    s.hello(&[]);
    s.switch_on_asking();
    let mut relay = Relay::attached(&name);
    relay.run(7, "system.about", json!({}));
    let request = s.expect_approve("system.about");
    let mut other = Relay::attached(&name);
    other.run(1, "system.about", json!({}));
    assert_eq!(code_of(&other.recv()), "session.busy");
    for (method, params) in [
        ("command.run", json!({"command": "system.about"})),
        ("journal.replay", json!({})),
        ("plan.propose", json!({"request": "x"})),
        ("plan.execute", json!({"plan_id": "plan-1"})),
        ("plan.reject", json!({"plan_id": "plan-1"})),
        ("agent.models", json!({})),
        (
            "session.hello",
            json!({"protocol_version": PROTOCOL_VERSION, "commands": [], "project": "/x/p.qgz"}),
        ),
    ] {
        let r = s.request(method, params);
        assert_eq!(code_of(&r), "session.busy", "{method} : {r}");
    }
    // L'interrupteur reste servi.
    let r = s.switch(json!({"enabled": true, "ask": true}));
    assert_eq!(r["result"], json!({"enabled": true, "ask": true}), "{r}");
    s.reply(&request, json!({"approved": true}));
    s.expect_call("started");
    s.expect_call("finished");
    assert_eq!(relay.recv()["id"], 7);
}

#[test]
fn losing_the_session_or_the_relay_during_a_request_runs_nothing() {
    let name = unique_name("perte-demande");
    let mut s = Session::start(&name);
    s.hello(&[]);
    // Interrupteur éteint : le relais reçoit la perte, aucune réponse.
    s.switch_on_asking();
    let mut relay = Relay::attached(&name);
    relay.run(7, "system.about", json!({}));
    let request = s.expect_approve("system.about");
    let off = s.switch(json!({"enabled": false}));
    assert_eq!(off["result"], json!({"enabled": false}));
    s.expect_withdrawn(&request);
    assert_eq!(relay.recv()["method"], "external.session_lost");
    relay.assert_closed();
    s.assert_silent();

    // Relais parti.
    s.switch_on_asking();
    let relay = {
        let mut relay = Relay::attached(&name);
        relay.run(8, "system.about", json!({}));
        relay
    };
    let request = s.expect_approve("system.about");
    relay.out.shutdown();
    relay.assert_closed();
    drop(relay);
    s.expect_withdrawn(&request);
    s.assert_silent();
    assert!(s.journal().is_empty());
    // La session est libérée.
    let r = s.request("command.run", json!({"command": "system.about"}));
    assert!(r["result"].is_object(), "{r}");

    // Fin d'entrée (QGIS fermé).
    let mut relay = Relay::attached(&name);
    relay.run(9, "system.about", json!({}));
    s.expect_approve("system.about");
    s.close().unwrap();
    let lost = relay.recv();
    assert_eq!(lost["method"], "external.session_lost", "{lost}");
    assert_eq!(lost["params"]["reason"], "session_ended");
    relay.assert_closed();
    // Seule l'étape de l'extension est au journal.
    let journal = s.journal();
    assert_eq!(journal.len(), 1);
    assert!(journal[0].get("origin").is_none());
}

#[test]
fn a_path_leaving_the_project_is_refused_and_no_absolute_path_goes_back() {
    let name = unique_name("chemin");
    let mut s = Session::start(&name);
    s.hello(&["layer.add"]);
    s.switch_on();
    let mut relay = Relay::attached(&name);
    let absolute = s.dir.join("x.gpkg").to_string_lossy().into_owned();
    for (id, path) in [(1, absolute.as_str()), (2, "/abs/x.gpkg"), (3, "../x.gpkg")] {
        relay.run(id, "layer.add", json!({"path": path}));
        let r = relay.recv();
        assert_eq!(code_of(&r), "external.path_refused", "{r}");
        assert!(!r.to_string().contains("cassini-canal-externe"), "{r}");
    }
    s.assert_silent();
    assert!(s.journal().is_empty());

    // Un chemin relatif passe ; l'ordre, sur le canal de l'extension, porte le chemin absolu.
    relay.run(4, "layer.add", json!({"path": "donnees/x.gpkg"}));
    let started = s.expect_call("started");
    assert_eq!(started["params"], json!({"path": "donnees/x.gpkg"}));
    let order = s.expect_order("layer.add");
    let real = order["params"]["params"]["path"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(real.contains("cassini-canal-externe"), "{order}");
    // L'erreur de l'extension cite le chemin réel : il ne part ni au relais ni dans `external.call`.
    s.send(
        json!({"jsonrpc":"2.0","id":order["id"],"error":{"code":-32000,"message":"absent",
        "data":{"code":"qgis.file_missing","message":format!("Fichier introuvable : {real}")}}}),
    );
    let finished = s.expect_call("finished");
    assert_eq!(finished["error"]["code"], "qgis.file_missing");
    let r = relay.recv();
    assert_eq!(code_of(&r), "qgis.file_missing");
    for text in [finished.to_string(), r.to_string()] {
        assert!(!text.contains("cassini-canal-externe"), "{text}");
        assert!(text.contains("x.gpkg"), "{text}");
    }
    let journal = s.journal();
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0]["params"]["path"], "donnees/x.gpkg");
    assert_eq!(journal[0]["origin"], "external");
}

fn buffer_params() -> Value {
    json!({"layer_id": "l", "distance": {"value": 500, "unit": "m"}})
}

#[test]
fn a_relay_cancels_its_call_by_its_id() {
    let name = unique_name("annule");
    let mut s = Session::start(&name);
    s.hello(&["analysis.buffer"]);
    s.switch_on();
    let mut relay = Relay::attached(&name);
    let mut other = Relay::attached(&name);
    relay.run(7, "analysis.buffer", buffer_params());
    s.expect_call("started");
    let order = s.expect_order("analysis.buffer");
    // Un autre id, une autre connexion, ou l'id du relais depuis l'extension : rien ne change.
    assert_eq!(code_of(&relay.cancel(8, json!(99))), "request.unknown");
    assert_eq!(code_of(&other.cancel(8, json!(7))), "request.unknown");
    assert_eq!(
        code_of(&s.request("request.cancel", json!({"id": 7}))),
        "request.unknown"
    );
    let r = relay.cancel(9, json!(7));
    assert_eq!(r["result"], json!({"cancelled": true}), "{r}");
    let notice = s.recv();
    assert_eq!(notice["method"], "request.cancel");
    assert_eq!(notice["params"]["id"], order["id"]);
    s.reply(&order, json!({}));
    let finished = s.expect_call("finished");
    assert_eq!(finished["error"]["code"], "registry.cancelled");
    assert_eq!(code_of(&relay.recv()), "registry.cancelled");
    let journal = s.journal();
    assert_eq!(journal[0]["cancelled"], true);
    assert_eq!(journal[0]["origin"], "external");
    // Hors appel, l'annulation ne vise rien.
    assert_eq!(code_of(&relay.cancel(10, json!(7))), "request.unknown");
}

#[test]
fn the_extension_cancels_a_relay_call_by_its_call_id() {
    let name = unique_name("panneau");
    let mut s = Session::start(&name);
    s.hello(&["analysis.buffer", "layer.describe"]);
    s.switch_on();
    let mut relay = Relay::attached(&name);
    relay.run(7, "analysis.buffer", buffer_params());
    let call_id = s.expect_call("started")["call_id"].clone();
    let order = s.expect_order("analysis.buffer");
    s.send(json!({"jsonrpc":"2.0","id":80,"method":"request.cancel","params":{"id":call_id}}));
    let notice = s.recv();
    assert_eq!(notice["method"], "request.cancel", "{notice}");
    assert_eq!(notice["params"]["id"], order["id"]);
    let r = s.recv();
    assert_eq!(
        (r["id"].clone(), r["result"].clone()),
        (json!(80), json!({"cancelled": true}))
    );
    s.reply(&order, json!({}));
    assert_eq!(
        s.expect_call("finished")["error"]["code"],
        "registry.cancelled"
    );
    assert_eq!(code_of(&relay.recv()), "registry.cancelled");
    // Une commande non annulable refuse l'annulation et continue.
    relay.run(8, "layer.describe", json!({"layer_id": "l"}));
    let call_id = s.expect_call("started")["call_id"].clone();
    assert_eq!(call_id, "external-2");
    let order = s.expect_order("layer.describe");
    let r = s.request("request.cancel", json!({"id": call_id}));
    assert_eq!(code_of(&r), "request.not_cancellable");
    s.reply(
        &order,
        json!({"name": "l", "fields": [], "crs": "EPSG:2154",
        "feature_count": 0, "geometry": "point"}),
    );
    s.expect_call("finished");
    assert!(relay.recv()["result"].is_object());
}

#[test]
fn switching_off_during_a_call_cancels_it_or_lets_it_finish() {
    let name = unique_name("coupe");
    let mut s = Session::start(&name);
    s.hello(&["analysis.buffer", "layer.describe"]);
    s.switch_on();
    // Annulable : l'ordre est annulé, le relais reçoit la perte de session et aucune réponse.
    let mut relay = Relay::attached(&name);
    relay.run(7, "analysis.buffer", buffer_params());
    s.expect_call("started");
    let order = s.expect_order("analysis.buffer");
    let off = s.switch(json!({"enabled": false}));
    assert_eq!(off["result"], json!({"enabled": false}));
    let notice = s.recv();
    assert_eq!(notice["method"], "request.cancel", "{notice}");
    assert_eq!(notice["params"]["id"], order["id"]);
    assert_eq!(relay.recv()["method"], "external.session_lost");
    relay.assert_closed();
    s.reply(&order, json!({}));
    assert_eq!(
        s.expect_call("finished")["error"]["code"],
        "registry.cancelled"
    );
    assert_eq!(s.journal()[0]["cancelled"], true);

    // Non annulable : l'ordre finit et l'étape est journalisée, sans réponse au relais.
    s.switch_on();
    let mut relay = Relay::attached(&name);
    relay.run(8, "layer.describe", json!({"layer_id": "l"}));
    s.expect_call("started");
    let order = s.expect_order("layer.describe");
    s.switch(json!({"enabled": false}));
    assert_eq!(relay.recv()["method"], "external.session_lost");
    relay.assert_closed();
    s.assert_silent();
    s.reply(
        &order,
        json!({"name": "l", "fields": [], "crs": "EPSG:2154",
        "feature_count": 0, "geometry": "point"}),
    );
    assert!(s.expect_call("finished")["result"].is_object());
    let journal = s.journal();
    assert_eq!(journal[1]["id"], "layer.describe");
    assert!(journal[1]["result"].is_object());
    // La session est libérée.
    let r = s.request("command.run", json!({"command": "system.about"}));
    assert!(r["result"].is_object(), "{r}");
}

#[test]
fn a_relay_that_leaves_during_a_call_frees_the_session() {
    let name = unique_name("parti");
    let mut s = Session::start(&name);
    s.hello(&["analysis.buffer"]);
    s.switch_on();
    let relay = {
        let mut relay = Relay::attached(&name);
        relay.run(7, "analysis.buffer", buffer_params());
        relay
    };
    s.expect_call("started");
    let order = s.expect_order("analysis.buffer");
    // Le relais part : sa connexion se ferme (sous Windows, le tube l'est au dernier handle).
    relay.out.shutdown();
    relay.assert_closed();
    drop(relay);
    let notice = s.recv();
    assert_eq!(notice["method"], "request.cancel", "{notice}");
    s.reply(&order, json!({}));
    assert_eq!(
        s.expect_call("finished")["error"]["code"],
        "registry.cancelled"
    );
    assert_eq!(s.journal()[0]["cancelled"], true);
    let r = s.request("command.run", json!({"command": "system.about"}));
    assert!(r["result"].is_object(), "{r}");
}

#[test]
fn closing_the_input_during_a_call_answers_nothing_but_the_loss() {
    let name = unique_name("fin-appel");
    let mut s = Session::start(&name);
    s.hello(&["analysis.buffer"]);
    s.switch_on();
    let mut relay = Relay::attached(&name);
    relay.run(7, "analysis.buffer", buffer_params());
    s.expect_call("started");
    s.expect_order("analysis.buffer");
    s.close().unwrap();
    let lost = relay.recv();
    assert_eq!(lost["method"], "external.session_lost", "{lost}");
    assert_eq!(lost["params"]["reason"], "session_ended");
    relay.assert_closed();
    let journal = s.journal();
    assert_eq!(journal[0]["cancelled"], true);
    assert_eq!(journal[0]["origin"], "external");
}

fn described() -> Value {
    json!({"name": "l", "fields": [], "crs": "EPSG:2154", "feature_count": 0, "geometry": "point"})
}

#[test]
fn absolute_paths_in_a_successful_result_do_not_reach_the_relay() {
    let name = unique_name("resultat");
    let mut s = Session::start(&name);
    s.hello(&["layer.describe"]);
    s.switch_on();
    let mut relay = Relay::attached(&name);
    relay.run(7, "layer.describe", json!({"layer_id": "l"}));
    s.expect_call("started");
    let order = s.expect_order("layer.describe");
    let inside = s
        .dir
        .join("donnees")
        .join("x.gpkg")
        .to_string_lossy()
        .into_owned();
    let work = s
        .dir
        .join("p.cassini")
        .join("1-a.gpkg")
        .to_string_lossy()
        .into_owned();
    s.reply(
        &order,
        json!({"name": inside, "fields": [], "crs": "EPSG:2154",
        "feature_count": 0, "geometry": "point", "work": work,
        "source": "/ailleurs/y.gpkg", "windows": "D:\\SIG\\cadastre.gpkg",
        "note": format!("Lu dans {inside} et /ailleurs/z.gpkg")}),
    );
    let finished = s.expect_call("finished");
    let r = relay.recv();
    let text = r["result"].to_string();
    assert!(!text.contains("cassini-canal-externe"), "{text}");
    for absolute in ["/ailleurs", "D:\\\\SIG", "SIG"] {
        assert!(!text.contains(absolute), "{absolute} : {text}");
    }
    assert_eq!(r["result"]["name"], "donnees/x.gpkg");
    assert_eq!(r["result"]["source"], "y.gpkg");
    assert_eq!(r["result"]["windows"], "cadastre.gpkg");
    // L'extension, elle, reçoit le résultat tel quel dans `external.call` (AD-9, 2026-11-04).
    assert_eq!(finished["result"]["name"], inside.as_str());
    assert_eq!(finished["result"]["work"], work.as_str());
}

#[test]
fn a_slope_from_the_relay_is_announced_absolute_and_answered_relative() {
    let name = unique_name("pente");
    let mut s = Session::start(&name);
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/mnt-plan-incline-2154.tif");
    std::fs::create_dir_all(s.dir.join("dem")).unwrap();
    std::fs::copy(&fixture, s.dir.join("dem/plan-incline.tif")).unwrap();
    s.hello(&[]);
    s.switch_on();
    let mut relay = Relay::attached(&name);
    relay.run(
        3,
        "terrain.slope",
        json!({"dem": "dem/plan-incline.tif", "unit": "degrees"}),
    );
    s.expect_call("started");
    let finished = s.expect_call("finished");
    let file = std::path::Path::new(finished["result"]["file"].as_str().unwrap()).to_path_buf();
    assert!(file.is_absolute(), "{finished}");
    assert_eq!(file, s.dir.join("p.cassini").join("1-terrain.slope.tif"));
    assert!(file.is_file());
    let r = relay.recv();
    assert_eq!(r["result"]["file"], "1-terrain.slope.tif", "{r}");
    assert!(!r.to_string().contains("cassini-canal-externe"), "{r}");
    assert_eq!(s.journal()[0]["result"]["file"], "1-terrain.slope.tif");
}

#[test]
fn a_relay_cannot_cancel_a_call_of_the_extension() {
    let name = unique_name("pas-a-lui");
    let mut s = Session::start(&name);
    s.hello(&["analysis.buffer"]);
    s.switch_on();
    let mut relay = Relay::attached(&name);
    s.send(json!({"jsonrpc":"2.0","id":50,"method":"command.run",
        "params":{"command":"analysis.buffer","params":buffer_params()}}));
    let order = s.expect_order("analysis.buffer");
    assert_eq!(code_of(&relay.cancel(9, json!(50))), "request.unknown");
    // Aucune annulation n'est partie vers l'extension.
    s.assert_silent();
    s.reply(
        &order,
        json!({"layer_id": "t", "name": "Tampon", "feature_count": 1,
        "crs": "EPSG:2154"}),
    );
    let r = s.recv();
    assert_eq!(r["id"], 50);
    assert!(r["result"].is_object(), "{r}");
}

/// Pendant un ordre de l'extension, l'appel du relais reçoit `session.busy` aussitôt.
fn relay_busy(relay: &mut Relay, id: u64) {
    relay.run(id, "system.about", json!({}));
    let r = relay.recv();
    assert_eq!(
        (r["id"].clone(), code_of(&r)),
        (json!(id), "session.busy"),
        "{r}"
    );
}

/// Une fois l'appel de l'extension soldé, celui du relais passe.
fn relay_served(s: &Session, relay: &mut Relay, id: u64) {
    relay.run(id, "system.about", json!({}));
    s.expect_call("started");
    s.expect_call("finished");
    let r = relay.recv();
    assert_eq!(r["id"], id);
    assert!(r["result"].is_object(), "{r}");
}

#[test]
fn a_relay_is_busy_during_a_replay_or_a_plan_execution() {
    let name = unique_name("rejeu-plan");
    let composer = FakeComposer {
        gate: None,
        step: Some(json!({"command": "layer.describe", "params": {"layer_id": "l"}})),
    };
    let mut s = Session::start_with(&name, Some(composer));
    s.hello(&["layer.describe"]);
    s.switch_on();
    let mut relay = Relay::attached(&name);
    // Une étape QGIS au journal, pour que le rejeu dicte un ordre.
    s.send(json!({"jsonrpc":"2.0","id":40,"method":"command.run",
        "params":{"command":"layer.describe","params":{"layer_id":"l"}}}));
    let order = s.expect_order("layer.describe");
    s.reply(&order, described());
    assert_eq!(s.recv()["id"], 40);

    s.send(json!({"jsonrpc":"2.0","id":41,"method":"journal.replay"}));
    let order = s.expect_order("layer.describe");
    relay_busy(&mut relay, 1);
    s.reply(&order, described());
    let r = s.recv();
    assert_eq!(r["id"], 41);
    assert_eq!(r["result"]["identical"], true, "{r}");
    relay_served(&s, &mut relay, 2);

    let proposed = s.request("plan.propose", json!({"request": "Décris la couche."}));
    let plan_id = proposed["result"]["plan_id"].clone();
    assert!(plan_id.is_string(), "{proposed}");
    s.send(json!({"jsonrpc":"2.0","id":42,"method":"plan.execute","params":{"plan_id":plan_id}}));
    assert_eq!(s.recv()["method"], "plan.step");
    let order = s.expect_order("layer.describe");
    relay_busy(&mut relay, 3);
    s.reply(&order, described());
    let r = s.recv();
    assert_eq!(r["id"], 42);
    assert!(r["result"]["steps"].is_array(), "{r}");
    relay_served(&s, &mut relay, 4);
}
