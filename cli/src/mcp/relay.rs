//! Le relais de `cassini mcp` (ticket 5.25, AD-13, AD-29) : la connexion au canal d'IA externe
//! qu'une session QGIS a ouvert.
//!
//! Le relais transporte et ne décide rien. Il se branche par `external::connect`, fait la poignée
//! de main `external.attach` (AD-7), puis transmet des `command.run` et des `request.cancel`, et
//! rien d'autre (AD-6) ; aucun chemin n'y part (AD-9). Un fil lecteur aiguille chaque réponse vers
//! l'appel qui l'attend, par son `id` ; sur `external.session_lost` ou à la fin du flux, il solde
//! toutes les attentes par une perte et marque la connexion morte. Il ne choisit aucun mode : c'est
//! le serveur MCP qui le fait, à chaque requête.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::Duration;

use cassini_core::error::ErrorReport;
use cassini_core::external::{self, ChannelName, Stream};
use cassini_core::registry::PROTOCOL_VERSION;
use serde_json::{json, Value};
use tokio::sync::oneshot;

/// Délai laissé à la session pour répondre à `external.attach`. Son fil lecteur y répond à toute
/// heure (composition et ordre en vol compris) : au-delà, la session ne répond plus.
const ATTACH_TIMEOUT: Duration = Duration::from_secs(5);

/// La réponse de la session à une requête du relais.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Answer {
    /// Le `result` de la réponse.
    Result(Value),
    /// L'objet `error` de la réponse, entier.
    Error(Value),
    /// La session est perdue (`external.session_lost`, fin du flux, écriture en panne) avant
    /// d'avoir répondu.
    Lost,
}

/// Ce qui attend une réponse : un appel MCP (`oneshot`) ou la poignée de main (canal bloquant).
type Waiter = Box<dyn FnOnce(Answer) + Send>;

/// Appelé une fois, à la perte de la session.
type OnLost = Box<dyn FnOnce() + Send>;

/// L'état partagé avec le fil lecteur.
struct Pending {
    alive: bool,
    waiters: HashMap<u64, Waiter>,
    on_lost: Option<OnLost>,
}

/// Pourquoi le relais n'a pas pu se brancher.
#[derive(Debug)]
pub(crate) enum AttachError {
    /// La session parle un autre protocole : son message nomme le binaire à mettre à jour.
    Mismatch(ErrorReport),
    /// Aucun canal ouvert, ou une session qui ne répond pas comme il faut : le mode autonome.
    Unavailable,
}

