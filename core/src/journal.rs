//! Journal en écriture : une session par projet, ajout seul, verrou exclusif
//! (AD-9, AD-10, AD-11).
//!
//! Une `Session` s'ouvre sur un fichier de projet. Elle crée le dossier de travail
//! `<projet sans extension>.cassini/` à côté du projet, prend un verrou exclusif
//! sur `journal.lock`, puis exécute chaque commande par le `Router` et ajoute une
//! ligne JSON à `journal.jsonl`. Sans le verrou, la session est en lecture seule :
//! rien n'est exécuté ni écrit.
//!
//! Le verrou est un fichier distinct du journal, pour que le journal reste lisible
//! par une session en lecture seule, y compris sous Windows, où un verrou pris sur
//! un fichier en interdit la lecture aux autres.

use std::fmt;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde_json::{json, Map, Value};

use crate::error::ErrorReport;
use crate::exec::{Executor, Router};
use crate::registry::{Registry, PROTOCOL_VERSION};

/// Nom du journal, dans le dossier de travail.
const JOURNAL_FILE: &str = "journal.jsonl";
/// Nom du verrou, dans le dossier de travail.
const LOCK_FILE: &str = "journal.lock";
/// Au-delà de cette taille (en octets), un résultat est résumé.
const SUMMARY_LIMIT: usize = 200;

/// Mode d'une session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// La session tient le verrou : elle exécute et journalise.
    Write,
    /// Une autre session tient le verrou : rien n'est exécuté ni écrit.
    ReadOnly,
}

/// Erreur propre au journal, avec un code stable `journal.cause`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalError {
    /// La session est en lecture seule.
    ReadOnly,
    /// Un fichier ne peut pas être désigné par un chemin relatif au projet.
    PathUnrepresentable,
    /// Le dossier de travail ou le journal n'est pas accessible. Le détail ne
    /// porte jamais de chemin (AD-9).
    Io(String),
}

impl JournalError {
    /// Code stable, destiné aux machines.
    pub fn code(&self) -> &'static str {
        match self {
            JournalError::ReadOnly => "journal.read_only",
            JournalError::PathUnrepresentable => "journal.path_unrepresentable",
            JournalError::Io(_) => "journal.io",
        }
    }
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JournalError::ReadOnly => write!(
                f,
                "Le journal de ce projet est tenu par une autre session : cette session est en lecture seule, rien n'est exécuté ni écrit."
            ),
            JournalError::PathUnrepresentable => write!(
                f,
                "Un fichier est hors du projet et ne peut pas être désigné par un chemin relatif au projet (autre disque ?)."
            ),
            JournalError::Io(why) => write!(
                f,
                "Le dossier de travail ou le journal n'est pas accessible (détail technique : {why})."
            ),
        }
    }
}

impl std::error::Error for JournalError {}

fn io_error(e: std::io::Error) -> JournalError {
    JournalError::Io(e.kind().to_string())
}

/// Une session de journalisation sur un projet.
pub struct Session<'a> {
    registry: &'a Registry,
    router: Router<'a>,
    project_dir: PathBuf,
    work_dir: PathBuf,
    mode: Mode,
    /// Tient le verrou tant que la session vit ; fermé, il est libéré.
    _lock: File,
    next_step: usize,
    registry_version: String,
}

impl<'a> Session<'a> {
    /// Ouvre une session sur le fichier de projet `project` (qui peut ne pas
    /// exister encore, son dossier doit exister). Crée le dossier de travail,
    /// prend le verrou ; si un autre le tient, la session est en lecture seule.
    pub fn open(registry: &'a Registry, project: &Path) -> Result<Self, JournalError> {
        let absolute = normalize(&std::path::absolute(project).map_err(io_error)?);
        let project_dir = absolute
            .parent()
            .ok_or_else(|| JournalError::Io("InvalidInput".to_string()))?
            .to_path_buf();
        let stem = absolute
            .file_stem()
            .ok_or_else(|| JournalError::Io("InvalidInput".to_string()))?
            .to_string_lossy()
            .into_owned();
        if !project_dir.is_dir() {
            return Err(JournalError::Io("NotFound".to_string()));
        }
        let work_dir = project_dir.join(format!("{stem}.cassini"));
        fs::create_dir_all(&work_dir).map_err(io_error)?;

        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(work_dir.join(LOCK_FILE))
            .map_err(io_error)?;
        let mode = match lock.try_lock() {
            Ok(()) => Mode::Write,
            Err(TryLockError::WouldBlock) => Mode::ReadOnly,
            Err(TryLockError::Error(e)) => return Err(io_error(e)),
        };
        let next_step = match mode {
            Mode::Write => count_lines(&work_dir.join(JOURNAL_FILE))? + 1,
            Mode::ReadOnly => 1,
        };
        Ok(Session {
            registry,
            router: Router::new(registry),
            project_dir,
            work_dir,
            mode,
            _lock: lock,
            next_step,
            registry_version: registry_version(registry),
        })
    }

