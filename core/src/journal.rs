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
use crate::registry::{Registry, RegistryError, PROTOCOL_VERSION};

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
    /// Le projet n'a pas de journal (dossier absent ou journal vide).
    NoJournal,
    /// Un repère `out:<k>.<n>` ne désigne aucune sortie d'une étape précédente.
    BadReference(String),
    /// Le rejeu ne redonne pas ce que le journal a gardé, à l'étape `step`.
    Diverged { step: usize, why: String },
}

impl JournalError {
    /// Code stable, destiné aux machines.
    pub fn code(&self) -> &'static str {
        match self {
            JournalError::ReadOnly => "journal.read_only",
            JournalError::PathUnrepresentable => "journal.path_unrepresentable",
            JournalError::Io(_) => "journal.io",
            JournalError::NoJournal => "replay.no_journal",
            JournalError::BadReference(_) => "replay.bad_reference",
            JournalError::Diverged { .. } => "replay.diverged",
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
            JournalError::NoJournal => write!(f, "Ce projet n'a pas de journal à rejouer."),
            JournalError::BadReference(r) => write!(
                f,
                "Le repère « {r} » ne désigne aucune sortie d'une étape précédente."
            ),
            JournalError::Diverged { step, why } => write!(
                f,
                "Le rejeu diverge du journal à l'étape {step} : {why}."
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
        let (project_dir, work_dir) = locate(project)?;
        if !project_dir.is_dir() {
            return Err(JournalError::Io("NotFound".to_string()));
        }
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
        // absolus, décidés par le noyau (AD-9, AD-10). Même résolution au rejeu.
        let prior = if spec.path_params.is_empty() {
            Vec::new()
        } else {
            journal_ids(&self.journal_path())
        };
        let resolver = Resolver {
            registry: self.registry,
            project_dir: &self.project_dir,
            out_dir: &self.work_dir,
        };
        let Resolved {
            logged,
            exec,
            inputs,
        } = resolver.resolve_call(id, spec, step, params, &prior)?;

        let outcome = self.router.call(id, exec);

        let mut line = Map::new();
        line.insert("step".into(), json!(step));
        line.insert("id".into(), json!(id));
        line.insert("params".into(), canonical(&logged));
        line.insert("inputs".into(), Value::Array(inputs));
        let cancelled = matches!(outcome, Err(RegistryError::Cancelled(_)));
        match &outcome {
            Ok(value) => line.insert("result".into(), summarize(value)),
            Err(RegistryError::Cancelled(_)) => line.insert("cancelled".into(), json!(true)),
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
        if cancelled {
            // Une étape annulée ne laisse pas de sortie partielle derrière elle (AD-12).
            for name in &spec.output_params {
                let file = output_file(step, id, &spec.output_params, name, &spec.output_ext);
                let _ = fs::remove_file(self.work_dir.join(file));
            }
        }
        self.append(&Value::Object(line))?;
        self.next_step += 1;
        outcome.map_err(ErrorReport::from)
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

/// Lignes non vides d'un texte de journal : la seule définition d'une ligne écrite.
fn content_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter(|l| !l.trim().is_empty())
}

fn count_lines(path: &Path) -> Result<usize, JournalError> {
    match fs::read(path) {
        Ok(bytes) => Ok(content_lines(&String::from_utf8_lossy(&bytes)).count()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(io_error(e)),
    }
}

/// Dossier du projet et dossier de travail, sans rien créer.
fn locate(project: &Path) -> Result<(PathBuf, PathBuf), JournalError> {
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
    let work_dir = project_dir.join(format!("{stem}.cassini"));
    Ok((project_dir, work_dir))
}

/// Ids des étapes déjà journalisées, dans l'ordre (vide si illisible).
fn journal_ids(path: &Path) -> Vec<String> {
    let text = fs::read_to_string(path).unwrap_or_default();
    content_lines(&text)
        .map(|l| {
            serde_json::from_str::<Value>(l)
                .ok()
                .and_then(|v| v.get("id").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_default()
        })
        .collect()
}

/// Nom du fichier de sortie `name` de l'étape `step`.
fn output_file(step: usize, id: &str, outputs: &[String], name: &str, ext: &str) -> String {
    if outputs.len() == 1 {
        format!("{step}-{id}{ext}")
    } else {
        format!("{step}-{id}-{name}{ext}")
    }
}

/// Paramètres d'un appel, tels que journalisés et tels que donnés à l'exécutant.
struct Resolved {
    logged: Value,
    exec: Value,
    inputs: Vec<Value>,
}

/// Résolution unique des chemins et des sorties d'un appel, partagée par
/// l'exécution d'origine et le rejeu (AD-11).
struct Resolver<'a> {
    registry: &'a Registry,
    project_dir: &'a Path,
    /// Dossier où l'exécutant écrit ses sorties : le dossier de travail, ou `replay/`.
    out_dir: &'a Path,
}

impl Resolver<'_> {
    /// `prior` : ids des étapes précédentes, l'étape 1 en premier.
    fn resolve_call(
        &self,
        id: &str,
        spec: &crate::registry::CommandSpec,
        step: usize,
        params: Value,
        prior: &[String],
    ) -> Result<Resolved, ErrorReport> {
        let prior = &prior[..prior.len().min(step.saturating_sub(1))];
        let mut logged = params.clone();
        let mut exec = params;
        for name in &spec.output_params {
            let file = output_file(step, id, &spec.output_params, name, &spec.output_ext);
            if let (Some(l), Some(e)) = (logged.as_object_mut(), exec.as_object_mut()) {
                l.insert(name.clone(), Value::String(file.clone()));
                e.insert(
                    name.clone(),
                    Value::String(self.out_dir.join(&file).to_string_lossy().into_owned()),
                );
            }
        }
        self.registry.validate_params(id, &logged)?;

        let mut inputs = Vec::new();
        for name in &spec.path_params {
            if let (Some(l), Some(e)) = (logged.get_mut(name), exec.get_mut(name)) {
                self.resolve_param(l, e, &mut inputs, prior)?;
            }
        }
        Ok(Resolved {
            logged,
            exec,
            inputs,
        })
    }

    /// Remplace, dans la copie journalisée, un chemin par sa forme relative au
    /// projet, et dans la copie exécutée par sa forme absolue. Une chaîne ou un
    /// tableau de chaînes ; toute autre valeur est laissée à la validation.
    /// Un repère `out:<k>.<n>` reste tel quel dans le journal.
    fn resolve_param(
        &self,
        logged: &mut Value,
        exec: &mut Value,
        inputs: &mut Vec<Value>,
        prior: &[String],
    ) -> Result<(), JournalError> {
        match (logged, exec) {
            (Value::String(l), Value::String(e)) => {
                if l.starts_with("out:") {
                    let absolute = self.resolve_reference(l, prior)?;
                    *e = absolute.to_string_lossy().into_owned();
                } else {
                    let (relative, absolute, portable) = self.resolve_path(l)?;
                    inputs.push(input_entry(&relative, &absolute, portable));
                    *l = relative;
                    *e = absolute.to_string_lossy().into_owned();
                }
            }
            (Value::Array(ls), Value::Array(es)) => {
                for (l, e) in ls.iter_mut().zip(es.iter_mut()) {
                    self.resolve_param(l, e, inputs, prior)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Chemin absolu de la sortie `n` de l'étape `k` désignée par `out:<k>.<n>`.
    fn resolve_reference(
        &self,
        reference: &str,
        prior: &[String],
    ) -> Result<PathBuf, JournalError> {
        let bad = || JournalError::BadReference(reference.to_string());
        let (k, n) = reference
            .strip_prefix("out:")
            .and_then(|r| r.split_once('.'))
            .filter(|(k, n)| {
                [k, n]
                    .iter()
                    .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            })
            .and_then(|(k, n)| Some((k.parse::<usize>().ok()?, n.parse::<usize>().ok()?)))
            .ok_or_else(bad)?;
        if k == 0 || n == 0 {
            return Err(bad());
        }
        let id = prior.get(k - 1).ok_or_else(bad)?;
        let spec = self.registry.spec(id).ok_or_else(bad)?;
        let outputs = &spec.output_params;
        let name = outputs.get(n - 1).ok_or_else(bad)?;
        Ok(self
            .out_dir
            .join(output_file(k, id, outputs, name, &spec.output_ext)))
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
            relative_to(self.project_dir, &absolute).ok_or(JournalError::PathUnrepresentable)?;
        let portable = !relative
            .components()
            .any(|c| matches!(c, Component::ParentDir));
        Ok((posix(&relative), absolute, portable))
    }
}

/// Bilan d'un rejeu réussi.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ReplayReport {
    /// Nombre d'étapes rejouées.
    pub steps: usize,
    /// Toujours vrai pour un rejeu réussi : le premier écart l'arrête.
    pub identical: bool,
}

/// Rejeu d'un journal : même résolution, même `Router`, comparaison à ce que le
/// journal a gardé. N'écrit pas le journal ; ses sorties vont sous `replay/`.
pub struct Replay<'a> {
    registry: &'a Registry,
    project_dir: PathBuf,
    work_dir: PathBuf,
    replay_dir: PathBuf,
    lines: Vec<Value>,
    ids: Vec<String>,
    warnings: Vec<String>,
}

impl<'a> Replay<'a> {
    /// Relit le journal du projet et fait les contrôles préalables : journal
    /// présent, toutes les commandes connues du registre. Rien n'est exécuté.
    pub fn load(registry: &'a Registry, project: &Path) -> Result<Self, ErrorReport> {
        let (project_dir, work_dir) = locate(project)?;
        let text = match fs::read_to_string(work_dir.join(JOURNAL_FILE)) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(JournalError::NoJournal.into())
            }
            Err(e) => return Err(io_error(e).into()),
        };
        let mut lines = Vec::new();
        for raw in content_lines(&text) {
            let line = serde_json::from_str::<Value>(raw)
                .map_err(|_| JournalError::Io("InvalidData".to_string()))?;
            lines.push(line);
        }
        if lines.is_empty() {
            return Err(JournalError::NoJournal.into());
        }
        let mut ids = Vec::new();
        let mut warnings = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let step = i + 1;
            let id = line
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| JournalError::Io("InvalidData".to_string()))?;
            if registry.spec(id).is_none() {
                return Err(
                    ErrorReport::from(crate::registry::RegistryError::UnknownCommand(
                        id.to_string(),
                    ))
                    .at_step(step),
                );
            }
            ids.push(id.to_string());
            for input in line
                .get("inputs")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if input.get("portable") == Some(&Value::Bool(false)) {
                    let path = input.get("path").and_then(Value::as_str).unwrap_or("?");
                    warnings.push(format!(
                        "Étape {step} : l'entrée « {path} » est hors du projet (non portable) ; le rejeu continue avec ce chemin."
                    ));
                }
            }
        }
        let replay_dir = work_dir.join("replay");
        Ok(Replay {
            registry,
            project_dir,
            work_dir,
            replay_dir,
            lines,
            ids,
            warnings,
        })
    }

