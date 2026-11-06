//! Canal d'IA externe côté session (AD-29, AD-2, AD-4, AD-14) : `session.external`, la poignée
//! de main `external.attach`, la table des connexions des relais et la notification
//! `external.session_lost`, et l'état d'occupation unique de la session.
//!
//! Le canal est éteint par défaut et n'est jamais rallumé seul : seul `session.external
//! {enabled: true}` de l'extension l'ouvre. Un fil accepte les relais ; chaque connexion a un fil
//! lecteur et un fil écrivain, qui vide une file en mémoire : la boucle n'écrit jamais elle-même
//! sur un relais et ne bloque donc jamais sur lui (AD-4). Le fil lecteur répond lui-même à tout ce
//! qui n'exige pas l'exécutant (`external.attach`, lignes illisibles, méthodes inconnues,
//! `session.busy`), composition de plan comprise ; il ne verse dans la file de la boucle que le
//! `command.run` dont il a réservé l'occupation, et `request.cancel` (ticket 5.28).

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::{
    busy, classify, invalid_params, respond, Incoming, Msg, RelayCall, RpcError, Source,
    METHOD_NOT_FOUND,
};
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
    /// Levé quand la boucle retire la connexion : le fil lecteur finit au plus la ligne en cours,
    /// puis n'en traite plus aucune, même déjà reçue ; l'attente de sa fin ne dépend donc pas de
    /// tout ce que le relais a envoyé.
    retired: Arc<AtomicBool>,
    reader: JoinHandle<()>,
    writer: JoinHandle<()>,
}

impl Conn {
    /// Le fil lecteur s'arrête avant sa prochaine ligne.
    fn retire(&self) {
        self.retired.store(true, Ordering::SeqCst);
    }

    /// Confie `farewell` à l'écrivain et ferme sa file : il écrit ce qui reste, puis s'arrête.
    fn farewell(&mut self, farewell: &Value) {
        if let Some(out) = self.out.take() {
            let _ = out.send(farewell.to_string());
        }
    }

