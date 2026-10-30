//! Canal d'IA externe sous Windows : tube nommé, par l'API Win32 (`windows-sys`).
//!
//! - `PIPE_REJECT_REMOTE_CLIENTS` : un client qui passe par le réseau (`\\machine\pipe\…`, même
//!   `\\localhost`) est refusé.
//! - `FILE_FLAG_FIRST_PIPE_INSTANCE` sur la première instance : si le nom existe, une autre session
//!   le tient (`external.channel_taken`). Les instances suivantes, ouvertes par le même noyau,
//!   servent d'autres relais.
//! - Descripteur `O:<SID>D:P(A;;GA;;;<SID>)` : seul l'utilisateur courant, propriétaire, y a accès.
//!   Le client vérifie ce propriétaire avant de parler, et se branche en `SECURITY_SQOS_PRESENT`
//!   au niveau identification : la session ne peut pas agir en son nom.
//!
//! Les entrées et sorties sont chevauchées : un `ReadFile` synchrone bloquerait le `WriteFile` du
//! même handle, alors qu'un fil lit et un autre écrit. Chaque opération attend aussi l'événement
//! d'arrêt de la connexion, que `shutdown` lève : aucun fil ne reste bloqué sur un relais.

use std::io;
use std::ptr::{null, null_mut};
use std::sync::Arc;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, ERROR_ACCESS_DENIED, ERROR_BROKEN_PIPE, ERROR_IO_PENDING,
    ERROR_NO_DATA, ERROR_OPERATION_ABORTED, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED,
    ERROR_PIPE_NOT_CONNECTED, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo,
    SDDL_REVISION_1, SE_KERNEL_OBJECT,
};
use windows_sys::Win32::Security::{
    EqualSid, GetTokenInformation, TokenUser, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    PSID, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED,
    OPEN_EXISTING, PIPE_ACCESS_DUPLEX, SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, WaitNamedPipeW, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, GetCurrentProcess, OpenProcessToken, SetEvent, WaitForMultipleObjects,
    WaitForSingleObject, INFINITE,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

use super::OpenError;

const BUFFER: u32 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// Nom du tube, terminé par un zéro, pour l'API Win32.
    wide: Vec<u16>,
}

