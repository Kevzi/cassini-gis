//! Canal d'IA externe côté session (AD-29, AD-2, AD-4, AD-14) : `session.external`, la poignée
//! de main `external.attach`, la table des connexions des relais et la notification
//! `external.session_lost`.
//!
//! Le canal est éteint par défaut et n'est jamais rallumé seul : seul `session.external
//! {enabled: true}` de l'extension l'ouvre. Un fil accepte les relais ; chaque connexion a un fil
//! lecteur, qui verse ses lignes dans la file unique de la boucle avec leur source, et un fil
//! écrivain, qui vide une file en mémoire : la boucle n'écrit jamais elle-même sur un relais et ne
//! bloque donc jamais sur lui (AD-4). L'exécution des appels d'un relais est le ticket suivant :
//! d'ici là, toute autre méthode que `external.attach` est inconnue, et rien n'est exécuté.

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::{classify, invalid_params, respond, Incoming, Msg, RpcError, Source, METHOD_NOT_FOUND};
use crate::external::{self, ChannelName, Listener, OpenError, Stream};
use crate::registry::PROTOCOL_VERSION;

/// Temps laissé au fil écrivain d'un relais pour remettre `external.session_lost` avant que la
/// connexion soit coupée ; un relais qui ne lit plus ne retient pas la session au-delà.
const FAREWELL: Duration = Duration::from_millis(500);

/// Identifiants des connexions, uniques sur toute la vie du processus.
static NEXT_CONN: AtomicU64 = AtomicU64::new(1);

/// Pourquoi les relais perdent la session.
#[derive(Clone, Copy)]
pub(super) enum Lost {
    /// L'utilisateur a éteint l'interrupteur.
    SwitchedOff,
    /// La session s'arrête (QGIS fermé, entrée standard close).
    SessionEnded,
}

/// Une connexion de relais, vue de la boucle.
struct Conn {
    /// File de l'écrivain : une ligne JSON par élément ; retirée pour laisser l'écrivain finir.
    out: Option<Sender<String>>,
    /// Se ferme quand l'écrivain a fini (tout écrit, ou panne).
    written: Receiver<()>,
    stream: Stream,
    reader: JoinHandle<()>,
    writer: JoinHandle<()>,
}

impl Conn {
    /// Confie `farewell` à l'écrivain et ferme sa file : il écrit ce qui reste, puis s'arrête.
    fn farewell(&mut self, farewell: &Value) {
        if let Some(out) = self.out.take() {
            let _ = out.send(farewell.to_string());
        }
    }

    /// Laisse l'écrivain finir jusqu'à `deadline` (aucune attente sans échéance), puis coupe la
    /// connexion et attend ses deux fils, qui rendent aussitôt la main.
    fn finish(mut self, deadline: Option<Instant>) {
        drop(self.out.take());
        if let Some(deadline) = deadline {
            let _ = self
                .written
                .recv_timeout(deadline.saturating_duration_since(Instant::now()));
        }
        self.stream.shutdown();
        let _ = self.writer.join();
        let _ = self.reader.join();
    }
}

type Conns = Arc<Mutex<HashMap<u64, Conn>>>;

/// Le canal ouvert : le fil d'acceptation et ce qu'il faut pour l'arrêter.
struct Open {
    name: ChannelName,
    stop: Arc<AtomicBool>,
    accept: JoinHandle<()>,
}

/// L'état du canal d'IA externe d'une session.
pub(super) struct Hub {
    /// Nom imposé (tests) ; `None` : le nom de l'utilisateur, résolu à l'allumage.
    name: Option<ChannelName>,
    /// Entrée de la file unique de la boucle, donnée aux fils lecteurs.
    tx: Sender<Msg>,
    open: std::cell::RefCell<Option<Open>>,
    conns: Conns,
    /// Le catalogue disponible de la session (AD-25), fixé à `session.hello`.
    catalogue: std::cell::RefCell<Option<Vec<Value>>>,
    /// La case « Me demander avant chaque appel » ; gardée pour le ticket suivant.
    ask: std::cell::Cell<bool>,
}

impl Hub {
    pub(super) fn new(name: Option<ChannelName>, tx: Sender<Msg>) -> Hub {
        Hub {
            name,
            tx,
            open: Default::default(),
            conns: Default::default(),
            catalogue: Default::default(),
            ask: Default::default(),
        }
    }

    /// Fixe le catalogue rendu aux relais : les seules commandes disponibles.
    pub(super) fn set_catalogue(&self, commands: Vec<Value>) {
        *self.catalogue.borrow_mut() = Some(commands);
    }

