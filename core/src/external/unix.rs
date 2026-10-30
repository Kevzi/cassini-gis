//! Canal d'IA externe sous Unix : socket de domaine dans un dossier privé de l'utilisateur.
//!
//! Le dossier est en 0700 et appartient à l'utilisateur courant, vérifié à chaque ouverture et à
//! chaque branchement : personne d'autre ne peut y poser ni y joindre une socket. L'uid est lu sur
//! un fichier qu'on vient de créer, sans libc. Jamais l'espace abstrait de Linux, qui n'a pas de
//! droits de fichier.

use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::OpenError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    dir: PathBuf,
    socket: PathBuf,
}

impl Name {
    pub fn socket(&self) -> &Path {
        &self.socket
    }
}

pub fn name(tag: Option<&str>) -> io::Result<Name> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute());
    let dir = match runtime {
        Some(run) => run.join("cassini"),
        None => std::env::temp_dir().join(format!("cassini-{}", current_uid()?)),
    };
    let file = match tag {
        Some(tag) => format!("external-{tag}.sock"),
        None => "external.sock".to_string(),
    };
    Ok(Name {
        socket: dir.join(file),
        dir,
    })
}

/// L'uid du processus, lu sur un fichier qu'on vient de créer dans le dossier temporaire.
fn current_uid() -> io::Result<u32> {
    static N: AtomicUsize = AtomicUsize::new(0);
    let tmp = std::env::temp_dir();
    for _ in 0..16 {
        let probe = tmp.join(format!(
            "cassini-uid-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        match OpenOptions::new().write(true).create_new(true).open(&probe) {
            Ok(file) => {
                let uid = file.metadata().map(|m| m.uid());
                let _ = fs::remove_file(&probe);
                return uid;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::other("impossible de lire l'utilisateur courant"))
}

fn not_private() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "le dossier du canal d'IA externe n'est pas privé à l'utilisateur",
    )
}

/// Vérifie que `dir` est un dossier (pas un lien), à l'utilisateur `uid`, en 0700.
fn check_private(dir: &Path, uid: u32) -> io::Result<()> {
    let meta = fs::symlink_metadata(dir)?;
    if !meta.file_type().is_dir() || meta.uid() != uid || meta.mode() & 0o777 != 0o700 {
        return Err(not_private());
    }
    Ok(())
}

/// Crée le dossier privé s'il manque, puis le vérifie : un dossier existant n'est jamais corrigé.
fn private_dir(dir: &Path) -> io::Result<()> {
    let uid = current_uid()?;
    match DirBuilder::new().mode(0o700).create(dir) {
        Ok(()) => {
            // Le masque de création peut retirer des droits : on pose le mode exact.
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    check_private(dir, uid)
}

pub struct Listener {
    listener: UnixListener,
    socket: PathBuf,
}

impl Listener {
    pub fn open(name: &Name) -> Result<Listener, OpenError> {
        private_dir(&name.dir)?;
        match fs::symlink_metadata(&name.socket) {
            Ok(meta) => match UnixStream::connect(&name.socket) {
                // Quelqu'un écoute : une autre session tient le canal.
                Ok(_) => return Err(OpenError::Taken),
                // Personne n'écoute : socket orpheline d'un noyau mort, supprimée.
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
                    ) =>
                {
                    if !meta.file_type().is_socket() {
                        return Err(OpenError::Io(io::Error::new(
                            io::ErrorKind::AlreadyExists,
                            "le nom du canal d'IA externe est pris par un fichier",
                        )));
                    }
                    match fs::remove_file(&name.socket) {
                        Ok(()) => {}
                        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                        Err(e) => return Err(OpenError::Io(e)),
                    }
                }
                Err(e) => return Err(OpenError::Io(e)),
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(OpenError::Io(e)),
        }
        let listener = UnixListener::bind(&name.socket).map_err(|e| {
            if e.kind() == io::ErrorKind::AddrInUse {
                OpenError::Taken
            } else {
                OpenError::Io(e)
            }
        })?;
        Ok(Listener {
            listener,
            socket: name.socket.clone(),
        })
    }

    pub fn accept(&mut self) -> io::Result<Stream> {
        self.listener.accept().map(|(s, _)| Stream(s))
    }
}

impl Drop for Listener {
    /// La socket est retirée tant qu'elle est encore liée : aucun autre noyau ne peut l'avoir prise.
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket);
    }
}

pub fn connect(name: &Name) -> io::Result<Stream> {
    // Le client vérifie lui aussi le dossier : il ne parle qu'à une socket de l'utilisateur.
    check_private(&name.dir, current_uid()?)?;
    UnixStream::connect(&name.socket).map(Stream)
}

pub struct Stream(UnixStream);

impl Stream {
    pub fn try_clone(&self) -> io::Result<Stream> {
        self.0.try_clone().map(Stream)
    }

    pub fn shutdown(&self) {
        let _ = self.0.shutdown(Shutdown::Both);
    }

    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }

    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_name(label: &str) -> Name {
        static N: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "cassini-ext-{label}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let dir = root.join("canal");
        Name {
            socket: dir.join("external.sock"),
            dir,
        }
    }

    fn cleanup(name: &Name) {
        let _ = fs::remove_dir_all(name.dir.parent().unwrap());
    }

    #[test]
    fn the_folder_is_created_private_to_the_user() {
        let name = temp_name("cree");
        let listener = Listener::open(&name).unwrap();
        let meta = fs::symlink_metadata(&name.dir).unwrap();
        assert!(meta.is_dir());
        assert_eq!(meta.mode() & 0o777, 0o700);
        assert_eq!(meta.uid(), current_uid().unwrap());
        assert!(fs::symlink_metadata(&name.socket)
            .unwrap()
            .file_type()
            .is_socket());
        drop(listener);
        // Fermé, le canal n'existe plus.
        assert!(!name.socket.exists());
        cleanup(&name);
    }

    #[test]
    fn a_folder_open_to_others_is_refused_and_left_alone() {
        let name = temp_name("ouvert");
        DirBuilder::new().mode(0o755).create(&name.dir).unwrap();
        fs::set_permissions(&name.dir, fs::Permissions::from_mode(0o755)).unwrap();
        let Err(OpenError::Io(e)) = Listener::open(&name) else {
            panic!("un dossier en 0755 devait être refusé");
        };
        assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(
            fs::symlink_metadata(&name.dir).unwrap().mode() & 0o777,
            0o755
        );
        assert!(!name.socket.exists());
        // Le client refuse lui aussi de s'y brancher.
        assert_eq!(
            connect(&name).err().unwrap().kind(),
            io::ErrorKind::PermissionDenied
        );
        cleanup(&name);
    }

    #[test]
    fn a_symlinked_folder_is_refused() {
        let name = temp_name("lien");
        let target = name.dir.with_file_name("cible");
        DirBuilder::new().mode(0o700).create(&target).unwrap();
        std::os::unix::fs::symlink(&target, &name.dir).unwrap();
        assert!(matches!(Listener::open(&name), Err(OpenError::Io(_))));
        cleanup(&name);
    }

    #[test]
    fn an_orphan_socket_is_removed_and_the_channel_opens() {
        let name = temp_name("orpheline");
        private_dir(&name.dir).unwrap();
        // Un noyau mort laisse sa socket : liée puis abandonnée, personne n'écoute.
        drop(UnixListener::bind(&name.socket).unwrap());
        assert!(name.socket.exists());
        let mut listener = Listener::open(&name).unwrap();
        let client = std::thread::spawn({
            let name = name.clone();
            move || connect(&name).map(drop)
        });
        listener.accept().unwrap();
        client.join().unwrap().unwrap();
        drop(listener);
        cleanup(&name);
    }

    #[test]
    fn a_second_session_finds_the_channel_taken() {
        let name = temp_name("pris");
        let first = Listener::open(&name).unwrap();
        assert!(matches!(Listener::open(&name), Err(OpenError::Taken)));
        drop(first);
        // Libéré, il se rouvre.
        drop(Listener::open(&name).unwrap());
        cleanup(&name);
    }

    #[test]
    fn a_plain_file_in_place_of_the_socket_is_not_removed() {
        let name = temp_name("fichier");
        private_dir(&name.dir).unwrap();
        fs::write(&name.socket, b"x").unwrap();
        assert!(matches!(Listener::open(&name), Err(OpenError::Io(_))));
        assert_eq!(fs::read(&name.socket).unwrap(), b"x");
        cleanup(&name);
    }

    #[test]
    fn shutdown_wakes_a_blocked_reader() {
        let name = temp_name("arret");
        let mut listener = Listener::open(&name).unwrap();
        let _client = connect(&name).unwrap();
        let server = listener.accept().unwrap();
        let mut reader = server.try_clone().unwrap();
        let t = std::thread::spawn(move || {
            let mut buf = [0u8; 8];
            reader.read(&mut buf)
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
        server.shutdown();
        assert_eq!(t.join().unwrap().unwrap(), 0);
        drop(listener);
        cleanup(&name);
    }
}