impl Name {
    pub fn pipe_name(&self) -> String {
        String::from_utf16_lossy(&self.wide[..self.wide.len() - 1])
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn name(tag: Option<&str>) -> io::Result<Name> {
    let user: String = std::env::var("USERNAME")
        .unwrap_or_default()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    // Sans nom d'utilisateur, le SID le remplace : le nom reste propre à l'utilisateur.
    let user = if user.is_empty() {
        TokenUserSid::current()?.string()?
    } else {
        user
    };
    let pipe = match tag {
        Some(tag) => format!(r"\\.\pipe\cassini-{user}-{tag}-external"),
        None => format!(r"\\.\pipe\cassini-{user}-external"),
    };
    Ok(Name { wide: wide(&pipe) })
}

/// Un handle du noyau, fermé à la fin.
struct Handle(HANDLE);

// SAFETY : un handle du noyau Windows peut servir depuis n'importe quel fil.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY : le handle est valide et n'est fermé qu'ici.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Un événement à réarmement manuel, non signalé au départ.
fn event() -> io::Result<Handle> {
    // SAFETY : aucun attribut ni nom ; le handle rendu est vérifié.
    let h = unsafe { CreateEventW(null(), 1, 0, null()) };
    if h.is_null() {
        return Err(io::Error::last_os_error());
    }
    Ok(Handle(h))
}

/// Le SID de l'utilisateur du jeton du processus.
struct TokenUserSid {
    /// Tampon de `TOKEN_USER`, aligné comme un pointeur.
    buf: Vec<u64>,
}

impl TokenUserSid {
    fn current() -> io::Result<TokenUserSid> {
        let mut token: HANDLE = null_mut();
        // SAFETY : pseudo-handle du processus, jeton ouvert en lecture et fermé par `Handle`.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let token = Handle(token);
        let mut len = 0u32;
        // SAFETY : premier appel pour connaître la taille, qui échoue par construction.
        unsafe { GetTokenInformation(token.0, TokenUser, null_mut(), 0, &mut len) };
        if len == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut buf = vec![0u64; (len as usize).div_ceil(8)];
        // SAFETY : le tampon fait au moins `len` octets.
        if unsafe {
            GetTokenInformation(token.0, TokenUser, buf.as_mut_ptr().cast(), len, &mut len)
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(TokenUserSid { buf })
    }

    fn sid(&self) -> PSID {
        // SAFETY : le tampon porte un `TOKEN_USER` rempli par le système, aligné.
        unsafe { (*(self.buf.as_ptr() as *const TOKEN_USER)).User.Sid }
    }

    fn string(&self) -> io::Result<String> {
        let mut text: *mut u16 = null_mut();
        // SAFETY : SID valide ; la chaîne rendue est libérée par `LocalFree`.
        if unsafe { ConvertSidToStringSidW(self.sid(), &mut text) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut len = 0;
        // SAFETY : chaîne terminée par un zéro.
        while unsafe { *text.add(len) } != 0 {
            len += 1;
        }
        // SAFETY : `len` caractères lisibles.
        let s = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, len) });
        // SAFETY : alloué par le système, libéré une fois.
        unsafe { LocalFree(text.cast()) };
        Ok(s)
    }
}

/// Le descripteur de sécurité du tube : l'utilisateur courant seul, et propriétaire.
struct Descriptor(PSECURITY_DESCRIPTOR);

// SAFETY : le descripteur n'est lu que par `CreateNamedPipeW`, jamais modifié.
unsafe impl Send for Descriptor {}

impl Descriptor {
    fn for_current_user() -> io::Result<Descriptor> {
        let sid = TokenUserSid::current()?.string()?;
        let sddl = wide(&format!("O:{sid}D:P(A;;GA;;;{sid})"));
        let mut sd: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY : chaîne terminée par un zéro ; le descripteur est libéré par `Drop`.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Descriptor(sd))
    }
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY : alloué par le système, libéré une fois.
        unsafe { LocalFree(self.0) };
    }
}