    /// Avertissements relevés avant la première étape (entrées non portables).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Réexécute chaque étape par `router`, dans l'ordre ; s'arrête au premier écart.
    pub fn run(&self, router: &Router) -> Result<ReplayReport, ErrorReport> {
        fs::create_dir_all(&self.replay_dir).map_err(io_error)?;
        let resolver = Resolver {
            registry: self.registry,
            project_dir: &self.project_dir,
            out_dir: &self.replay_dir,
        };
        let mut replayed = 0;
        for (i, line) in self.lines.iter().enumerate() {
            let step = i + 1;
            // Une étape annulée n'a ni résultat ni erreur : on la saute (AD-12).
            if line.get("cancelled") == Some(&Value::Bool(true)) {
                continue;
            }
            replayed += 1;
            let id = self.ids[i].as_str();
            let spec = self
                .registry
                .spec(id)
                .ok_or_else(|| crate::registry::RegistryError::UnknownCommand(id.to_string()))
                .map_err(|e| ErrorReport::from(e).at_step(step))?;
            let params = line.get("params").cloned().unwrap_or_else(|| json!({}));
            let resolved = resolver
                .resolve_call(id, spec, step, params, &self.ids[..i])
                .map_err(|e| e.at_step(step))?;
            let outcome = router.call(id, resolved.exec);
            if let Err(e @ crate::registry::RegistryError::Unavailable(_)) = &outcome {
                return Err(ErrorReport::from(e).at_step(step));
            }
            let diverged = |why: String| ErrorReport::from(JournalError::Diverged { step, why });
            let same = match (&outcome, line.get("result"), line.get("error")) {
                (Ok(value), Some(kept), None) => summarize(value) == *kept,
                (Err(e), None, Some(Value::String(code))) => e.code() == code,
                _ => false,
            };
            if !same {
                return Err(diverged(format!(
                    "la commande {id} ne redonne pas le même résultat ou la même erreur"
                )));
            }
            for name in &spec.output_params {
                let file = output_file(step, id, &spec.output_params, name, &spec.output_ext);
                let kept = fs::read(self.work_dir.join(&file)).ok();
                let again = fs::read(self.replay_dir.join(&file)).ok();
                if kept != again {
                    return Err(diverged(format!(
                        "la sortie « {name} » de la commande {id} n'a pas les mêmes octets"
                    )));
                }
            }
        }
        Ok(ReplayReport {
            steps: replayed,
            identical: true,
        })
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

    #[test]
    fn output_file_uses_the_declared_extension() {
        let one = ["out".to_string()];
        assert_eq!(
            output_file(1, "analysis.buffer", &one, "out", ".gpkg"),
            "1-analysis.buffer.gpkg"
        );
        assert_eq!(output_file(2, "x", &one, "out", ".out"), "2-x.out");
        let two = ["a".to_string(), "b".to_string()];
        assert_eq!(output_file(3, "x", &two, "b", ".gpkg"), "3-x-b.gpkg");
    }

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

    fn replay_of(
        registry: &Registry,
        project: &Path,
    ) -> Result<(ReplayReport, Vec<String>), ErrorReport> {
        let replay = Replay::load(registry, project)?;
        let warnings = replay.warnings().to_vec();
        replay
            .run(&Router::new(registry))
            .map(|report| (report, warnings))
    }

    #[test]
    fn an_output_reference_is_journaled_as_is_and_replays_identically() {
        let (dir, project, token) = temp_project();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project).unwrap();
        s.call("test.write_output", json!({"text": "salut"}))
            .unwrap();
        let out = s
            .call("test.read_file", json!({"path": "out:1.1"}))
            .unwrap();
        assert_eq!(out, json!({"bytes": 5}));
        let l = lines(&s);
        assert_eq!(l[1]["params"], json!({"path": "out:1.1"}));
        assert_eq!(l[1]["inputs"], json!([]));
        let before = fs::read_to_string(s.journal_path()).unwrap();

        // La session tient encore le verrou : le rejeu reste permis.
        let (report, warnings) = replay_of(&registry, &project).unwrap();
        assert_eq!(
            report,
            ReplayReport {
                steps: 2,
                identical: true
            }
        );
        assert!(warnings.is_empty());
        assert_eq!(fs::read_to_string(s.journal_path()).unwrap(), before);
        let work = s.work_dir().to_path_buf();
        assert_eq!(
            fs::read_to_string(work.join("replay/1-test.write_output.out")).unwrap(),
            "salut"
        );
        assert_eq!(
            fs::read_to_string(work.join("1-test.write_output.out")).unwrap(),
            "salut"
        );
        assert!(!before.contains(&token));
        drop(s);
        let _ = fs::remove_dir_all(dir);
    }