/// Une connexion vivante (ou morte) au canal d'IA externe d'une session.
pub(crate) struct Relay {
    writer: Mutex<Stream>,
    stream: Stream,
    pending: Arc<Mutex<Pending>>,
    next_id: AtomicU64,
    commands: Vec<Value>,
    reader: Mutex<Option<JoinHandle<()>>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Relay {
    /// Se branche au canal `name` et fait la poignée de main. Bloquant (au plus
    /// `ATTACH_TIMEOUT` après la connexion) : à appeler hors du runtime.
    pub(crate) fn attach(name: &ChannelName) -> Result<Relay, AttachError> {
        let stream = external::connect(name).map_err(|_| AttachError::Unavailable)?;
        Relay::over(stream)
    }

    /// La poignée de main sur une connexion déjà établie.
    fn over(stream: Stream) -> Result<Relay, AttachError> {
        let (Ok(read_half), Ok(write_half)) = (stream.try_clone(), stream.try_clone()) else {
            stream.shutdown();
            return Err(AttachError::Unavailable);
        };
        let pending = Arc::new(Mutex::new(Pending {
            alive: true,
            waiters: HashMap::new(),
            on_lost: None,
        }));
        let reader = {
            let pending = pending.clone();
            std::thread::spawn(move || read_loop(read_half, &pending))
        };
        let mut relay = Relay {
            writer: Mutex::new(write_half),
            stream,
            pending,
            next_id: AtomicU64::new(1),
            commands: Vec::new(),
            reader: Mutex::new(Some(reader)),
        };
        let (tx, rx) = mpsc::channel();
        relay.request(
            "external.attach",
            json!({"protocol_version": PROTOCOL_VERSION}),
            Box::new(move |answer| {
                let _ = tx.send(answer);
            }),
        );
        let outcome = match rx.recv_timeout(ATTACH_TIMEOUT) {
            Ok(Answer::Result(result)) => match result.get("commands").and_then(Value::as_array) {
                Some(commands) => Ok(commands.clone()),
                None => Err(AttachError::Unavailable),
            },
            Ok(Answer::Error(error)) => match report(&error) {
                Some(report) if report.code == "session.protocol_mismatch" => {
                    Err(AttachError::Mismatch(report))
                }
                _ => Err(AttachError::Unavailable),
            },
            Ok(Answer::Lost) | Err(_) => Err(AttachError::Unavailable),
        };
        match outcome {
            Ok(commands) => {
                relay.commands = commands;
                Ok(relay)
            }
            Err(e) => {
                relay.shutdown();
                Err(e)
            }
        }
    }

    /// Le catalogue rendu par `external.attach` : les commandes disponibles de la session.
    pub(crate) fn commands(&self) -> &[Value] {
        &self.commands
    }

    /// La connexion est-elle encore vivante ?
    pub(crate) fn alive(&self) -> bool {
        lock(&self.pending).alive
    }

    /// `on_lost` sera appelé une fois, à la perte de la session ; tout de suite si elle est déjà
    /// perdue.
    pub(crate) fn on_lost(&self, on_lost: impl FnOnce() + Send + 'static) {
        let mut pending = lock(&self.pending);
        if pending.alive {
            pending.on_lost = Some(Box::new(on_lost));
        } else {
            drop(pending);
            on_lost();
        }
    }

    /// Transmet un `command.run` : rend son `id` (aucun si la session est déjà perdue) et la
    /// réponse à attendre.
    pub(crate) fn run(
        &self,
        command: &str,
        params: Value,
    ) -> (Option<u64>, oneshot::Receiver<Answer>) {
        let (tx, rx) = oneshot::channel();
        let id = self.request(
            "command.run",
            json!({"command": command, "params": params}),
            Box::new(move |answer| {
                let _ = tx.send(answer);
            }),
        );
        (id, rx)
    }

    /// Demande l'annulation du `command.run` `id` (notification `request.cancel` : la réponse
    /// du `command.run` dit l'issue).
    pub(crate) fn cancel(&self, id: u64) {
        let message = json!({"jsonrpc": "2.0", "method": "request.cancel", "params": {"id": id}});
        self.write(&message);
    }

    /// Envoie une requête ; `waiter` reçoit sa réponse, ou `Answer::Lost`. Rend l'`id`, aucun si
    /// la session est déjà perdue (`waiter` a alors reçu la perte).
    fn request(&self, method: &str, params: Value, waiter: Waiter) -> Option<u64> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        {
            let mut pending = lock(&self.pending);
            if !pending.alive {
                drop(pending);
                waiter(Answer::Lost);
                return None;
            }
            pending.waiters.insert(id, waiter);
        }
        let message = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.write(&message);
        Some(id)
    }

    /// Écrit une ligne ; une écriture en panne coupe la connexion, et le fil lecteur solde alors
    /// les attentes par une perte.
    fn write(&self, message: &Value) {
        // Le message et sa fin de ligne partent en une seule écriture : formatée sur la
        // connexion, qui n'a pas de tampon, une valeur JSON partirait en un appel système par
        // morceau (5.44).
        let mut line = serde_json::to_vec(message).expect("une valeur JSON se sérialise toujours");
        line.push(b'\n');
        let mut writer = lock(&self.writer);
        if writer.write_all(&line).is_err() {
            self.stream.shutdown();
        }
    }