    fn enable(&self, ask: bool) -> Result<(), RpcError> {
        if self.open.borrow().is_some() {
            self.ask.set(ask);
            return Ok(());
        }
        let io_error = |e: io::Error| {
            RpcError::coded(
                "external.io",
                format!("Le canal d'IA externe n'a pas pu s'ouvrir : {e}."),
            )
        };
        let name = match &self.name {
            Some(name) => name.clone(),
            None => ChannelName::for_user().map_err(io_error)?,
        };
        let listener = Listener::open(&name).map_err(|e| match e {
            OpenError::Taken => RpcError::coded(
                "external.channel_taken",
                "Une autre session QGIS de cet utilisateur tient déjà le canal d'IA externe : \
                 éteignez-le dans l'autre fenêtre de QGIS, puis rallumez-le ici."
                    .to_string(),
            ),
            OpenError::Io(e) => io_error(e),
        })?;
        let stop = Arc::new(AtomicBool::new(false));
        let accept = {
            let stop = stop.clone();
            let conns = self.conns.clone();
            let tx = self.tx.clone();
            thread::spawn(move || accept_loop(listener, &stop, &conns, &tx))
        };
        self.ask.set(ask);
        *self.open.borrow_mut() = Some(Open { name, stop, accept });
        Ok(())
    }

    /// Ferme le canal : plus d'acceptation, la socket est retirée, chaque relais reçoit
    /// `external.session_lost` puis sa connexion est coupée. Sans effet si le canal est éteint.
    pub(super) fn close(&self, why: Lost) {
        let Some(open) = self.open.borrow_mut().take() else {
            return;
        };
        open.stop.store(true, Ordering::SeqCst);
        // Le fil d'acceptation est réveillé par une connexion à soi-même ; s'il est déjà sorti
        // (panne), la connexion échoue et il n'y a rien à attendre.
        // Sans réveil, le fil resterait bloqué : il est laissé à lui-même plutôt qu'attendu.
        if external::wake(&open.name).is_ok() {
            let _ = open.accept.join();
        }
        let mut conns: Vec<Conn> = self
            .conns
            .lock()
            .map(|mut c| c.drain().map(|(_, conn)| conn).collect())
            .unwrap_or_default();
        // Tous les adieux partent d'abord, puis une seule échéance vaut pour tous les relais :
        // ceux qui ne lisent plus retiennent la session au plus `FAREWELL` en tout (AD-4).
        let farewell = session_lost(why);
        for conn in &mut conns {
            conn.farewell(&farewell);
        }
        let deadline = Instant::now() + FAREWELL;
        for conn in conns {
            conn.finish(Some(deadline));
        }
    }

    /// Un relais s'est débranché : sa connexion est retirée.
    pub(super) fn detached(&self, conn: u64) {
        let gone = self.conns.lock().ok().and_then(|mut c| c.remove(&conn));
        if let Some(conn) = gone {
            conn.finish(None);
        }
    }

    fn send(&self, conn: u64, message: &Value) {
        if let Ok(conns) = self.conns.lock() {
            if let Some(c) = conns.get(&conn) {
                if let Some(out) = &c.out {
                    let _ = out.send(message.to_string());
                }
            }
        }
    }

    /// Une ligne reçue d'un relais. Elle n'exige jamais l'exécutant : elle est traitée à toute
    /// heure, ordre en vol compris.
    pub(super) fn on_line(&self, conn: u64, line: &str) {
        let response = match classify(line) {
            Err(response) => Some(response),
            // Le noyau n'envoie aucune requête au relais : une réponse est ignorée.
            Ok(Incoming::Reply { .. }) => None,
            Ok(Incoming::Call { id, method, params }) => {
                let outcome = match method.as_str() {
                    "external.attach" => self.attach(params.as_ref()),
                    // `command.run` et `request.cancel` du relais : ticket suivant.
                    _ => Err(RpcError::protocol(METHOD_NOT_FOUND, "Méthode inconnue.")),
                };
                respond(id, outcome)
            }
        };
        if let Some(response) = response {
            self.send(conn, &response);
        }
    }