    /// Laisse l'écrivain finir jusqu'à `deadline` (aucune attente sans échéance), puis coupe la
    /// connexion et attend ses deux fils, qui rendent aussitôt la main.
    fn finish(mut self, deadline: Option<Instant>) {
        self.retire();
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

/// L'état d'occupation unique du processus (AD-29), partagé par la boucle et les fils lecteurs.
#[derive(Default)]
struct Occupancy {
    /// Qui tient l'exécutant : composition ou exécution de plan, `command.run`, `journal.replay`
    /// de l'extension, ou appel d'un relais (réservé par son fil lecteur, encore en file compris).
    busy: Option<Source>,
    /// Un plan proposé attend sa validation : il n'occupe que pour les relais.
    plan_pending: bool,
}

/// Ce que la boucle et les fils lecteurs des relais partagent.
#[derive(Default)]
struct Shared {
    conns: Mutex<HashMap<u64, Conn>>,
    /// Le catalogue disponible de la session (AD-25), fixé à `session.hello`.
    catalogue: Mutex<Option<Vec<Value>>>,
    occupancy: Mutex<Occupancy>,
    /// La case « Me demander avant chaque appel ».
    ask: AtomicBool,
}

impl Shared {
    fn occupancy(&self) -> std::sync::MutexGuard<'_, Occupancy> {
        self.occupancy.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Réserve l'exécutant pour `source` : vrai si rien ne l'occupe. Un relais attend aussi
    /// qu'aucun plan ne soit en attente de validation.
    fn reserve(&self, source: Source) -> bool {
        let mut occupancy = self.occupancy();
        let free =
            occupancy.busy.is_none() && (source == Source::Extension || !occupancy.plan_pending);
        if free {
            occupancy.busy = Some(source);
        }
        free
    }

    /// Répond à un relais par la table des connexions ; une connexion disparue ne reçoit rien.
    fn send(&self, conn: u64, message: &Value) {
        if let Ok(conns) = self.conns.lock() {
            if let Some(c) = conns.get(&conn) {
                if let Some(out) = &c.out {
                    let _ = out.send(message.to_string());
                }
            }
        }
    }
}

/// Le canal ouvert : le fil d'acceptation et ce qu'il faut pour l'arrêter.
struct Open {
    name: ChannelName,
    /// Posé à la fermeture, avec la raison que reçoit un relais accepté après l'arrêt.
    stop: Arc<OnceLock<Lost>>,
    accept: JoinHandle<()>,
}

/// L'état du canal d'IA externe d'une session.
pub(super) struct Hub {
    /// Nom imposé (tests) ; `None` : le nom de l'utilisateur, résolu à l'allumage.
    name: Option<ChannelName>,
    /// Entrée de la file unique de la boucle, donnée aux fils lecteurs.
    tx: Sender<Msg>,
    open: std::cell::RefCell<Option<Open>>,
    shared: Arc<Shared>,
}

impl Hub {
    pub(super) fn new(name: Option<ChannelName>, tx: Sender<Msg>) -> Hub {
        Hub {
            name,
            tx,
            open: Default::default(),
            shared: Default::default(),
        }
    }

    /// Fixe le catalogue rendu aux relais : les seules commandes disponibles.
    pub(super) fn set_catalogue(&self, commands: Vec<Value>) {
        if let Ok(mut catalogue) = self.shared.catalogue.lock() {
            *catalogue = Some(commands);
        }
    }

    /// Réserve l'exécutant pour `source` (voir `Occupancy`) : vrai si rien ne l'occupe.
    pub(super) fn reserve(&self, source: Source) -> bool {
        self.shared.reserve(source)
    }

    /// Libère l'exécutant, quelle que soit l'issue de ce qui l'occupait.
    pub(super) fn release(&self) {
        self.shared.occupancy().busy = None;
    }

    /// Quelqu'un tient l'exécutant (un relais dont l'appel attend encore en file compris).
    pub(super) fn is_busy(&self) -> bool {
        self.shared.occupancy().busy.is_some()
    }

    /// Libère l'exécutant s'il est tenu par `source`, et seulement alors.
    pub(super) fn release_for(&self, source: Source) {
        let mut occupancy = self.shared.occupancy();
        if occupancy.busy == Some(source) {
            occupancy.busy = None;
        }
    }

    /// Un plan attend, ou n'attend plus, sa validation.
    pub(super) fn set_plan_pending(&self, pending: bool) {
        self.shared.occupancy().plan_pending = pending;
    }

    /// La case « Me demander avant chaque appel ».
    pub(super) fn ask(&self) -> bool {
        self.shared.ask.load(Ordering::SeqCst)
    }

    /// La connexion `conn` est-elle encore inscrite (ni débranchée, ni coupée par l'interrupteur) ?
    pub(super) fn has_conn(&self, conn: u64) -> bool {
        self.shared
            .conns
            .lock()
            .is_ok_and(|conns| conns.contains_key(&conn))
    }

    fn enable(&self, ask: bool) -> Result<(), RpcError> {
        if self.open.borrow().is_some() {
            self.shared.ask.store(ask, Ordering::SeqCst);
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
        let stop = Arc::new(OnceLock::new());
        let accept = {
            let stop = stop.clone();
            let shared = self.shared.clone();
            let tx = self.tx.clone();
            thread::spawn(move || accept_loop(listener, &stop, &shared, &tx))
        };
        self.shared.ask.store(ask, Ordering::SeqCst);
        *self.open.borrow_mut() = Some(Open { name, stop, accept });
        Ok(())
    }

    /// Ferme le canal : plus d'acceptation, la socket est retirée, chaque relais reçoit
    /// `external.session_lost` puis sa connexion est coupée. Sans effet si le canal est éteint.
    /// Un relais connecté juste avant, pas encore accepté, reçoit lui aussi l'adieu s'il est le
    /// premier en file ; un deuxième en file, ou un relais arrivé après le réveil, voit seulement
    /// la connexion se fermer.
    pub(super) fn close(&self, why: Lost) {
        let Some(open) = self.open.borrow_mut().take() else {
            return;
        };
        let _ = open.stop.set(why);
        // Le fil d'acceptation est réveillé par une connexion à soi-même ; s'il est déjà sorti
        // (panne), la connexion échoue et il n'y a rien à attendre.
        // Sans réveil, le fil resterait bloqué : il est laissé à lui-même plutôt qu'attendu.
        if external::wake(&open.name).is_ok() {
            let _ = open.accept.join();
        }
        let mut conns: Vec<Conn> = self
            .shared
            .conns
            .lock()
            .map(|mut c| c.drain().map(|(_, conn)| conn).collect())
            .unwrap_or_default();
        // Tous les adieux partent d'abord, puis une seule échéance vaut pour tous les relais :
        // ceux qui ne lisent plus retiennent la session au plus `FAREWELL` en tout (AD-4).
        // Les lecteurs cessent d'abord de traiter les lignes en file : un relais qui a beaucoup
        // envoyé ne retient pas l'extinction au-delà de l'échéance.
        for conn in &conns {
            conn.retire();
        }
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
        let gone = self
            .shared
            .conns
            .lock()
            .ok()
            .and_then(|mut c| c.remove(&conn));
        if let Some(conn) = gone {
            conn.finish(None);
        }
    }

    /// Répond à un relais ; une connexion disparue ne reçoit rien.
    pub(super) fn send(&self, conn: u64, message: &Value) {
        self.shared.send(conn, message);
    }
}

/// Une ligne reçue d'un relais, traitée par son fil lecteur, à toute heure (composition et ordre
/// en vol compris). Seuls un `command.run` dont l'occupation est réservée et `request.cancel` vont
/// à la boucle. Rend faux si la boucle n'écoute plus.
fn on_line(shared: &Shared, tx: &Sender<Msg>, conn: u64, line: &str) -> bool {
    let response = match classify(line) {
        Err(response) => Some(response),
        // Le noyau n'envoie aucune requête au relais : une réponse est ignorée.
        Ok(Incoming::Reply { .. }) => None,
        Ok(Incoming::Call { id, method, params }) => match method.as_str() {
            "external.attach" => respond(id, attach(shared, params.as_ref())),
            // Un appel sans identifiant ne pourrait être ni répondu ni annulé : il est ignoré.
            "command.run" => match id {
                None => None,
                Some(id) => match run(shared, conn) {
                    Ok(()) => {
                        let call = RelayCall::Run { id, params };
                        return tx.send(Msg::Relay(conn, call)).is_ok();
                    }
                    Err(e) => respond(Some(id), Err(e)),
                },
            },
            "request.cancel" => {
                let call = RelayCall::Cancel { id, params };
                return tx.send(Msg::Relay(conn, call)).is_ok();
            }
            _ => respond(
                id,
                Err(RpcError::protocol(METHOD_NOT_FOUND, "Méthode inconnue.")),
            ),
        },
    };
    if let Some(response) = response {
        shared.send(conn, &response);
    }
    true
}

/// Le `command.run` d'un relais peut-il partir ? Session ouverte, exécutant réservé.
fn run(shared: &Shared, conn: u64) -> Result<(), RpcError> {
    let started = shared.catalogue.lock().is_ok_and(|c| c.is_some());
    if !started {
        return Err(not_started_for_relay());
    }
    if !shared.reserve(Source::External(conn)) {
        // Rien ne tient l'exécutant : c'est un plan proposé qui attend sa validation.
        if shared.occupancy().busy.is_none() {
            return Err(RpcError::coded(
                "session.busy",
                "Un plan proposé attend la validation de l'utilisateur dans QGIS : réessayez \
                 quand il aura été exécuté ou rejeté."
                    .to_string(),
            ));
        }
        return Err(busy());
    }
    Ok(())
}

fn not_started_for_relay() -> RpcError {
    RpcError::coded(
        "session.not_started",
        "La session QGIS n'est pas ouverte.".to_string(),
    )
}

/// `external.attach {protocol_version}` : la poignée de main du relais (AD-7, AD-29).
fn attach(shared: &Shared, params: Option<&Value>) -> Result<Value, RpcError> {
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
            format!(
                "Le relais parle le protocole {version}, la session QGIS le {expected} : {update}."
            ),
        ));
    }
    let commands = shared.catalogue.lock().ok().and_then(|c| c.clone());
    let Some(commands) = commands else {
        return Err(not_started_for_relay());
    };
    Ok(json!({
        "protocol_version": PROTOCOL_VERSION,
        "core_version": crate::VERSION,
        "commands": commands,
    }))
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
    Ok(json!({"enabled": true, "ask": hub.ask()}))
}