    /// Ferme la connexion et attend le fil lecteur. Aucune perte n'est plus annoncée.
    pub(crate) fn shutdown(&self) {
        lock(&self.pending).on_lost = None;
        self.stream.shutdown();
        let reader = lock(&self.reader).take();
        if let Some(reader) = reader {
            let _ = reader.join();
        }
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// L'`ErrorReport` porté par `data` d'une erreur JSON-RPC (AD-21), s'il y en a un.
pub(crate) fn report(error: &Value) -> Option<ErrorReport> {
    let data = error.get("data")?;
    Some(ErrorReport {
        code: data.get("code")?.as_str()?.to_string(),
        message: data.get("message")?.as_str()?.to_string(),
        step: data
            .get("step")
            .and_then(Value::as_u64)
            .and_then(|s| usize::try_from(s).ok()),
    })
}

/// Le fil lecteur : aiguille les réponses par `id`. Toute autre ligne est ignorée ; la session
/// n'envoie aucune requête au relais.
fn read_loop(input: Stream, pending: &Mutex<Pending>) {
    let mut input = BufReader::new(input);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match input.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let Ok(message) = serde_json::from_slice::<Value>(&buf) else {
            continue;
        };
        if message.get("method").and_then(Value::as_str) == Some("external.session_lost") {
            break;
        }
        if message.get("method").is_some() {
            continue;
        }
        let Some(id) = message.get("id").and_then(Value::as_u64) else {
            continue;
        };
        let answer = if let Some(result) = message.get("result") {
            Answer::Result(result.clone())
        } else if let Some(error) = message.get("error") {
            Answer::Error(error.clone())
        } else {
            continue;
        };
        let waiter = lock(pending).waiters.remove(&id);
        if let Some(waiter) = waiter {
            waiter(answer);
        }
    }
    let (waiters, on_lost) = {
        let mut pending = lock(pending);
        pending.alive = false;
        (std::mem::take(&mut pending.waiters), pending.on_lost.take())
    };
    for waiter in waiters.into_values() {
        waiter(Answer::Lost);
    }
    if let Some(on_lost) = on_lost {
        on_lost();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cassini_core::external::Listener;
    use std::io::Read;

    /// Une session simulée : accepte une connexion et la rend, lecteur et écrivain.
    fn fake_session(tag: &str) -> (ChannelName, std::thread::JoinHandle<Stream>) {
        let name = ChannelName::tagged(&format!("relay-{tag}-{}", std::process::id())).unwrap();
        let mut listener = Listener::open(&name).unwrap();
        let accept = std::thread::spawn(move || {
            let stream = listener.accept().unwrap();
            drop(listener);
            stream
        });
        (name, accept)
    }

    fn read_line(input: &mut BufReader<Stream>) -> Value {
        let mut line = String::new();
        input.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    #[test]
    fn without_a_channel_the_relay_is_unavailable() {
        let name = ChannelName::tagged(&format!("relay-none-{}", std::process::id())).unwrap();
        assert!(matches!(
            Relay::attach(&name),
            Err(AttachError::Unavailable)
        ));
    }

    #[test]
    fn a_relay_attaches_runs_and_receives_the_loss() {
        let (name, accept) = fake_session("run");
        let session = std::thread::spawn(move || {
            let stream = accept.join().unwrap();
            let mut input = BufReader::new(stream.try_clone().unwrap());
            let mut out = stream.try_clone().unwrap();
            let attach = read_line(&mut input);
            assert_eq!(attach["method"], "external.attach");
            assert_eq!(attach["params"]["protocol_version"], PROTOCOL_VERSION);
            let reply = json!({"jsonrpc":"2.0","id":attach["id"],"result":{
                "protocol_version":PROTOCOL_VERSION,"core_version":"0","commands":[{"id":"a.b","available":true}]}});
            writeln!(out, "{reply}").unwrap();
            let run = read_line(&mut input);
            assert_eq!(run["method"], "command.run");
            assert_eq!(run["params"], json!({"command":"a.b","params":{"x":1}}));
            let reply = json!({"jsonrpc":"2.0","id":run["id"],"error":{"code":-32000,"message":"m",
                "data":{"code":"qgis.layer_missing","message":"La couche est absente."}}});
            writeln!(out, "{reply}").unwrap();
            // Un second appel reste en vol quand l'interrupteur s'éteint.
            let _second = read_line(&mut input);
            let lost = json!({"jsonrpc":"2.0","method":"external.session_lost",
                "params":{"reason":"switched_off","message":"x"}});
            writeln!(out, "{lost}").unwrap();
            stream.shutdown();
            let mut rest = Vec::new();
            let _ = input.read_to_end(&mut rest);
        });
        let relay = Relay::attach(&name).expect("le relais se branche");
        assert_eq!(relay.commands(), &[json!({"id":"a.b","available":true})]);
        let (lost_tx, lost_rx) = mpsc::channel();
        relay.on_lost(move || lost_tx.send(()).unwrap());
        let (id, rx) = relay.run("a.b", json!({"x": 1}));
        assert!(id.is_some());
        let answer = rx.blocking_recv().unwrap();
        let Answer::Error(error) = answer else {
            panic!("{answer:?}")
        };
        assert_eq!(report(&error).unwrap().code, "qgis.layer_missing");
        let (_, rx) = relay.run("a.b", json!({}));
        assert_eq!(rx.blocking_recv().unwrap(), Answer::Lost);
        lost_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!relay.alive());
        // Perdue, la connexion ne transmet plus rien.
        let (id, rx) = relay.run("a.b", json!({}));
        assert_eq!(id, None);
        assert_eq!(rx.blocking_recv().unwrap(), Answer::Lost);
        session.join().unwrap();
    }

    /// Une session qui accepte la connexion mais ne répond jamais à `external.attach` : le
    /// relais renonce après `ATTACH_TIMEOUT`, et le serveur reste autonome.
    #[test]
    fn a_silent_session_is_unavailable_after_the_attach_timeout() {
        let (name, accept) = fake_session("silent");
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let session = std::thread::spawn(move || {
            let stream = accept.join().unwrap();
            let mut input = BufReader::new(stream.try_clone().unwrap());
            let attach = read_line(&mut input);
            assert_eq!(attach["method"], "external.attach");
            // Silence jusqu'à la fin du test ; la connexion reste ouverte.
            let _ = done_rx.recv();
            drop(stream);
        });
        let started = std::time::Instant::now();
        let outcome = Relay::attach(&name);
        let elapsed = started.elapsed();
        assert!(matches!(outcome, Err(AttachError::Unavailable)));
        assert!(elapsed >= ATTACH_TIMEOUT, "{elapsed:?}");
        assert!(
            elapsed < ATTACH_TIMEOUT + Duration::from_secs(5),
            "{elapsed:?}"
        );
        done_tx.send(()).unwrap();
        session.join().unwrap();
    }

    #[test]
    fn a_protocol_mismatch_carries_the_message_of_the_session() {
        let (name, accept) = fake_session("mismatch");
        let session = std::thread::spawn(move || {
            let stream = accept.join().unwrap();
            let mut input = BufReader::new(stream.try_clone().unwrap());
            let mut out = stream.try_clone().unwrap();
            let attach = read_line(&mut input);
            let reply = json!({"jsonrpc":"2.0","id":attach["id"],"error":{"code":-32000,"message":"m",
                "data":{"code":"session.protocol_mismatch","message":"mettez à jour le relais"}}});
            writeln!(out, "{reply}").unwrap();
            let mut rest = Vec::new();
            let _ = input.read_to_end(&mut rest);
        });
        match Relay::attach(&name) {
            Err(AttachError::Mismatch(report)) => {
                assert_eq!(report.code, "session.protocol_mismatch");
                assert_eq!(report.message, "mettez à jour le relais");
            }
            other => panic!("{:?}", other.err()),
        }
        session.join().unwrap();
    }
}
