//! Canal d'IA externe côté session (AD-29, ticket 5.24) : une extension simulée par des flux en
//! mémoire, un relais brut branché au vrai canal (socket Unix ou tube nommé), un nom de canal
//! propre à chaque test. Chaque ligne de la matrice du ticket a son test.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cassini_core::external::{self, ChannelName};
use cassini_core::registry::{Registry, PROTOCOL_VERSION};
use cassini_core::serve::serve_with_channel;
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

impl Session {
    fn start(name: &ChannelName) -> Session {
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
            serve_with_channel(&registry, None, name, reader, writer)
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
    relay.send(json!({"jsonrpc":"2.0","id":7,"method":"command.run",
        "params":{"command":"layer.describe","params":{"layer_id":"l"}}}));
    let r = relay.recv();
    assert_eq!(r["id"], 7);
    assert_eq!(r["error"]["code"], -32601);
    relay.send(json!({"jsonrpc":"2.0","id":8,"method":"session.hello",
        "params":{"protocol_version":PROTOCOL_VERSION,"commands":[],"project":"/x/p.qgz"}}));
    assert_eq!(relay.recv()["error"]["code"], -32601);
    // Une notification ne reçoit rien ; une ligne illisible, une erreur à identifiant nul.
    relay.send(json!({"jsonrpc":"2.0","method":"command.run","params":{}}));
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
    // relais muets retiennent l'extinction une demi-seconde, pas deux.
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
    // Un appel de l'extension passe après toutes ces lignes : elles sont traitées.
    let r = s.request("command.run", json!({"command": "system.about"}));
    assert!(r["result"].is_object(), "{r}");
    let started = std::time::Instant::now();
    let off = s.switch(json!({"enabled": false}));
    assert_eq!(off["result"], json!({"enabled": false}));
    assert!(
        started.elapsed() < Duration::from_millis(1500),
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