    /// Écrit un journal à la main, une ligne JSON par étape.
    fn hand_journal(project: &Path, rows: &[Value]) {
        let work = project.parent().unwrap().join("p.cassini");
        fs::create_dir_all(&work).unwrap();
        let text: String = rows.iter().map(|r| format!("{r}\n")).collect();
        fs::write(work.join(JOURNAL_FILE), text).unwrap();
    }

    #[test]
    fn an_unknown_command_fails_by_name_and_executes_nothing() {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        hand_journal(
            &project,
            &[
                json!({"step":1,"id":"test.write_output","params":{"out":"1-test.write_output.out","text":"a"},"inputs":[],"result":{"bytes":1}}),
                json!({"step":2,"id":"no.such","params":{},"inputs":[],"result":{}}),
            ],
        );
        let err = replay_of(&registry, &project).unwrap_err();
        assert_eq!(err.code, "registry.unknown_command");
        assert!(err.message.contains("no.such"));
        assert_eq!(err.step, Some(2));
        assert!(!dir.join("p.cassini/replay").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_qgis_command_without_executor_is_unavailable_on_replay() {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project).unwrap();
        s.call("test.qgis_probe", json!({"layer": "l"}))
            .unwrap_err();
        drop(s);
        let err = replay_of(&registry, &project).unwrap_err();
        assert_eq!(err.code, "registry.unavailable");
        assert!(err.message.contains("test.qgis_probe"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_non_portable_input_is_warned_and_replay_goes_on() {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project).unwrap();
        s.call("test.read_file", json!({"path": "../x.tif"}))
            .unwrap();
        drop(s);
        let (report, warnings) = replay_of(&registry, &project).unwrap();
        assert_eq!(report.steps, 1);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("../x.tif"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_changed_input_diverges_at_its_step() {
        let (dir, project, _) = temp_project();
        fs::write(dir.join("data.txt"), "abc").unwrap();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project).unwrap();
        s.call("system.about", json!({})).unwrap();
        s.call("test.read_file", json!({"path": "data.txt"}))
            .unwrap();
        drop(s);
        fs::write(dir.join("data.txt"), "abcdef").unwrap();
        let err = replay_of(&registry, &project).unwrap_err();
        assert_eq!(err.code, "replay.diverged");
        assert_eq!(err.step, Some(2));
        assert!(!err.message.contains("cassini-journal-"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn an_output_with_other_bytes_diverges() {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project).unwrap();
        s.call("test.write_output", json!({"text": "salut"}))
            .unwrap();
        let original = s.work_dir().join("1-test.write_output.out");
        drop(s);
        fs::write(original, "autre").unwrap();
        let err = replay_of(&registry, &project).unwrap_err();
        assert_eq!(err.code, "replay.diverged");
        assert_eq!(err.step, Some(1));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn an_invalid_reference_is_refused_both_ways() {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project).unwrap();
        s.call("system.about", json!({})).unwrap();
        for bad in ["out:9.1", "out:1.1", "out:1", "out:x.1", "out:0.1"] {
            let err = s.call("test.read_file", json!({"path": bad})).unwrap_err();
            assert_eq!(err.code, "replay.bad_reference", "{bad}");
        }
        assert_eq!(lines(&s).len(), 1);
        drop(s);
        hand_journal(
            &project,
            &[
                json!({"step":1,"id":"test.read_file","params":{"path":"out:9.1"},"inputs":[],"result":{"bytes":0}}),
            ],
        );
        let err = replay_of(&registry, &project).unwrap_err();
        assert_eq!(err.code, "replay.bad_reference");
        assert_eq!(err.step, Some(1));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn no_journal_is_refused() {
        let (dir, project, _) = temp_project();
        let registry = Registry::standard();
        let err = replay_of(&registry, &project).unwrap_err();
        assert_eq!(err.code, "replay.no_journal");
        fs::create_dir_all(dir.join("p.cassini")).unwrap();
        fs::write(dir.join("p.cassini").join(JOURNAL_FILE), "\n").unwrap();
        let err = replay_of(&registry, &project).unwrap_err();
        assert_eq!(err.code, "replay.no_journal");
        let _ = fs::remove_dir_all(dir);
    }

    /// QGIS de test : rend un inventaire fixe, sans aucun chemin.
    struct InventoryQgis;
    impl Executor for InventoryQgis {
        fn execute(&self, id: &str, _p: Value) -> Result<Value, RegistryError> {
            assert_eq!(id, "project.inventory");
            Ok(json!({
                "layers": [
                    {"layer_id": "a_1", "name": "gares", "source": "gares.gpkg", "crs": "EPSG:2154"},
                    {"layer_id": "b_2", "name": "parcelles", "source": "parcelles.shp", "crs": "EPSG:2154"}
                ],
                "project_crs": "EPSG:2154",
                "extent": {"xmin": 1.0, "ymin": 2.0, "xmax": 3.0, "ymax": 4.0, "crs": "EPSG:2154"}
            }))
        }
    }

    fn copy_dir(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    #[test]
    fn a_project_copied_with_its_journal_folder_replays_identically() {
        let (dir, project, token) = temp_project();
        fs::write(&project, "projet").unwrap();
        fs::write(dir.join("data.txt"), "abc").unwrap();
        let registry = Registry::standard();
        let mut s = Session::open(&registry, &project)
            .unwrap()
            .with_qgis(InventoryQgis);
        s.call("test.read_file", json!({"path": "data.txt"}))
            .unwrap();
        s.call("project.inventory", json!({})).unwrap();
        let journal = fs::read_to_string(s.journal_path()).unwrap();
        drop(s);
        assert!(!journal.contains(&token));

        // Le projet et son dossier `.cassini` partent ensemble ailleurs.
        let (moved, moved_project, moved_token) = temp_project();
        fs::copy(&project, &moved_project).unwrap();
        fs::copy(dir.join("data.txt"), moved.join("data.txt")).unwrap();
        copy_dir(&dir.join("p.cassini"), &moved.join("p.cassini"));
        let _ = fs::remove_dir_all(&dir);

        let replay = Replay::load(&registry, &moved_project).unwrap();
        assert!(replay.warnings().is_empty());
        let report = replay
            .run(&Router::new(&registry).with_qgis(InventoryQgis))
            .unwrap();
        assert_eq!(
            report,
            ReplayReport {
                steps: 2,
                identical: true
            }
        );
        let copied = fs::read_to_string(moved.join("p.cassini").join(JOURNAL_FILE)).unwrap();
        assert_eq!(copied, journal);
        assert!(!copied.contains(&moved_token));
        let _ = fs::remove_dir_all(moved);
    }

    #[test]
    fn lexical_paths_behave() {
        let base = Path::new("/p/proj");
        let t = normalize(&base.join("a/../b/./c.tif"));
        assert_eq!(posix(&relative_to(base, &t).unwrap()), "b/c.tif");
        assert_eq!(posix(&relative_to(base, base).unwrap()), ".");
    }
}