    /// `external.attach {protocol_version}` : la poignée de main du relais (AD-7, AD-29).
    fn attach(&self, params: Option<&Value>) -> Result<Value, RpcError> {
        let version = params
            .and_then(|p| p.get("protocol_version"))
            .and_then(Value::as_u64)
            .ok_or_else(|| invalid_params("protocol_version doit être un entier."))?;
        let expected = u64::from(PROTOCOL_VERSION);
        if version != expected {
            let update = if version < expected {
                "mettez à jour le binaire cassini du relais (cassini mcp)"
            } else {
                "mettez à jour le paquet du noyau Cassini de la session QGIS"
            };
            return Err(RpcError::coded(
                "session.protocol_mismatch",
                format!("Le relais parle le protocole {version}, la session QGIS le {expected} : {update}."),
            ));
        }
        let Some(commands) = self.catalogue.borrow().clone() else {
            return Err(RpcError::coded(
                "session.not_started",
                "La session QGIS n'est pas ouverte.".to_string(),
            ));
        };
        Ok(json!({
            "protocol_version": PROTOCOL_VERSION,
            "core_version": crate::VERSION,
            "commands": commands,
        }))
    }
}

impl Drop for Hub {
    fn drop(&mut self) {
        self.close(Lost::SessionEnded);
    }
}

fn session_lost(why: Lost) -> Value {
    let (reason, message) = match why {
        Lost::SwitchedOff => (
            "switched_off",
            "L'utilisateur a fermé le canal d'IA externe dans QGIS.",
        ),
        Lost::SessionEnded => ("session_ended", "La session QGIS s'est arrêtée."),
    };
    json!({"jsonrpc": "2.0", "method": "external.session_lost",
        "params": {"reason": reason, "message": message}})
}

/// `session.external {enabled, ask}` (extension vers noyau) : allume ou éteint le canal.
pub(super) fn session_external(hub: &Hub, params: Option<&Value>) -> Result<Value, RpcError> {
    let enabled = params
        .and_then(|p| p.get("enabled"))
        .and_then(Value::as_bool)
        .ok_or_else(|| invalid_params("enabled (booléen) est requis."))?;
    if !enabled {
        hub.close(Lost::SwitchedOff);
        return Ok(json!({"enabled": false}));
    }
    let ask = params
        .and_then(|p| p.get("ask"))
        .and_then(Value::as_bool)
        .ok_or_else(|| invalid_params("ask (booléen) est requis pour allumer le canal."))?;
    hub.enable(ask)?;
    Ok(json!({"enabled": true, "ask": hub.ask.get()}))
}

/// Le fil d'acceptation : chaque relais reçoit un fil lecteur et un fil écrivain.
fn accept_loop(mut listener: Listener, stop: &AtomicBool, conns: &Conns, tx: &Sender<Msg>) {
    loop {
        let accepted = listener.accept();
        if stop.load(Ordering::SeqCst) {
            // La connexion de réveil (ou un relais arrivé trop tard) est refermée.
            if let Ok(stream) = accepted {
                stream.shutdown();
            }
            return;
        }
        match accepted {
            Ok(stream) => start_conn(stream, conns, tx),
            // Une panne passagère (descripteurs épuisés) ne fait pas tourner le fil à vide.
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }
}

/// Inscrit la connexion et lance ses deux fils. La table reste verrouillée jusqu'à
/// l'inscription : une réponse de la boucle à la première ligne du relais trouve toujours sa
/// connexion.
fn start_conn(stream: Stream, conns: &Conns, tx: &Sender<Msg>) {
    let id = NEXT_CONN.fetch_add(1, Ordering::SeqCst);
    let (Ok(read_half), Ok(mut write_half)) = (stream.try_clone(), stream.try_clone()) else {
        stream.shutdown();
        return;
    };
    let Ok(mut table) = conns.lock() else {
        stream.shutdown();
        return;
    };
    let (out, lines) = mpsc::channel::<String>();
    let (done, written) = mpsc::channel::<()>();
    let writer = thread::spawn(move || {
        let _done = done;
        for line in lines {
            if writeln!(write_half, "{line}").is_err() {
                return;
            }
        }
    });
    let tx = tx.clone();
    let reader = thread::spawn(move || {
        let mut input = BufReader::new(read_half);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match input.read_until(b'\n', &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let text = String::from_utf8_lossy(&buf);
                    let line = text.trim();
                    if !line.is_empty()
                        && tx
                            .send(Msg::Line(Source::External(id), line.to_string()))
                            .is_err()
                    {
                        return;
                    }
                }
            }
        }
        let _ = tx.send(Msg::Detached(id));
    });
    table.insert(
        id,
        Conn {
            out: Some(out),
            written,
            stream,
            reader,
            writer,
        },
    );
}