/// Le fil d'acceptation : chaque relais reçoit un fil lecteur et un fil écrivain.
fn accept_loop(
    mut listener: Listener,
    stop: &OnceLock<Lost>,
    shared: &Arc<Shared>,
    tx: &Sender<Msg>,
) {
    loop {
        let accepted = listener.accept();
        if let Some(&why) = stop.get() {
            // La file d'attente du canal sert dans l'ordre d'arrivée : un relais connecté juste
            // avant la fermeture passe avant la connexion de réveil (bogue 5.42). Il reçoit
            // l'adieu en une seule écriture, qu'une connexion neuve, au tampon vide, ne bloque
            // pas ; la connexion de réveil le reçoit aussi, sans effet. Puis la connexion est
            // refermée.
            if let Ok(mut stream) = accepted {
                let _ = stream.write_all(format!("{}\n", session_lost(why)).as_bytes());
                stream.shutdown();
            }
            return;
        }
        match accepted {
            Ok(stream) => start_conn(stream, shared, tx),
            // Une panne passagère (descripteurs épuisés) ne fait pas tourner le fil à vide.
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }
}

/// Inscrit la connexion et lance ses deux fils. La table reste verrouillée jusqu'à
/// l'inscription : une réponse à la première ligne du relais trouve toujours sa connexion. Le fil
/// lecteur répond par la table, sans garder d'entrée de la file de l'écrivain : sinon l'écrivain
/// ne finirait qu'à l'échéance des adieux.
fn start_conn(stream: Stream, shared: &Arc<Shared>, tx: &Sender<Msg>) {
    let id = NEXT_CONN.fetch_add(1, Ordering::SeqCst);
    let (Ok(read_half), Ok(mut write_half)) = (stream.try_clone(), stream.try_clone()) else {
        stream.shutdown();
        return;
    };
    let Ok(mut table) = shared.conns.lock() else {
        stream.shutdown();
        return;
    };
    let (out, lines) = mpsc::channel::<String>();
    let (done, written) = mpsc::channel::<()>();
    let writer = thread::spawn(move || {
        let _done = done;
        for line in lines {
            // La ligne et sa fin partent en une seule écriture (5.44).
            let mut bytes = line.into_bytes();
            bytes.push(b'\n');
            if write_half.write_all(&bytes).is_err() {
                return;
            }
        }
    });
    let tx = tx.clone();
    let reader_shared = shared.clone();
    let retired = Arc::new(AtomicBool::new(false));
    let reader_retired = retired.clone();
    let reader = thread::spawn(move || {
        let shared = reader_shared;
        let mut input = BufReader::new(read_half);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            let read = input.read_until(b'\n', &mut buf);
            // Connexion retirée par la boucle : rien à traiter, et pas de `Msg::Detached`, même
            // en fin de flux, puisque la connexion n'est déjà plus dans la table.
            if reader_retired.load(Ordering::SeqCst) {
                return;
            }
            match read {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let text = String::from_utf8_lossy(&buf);
                    let line = text.trim();
                    if !line.is_empty() && !on_line(&shared, &tx, id, line) {
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
            retired,
            reader,
            writer,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un relais connecté juste avant la fermeture, encore dans la file d'attente du canal, est
    /// accepté après l'arrêt : il reçoit l'adieu, puis la fin du flux (bogue 5.42).
    #[test]
    fn a_relay_queued_at_the_close_gets_the_farewell() {
        let name = ChannelName::tagged(&format!("file-{}", std::process::id())).unwrap();
        let listener = Listener::open(&name).unwrap();
        // Connecté, pas encore accepté : le fil d'acceptation n'est pas lancé.
        let client = external::connect(&name).unwrap();
        let stop = Arc::new(OnceLock::new());
        assert!(stop.set(Lost::SessionEnded).is_ok());
        let shared = Arc::new(Shared::default());
        let (tx, _rx) = mpsc::channel();
        let accept = {
            let stop = stop.clone();
            thread::spawn(move || accept_loop(listener, &stop, &shared, &tx))
        };
        let mut input = BufReader::new(client);
        let mut line = String::new();
        input.read_line(&mut line).unwrap();
        let lost: Value = serde_json::from_str(line.trim()).expect("l'adieu était attendu");
        assert_eq!(lost["method"], "external.session_lost");
        assert_eq!(lost["params"]["reason"], "session_ended");
        line.clear();
        assert_eq!(
            input.read_line(&mut line).unwrap(),
            0,
            "fin de flux attendue"
        );
        accept.join().unwrap();
    }
}
