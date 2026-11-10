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

use std::collections::{BTreeSet, HashMap, HashSet};
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

use crate::commands::layer_open_raster::OPEN_RASTER;
use crate::commands::layer_open_vector::OPEN_VECTOR;
use crate::commands::layer_set_style::SET_STYLE;
use crate::error::ErrorReport;
use crate::exec::{Executor, Router};
use crate::geojson::VECTOR_EXTENSIONS;
use crate::raster_path::RASTER_EXTENSIONS;
use crate::registry::{Registry, RegistryError, INVENTORY, ORDERS_FIELD, PROTOCOL_VERSION};
use crate::style::spec::RESULT_RAMP_KIND;

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
    /// Aucun dossier de sortie du rejeu n'a pu être vidé : un fichier d'un rejeu
    /// précédent est encore tenu. Porte son chemin relatif au dossier de travail,
    /// en écriture POSIX (AD-9).
    OutputLocked(String),
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
            JournalError::OutputLocked(_) => "replay.output_locked",
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
            JournalError::OutputLocked(file) => write!(
                f,
                "Le rejeu ne trouve aucun dossier de sortie vide : le fichier « {file} » d'un rejeu précédent est encore tenu (par QGIS ?). Fermez les couches du rejeu précédent ou rouvrez le projet, puis relancez."
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
    /// Vrai si l'exécutant QGIS annonce `layer.open_raster` : la session la dicte alors après
    /// chaque étape du noyau qui écrit un raster (ticket 4.9). Faux sans QGIS (CLI, MCP autonome).
    opens_rasters: bool,
    /// Vrai si l'exécutant QGIS annonce `layer.open_vector` : la session la dicte alors après
    /// chaque étape du noyau qui écrit un GeoJSON (AD-30, ticket 11.8).
    opens_vectors: bool,
    /// Commandes que l'exécutant QGIS annonce. Une commande qui lit l'inventaire ou dicte des
    /// ordres (`CommandSpec::required_gestures`) n'est exécutée que si tous ses gestes y sont
    /// (entrée 11.20) ; sinon `style.unavailable`, sans rien écrire. Vide sans QGIS.
    gestures: HashSet<String>,
    /// Formes de `StyleSpec` que l'extension annonce savoir appliquer (`style_kinds` de
    /// `session.hello`). Le style d'un raster de résultat n'est dicté que si `result_ramp` y est
    /// (décision du propriétaire, 2026-11-09). Vide sans QGIS ou pour une extension plus ancienne.
    style_kinds: HashSet<String>,
    /// Ordres que la dernière étape demande de dicter juste après sa ligne (amendement d'AD-30).
    pending_orders: Vec<(String, Value)>,
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
            opens_rasters: false,
            opens_vectors: false,
            gestures: HashSet::new(),
            style_kinds: HashSet::new(),
            pending_orders: Vec::new(),
        })
    }

    /// Branche un exécutant QGIS à la place du bouchon.
    pub fn with_qgis(self, qgis: impl Executor + 'a) -> Self {
        Session {
            router: self.router.with_qgis(qgis),
            ..self
        }
    }

    /// Dit si l'exécutant QGIS sait ouvrir un raster du noyau (`layer.open_raster` annoncée) :
    /// la session dicte alors cette ouverture après chaque sortie raster (ticket 4.9).
    pub fn with_raster_opening(self, announced: bool) -> Self {
        Session {
            opens_rasters: announced,
            ..self
        }
    }

    /// Dit si l'exécutant QGIS sait ouvrir un GeoJSON du noyau (`layer.open_vector` annoncée) :
    /// la session dicte alors cette ouverture après chaque sortie vecteur (AD-30, ticket 11.8).
    pub fn with_vector_opening(self, announced: bool) -> Self {
        Session {
            opens_vectors: announced,
            ..self
        }
    }

    /// Dit quelles commandes l'exécutant QGIS annonce : une mise en forme (`map.topographic_style`,
    /// `map.graduated_style`) n'est exécutée que si tous ses gestes y sont (entrée 11.20).
    pub fn with_gestures<I: IntoIterator<Item = String>>(self, announced: I) -> Self {
        Session {
            gestures: announced.into_iter().collect(),
            ..self
        }
    }

    /// Dit quelles formes de `StyleSpec` l'extension sait appliquer (`style_kinds` de
    /// `session.hello`) : le style d'un raster de résultat demande `result_ramp`.
    pub fn with_style_kinds<I: IntoIterator<Item = String>>(self, announced: I) -> Self {
        Session {
            style_kinds: announced.into_iter().collect(),
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

    /// Chemin absolu d'un fichier d'entrée `raw` tel qu'un appel le résoudrait : relatif au
    /// dossier du projet, ou absolu (AD-9). Lecture seule, rien n'est journalisé (ticket 11.22).
    /// `None` si le chemin ne se représente pas depuis le projet.
    pub fn input_path(&self, raw: &str) -> Option<PathBuf> {
        let resolver = Resolver {
            registry: self.registry,
            project_dir: &self.project_dir,
            out_dir: &self.work_dir,
        };
        resolver
            .resolve_path(raw)
            .ok()
            .map(|(_, absolute, _)| absolute)
    }

    /// Les `limit` dernières étapes du journal, pour que le panneau les montre à la réouverture
    /// du projet : chacune réduite à son numéro, sa commande, ses paramètres (résumés au-delà de
    /// 200 octets), son issue (`result`, `error` ou `cancelled`) et, s'il y a lieu, `origin`,
    /// `plan_step`, `layer_id` et `at` (date de l'étape, en ms depuis 1970). Une ligne
    /// illisible est sautée ; un journal absent ou illisible rend une liste vide.
    pub fn history(&self, limit: usize) -> Vec<Value> {
        history_of(&self.journal_path(), limit)
    }

    /// Exécute une commande par le routeur et journalise l'étape.
    ///
    /// Une commande refusée avant exécution (id inconnu, paramètres invalides,
    /// chemin non représentable) n'écrit rien. Une session en lecture seule
    /// répond `journal.read_only` sans rien exécuter.
    pub fn call(&mut self, id: &str, params: Value) -> Result<Value, ErrorReport> {
        self.call_from(id, params, None)
    }

    /// Comme `call`, en notant l'origine de l'étape (`"origin"`, par exemple `external` pour un
    /// appel d'une IA externe, AD-29). Le rejeu l'ignore : il ne lit que des clés nommées.
    pub fn call_from(
        &mut self,
        id: &str,
        params: Value,
        origin: Option<&str>,
    ) -> Result<Value, ErrorReport> {
        self.call_logged(id, params, origin, None)
    }

    /// Comme `call`, pour l'étape `plan_step` (à partir de 1) d'un plan exécuté : la ligne porte
    /// la clé `"plan_step"` (AD-11, ticket 5.34), d'où le noyau déduit les couches intermédiaires.
    /// Le rejeu l'ignore.
    pub fn call_in_plan(
        &mut self,
        id: &str,
        params: Value,
        plan_step: usize,
    ) -> Result<Value, ErrorReport> {
        self.call_logged(id, params, None, Some(plan_step))
    }

    /// Exécute et journalise l'étape, puis, si elle a écrit un raster ou un GeoJSON, dicte son
    /// ouverture.
    ///
    /// Une commande du noyau dont la sortie est un GeoTIFF est suivie, quand l'extension annonce
    /// `layer.open_raster`, d'une étape `layer.open_raster {path: "out:<k>.<n>"}` par sortie, de
    /// même `origin` et de même `plan_step` (ticket 4.9) ; une sortie GeoJSON l'est de même par
    /// `layer.open_vector`, si l'extension l'annonce (AD-30, ticket 11.8). La sortie entre au
    /// projet par une étape journalisée, avec son `layer_id`, que le rejeu rejoue et sait laisser
    /// hors de la carte. Une ouverture en échec est l'erreur de l'appel, nommée (AD-21). Un
    /// raster ouvert d'une commande qui déclare son style (`CommandSpec::result_style`) est suivi,
    /// si l'extension annonce `layer.set_style`, d'une étape `layer.set_style` sur la couche
    /// ouverte, au style composé par le noyau (bogue du fond noir, 2026-11-09). Le résultat rendu
    /// est celui de l'étape demandée.
    fn call_logged(
        &mut self,
        id: &str,
        params: Value,
        origin: Option<&str>,
        plan_step: Option<usize>,
    ) -> Result<Value, ErrorReport> {
        let step = self.next_step;
        let value = self.call_step(id, params, origin, plan_step)?;
        // Les ordres que l'étape demande (`layer.set_style`, puis `layer.arrange`) : chacun une
        // étape du journal, dans l'ordre, juste après sa ligne ; le premier refus arrête la suite
        // et devient l'erreur de l'appel (amendement d'AD-30, entrée 11.20).
        for (command, order) in std::mem::take(&mut self.pending_orders) {
            self.call_step(&command, order, origin, plan_step)?;
        }
        if let Some(opener) = self.opener_of(id) {
            let style = self.result_style_of(id, opener, &value)?;
            for path in self.outputs_written(id, step) {
                let opened = self.call_step(opener, json!({ "path": path }), origin, plan_step)?;
                // Le style du raster de résultat, juste après son ouverture, sur la couche
                // qu'elle rend : un refus est l'erreur de l'appel, comme un ordre de mise en
                // forme (bogue du fond noir, 2026-11-09).
                if let (Some(style), Some(layer_id)) =
                    (&style, opened.get("layer_id").and_then(Value::as_str))
                {
                    let order = json!({ "layer_id": layer_id, "style": style });
                    self.call_step(SET_STYLE, order, origin, plan_step)?;
                }
            }
        }
        Ok(value)
    }

    /// Complète un carroyage `mgrs` du paramètre `name` : ôte `epsg` et `squares` de
    /// l'appelant, lit l'inventaire auprès de QGIS et y pose la zone UTM et les carrés de
    /// 100 km de l'emprise du projet. Sans emprise lisible, `layout.no_extent`, avant tout ordre
    /// d'export et sans ligne au journal. Les autres formes de carroyage ne lisent rien.
    fn fill_mgrs_grid(&self, id: &str, params: &mut Value, name: &str) -> Result<(), ErrorReport> {
        let Some(grid) = params.get_mut(name).and_then(Value::as_object_mut) else {
            return Ok(());
        };
        if grid.get("kind").and_then(Value::as_str) != Some("mgrs") {
            return Ok(());
        }
        grid.remove("epsg");
        grid.remove("squares");
        // Des paramètres invalides (intervalle hors liste, champ en trop) sont refusés tels quels,
        // avant toute lecture auprès de QGIS.
        self.registry.validate_params(id, params)?;
        let inventory = self.router.call(INVENTORY, json!({}))?;
        let (epsg, squares) = crate::commands::layout_export_pdf::mgrs_grid_of(&inventory)
            .ok_or_else(|| RegistryError::Delegated {
                code: "layout.no_extent".into(),
                message: "Le projet n'a pas d'emprise lisible en degrés (couches vides, ou plus \
                          de deux fuseaux UTM) : aucun carroyage MGRS ne peut être posé."
                    .into(),
            })?;
        if let Some(grid) = params.get_mut(name).and_then(Value::as_object_mut) {
            grid.insert("epsg".into(), json!(epsg));
            grid.insert("squares".into(), json!(squares));
        }
        Ok(())
    }

    /// Commande d'ouverture que la session dicte après une étape de `id` : `layer.open_raster`
    /// pour une sortie GeoTIFF, d'une commande du noyau ou d'une commande déléguée qui écrit un
    /// raster (amendement du 2026-11-05 d'AD-24, `data.convert_raster`) ; `layer.open_vector`
    /// pour une sortie GeoJSON d'une commande du noyau seulement (une couche vecteur déléguée
    /// est posée au projet par son geste) ; et si l'extension annonce le geste ; `None` sinon.
    fn opener_of(&self, id: &str) -> Option<&'static str> {
        let spec = self.registry.spec(id)?;
        let core = match spec.runs_in {
            Some(crate::registry::RunsIn::Core) => true,
            Some(crate::registry::RunsIn::Qgis) => false,
            None => return None,
        };
        let ext = spec.output_ext.as_str();
        let opener = if RASTER_EXTENSIONS.contains(&ext) && self.opens_rasters {
            OPEN_RASTER
        } else if core && VECTOR_EXTENSIONS.contains(&ext) && self.opens_vectors {
            OPEN_VECTOR
        } else {
            return None;
        };
        self.registry.spec(opener).map(|_| opener)
    }

    /// Style à dicter après l'ouverture d'un raster de `id` : celui que la commande déclare
    /// (`CommandSpec::result_style`), composé d'après son résultat `value`, si l'ouverture est
    /// celle d'un raster, si l'extension annonce `layer.set_style` et si elle annonce savoir
    /// appliquer la forme `result_ramp` (`style_kinds` de `session.hello`, décision du
    /// propriétaire du 2026-11-09) ; `Ok(None)` sinon, et le raster est ouvert sans style, comme
    /// avant. Un style composé invalide ou non sérialisable est un défaut du noyau, nommé
    /// (`style.invalid`), jamais un style sauté en silence.
    fn result_style_of(
        &self,
        id: &str,
        opener: &str,
        value: &Value,
    ) -> Result<Option<Value>, ErrorReport> {
        let Some(style) = self.registry.spec(id).and_then(|spec| spec.result_style) else {
            return Ok(None);
        };
        if opener != OPEN_RASTER
            || !self.gestures.contains(SET_STYLE)
            || !self.style_kinds.contains(RESULT_RAMP_KIND)
            || self.registry.spec(SET_STYLE).is_none()
        {
            return Ok(None);
        }
        let composed = style.compose(value);
        let core_fault = |why: String| {
            ErrorReport::from(RegistryError::Failed {
                code: crate::style::spec::STYLE_INVALID.to_string(),
                message: format!(
                    "Le noyau a composé un style invalide pour le résultat de « {id} » ({why}) : \
                     le raster n'est pas ouvert."
                ),
            })
        };
        composed.validate().map_err(|e| core_fault(e.message))?;
        serde_json::to_value(&composed)
            .map(Some)
            .map_err(|e| core_fault(e.to_string()))
    }

    /// Repères `out:<step>.<n>` des sorties que l'étape `step` (commande `id`, réussie) a écrites.
    fn outputs_written(&self, id: &str, step: usize) -> Vec<String> {
        let Some(spec) = self.registry.spec(id) else {
            return Vec::new();
        };
        (1..=spec.output_params.len())
            .map(|n| format!("out:{step}.{n}"))
            .collect()
    }

    fn call_step(
        &mut self,
        id: &str,
        params: Value,
        origin: Option<&str>,
        plan_step: Option<usize>,
    ) -> Result<Value, ErrorReport> {
        // Seule l'étape qui vient de réussir peut laisser des ordres à dicter.
        self.pending_orders.clear();
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
        // Le paramètre que seul le rejeu remplit n'est jamais celui de l'appelant (ticket 5.36).
        let mut params = params;
        if let Value::Object(map) = &mut params {
            if let Some(name) = &spec.leave_out_param {
                map.remove(name);
            }
            if let Some((name, _)) = &spec.replay_param {
                map.remove(name);
            }
            // L'inventaire n'est jamais celui de l'appelant : le noyau le lit lui-même (11.20).
            if let Some(name) = &spec.inventory_param {
                map.remove(name);
            }
        }
        // Un carroyage MGRS : sa zone et ses carrés de 100 km sont ceux du noyau, d'après
        // l'emprise du projet lue auprès de QGIS, jamais ceux de l'appelant ; ils sont
        // journalisés avec les paramètres, et le rejeu les repasse tels quels (epic 10, entrée 31).
        if let Some(name) = &spec.mgrs_grid_param {
            self.fill_mgrs_grid(id, &mut params, name)?;
        }
        // Une mise en forme sans les gestes qu'elle demande n'est pas exécutée, et rien n'est
        // écrit (AD-25, entrée 11.20).
        let gestures = spec.required_gestures();
        if !gestures.iter().all(|g| self.gestures.contains(*g)) {
            return Err(crate::style::unavailable().into());
        }
        let Resolved {
            logged,
            mut exec,
            inputs,
        } = resolver.resolve_call(id, spec, step, params, &prior)?;
        // L'inventaire est lu auprès de QGIS, hors du journal, juste avant l'exécution. Seul
        // l'ordre de sa légende est gardé, pour le rejeu : le résultat en dépend.
        let mut legend = None;
        if let Some(name) = &spec.inventory_param {
            let inventory = read_inventory(&self.router)?;
            legend = Some(legend_of(&inventory));
            if let Value::Object(map) = &mut exec {
                map.insert(name.clone(), inventory);
            }
        }

        let outcome = self.router.call(id, exec);
        // Les ordres cachés du résultat sont ôtés avant tout : ils ne sont ni journalisés, ni
        // rendus à l'appelant, mais dictés juste après cette ligne.
        let outcome = outcome.and_then(|value| {
            let (value, orders) = take_orders(spec, value)?;
            self.pending_orders = orders;
            Ok(value)
        });

        let mut line = Map::new();
        line.insert("step".into(), json!(step));
        line.insert("id".into(), json!(id));
        line.insert("params".into(), canonical(&logged));
        line.insert("inputs".into(), Value::Array(inputs));
        // Date et heure de l'étape, en millisecondes depuis 1970 (UTC), pour l'historique du
        // panneau ; le rejeu ne la compare pas.
        if let Ok(now) = std::time::SystemTime::now().duration_since(UNIX_EPOCH) {
            line.insert("at".into(), json!(now.as_millis() as u64));
        }
        if let Some(origin) = origin {
            line.insert("origin".into(), json!(origin));
        }
        if let Some(plan_step) = plan_step {
            line.insert("plan_step".into(), json!(plan_step));
        }
        let cancelled = matches!(outcome, Err(RegistryError::Cancelled(_)));
        match &outcome {
            Ok(value) => {
                // L'id de couche rendu par l'étape : le rejeu le lie à celui qu'il obtient.
                // Une commande du genre `none` (amendement d'AD-24) ne pose aucune couche.
                if let Some(layer_id) = value
                    .get("layer_id")
                    .and_then(Value::as_str)
                    .filter(|_| !spec.output_none)
                {
                    line.insert("layer_id".into(), json!(layer_id));
                }
                line.insert(
                    "result".into(),
                    summarize(&relativize(value, &self.work_dir)),
                );
                // Ce que le rejeu repassera est gardé à part, sous la clé réservée `replay` :
                // un résultat résumé (plus de 200 octets) n'en garderait que taille et empreinte.
                let mut replay = Map::new();
                // Le paramètre que la commande a rempli elle-même (la date d'un azimut magnétique,
                // M4) est gardé de même, que l'appelant l'ait donné ou non.
                for (name, field) in spec.replay_param.iter().chain(&spec.filled_param) {
                    if let Some(kept) = value.get(field) {
                        replay.insert(name.clone(), kept.clone());
                    }
                }
                if let Some(legend) = legend.take() {
                    replay.insert(LEGEND_KEY.into(), legend);
                }
                if !replay.is_empty() {
                    line.insert("replay".into(), Value::Object(replay));
                }
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
            // Une étape annulée ne laisse pas de sortie partielle derrière elle (AD-12), ni
            // les fichiers compagnons d'un Shapefile.
            let ext = spec.output_ext_for(&logged);
            for name in &spec.output_params {
                let file = output_file(step, id, &spec.output_params, name, ext);
                for written in output_family(&file) {
                    let _ = fs::remove_file(self.work_dir.join(written));
                }
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

/// Clé, sous `replay`, de l'ordre de la légende vu par une commande qui lit l'inventaire.
const LEGEND_KEY: &str = "legend";

/// Ids des couches de l'inventaire, dans l'ordre de la légende (de haut en bas).
fn legend_of(inventory: &Value) -> Value {
    let ids = inventory
        .get("layers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|l| l.get("layer_id").and_then(Value::as_str))
        .map(|id| json!(id))
        .collect();
    Value::Array(ids)
}

/// Lit l'inventaire du projet auprès de QGIS, hors du journal. QGIS absent : `style.unavailable`.
fn read_inventory(router: &Router) -> Result<Value, RegistryError> {
    router.call(INVENTORY, json!({})).map_err(|e| match e {
        RegistryError::Unavailable(_) => crate::style::unavailable(),
        other => other,
    })
}

/// Sépare d'un résultat le champ caché des ordres à dicter, pour une commande qui en déclare
/// (`CommandSpec::dictates`). Seuls les ordres qu'elle déclare sont gardés.
fn take_orders(
    spec: &crate::registry::CommandSpec,
    mut value: Value,
) -> Result<(Value, Vec<(String, Value)>), RegistryError> {
    if spec.dictates.is_empty() {
        return Ok((value, Vec::new()));
    }
    let raw = value
        .as_object_mut()
        .and_then(|m| m.remove(ORDERS_FIELD))
        .unwrap_or(Value::Null);
    let mut orders = Vec::new();
    for o in raw.as_array().into_iter().flatten() {
        let command = o.get("command").and_then(Value::as_str).unwrap_or_default();
        // Un ordre que la commande ne déclare pas est un défaut du noyau : jamais ignoré.
        if !spec.dictates.iter().any(|d| d == command) {
            return Err(RegistryError::Failed {
                code: "registry.invalid_order".to_string(),
                message: format!(
                    "La commande « {} » demande l'ordre « {command} », qu'elle ne déclare pas : \
                     rien n'est dicté.",
                    spec.id
                ),
            });
        }
        orders.push((
            command.to_string(),
            o.get("params").cloned().unwrap_or(json!({})),
        ));
    }
    Ok((value, orders))
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

/// Voir `Session::history` ; lit le journal `path`.
pub(crate) fn history_of(path: &Path, limit: usize) -> Vec<Value> {
    let Ok(bytes) = fs::read(path) else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&bytes);
    let mut kept: Vec<Value> = content_lines(&text)
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|line| {
            let command = line.get("id")?.as_str()?;
            let mut entry = json!({
                "step": line.get("step").cloned().unwrap_or(Value::Null),
                "command": command,
                // Des paramètres longs (géométries, listes de points) sont résumés comme un
                // résultat : la réponse à `session.hello` reste courte.
                "params": summarize(&line.get("params").cloned().unwrap_or_else(|| json!({}))),
            });
            for key in [
                "result",
                "error",
                "cancelled",
                "origin",
                "plan_step",
                "layer_id",
                "at",
            ] {
                if let Some(v) = line.get(key) {
                    entry[key] = v.clone();
                }
            }
            Some(entry)
        })
        .collect();
    let skip = kept.len().saturating_sub(limit);
    kept.drain(..skip);
    kept
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

/// Extensions des fichiers compagnons qu'écrit un Shapefile à côté de son `.shp`.
const SHAPEFILE_COMPANIONS: [&str; 5] = [".shx", ".dbf", ".prj", ".cpg", ".qix"];

/// Le fichier de sortie `file` et, pour un Shapefile, ses fichiers compagnons (même radical) :
/// tout ce que l'écriture de cette sortie dépose dans le dossier de travail.
pub(crate) fn output_family(file: &str) -> Vec<String> {
    let mut family = vec![file.to_string()];
    if let Some(stem) = file.strip_suffix(".shp") {
        family.extend(
            SHAPEFILE_COMPANIONS
                .iter()
                .map(|ext| format!("{stem}{ext}")),
        );
    }
    family
}

/// Nom du fichier de sortie `name` de l'étape `step`.
fn output_file(step: usize, id: &str, outputs: &[String], name: &str, ext: &str) -> String {
    if outputs.len() == 1 {
        format!("{step}-{id}{ext}")
    } else {
        format!("{step}-{id}-{name}{ext}")
    }
}

/// Noms des fichiers de sortie intermédiaires d'après le journal `journal` (ticket 5.34) : les
/// sorties d'une étape de plan qu'une étape réussie du même plan, qui rend une couche, a consommées.
///
/// Un plan est une suite de lignes portant `plan_step`, qui commence à 1 ; une ligne sans la clé
/// (appel hors plan, journal ancien) n'appartient à aucun plan. Une étape consomme une sortie
/// antérieure du même plan quand une valeur de premier niveau de ses `params` (où le noyau a déjà
/// résolu `step:k` ou `step:k.layer_id`) est le `layer_id` rendu par cette étape antérieure, ou une
/// sortie entière qui le porte. Les noms sont ceux de `output_file`, que le rejeu reprend sous
/// `replay/`. Journal absent ou illisible : aucun nom ; une ligne illisible ne cache rien.
pub fn intermediate_files(registry: &Registry, journal: &Path) -> HashSet<String> {
    let mut hidden = HashSet::new();
    let Ok(bytes) = fs::read(journal) else {
        return hidden;
    };
    let text = String::from_utf8_lossy(&bytes);
    // Couches rendues par les étapes déjà lues du plan en cours : id -> fichiers de sortie.
    let mut produced: HashMap<String, Vec<String>> = HashMap::new();
    // Ids des étapes déjà lues, pour lire un repère `out:<k>.<n>` (vide pour une ligne illisible).
    let mut prior: Vec<String> = Vec::new();
    for (i, raw) in content_lines(&text).enumerate() {
        let step = i + 1;
        let line = serde_json::from_str::<Value>(raw).ok();
        let id_here = line
            .as_ref()
            .and_then(|l| l.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        prior.push(id_here);
        let earlier = &prior[..i];
        let Some(line) = line else {
            continue;
        };
        let plan_step = line.get("plan_step").and_then(Value::as_u64);
        if plan_step.is_none_or(|n| n <= 1) {
            produced.clear();
        }
        if plan_step.is_none() {
            continue;
        }
        let Some(layer_id) = line.get("layer_id").and_then(Value::as_str) else {
            continue;
        };
        if line.get("error").is_some() || line.get("cancelled").is_some() {
            continue;
        }
        if let Some(Value::Object(params)) = line.get("params") {
            for value in params.values() {
                let consumed = match value {
                    Value::String(s) => Some(s.as_str()),
                    Value::Object(output) => output.get("layer_id").and_then(Value::as_str),
                    _ => None,
                };
                if let Some(files) = consumed.and_then(|id| produced.get(id)) {
                    hidden.extend(files.iter().cloned());
                }
            }
        }
        let Some(id) = line.get("id").and_then(Value::as_str) else {
            continue;
        };
        let mut files: Vec<String> = registry
            .spec(id)
            .map(|spec| {
                let ext = spec.output_ext_for(line.get("params").unwrap_or(&Value::Null));
                spec.output_params
                    .iter()
                    .map(|name| output_file(step, id, &spec.output_params, name, ext))
                    .collect()
            })
            .unwrap_or_default();
        // Une couche ouverte sur la sortie d'une étape antérieure (`layer.open_raster` dictée
        // après un raster du noyau, ticket 4.9, `layer.open_vector` après un GeoJSON, AD-30) est
        // reconnue par ce fichier-là.
        if let (OPEN_RASTER | OPEN_VECTOR, Some(Value::Object(params))) = (id, line.get("params")) {
            files.extend(
                params
                    .values()
                    .filter_map(Value::as_str)
                    .filter_map(|r| reference_file(registry, r, earlier)),
            );
        }
        produced.insert(layer_id.to_string(), files);
    }
    hidden
}

/// Le dernier plan exécuté d'après le journal `journal` (ticket 5.48), pour la vue triée : les
/// fichiers de sortie de ses étapes réussies (`results`, noms de `output_file`, que le rejeu reprend
/// sous `replay/`) et les couches qu'il cite (`cited` : valeurs texte de premier niveau de ses
/// `params`, et `layer_id` d'une sortie entière, hors couches qu'il a produites lui-même).
///
/// Le dernier plan est la suite de lignes à `plan_step` qui part de la dernière ligne à
/// `plan_step` 1 ; l'ouverture dictée d'une sortie (`layer.open_raster`, `layer.open_vector`), qui
/// porte la place de l'étape qu'elle suit, n'en commence pas un. Une ligne hors plan est ignorée.
/// Journal absent ou illisible : plan vide ; une ligne illisible ne compte pas.
pub fn last_plan(registry: &Registry, journal: &Path) -> crate::agent_inventory::LastPlan {
    let mut plan = crate::agent_inventory::LastPlan::default();
    let Ok(bytes) = fs::read(journal) else {
        return plan;
    };
    let text = String::from_utf8_lossy(&bytes);
    let mut lines: Vec<(usize, Value)> = Vec::new();
    for (i, raw) in content_lines(&text).enumerate() {
        let Some(line) = serde_json::from_str::<Value>(raw).ok() else {
            continue;
        };
        let Some(plan_step) = line.get("plan_step").and_then(Value::as_u64) else {
            continue;
        };
        let id = line.get("id").and_then(Value::as_str).unwrap_or_default();
        if plan_step <= 1 && !matches!(id, OPEN_RASTER | OPEN_VECTOR) {
            lines.clear();
        }
        lines.push((i + 1, line));
    }
    let produced: HashSet<&str> = lines
        .iter()
        .filter_map(|(_, l)| l.get("layer_id").and_then(Value::as_str))
        .collect();
    for (step, line) in &lines {
        if let Some(Value::Object(params)) = line.get("params") {
            for value in params.values() {
                let cited = match value {
                    Value::String(s) => Some(s.as_str()),
                    Value::Object(output) => output.get("layer_id").and_then(Value::as_str),
                    _ => None,
                };
                if let Some(id) = cited.filter(|id| !produced.contains(id)) {
                    plan.cited.insert(id.to_string());
                }
            }
        }
        if line.get("error").is_some() || line.get("cancelled").is_some() {
            continue;
        }
        let Some(id) = line.get("id").and_then(Value::as_str) else {
            continue;
        };
        if let Some(spec) = registry.spec(id) {
            let ext = spec.output_ext_for(line.get("params").unwrap_or(&Value::Null));
            plan.results.extend(
                spec.output_params
                    .iter()
                    .map(|name| output_file(*step, id, &spec.output_params, name, ext)),
            );
        }
    }
    plan
}

/// Nom du fichier de sortie désigné par le repère `out:<k>.<n>`, d'après `prior`, les ids des
/// étapes du journal (l'étape 1 en premier) ; `None` si le repère ne désigne aucune sortie. Une
/// commande dont l'extension dépend de ses paramètres (`output_ext_from`, `data.convert`) n'est
/// pas désignable ainsi : `prior` n'en porte pas les paramètres, et deviner l'extension
/// désignerait un fichier qui n'existe pas.
fn reference_file(registry: &Registry, reference: &str, prior: &[String]) -> Option<String> {
    let (k, n) = reference
        .strip_prefix("out:")
        .and_then(|r| r.split_once('.'))
        .filter(|(k, n)| {
            [k, n]
                .iter()
                .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        })
        .and_then(|(k, n)| Some((k.parse::<usize>().ok()?, n.parse::<usize>().ok()?)))?;
    if k == 0 || n == 0 {
        return None;
    }
    let id = prior.get(k - 1)?;
    let spec = registry.spec(id)?;
    if spec.output_ext_by.is_some() {
        return None;
    }
    let outputs = &spec.output_params;
    let name = outputs.get(n - 1)?;
    Some(output_file(k, id, outputs, name, &spec.output_ext))
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
    /// Dossier où l'exécutant écrit ses sorties : le dossier de travail, ou celui du
    /// rejeu (`replay/`, ou un repli `replay-<n>/` si `replay/` ne se vide pas).
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
        // L'extension suit le format demandé quand la commande en a un (`data.convert`) : le
        // noyau nomme le fichier, jamais l'exécutant (AD-10).
        let ext = spec.output_ext_for(&params).to_string();
        let mut logged = params.clone();
        let mut exec = params;
        for name in &spec.output_params {
            let file = output_file(step, id, &spec.output_params, name, &ext);
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
            match name.split_once('.') {
                None => {
                    if let (Some(l), Some(e)) = (logged.get_mut(name), exec.get_mut(name)) {
                        self.resolve_param(l, e, &mut inputs, prior)?;
                    }
                }
                // `liste.champ` : le champ de chaque objet du tableau, dans l'ordre (ticket 11.9).
                Some((list, field)) => {
                    if let (Some(Value::Array(ls)), Some(Value::Array(es))) =
                        (logged.get_mut(list), exec.get_mut(list))
                    {
                        for (l, e) in ls.iter_mut().zip(es.iter_mut()) {
                            if let (Some(l), Some(e)) = (l.get_mut(field), e.get_mut(field)) {
                                self.resolve_param(l, e, &mut inputs, prior)?;
                            }
                        }
                    }
                }
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
        let file = reference_file(self.registry, reference, prior)
            .ok_or_else(|| JournalError::BadReference(reference.to_string()))?;
        Ok(self.out_dir.join(file))
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
/// journal a gardé. N'écrit pas le journal ; ses sorties vont sous `replay/`, ou sous
/// un repli `replay-<n>/` quand `replay/` ne se vide pas (fichier tenu par QGIS).
pub struct Replay<'a> {
    registry: &'a Registry,
    project_dir: PathBuf,
    work_dir: PathBuf,
    replay_dir: PathBuf,
    lines: Vec<Value>,
    ids: Vec<String>,
    warnings: Vec<String>,
    /// Commandes que l'exécutant QGIS du rejeu annonce. Une ligne de mise en forme dont les
    /// gestes n'y sont pas s'arrête sur `style.unavailable` (entrée 11.20). Vide par défaut :
    /// sans QGIS (CLI `replay`), aucune mise en forme ne se rejoue.
    gestures: HashSet<String>,
    /// Formes de `StyleSpec` que l'exécutant du rejeu annonce. Une ligne `layer.set_style` de
    /// forme `result_ramp` s'arrête sur `style.unavailable` si elle n'y est pas.
    style_kinds: HashSet<String>,
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
            gestures: HashSet::new(),
            style_kinds: HashSet::new(),
        })
    }

    /// Dit quelles formes de `StyleSpec` l'exécutant du rejeu sait appliquer.
    pub fn with_style_kinds<I: IntoIterator<Item = String>>(mut self, announced: I) -> Self {
        self.style_kinds = announced.into_iter().collect();
        self
    }

    /// Dit quelles commandes l'exécutant QGIS du rejeu annonce (entrée 11.20).
    pub fn with_gestures<I: IntoIterator<Item = String>>(mut self, announced: I) -> Self {
        self.gestures = announced.into_iter().collect();
        self
    }

    /// Gestes qu'une ligne de mise en forme demande : ceux de la commande, ou la commande dictée
    /// elle-même (`layer.set_style`, `layer.arrange`). Vide pour toute autre ligne.
    fn styling_gestures<'s>(&self, spec: &'s crate::registry::CommandSpec) -> Vec<&'s str> {
        let mut gestures = spec.required_gestures();
        // Une commande dictée par une mise en forme, ou `layer.set_style` que la session dicte
        // après l'ouverture d'un raster de résultat (bogue du fond noir, 2026-11-09).
        let dictated = self.registry.specs().any(|s| s.dictates.contains(&spec.id))
            || (spec.id == SET_STYLE && self.registry.specs().any(|s| s.result_style.is_some()));
        if dictated {
            gestures.push(spec.id.as_str());
        }
        gestures
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

    /// Dossier de sortie du rejeu, vide : `replay/` s'il se vide, sinon le premier
    /// repli `replay-<n>/` (n = 2 à `REPLAY_FALLBACKS + 1`) absent ou vidable. Un
    /// fichier tenu par un autre programme (QGIS sous Windows) n'est ainsi jamais
    /// dans le dossier où le rejeu écrit et relit. Seul le dossier qu'on s'apprête à
    /// utiliser est vidé. Si aucun ne convient : `replay.output_locked`, qui nomme un
    /// fichier resté dans `replay/`.
    fn fresh_replay_dir(&self) -> Result<PathBuf, ErrorReport> {
        let fallbacks =
            (2..=REPLAY_FALLBACKS + 1).map(|n| self.work_dir.join(format!("replay-{n}")));
        let mut last_error = None;
        for dir in std::iter::once(self.replay_dir.clone()).chain(fallbacks) {
            match empty_dir(&dir) {
                Ok(()) => return Ok(dir),
                Err(e) => last_error = Some(e),
            }
        }
        if let Some(file) = first_file(&self.replay_dir) {
            let relative = relative_to(&self.work_dir, &file)
                .map(|r| posix(&r))
                .unwrap_or_else(|| "replay".to_string());
            return Err(JournalError::OutputLocked(relative).into());
        }
        let kind = last_error.map_or(std::io::ErrorKind::Other, |e| e.kind());
        Err(JournalError::Io(kind.to_string()).into())
    }

    /// Ordre de la légende vu par la ligne `i` à l'origine (ids du journal), gardé sous
    /// `replay.legend`. Un journal plus ancien ne le garde pas : à défaut, l'ordre des couches
    /// nommées (`layer_id`) par les ordres dictés qui suivent la ligne, eux-mêmes donnés dans
    /// l'ordre de l'inventaire ; les couches qu'ils ne nomment pas gardent leur place.
    fn legend_at(&self, i: usize, spec: &crate::registry::CommandSpec) -> Vec<String> {
        let ids = |v: &Value| -> Vec<String> {
            v.as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        };
        // Une clé mal formée (pas une liste de textes) compte comme absente.
        let kept = self.lines[i].get("replay").and_then(|r| r.get(LEGEND_KEY));
        if let Some(kept) =
            kept.filter(|k| k.as_array().is_some_and(|a| a.iter().all(Value::is_string)))
        {
            return ids(kept);
        }
        // Seuls les `layer.set_style` dictés avant le rangement : au-delà, ce n'est plus cette
        // mise en forme.
        self.lines[i + 1..]
            .iter()
            .zip(&self.ids[i + 1..])
            .take_while(|(_, id)| *id != "layer.arrange" && spec.dictates.iter().any(|d| d == *id))
            .filter(|(_, id)| *id == "layer.set_style")
            .filter_map(|(line, _)| line.get("params")?.get("layer_id")?.as_str())
            .map(str::to_string)
            .collect()
    }

    /// Réexécute chaque étape par `router`, dans l'ordre ; s'arrête au premier écart.
    ///
    /// `replayed_layers` : ids des couches posées par les rejeux précédents dans la même
    /// session de l'appelant (ensemble vide s'il n'en garde pas la mémoire). Avec les ids du
    /// journal, ils sont passés au paramètre `leave_out_param` d'une commande qui le déclare :
    /// la carte du rejeu ne montre que ce que l'exécution d'origine montrait (ticket 5.36).
    /// Les ids des couches posées par ce rejeu y sont ajoutés, qu'il réussisse ou non.
    pub fn run(
        &self,
        router: &Router,
        replayed_layers: &mut HashSet<String>,
    ) -> Result<ReplayReport, ErrorReport> {
        // Couches d'une exécution précédente : session d'origine (ids du journal) et rejeux
        // antérieurs. Figé avant la première étape : jamais les ids de ce rejeu.
        let leave_out: Vec<Value> = self
            .lines
            .iter()
            .filter_map(|line| line.get("layer_id").and_then(Value::as_str))
            .chain(replayed_layers.iter().map(String::as_str))
            .collect::<BTreeSet<&str>>()
            .into_iter()
            .map(|id| json!(id))
            .collect();
        let leave_out = Value::Array(leave_out);
        // Un rejeu précédent ne doit pas laisser de fichier qu'un autre pourrait reprendre.
        let out_dir = self.fresh_replay_dir()?;
        let resolver = Resolver {
            registry: self.registry,
            project_dir: &self.project_dir,
            out_dir: &out_dir,
        };
        let mut replayed = 0;
        // Ids de couche de la session d'origine -> ids obtenus par ce rejeu. Sert dans les
        // deux sens : ancien -> neuf dans les paramètres, neuf -> ancien dans les résultats
        // (un inventaire liste les couches créées plus tôt), avant de les comparer.
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
            // Une mise en forme sans ses gestes s'arrête net, nommée, à cette étape.
            if !self
                .styling_gestures(spec)
                .iter()
                .all(|g| self.gestures.contains(*g))
            {
                return Err(ErrorReport::from(crate::style::unavailable()).at_step(step));
            }
            // Un style de raster de résultat demande que l'exécutant sache appliquer sa forme.
            let kind = (id == SET_STYLE)
                .then(|| line.get("params")?.get("style")?.get("kind")?.as_str())
                .flatten();
            if kind == Some(RESULT_RAMP_KIND) && !self.style_kinds.contains(RESULT_RAMP_KIND) {
                return Err(ErrorReport::from(crate::style::unavailable()).at_step(step));
            }
            let mut params = line.get("params").cloned().unwrap_or_else(|| json!({}));
            substitute(&mut params, &bound);
            // Après la substitution : les ids du journal ne doivent pas devenir ceux de ce rejeu.
            if let Value::Object(map) = &mut params {
                if let Some(name) = &spec.leave_out_param {
                    map.insert(name.clone(), leave_out.clone());
                }
                // Ce que l'exécution d'origine a décidé (le centre d'un export à l'échelle) est
                // repassé tel que journalisé : le rejeu reproduit, il ne redécide pas (AD-11).
                // Lu sous la clé réservée `replay` de la ligne, qui survit au résumé du résultat.
                if let Some((name, _)) = &spec.replay_param {
                    map.remove(name);
                    if let Some(kept) = line.get("replay").and_then(|r| r.get(name)) {
                        map.insert(name.clone(), kept.clone());
                    }
                }
                // Le paramètre rempli par la commande (M4) : la valeur journalisée sous `replay`
                // remplace celle de la ligne ; sans clé `replay` (journal ancien, ou résultat
                // sans ce champ), les paramètres de la ligne sont rejoués tels quels.
                if let Some((name, _)) = &spec.filled_param {
                    if let Some(kept) = line.get("replay").and_then(|r| r.get(name)) {
                        map.insert(name.clone(), kept.clone());
                    }
                }
            }
            let resolved = resolver
                .resolve_call(id, spec, step, params, &self.ids[..i])
                .map_err(|e| e.at_step(step))?;
            let mut exec = resolved.exec;
            // La ligne d'une mise en forme relit l'inventaire, sans les couches d'une exécution
            // précédente, mais ne dicte rien : ses ordres sont les lignes qui la suivent (11.20).
            // Ses couches y sont remises dans l'ordre de la légende d'origine : un `layer.arrange`
            // déjà joué dans la session QGIS a pu la remanier, et le résultat suit cet ordre.
            if let Some(name) = &spec.inventory_param {
                let inventory =
                    read_inventory(router).map_err(|e| ErrorReport::from(e).at_step(step))?;
                let legend: Vec<String> = self
                    .legend_at(i, spec)
                    .into_iter()
                    .map(|old| bound.get(&old).cloned().unwrap_or(old))
                    .collect();
                let inventory = in_legend_order(without_layers(inventory, &leave_out), &legend);
                if let Value::Object(map) = &mut exec {
                    map.insert(name.clone(), inventory);
                }
            }
            let outcome = router
                .call(id, exec)
                .and_then(|value| take_orders(spec, value).map(|(value, _)| value));
            // La couche est dans le projet même si l'étape diverge : on s'en souvient.
            if let Some(new) = outcome
                .as_ref()
                .ok()
                .filter(|_| !spec.output_none)
                .and_then(|v| v.get("layer_id"))
                .and_then(Value::as_str)
            {
                replayed_layers.insert(new.to_string());
            }
            if let Err(e @ crate::registry::RegistryError::Unavailable(_)) = &outcome {
                return Err(ErrorReport::from(e).at_step(step));
            }
            let diverged = |why: String| ErrorReport::from(JournalError::Diverged { step, why });
            let same = match (&outcome, line.get("result"), line.get("error")) {
                (Ok(value), Some(kept), None) => {
                    // Chaque neuf vient d'une étape distincte : la table inverse est sûre.
                    let back: HashMap<String, String> = bound
                        .iter()
                        .map(|(old, new)| (new.clone(), old.clone()))
                        .collect();
                    let mut again = relativize(value, &out_dir);
                    substitute(&mut again, &back);
                    same_result(&again, kept)
                }
                (Err(e), None, Some(Value::String(code))) => e.code() == code,
                _ => false,
            };
            if !same {
                // Une lecture qui voit une couche d'une exécution précédente : le projet n'est
                // plus dans l'état de son ouverture. On en dit la cause et le remède.
                let stale = outcome
                    .as_ref()
                    .ok()
                    .and_then(|value| earlier_layer(value, &leave_out));
                return Err(diverged(match stale {
                    Some(layer) => format!(
                        "la commande {id} voit la couche « {layer} », posée par une exécution \
                         précédente : le projet n'est plus dans l'état où le journal l'a \
                         trouvé. Rouvrez le projet tel qu'il était avant ces étapes, sans \
                         l'enregistrer, puis rejouez"
                    ),
                    None => format!(
                        "la commande {id} ne redonne pas le même résultat ou la même erreur"
                    ),
                }));
            }
            if let (Ok(value), Some(old), false) = (
                &outcome,
                line.get("layer_id").and_then(Value::as_str),
                spec.output_none,
            ) {
                if let Some(new) = value.get("layer_id").and_then(Value::as_str) {
                    bound.insert(old.to_string(), new.to_string());
                }
            }
            // Une étape QGIS qui redonne la même erreur n'a pas de sortie à exiger : seul son
            // code d'erreur compte. Une commande du noyau garde la comparaison des octets.
            if outcome.is_err() && spec.runs_in == Some(crate::registry::RunsIn::Qgis) {
                continue;
            }
            let ext = spec.output_ext_for(line.get("params").unwrap_or(&Value::Null));
            for name in &spec.output_params {
                let file = output_file(step, id, &spec.output_params, name, ext);
                let again = fs::read(out_dir.join(&file)).ok();
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

/// Premier id de `leave_out` (couches d'une exécution précédente) que nomme le résultat brut
/// `value` d'une étape : la lecture voit une couche qu'aucune étape de ce rejeu n'a posée (les
/// ids de ce rejeu sont neufs). `None` pour une vraie divergence.
fn earlier_layer(value: &Value, leave_out: &Value) -> Option<String> {
    fn strings<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
        match value {
            Value::String(s) => out.push(s),
            Value::Array(items) => items.iter().for_each(|v| strings(v, out)),
            Value::Object(map) => map.values().for_each(|v| strings(v, out)),
            _ => {}
        }
    }
    let earlier: HashSet<&str> = leave_out
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut seen = Vec::new();
    strings(value, &mut seen);
    seen.into_iter()
        .find(|s| earlier.contains(s))
        .map(str::to_string)
}

/// L'inventaire sans les couches dont l'id est dans `leave_out` (un tableau d'ids) : au rejeu, la
/// mise en forme ne voit que ce que l'exécution d'origine voyait (ticket 5.36, entrée 11.20).
fn without_layers(mut inventory: Value, leave_out: &Value) -> Value {
    let gone: HashSet<&str> = leave_out
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if let Some(layers) = inventory.get_mut("layers").and_then(Value::as_array_mut) {
        layers.retain(|l| {
            l.get("layer_id")
                .and_then(Value::as_str)
                .is_none_or(|id| !gone.contains(id))
        });
    }
    inventory
}

/// L'inventaire dont les couches nommées par `legend` (ids de ce rejeu) reprennent, entre elles,
/// l'ordre de `legend`, aux places qu'elles occupent ; les autres ne bougent pas. Seul l'ordre
/// change : une couche en plus ou en moins reste un écart, que la comparaison du résultat voit.
fn in_legend_order(mut inventory: Value, legend: &[String]) -> Value {
    let mut rank: HashMap<&str, usize> = HashMap::new();
    for (k, id) in legend.iter().enumerate() {
        rank.entry(id.as_str()).or_insert(k);
    }
    if let Some(layers) = inventory.get_mut("layers").and_then(Value::as_array_mut) {
        let rank_of = |l: &Value| {
            l.get("layer_id")
                .and_then(Value::as_str)
                .and_then(|id| rank.get(id).copied())
        };
        let slots: Vec<usize> = (0..layers.len())
            .filter(|&k| rank_of(&layers[k]).is_some())
            .collect();
        let mut ranked: Vec<Value> = slots.iter().map(|&k| layers[k].clone()).collect();
        ranked.sort_by_key(|l| rank_of(l));
        for (slot, layer) in slots.into_iter().zip(ranked) {
            layers[slot] = layer;
        }
    }
    inventory
}

/// Nombre de dossiers de repli `replay-<n>/` essayés après `replay/` : un plafond,
/// pour ne jamais boucler sans fin.
const REPLAY_FALLBACKS: usize = 20;

/// Vide `dir` (ou le crée) et vérifie qu'il ne reste rien dedans.
fn empty_dir(dir: &Path) -> std::io::Result<()> {
    // Absent (NotFound) ou suppression partielle : le contrôle de vide qui suit tranche.
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir)?;
    if fs::read_dir(dir)?.next().is_some() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::DirectoryNotEmpty,
            "dossier de sortie non vide",
        ));
    }
    Ok(())
}

/// Premier fichier trouvé sous `dir`, en descendant dans les vrais sous-dossiers
/// (les liens symboliques ne sont pas suivis, mais un lien compte comme fichier) ;
/// `None` s'il n'y en a aucun.
fn first_file(dir: &Path) -> Option<PathBuf> {
    let mut entries: Vec<(PathBuf, bool)> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| {
            let e = e.ok()?;
            Some((e.path(), e.file_type().ok()?.is_dir()))
        })
        .collect();
    entries.sort();
    entries.into_iter().find_map(|(path, is_dir)| {
        if is_dir {
            first_file(&path)
        } else {
            Some(path)
        }
    })
}

/// Remplace, dans toutes les chaînes entières de `value`, une clé de `bound` par sa valeur
/// (ancien id de couche par le neuf, ou l'inverse avec la table retournée).
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