fn create_instance(name: &Name, sd: &Descriptor, first: bool) -> io::Result<Handle> {
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd.0,
        bInheritHandle: 0,
    };
    let mut open_mode = PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED;
    if first {
        open_mode |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    // SAFETY : nom terminé par un zéro, attributs valides le temps de l'appel.
    let h = unsafe {
        CreateNamedPipeW(
            name.wide.as_ptr(),
            open_mode,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER,
            BUFFER,
            0,
            &attributes,
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    Ok(Handle(h))
}

pub struct Listener {
    name: Name,
    sd: Descriptor,
    /// L'instance qui attend le prochain client.
    pending: Option<Handle>,
}

impl Listener {
    pub fn open(name: &Name) -> Result<Listener, OpenError> {
        let sd = Descriptor::for_current_user()?;
        let first = create_instance(name, &sd, true).map_err(|e| {
            match e.raw_os_error().map(|c| c as u32) {
                // Le nom existe déjà : une autre session le tient.
                Some(ERROR_ACCESS_DENIED) | Some(ERROR_PIPE_BUSY) => OpenError::Taken,
                _ => OpenError::Io(e),
            }
        })?;
        Ok(Listener {
            name: name.clone(),
            sd,
            pending: Some(first),
        })
    }

    pub fn accept(&mut self) -> io::Result<Stream> {
        let handle = match self.pending.take() {
            Some(h) => h,
            None => create_instance(&self.name, &self.sd, false)?,
        };
        if let Err(e) = connect_instance(&handle) {
            // L'instance est remise à neuf pour le prochain client.
            // SAFETY : handle valide d'une instance serveur.
            unsafe { DisconnectNamedPipe(handle.0) };
            self.pending = Some(handle);
            return Err(e);
        }
        // Une nouvelle instance attend déjà le client suivant ; sinon elle sera créée au prochain
        // `accept`.
        self.pending = create_instance(&self.name, &self.sd, false).ok();
        Stream::new(handle)
    }
}

/// Attend qu'un client se branche sur l'instance.
fn connect_instance(handle: &Handle) -> io::Result<()> {
    let ev = event()?;
    let mut ov = OVERLAPPED {
        hEvent: ev.0,
        ..Default::default()
    };
    // SAFETY : `ov` et son événement vivent jusqu'à la fin de l'opération, attendue ci-dessous.
    if unsafe { ConnectNamedPipe(handle.0, &mut ov) } != 0 {
        return Ok(());
    }
    // SAFETY : lecture de l'erreur du dernier appel.
    match unsafe { GetLastError() } {
        ERROR_PIPE_CONNECTED => Ok(()),
        ERROR_IO_PENDING => {
            let mut n = 0u32;
            // SAFETY : attente bloquante de l'opération lancée sur `ov`.
            if unsafe { GetOverlappedResult(handle.0, &ov, &mut n, 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
        code => Err(io::Error::from_raw_os_error(code as i32)),
    }
}

pub fn connect(name: &Name) -> io::Result<Stream> {
    let user = TokenUserSid::current()?;
    let mut attempts = 0;
    let handle = loop {
        // SAFETY : nom terminé par un zéro ; le handle rendu est vérifié.
        let h = unsafe {
            CreateFileW(
                name.wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                null_mut(),
            )
        };
        if h != INVALID_HANDLE_VALUE {
            break Handle(h);
        }
        let e = io::Error::last_os_error();
        attempts += 1;
        if e.raw_os_error() != Some(ERROR_PIPE_BUSY as i32) || attempts > 10 {
            return Err(e);
        }
        // Toutes les instances sont prises un court instant : on attend la suivante.
        // SAFETY : nom terminé par un zéro.
        unsafe { WaitNamedPipeW(name.wide.as_ptr(), 500) };
    };
    check_owner(&handle, &user)?;
    Stream::new(handle)
}

/// Refuse un tube qui n'appartient pas à l'utilisateur courant (nom pris par un autre compte).
fn check_owner(handle: &Handle, user: &TokenUserSid) -> io::Result<()> {
    let mut owner: PSID = null_mut();
    let mut sd: PSECURITY_DESCRIPTOR = null_mut();
    // SAFETY : handle ouvert avec READ_CONTROL (GENERIC_READ) ; `sd` libéré ci-dessous.
    let status = unsafe {
        GetSecurityInfo(
            handle.0,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            null_mut(),
            null_mut(),
            &mut sd,
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    // SAFETY : deux SID valides ; `owner` pointe dans `sd`, libéré après la comparaison.
    let same = unsafe { EqualSid(owner, user.sid()) } != 0;
    unsafe { LocalFree(sd) };
    if !same {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "le canal d'IA externe n'appartient pas à l'utilisateur courant",
        ));
    }
    Ok(())
}

struct Pipe {
    handle: Handle,
    /// Levé par `shutdown` : toute opération en cours ou à venir rend la main.
    stop: Handle,
}

pub struct Stream(Arc<Pipe>);

impl Stream {
    fn new(handle: Handle) -> io::Result<Stream> {
        Ok(Stream(Arc::new(Pipe {
            handle,
            stop: event()?,
        })))
    }

    pub fn try_clone(&self) -> io::Result<Stream> {
        Ok(Stream(self.0.clone()))
    }

    pub fn shutdown(&self) {
        // SAFETY : événement valide.
        unsafe { SetEvent(self.0.stop.0) };
    }

    fn stopped(&self) -> bool {
        // SAFETY : événement valide, attente nulle.
        unsafe { WaitForSingleObject(self.0.stop.0, 0) == WAIT_OBJECT_0 }
    }

    /// Une opération chevauchée, attendue jusqu'à sa fin ou jusqu'à l'arrêt.
    fn overlapped(&self, op: impl FnOnce(HANDLE, *mut OVERLAPPED) -> i32) -> io::Result<u32> {
        if self.stopped() {
            return Err(io::Error::from_raw_os_error(ERROR_OPERATION_ABORTED as i32));
        }
        let handle = self.0.handle.0;
        let ev = event()?;
        let mut ov = OVERLAPPED {
            hEvent: ev.0,
            ..Default::default()
        };
        if op(handle, &mut ov) == 0 {
            // SAFETY : lecture de l'erreur du dernier appel.
            let code = unsafe { GetLastError() };
            if code != ERROR_IO_PENDING {
                return Err(io::Error::from_raw_os_error(code as i32));
            }
            let waits = [ev.0, self.0.stop.0];
            // SAFETY : deux handles valides.
            let woke = unsafe { WaitForMultipleObjects(2, waits.as_ptr(), 0, INFINITE) };
            if woke != WAIT_OBJECT_0 {
                // Arrêt : l'opération est annulée, puis attendue (`ov` doit lui survivre).
                // SAFETY : annule la seule opération de `ov` sur ce handle.
                unsafe { CancelIoEx(handle, &ov) };
            }
        }
        let mut n = 0u32;
        // SAFETY : attend la fin de l'opération de `ov`, qui vit jusqu'ici.
        if unsafe { GetOverlappedResult(handle, &ov, &mut n, 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(n)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = buf.len().min(u32::MAX as usize) as u32;
        let ptr = buf.as_mut_ptr();
        // SAFETY : le tampon vit jusqu'à la fin de l'opération, attendue par `overlapped`.
        match self.overlapped(|h, ov| unsafe { ReadFile(h, ptr, len, null_mut(), ov) }) {
            Ok(n) => Ok(n as usize),
            // Autre bout fermé, ou arrêt : fin de flux.
            Err(e)
                if matches!(
                    e.raw_os_error().map(|c| c as u32),
                    Some(ERROR_BROKEN_PIPE)
                        | Some(ERROR_PIPE_NOT_CONNECTED)
                        | Some(ERROR_OPERATION_ABORTED)
                        | Some(ERROR_NO_DATA)
                ) =>
            {
                Ok(0)
            }
            Err(e) => Err(e),
        }
    }

    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len().min(u32::MAX as usize) as u32;
        let ptr = buf.as_ptr();
        // SAFETY : le tampon vit jusqu'à la fin de l'opération, attendue par `overlapped`.
        self.overlapped(|h, ov| unsafe { WriteFile(h, ptr, len, null_mut(), ov) })
            .map(|n| n as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique(label: &str) -> Name {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        name(Some(&format!(
            "{label}-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        )))
        .unwrap()
    }

    #[test]
    fn a_second_session_finds_the_pipe_taken() {
        let n = unique("pris");
        let first = Listener::open(&n).unwrap();
        assert!(matches!(Listener::open(&n), Err(OpenError::Taken)));
        drop(first);
        drop(Listener::open(&n).unwrap());
    }

    #[test]
    fn without_a_session_there_is_no_pipe() {
        let n = unique("absent");
        assert!(connect(&n).is_err());
    }

    #[test]
    fn shutdown_wakes_a_blocked_reader() {
        let n = unique("arret");
        let mut listener = Listener::open(&n).unwrap();
        let client = std::thread::spawn({
            let n = n.clone();
            move || connect(&n)
        });
        let server = listener.accept().unwrap();
        let _client = client.join().unwrap().unwrap();
        let mut reader = server.try_clone().unwrap();
        let t = std::thread::spawn(move || {
            let mut buf = [0u8; 8];
            reader.read(&mut buf)
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
        server.shutdown();
        assert_eq!(t.join().unwrap().unwrap(), 0);
    }

    #[test]
    fn the_pipe_belongs_to_the_current_user() {
        let n = unique("proprio");
        let mut listener = Listener::open(&n).unwrap();
        let client = std::thread::spawn({
            let n = n.clone();
            move || connect(&n).map(drop)
        });
        listener.accept().unwrap();
        // `connect` a vérifié le propriétaire du tube.
        client.join().unwrap().unwrap();
    }
}