    /// Branche un exécutant QGIS à la place du bouchon.
    pub fn with_qgis(self, qgis: impl Executor + 'a) -> Self {
        Session {
            router: self.router.with_qgis(qgis),
            ..self
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Numéro de la prochaine étape qui sera journalisée (à partir de 1).
    pub fn next_step(&self) -> usize {
        self.next_step
    }

    /// Le dossier de travail (AD-10).
    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    /// Le fichier du journal.
    pub fn journal_path(&self) -> PathBuf {
        self.work_dir.join(JOURNAL_FILE)
    }

    /// Exécute une commande par le routeur et journalise l'étape.
    ///
    /// Une commande refusée avant exécution (id inconnu, paramètres invalides,
    /// chemin non représentable) n'écrit rien. Une session en lecture seule
    /// répond `journal.read_only` sans rien exécuter.
    pub fn call(&mut self, id: &str, params: Value) -> Result<Value, ErrorReport> {
        if self.mode == Mode::ReadOnly {
            return Err(JournalError::ReadOnly.into());
        }
        let spec = self
            .registry
            .spec(id)
            .ok_or_else(|| crate::registry::RegistryError::UnknownCommand(id.to_string()))?;
        let step = self.next_step;

        // Le journal garde les valeurs relatives ; l'exécutant reçoit des chemins
        // absolus, décidés par le noyau (AD-9, AD-10).
        let mut logged = params.clone();
        let mut exec = params;
        for name in &spec.output_params {
            let file = if spec.output_params.len() == 1 {
                format!("{step}-{id}.out")
            } else {
                format!("{step}-{id}-{name}.out")
            };
            if let (Some(l), Some(e)) = (logged.as_object_mut(), exec.as_object_mut()) {
                l.insert(name.clone(), Value::String(file.clone()));
                e.insert(
                    name.clone(),
                    Value::String(self.work_dir.join(&file).to_string_lossy().into_owned()),
                );
            }
        }
        self.registry.validate_params(id, &logged)?;

        let mut inputs = Vec::new();
        for name in &spec.path_params {
            if let (Some(l), Some(e)) = (logged.get_mut(name), exec.get_mut(name)) {
                self.resolve_param(l, e, &mut inputs)?;
            }
        }

        let outcome = self.router.call(id, exec);

        let mut line = Map::new();
        line.insert("step".into(), json!(step));
        line.insert("id".into(), json!(id));
        line.insert("params".into(), canonical(&logged));
        line.insert("inputs".into(), Value::Array(inputs));
        match &outcome {
            Ok(value) => line.insert("result".into(), summarize(value)),
            Err(e) => line.insert("error".into(), json!(e.code())),
        };
        line.insert(
            "versions".into(),
            json!({
                "core": crate::VERSION,
                "protocol": PROTOCOL_VERSION,
                "registry": self.registry_version,
            }),
        );
        self.append(&Value::Object(line))?;
        self.next_step += 1;
        outcome.map_err(ErrorReport::from)
    }

    /// Remplace, dans la copie journalisée, un chemin par sa forme relative au
    /// projet, et dans la copie exécutée par sa forme absolue. Une chaîne ou un
    /// tableau de chaînes ; toute autre valeur est laissée à la validation.
    fn resolve_param(
        &self,
        logged: &mut Value,
        exec: &mut Value,
        inputs: &mut Vec<Value>,
    ) -> Result<(), JournalError> {
        match (logged, exec) {
            (Value::String(l), Value::String(e)) => {
                let (relative, absolute, portable) = self.resolve_path(l)?;
                inputs.push(input_entry(&relative, &absolute, portable));
                *l = relative;
                *e = absolute.to_string_lossy().into_owned();
            }
            (Value::Array(ls), Value::Array(es)) => {
                for (l, e) in ls.iter_mut().zip(es.iter_mut()) {
                    self.resolve_param(l, e, inputs)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Forme relative POSIX, forme absolue et portabilité d'un chemin d'entrée.
    fn resolve_path(&self, raw: &str) -> Result<(String, PathBuf, bool), JournalError> {
        let given = Path::new(raw);
        let absolute = if given.is_absolute() {
            normalize(given)
        } else {
            normalize(&self.project_dir.join(given))
        };
        let relative =
            relative_to(&self.project_dir, &absolute).ok_or(JournalError::PathUnrepresentable)?;
        let portable = !relative
            .components()
            .any(|c| matches!(c, Component::ParentDir));
        Ok((posix(&relative), absolute, portable))
    }

    /// Ajoute une ligne au journal, en un seul `write`, et la rend durable.
    fn append(&self, line: &Value) -> Result<(), JournalError> {
        let path = self.journal_path();
        let mut text = String::new();
        // Une fin de ligne manquante (arrêt brutal) ne doit pas coller deux étapes.
        if let Ok(existing) = fs::read(&path) {
            if !existing.is_empty() && !existing.ends_with(b"\n") {
                text.push('\n');
            }
        }
        text.push_str(&line.to_string());
        text.push('\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(io_error)?;
        file.write_all(text.as_bytes()).map_err(io_error)?;
        file.sync_data().map_err(io_error)?;
        Ok(())
    }
}

fn count_lines(path: &Path) -> Result<usize, JournalError> {
    match fs::read(path) {
        Ok(bytes) => Ok(bytes
            .split(|b| *b == b'\n')
            .filter(|l| !l.iter().all(u8::is_ascii_whitespace))
            .count()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(io_error(e)),
    }
}

fn input_entry(relative: &str, absolute: &Path, portable: bool) -> Value {
    let meta = fs::metadata(absolute).ok();
    let size = meta.as_ref().map(|m| json!(m.len())).unwrap_or(Value::Null);
    let modified = meta
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| json!(d.as_millis() as u64))
        .unwrap_or(Value::Null);
    json!({"path": relative, "portable": portable, "size": size, "modified": modified})
}

/// Résultat gardé s'il tient en 200 octets, sinon son résumé.
fn summarize(value: &Value) -> Value {
    let text = value.to_string();
    if text.len() <= SUMMARY_LIMIT {
        canonical(value)
    } else {
        json!({"truncated": true, "bytes": text.len()})
    }
}

/// Copie dont les clés d'objet sont triées, à tous les niveaux.
pub fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let mut out = Map::new();
            for (k, v) in entries {
                out.insert(k.clone(), canonical(v));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

/// Empreinte (FNV-1a 64 bits, sans dépendance) des ids et schémas du registre,
/// dans l'ordre des ids : elle change dès qu'une commande ou un schéma change.
pub fn registry_version(registry: &Registry) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    let mut feed = |bytes: &[u8]| {
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x100000001b3);
    };
    for (spec, _) in registry.implementations() {
        feed(spec.id.as_bytes());
        feed(canonical(&spec.params_schema).to_string().as_bytes());
        feed(canonical(&spec.output_schema).to_string().as_bytes());
    }
    format!("{hash:016x}")
}

/// Normalisation lexicale : retire `.` et résout `..` sans toucher au disque.
fn normalize(path: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => match out.last() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                _ => out.push(c),
            },
            other => out.push(other),
        }
    }
    out.iter().collect()
}

/// Chemin de `target` relatif à `base`, tous deux absolus et normalisés ;
/// `None` si leurs racines diffèrent (autre disque).
fn relative_to(base: &Path, target: &Path) -> Option<PathBuf> {
    let b: Vec<Component> = base.components().collect();
    let t: Vec<Component> = target.components().collect();
    let root_len = b
        .iter()
        .take_while(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
        .count();
    let common = b.iter().zip(t.iter()).take_while(|(x, y)| x == y).count();
    if common < root_len {
        return None;
    }
    let mut out = PathBuf::new();
    for _ in common..b.len() {
        out.push("..");
    }
    for c in &t[common..] {
        out.push(c.as_os_str());
    }
    Some(out)
}

/// Écriture POSIX d'un chemin relatif ; `.` pour le dossier du projet lui-même.
fn posix(relative: &Path) -> String {
    let parts: Vec<String> = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::registry::RegistryError;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Dossier de projet neuf et unique ; rend (dossier, fichier de projet, jeton unique).
    fn temp_project() -> (PathBuf, PathBuf, String) {
        let token = format!(
            "cassini-journal-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let dir = std::env::temp_dir().join(&token);
        fs::create_dir_all(&dir).unwrap();
        let project = dir.join("p.qgz");
        (dir, project, token)
    }

    fn lines(session: &Session) -> Vec<Value> {
        fs::read_to_string(session.journal_path())
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    struct FakeQgis;
    impl Executor for FakeQgis {
        fn execute(&self, _id: &str, _p: Value) -> Result<Value, RegistryError> {
            Ok(json!({"ok": true}))
        }
    }

    #[test]
    fn three_commands_give_three_lines_without_absolute_paths() {
        let (dir, project, token) = temp_project();
        fs::write(dir.join("data.txt"), "abc").unwrap();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project).unwrap();
        assert_eq!(s.mode(), Mode::Write);
        s.call("system.about", json!({})).unwrap();
        // Chemin donné en absolu : journalisé en relatif.
        let absolute = dir.join("data.txt").to_string_lossy().into_owned();
        let out = s.call("test.read_file", json!({"path": absolute})).unwrap();
        assert_eq!(out, json!({"bytes": 3}));
        s.call("test.write_output", json!({"text": "salut"}))
            .unwrap();

        let raw = fs::read_to_string(s.journal_path()).unwrap();
        assert_eq!(raw.lines().count(), 3);
        assert!(
            !raw.contains(&token),
            "chemin absolu dans le journal : {raw}"
        );
        let l = lines(&s);
        assert_eq!(l[0]["step"], 1);
        assert_eq!(l[1]["params"], json!({"path": "data.txt"}));
        assert_eq!(l[1]["inputs"][0]["path"], "data.txt");
        assert_eq!(l[1]["inputs"][0]["portable"], true);
        assert_eq!(l[1]["inputs"][0]["size"], 3);
        assert!(l[1]["inputs"][0]["modified"].is_u64());
        assert_eq!(l[2]["params"]["out"], "3-test.write_output.out");
        assert_eq!(l[2]["result"], json!({"bytes": 5}));
        assert_eq!(l[2]["versions"]["core"], crate::VERSION);
        assert_eq!(l[2]["versions"]["protocol"], PROTOCOL_VERSION);
        // La sortie est dans le dossier de travail, et nulle part ailleurs.
        assert_eq!(
            fs::read_to_string(s.work_dir().join("3-test.write_output.out")).unwrap(),
            "salut"
        );
        drop(s);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_second_session_is_read_only_and_writes_nothing() {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        let mut first = Session::open(&registry, &project).unwrap();
        first.call("system.about", json!({})).unwrap();
        let before = fs::read_to_string(first.journal_path()).unwrap();

        let mut second = Session::open(&registry, &project).unwrap();
        assert_eq!(second.mode(), Mode::ReadOnly);
        let err = second
            .call("test.write_output", json!({"text": "x"}))
            .unwrap_err();
        assert_eq!(err.code, "journal.read_only");
        assert_eq!(fs::read_to_string(first.journal_path()).unwrap(), before);
        assert!(!first.work_dir().join("2-test.write_output.out").exists());
        drop((first, second));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn closing_a_session_frees_the_lock_and_numbering_continues() {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        {
            let mut a = Session::open(&registry, &project).unwrap();
            a.call("system.about", json!({})).unwrap();
            a.call("system.about", json!({})).unwrap();
        }
        let mut b = Session::open(&registry, &project).unwrap();
        assert_eq!(b.mode(), Mode::Write);
        assert_eq!(b.next_step(), 3);
        b.call("system.about", json!({})).unwrap();
        assert_eq!(lines(&b)[2]["step"], 3);
        drop(b);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_file_outside_the_project_is_not_portable() {
        let (dir, project, token) = temp_project();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project).unwrap();
        s.call("test.read_file", json!({"path": "../x.tif"}))
            .unwrap();
        let l = lines(&s);
        assert_eq!(l[0]["inputs"][0]["path"], "../x.tif");
        assert_eq!(l[0]["inputs"][0]["portable"], false);
        assert_eq!(l[0]["inputs"][0]["size"], Value::Null);
        // Un absolu hors projet sur le même disque : relatif, non portable.
        let outside = std::env::temp_dir().join("other-folder").join("y.tif");
        s.call("test.read_file", json!({"path": outside.to_string_lossy()}))
            .unwrap();
        let second = &lines(&s)[1];
        assert_eq!(second["inputs"][0]["portable"], false);
        assert_eq!(second["inputs"][0]["path"], "../other-folder/y.tif");
        let raw = fs::read_to_string(s.journal_path()).unwrap();
        assert!(!raw.contains(&token));
        drop(s);
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(windows)]
    #[test]
    fn another_drive_is_unrepresentable() {
        assert_eq!(relative_to(Path::new(r"C:\a\b"), Path::new(r"D:\x")), None);
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project).unwrap();
        let here = dir
            .to_string_lossy()
            .chars()
            .next()
            .unwrap()
            .to_ascii_uppercase();
        let other = if here == 'Z' { "Y" } else { "Z" };
        let err = s
            .call(
                "test.read_file",
                json!({"path": format!(r"{other}:\x.tif")}),
            )
            .unwrap_err();
        assert_eq!(err.code, "journal.path_unrepresentable");
        assert!(lines(&s).is_empty());
        drop(s);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_refused_command_writes_nothing() {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project).unwrap();
        let err = s
            .call("test.read_file", json!({"path": "a", "extra": 1}))
            .unwrap_err();
        assert_eq!(err.code, "registry.invalid_params");
        let err = s.call("no.such", json!({})).unwrap_err();
        assert_eq!(err.code, "registry.unknown_command");
        assert!(!s.journal_path().exists());
        assert_eq!(s.next_step(), 1);
        drop(s);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_failed_execution_is_journaled_with_its_code() {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project).unwrap();
        let err = s
            .call("test.qgis_probe", json!({"layer": "l"}))
            .unwrap_err();
        assert_eq!(err.code, "registry.unavailable");
        let l = lines(&s);
        assert_eq!(l[0]["error"], "registry.unavailable");
        assert!(l[0].get("result").is_none());
        drop(s);

        let mut s = Session::open(&registry, &project)
            .unwrap()
            .with_qgis(FakeQgis);
        assert_eq!(
            s.call("test.qgis_probe", json!({"layer": "l"})).unwrap(),
            json!({"ok": true})
        );
        assert_eq!(lines(&s)[1]["result"], json!({"ok": true}));
        drop(s);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_big_result_is_summarized() {
        let big = json!({"text": "x".repeat(500)});
        let summary = summarize(&big);
        assert_eq!(summary["truncated"], true);
        assert_eq!(summary["bytes"], big.to_string().len());
        assert_eq!(
            summarize(&json!({"b": 1, "a": 2})).to_string(),
            r#"{"a":2,"b":1}"#
        );
    }

    #[test]
    fn params_are_canonical_and_the_registry_version_is_stable() {
        let v = canonical(&json!({"b": {"d": 1, "c": 2}, "a": [{"z": 1, "y": 2}]}));
        assert_eq!(v.to_string(), r#"{"a":[{"y":2,"z":1}],"b":{"c":2,"d":1}}"#);
        let registry = Registry::standard();
        let a = registry_version(&registry);
        assert_eq!(a, registry_version(&Registry::standard()));
        assert_eq!(a.len(), 16);
        assert_ne!(a, registry_version(&Registry::new()));
    }

    #[test]
    fn lexical_paths_behave() {
        let base = Path::new("/p/proj");
        let t = normalize(&base.join("a/../b/./c.tif"));
        assert_eq!(posix(&relative_to(base, &t).unwrap()), "b/c.tif");
        assert_eq!(posix(&relative_to(base, base).unwrap()), ".");
    }
}
