//! Transport du canal d'IA externe (AD-2, AD-14, AD-29).
//!
//! Le canal n'existe que quand l'utilisateur l'autorise dans le panneau : la session QGIS
//! (`cassini serve`) l'ouvre, `cassini mcp` en mode relais s'y branche (ticket 5.25). Il est
//! local et réservé à l'utilisateur courant, sans tokio, sur des fils bloquants :
//!
//! - Unix : socket de domaine (`std::os::unix::net`) dans un dossier propre à l'utilisateur, en
//!   0700 (mode et propriétaire vérifiés), jamais l'espace abstrait de Linux ; une socket orpheline
//!   laissée par un noyau mort est supprimée avant d'ouvrir.
//! - Windows : tube nommé (`windows-sys`), `PIPE_REJECT_REMOTE_CLIENTS`,
//!   `FILE_FLAG_FIRST_PIPE_INSTANCE` sur la première instance, descripteur réservé à l'utilisateur
//!   courant ; le client se branche en `SECURITY_SQOS_PRESENT`. AF_UNIX y est interdit (Winsock).
//!
//! Le nom du canal n'est jamais écrit dans un message : aucun chemin ne passe (AD-9).

use std::io::{self, Read, Write};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as sys;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as sys;

/// Variable de test : posée, elle isole le nom du canal par un suffixe. Lue seulement si elle est
/// posée ; jamais un réglage de l'utilisateur.
pub const TEST_TAG_VAR: &str = "CASSINI_EXTERNAL_TAG";

/// Le nom du canal d'IA externe de l'utilisateur courant.
///
/// Windows : `\\.\pipe\cassini-<utilisateur>-external`. Unix :
/// `$XDG_RUNTIME_DIR/cassini/external.sock`, sinon `<tmp>/cassini-<uid>/external.sock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelName(sys::Name);

impl ChannelName {
    /// Le nom du canal de l'utilisateur courant ; isolé par [`TEST_TAG_VAR`] si elle est posée.
    pub fn for_user() -> io::Result<ChannelName> {
        let tag = std::env::var(TEST_TAG_VAR).ok();
        ChannelName::tagged(tag.as_deref().unwrap_or(""))
    }

    /// Le nom du canal isolé par `tag` (tests) ; un `tag` vide donne le nom ordinaire. Seuls les
    /// lettres et chiffres ASCII, `-` et `_` sont gardés.
    pub fn tagged(tag: &str) -> io::Result<ChannelName> {
        let tag: String = tag
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .take(40)
            .collect();
        let tag = (!tag.is_empty()).then_some(tag);
        sys::name(tag.as_deref()).map(ChannelName)
    }

    /// Chemin de la socket (tests, et preuve de la socket orpheline).
    #[cfg(unix)]
    pub fn socket_path(&self) -> &std::path::Path {
        self.0.socket()
    }

    /// Nom du tube, `\\.\pipe\…` (tests, et preuve du refus d'un client distant).
    #[cfg(windows)]
    pub fn pipe_name(&self) -> String {
        self.0.pipe_name()
    }
}

/// Pourquoi le canal n'a pas pu s'ouvrir.
#[derive(Debug)]
pub enum OpenError {
    /// Une autre session de cet utilisateur tient déjà le canal (`external.channel_taken`).
    Taken,
    /// Toute autre panne du système.
    Io(io::Error),
}

impl From<io::Error> for OpenError {
    fn from(e: io::Error) -> Self {
        OpenError::Io(e)
    }
}

/// Le côté écoute du canal, tenu par la session.
pub struct Listener(sys::Listener);

impl Listener {
    /// Ouvre le canal. `OpenError::Taken` si une autre session le tient déjà.
    pub fn open(name: &ChannelName) -> Result<Listener, OpenError> {
        sys::Listener::open(&name.0).map(Listener)
    }

    /// Attend le prochain client. Bloquant : [`wake`] le débloque.
    pub fn accept(&mut self) -> io::Result<Stream> {
        self.0.accept().map(Stream)
    }
}

/// Débloque un `accept` en cours en se branchant soi-même au canal ; l'appelant de `accept`, qui
/// sait qu'il doit s'arrêter, referme la première connexion qu'il reçoit ensuite (la session y
/// écrit d'abord son adieu, au cas où ce serait un relais).
pub fn wake(name: &ChannelName) -> io::Result<()> {
    sys::connect(&name.0).map(drop)
}

/// Se branche au canal de la session (client : `cassini mcp` en relais, ticket 5.25).
pub fn connect(name: &ChannelName) -> io::Result<Stream> {
    sys::connect(&name.0).map(Stream)
}

/// Une connexion du canal, lisible et inscriptible depuis deux fils (un clone par fil).
pub struct Stream(sys::Stream);

impl Stream {
    /// Une seconde poignée sur la même connexion, pour lire et écrire depuis deux fils.
    pub fn try_clone(&self) -> io::Result<Stream> {
        self.0.try_clone().map(Stream)
    }

    /// Ferme la connexion dans les deux sens : une lecture ou une écriture bloquée, sur
    /// n'importe quel clone, rend aussitôt la main. Les octets déjà écrits restent lisibles par
    /// l'autre bout.
    pub fn shutdown(&self) {
        self.0.shutdown();
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_keeps_only_safe_characters() {
        let a = ChannelName::tagged("a/../b c").unwrap();
        let b = ChannelName::tagged("a..bc").unwrap();
        let plain = ChannelName::tagged("").unwrap();
        assert_ne!(a, plain);
        // `/`, `.` et l'espace sont retirés : aucun chemin ne sort du dossier du canal.
        assert_eq!(a, ChannelName::tagged("abc").unwrap());
        assert_eq!(b, a);
        assert_eq!(ChannelName::tagged("./ ").unwrap(), plain);
    }

    #[test]
    fn a_client_and_the_session_exchange_bytes_both_ways() {
        let name = ChannelName::tagged(&format!("mod-{}", std::process::id())).unwrap();
        let mut listener = Listener::open(&name).unwrap();
        let server = std::thread::spawn(move || {
            let mut s = listener.accept().unwrap();
            let mut buf = [0u8; 5];
            s.read_exact(&mut buf).unwrap();
            s.write_all(b"pong\n").unwrap();
            buf
        });
        let mut client = connect(&name).unwrap();
        client.write_all(b"ping\n").unwrap();
        let mut got = [0u8; 5];
        client.read_exact(&mut got).unwrap();
        assert_eq!(&got, b"pong\n");
        assert_eq!(&server.join().unwrap(), b"ping\n");
    }
}
