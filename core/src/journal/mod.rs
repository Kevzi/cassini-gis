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

use std::collections::HashMap;
use std::fmt;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde_json::{json, Map, Value};

mod canonical;
mod paths;
mod summary;
#[cfg(test)]
mod tests;

pub use canonical::{canonical, registry_version};
/// Chemins sous le dossier de travail rendus relatifs (aussi pour la réponse de `plan.execute`).
pub(crate) use paths::relativize;
use paths::{normalize, posix, relative_to};
use summary::{same_result, summarize};

use crate::error::ErrorReport;
use crate::exec::{Executor, Router};
use crate::registry::{Registry, RegistryError, PROTOCOL_VERSION};

/// Nom du journal, dans le dossier de travail.
const JOURNAL_FILE: &str = "journal.jsonl";
/// Nom du verrou, dans le dossier de travail.
const LOCK_FILE: &str = "journal.lock";

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
    /// Une ligne du journal est illisible (JSON invalide ou sans `id`) : `step`
    /// est son rang parmi les lignes écrites, à partir de 1.
    UnreadableLine { step: usize },
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
            JournalError::UnreadableLine { .. } => "replay.unreadable_line",
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
            JournalError::UnreadableLine { step } => write!(
                f,
                "La ligne du journal à l'étape {step} est illisible : le journal ne peut pas être rejoué."
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
            Ok(value) => {
                // L'id de couche rendu par l'étape : le rejeu le lie à celui qu'il obtient.
                if let Some(layer_id) = value.get("layer_id").and_then(Value::as_str) {
                    line.insert("layer_id".into(), json!(layer_id));
                }
                line.insert(
                    "result".into(),
                    summarize(&relativize(value, &self.work_dir)),
                );
            }
            Err(RegistryError::Cancelled(_)) => {
                line.insert("cancelled".into(), json!(true));
            }
            Err(e) => {
                line.insert("error".into(), json!(e.code()));
            }
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
        for (i, raw) in content_lines(&text).enumerate() {
            let line = serde_json::from_str::<Value>(raw)
                .map_err(|_| JournalError::UnreadableLine { step: i + 1 })?;
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
                .ok_or(JournalError::UnreadableLine { step })?;
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

    /// Les fichiers d'entrée sont-ils ceux que le journal a vus ? Relit `size` et
    /// `modified` (les valeurs absentes du journal ne sont pas contrôlées) ; un
    /// fichier modifié ou disparu arrête le rejeu à cette étape.
    fn check_inputs(
        &self,
        resolver: &Resolver,
        line: &Value,
        step: usize,
    ) -> Result<(), ErrorReport> {
        let entries = line.get("inputs").and_then(Value::as_array);
        for input in entries.into_iter().flatten() {
            let Some(path) = input.get("path").and_then(Value::as_str) else {
                continue;
            };
            let (size, modified) = (input.get("size"), input.get("modified"));
            if size.is_none_or(Value::is_null) && modified.is_none_or(Value::is_null) {
                continue;
            }
            let absolute = resolver
                .resolve_path(path)
                .map_err(|e| ErrorReport::from(e).at_step(step))?
                .1;
            let now = input_entry(path, &absolute, true);
            let unchanged = |key: &str, kept: Option<&Value>| {
                kept.is_none_or(Value::is_null) || kept == now.get(key)
            };
            if !(unchanged("size", size) && unchanged("modified", modified)) {
                return Err(ErrorReport::from(JournalError::Diverged {
                    step,
                    why: format!(
                        "l'entrée « {path} » a changé ou disparu depuis la journalisation"
                    ),
                }));
            }
        }
        Ok(())
    }

    /// Réexécute chaque étape par `router`, dans l'ordre ; s'arrête au premier écart.
    pub fn run(&self, router: &Router) -> Result<ReplayReport, ErrorReport> {
        // Un rejeu précédent ne doit pas laisser de fichier qu'un autre pourrait reprendre.
        let _ = fs::remove_dir_all(&self.replay_dir);
        fs::create_dir_all(&self.replay_dir).map_err(io_error)?;
        let resolver = Resolver {
            registry: self.registry,
            project_dir: &self.project_dir,
            out_dir: &self.replay_dir,
        };
        let mut replayed = 0;
        // Ids de couche de la session d'origine -> ids obtenus par ce rejeu.
        let mut bound: HashMap<String, String> = HashMap::new();
        for (i, line) in self.lines.iter().enumerate() {
            let step = i + 1;
            // Une étape annulée n'a ni résultat ni erreur : on la saute (AD-12).
            if line.get("cancelled") == Some(&Value::Bool(true)) {
                continue;
            }
            replayed += 1;
            self.check_inputs(&resolver, line, step)?;
            let id = self.ids[i].as_str();
            let spec = self
                .registry
                .spec(id)
                .ok_or_else(|| crate::registry::RegistryError::UnknownCommand(id.to_string()))
                .map_err(|e| ErrorReport::from(e).at_step(step))?;
            let mut params = line.get("params").cloned().unwrap_or_else(|| json!({}));
            substitute(&mut params, &bound);
            let resolved = resolver
                .resolve_call(id, spec, step, params, &self.ids[..i])
                .map_err(|e| e.at_step(step))?;
            let outcome = router.call(id, resolved.exec);
            if let Err(e @ crate::registry::RegistryError::Unavailable(_)) = &outcome {
                return Err(ErrorReport::from(e).at_step(step));
            }
            let diverged = |why: String| ErrorReport::from(JournalError::Diverged { step, why });
            let same = match (&outcome, line.get("result"), line.get("error")) {
                (Ok(value), Some(kept), None) => {
                    same_result(&relativize(value, &self.replay_dir), kept)
                }
                (Err(e), None, Some(Value::String(code))) => e.code() == code,
                _ => false,
            };
            if !same {
                return Err(diverged(format!(
                    "la commande {id} ne redonne pas le même résultat ou la même erreur"
                )));
            }
            if let (Ok(value), Some(old)) = (&outcome, line.get("layer_id").and_then(Value::as_str))
            {
                if let Some(new) = value.get("layer_id").and_then(Value::as_str) {
                    bound.insert(old.to_string(), new.to_string());
                }
            }
            for name in &spec.output_params {
                let file = output_file(step, id, &spec.output_params, name, &spec.output_ext);
                let again = fs::read(self.replay_dir.join(&file)).ok();
                if spec.runs_in == Some(crate::registry::RunsIn::Qgis) {
                    // GeoPackage et PDF portent dates et identifiants : la fidélité d'une
                    // sortie de QGIS tient au résultat typé, pas aux octets.
                    if again.is_none_or(|bytes| bytes.is_empty()) {
                        return Err(diverged(format!(
                            "la sortie « {name} » de la commande {id} est absente ou vide"
                        )));
                    }
                } else if fs::read(self.work_dir.join(&file)).ok() != again {
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

/// Remplace, dans toutes les chaînes de `value`, un ancien id de couche par le neuf.
fn substitute(value: &mut Value, bound: &HashMap<String, String>) {
    match value {
        Value::String(s) => {
            if let Some(new) = bound.get(s.as_str()) {
                *s = new.clone();
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| substitute(v, bound)),
        Value::Object(map) => map.values_mut().for_each(|v| substitute(v, bound)),
        _ => {}
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
