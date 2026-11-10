//! Plans de l'agent sur le canal (AD-5, AD-23) : `plan.propose`, `plan.execute`,
//! `plan.reject`, la notification `plan.step`, et la liste des modèles `agent.models`.
//!
//! Le noyau garde le plan : l'extension ne renvoie que son `plan_id`, si bien que le plan
//! montré est le plan exécuté. Un seul plan attend à la fois ; une nouvelle proposition
//! l'efface, `plan.execute` et `plan.reject` le consomment. Chaque étape passe par
//! `Session::call` (journal, chemins, annulation), une à la fois, les références `step:k`
//! étant résolues depuis les sorties déjà obtenues : le journal garde les valeurs résolues,
//! sauf un raster (`RasterPath`), dont `step:k` devient le repère `out:<ligne du journal>.1`.
//! Avant d'être gardé, le plan du modèle est complété par les conversions de points
//! (`plan_completion`, ticket 11.22) : seul `plan.propose` le fait, jamais `command.run`.

use std::path::Path;

use serde_json::{json, Value};

use super::{invalid_params, is_available, Channel, RpcError, State};
use crate::agent_inventory::{
    agent_inventory, sorted_view_learned, LastPlan, ModelInventory, ShortIds, ViewLimits, WorkFile,
};
use crate::dem::epsg_of;
use crate::error::ErrorReport;
use crate::exec::Executor;
use crate::journal::{intermediate_files, last_plan};
use crate::plan::{raster_markers, resolve_references, Plan};
use crate::plan_completion::{complete_conversions, TRANSFORM_POINT};
use crate::registry::{Registry, RegistryError, RunsIn};

/// Ce qui compose un plan depuis une demande en langage naturel (port du noyau ; son
/// implémentation par le modèle local vit hors de `core`, qui ne dépend pas de l'IA).
pub trait Composer {
    /// Compose le plan de `request`. `inventory` demande l'inventaire du projet à
    /// l'extension (`project.inventory`, hors journal) et le rend tel que le modèle doit le voir
    /// (`ModelInventory` : forme apprise, ordre stable, identifiants courts, budget) : à appeler
    /// seulement une fois prêt à composer, pour qu'un modèle absent ne coûte aucun ordre. Les
    /// identifiants du plan rendu sont retraduits par le noyau (ticket 5.19). Le plan rendu est
    /// vérifié ensuite par le noyau ; un refus du modèle se rend en erreur `agent.refused`, une
    /// question (`Plan::Question`) en résultat `{"question", "source"}`, sans `plan_id` : elle n'est
    /// jamais gardée, donc jamais exécutable. `model` est un nom de la liste de `models`
    /// (comparé tel quel) ; absent, le premier modèle sans problème.
    fn compose(
        &self,
        registry: &Registry,
        request: &str,
        model: Option<&str>,
        inventory: &mut dyn FnMut() -> Result<ModelInventory, ErrorReport>,
    ) -> Result<Plan, ErrorReport>;

    /// Les modèles que l'utilisateur peut choisir, triés par nom : chacun
    /// `{"file", "size_bytes"}` et un `problem` facultatif. Ni dossier ni chemin (AD-9).
    /// Écart assumé à AD-15 (2026-10-18) : rendu hors registre, jamais journalisé.
    fn models(&self, registry: &Registry) -> Result<Vec<Value>, ErrorReport>;
}

fn unavailable() -> RpcError {
    RpcError::coded(
        "agent.unavailable",
        "L'agent n'est pas disponible dans ce noyau : aucune composition de plan n'y est branchée."
            .to_string(),
    )
}

/// `agent.models` : les modèles du dossier des modèles, sans chemin, hors journal.
pub(super) fn models(state: &State<'_>) -> Result<Value, RpcError> {
    let Some(composer) = state.composer else {
        return Err(unavailable());
    };
    let models = composer
        .models(state.registry)
        .map_err(RpcError::business)?;
    Ok(json!({"models": models}))
}

/// Le plan en attente de validation.
pub(super) struct Pending {
    id: String,
    plan: Plan,
}

fn plan_unknown() -> RpcError {
    RpcError::coded(
        "plan.unknown",
        "Aucun plan en attente ne porte cet identifiant : rien n'est exécuté.".to_string(),
    )
}

fn plan_id(params: Option<&Value>) -> Result<&str, RpcError> {
    params
        .and_then(|p| p.get("plan_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("plan_id doit être un identifiant."))
}

fn plan_invalid(step: usize, message: String) -> ErrorReport {
    ErrorReport {
        code: "agent.plan_invalid".to_string(),
        message,
        step: Some(step),
    }
}

/// Vérifie un plan : `check_plan`, puis la disponibilité de chaque commande (AD-7).
/// Rien n'est réparé.
fn verify(state: &State<'_>, plan: &Plan) -> Result<(), ErrorReport> {
    state
        .registry
        .check_plan(plan)
        .map_err(|e| plan_invalid(e.step, e.message))?;
    let Plan::Calls { calls } = plan else {
        return Ok(());
    };
    for (index, call) in calls.iter().enumerate() {
        let runs_in = state.registry.spec(&call.command).and_then(|s| s.runs_in);
        if !is_available(state, &call.command, runs_in) {
            return Err(plan_invalid(
                index + 1,
                format!(
                    "La commande « {} » n'est pas disponible : l'extension ne l'a pas annoncée.",
                    call.command
                ),
            ));
        }
    }
    Ok(())
}

/// `plan.propose {request, model?}` : inventaire, composition, vérification ; le plan est gardé.
/// Une question (du modèle ou du garde-fou) est rendue telle quelle (`{"question", "source"}`)
/// et rien n'attend.
pub(super) fn propose(state: &mut State<'_>, params: Option<&Value>) -> Result<Value, RpcError> {
    // Une nouvelle proposition remplace l'ancienne, même si elle échoue (paramètres compris).
    state.pending = None;
    let request = params
        .and_then(|p| p.get("request"))
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("request doit être un texte."))?;
    let model = match params.and_then(|p| p.get("model")) {
        None | Some(Value::Null) => None,
        Some(Value::String(name)) => Some(name.as_str()),
        Some(_) => return Err(invalid_params("model doit être un nom de modèle.")),
    };
    let Some(composer) = state.composer else {
        return Err(unavailable());
    };
    let work_dir = state.session.as_ref().map(|s| s.work_dir().to_path_buf());
    let journal = state.session.as_ref().map(|s| s.journal_path());
    let registry = state.registry;
    // L'inventaire lu pour la première composition, gardé pour la redemande élargie : ni second
    // ordre à l'extension, ni autre état du projet entre les deux vues.
    let mut read: Option<Read> = None;
    // La dernière vue montrée au modèle : sa table d'identifiants courts retraduit le plan
    // (ticket 5.19), ses couches réduites décident de la redemande (5.48), ses couches de résultat
    // laissées hors de la vue sont rendues à l'utilisateur (5.33).
    let mut shown: Option<(ModelInventory, ShortIds)> = None;
    let first = {
        let link = state.link.clone();
        let announced = &state.announced;
        let read = &mut read;
        let shown = &mut shown;
        let mut inventory = || {
            if !announced.contains("project.inventory") {
                return Err(ErrorReport::from(RegistryError::Unavailable(
                    "project.inventory".to_string(),
                )));
            }
            // Directement par le canal, hors `Session` : l'inventaire n'est pas journalisé.
            // Le modèle voit la forme de son entraînement, traduite ici (ticket 5.10, t. 4a),
            // rangée dans un ordre stable aux identifiants courts (ticket 5.19), bornée au budget
            // d'inventaire (ticket 5.18) par paquets (5.19) : les couches de résultat les plus
            // anciennes sortent de sa vue, jamais du projet. Les fichiers sont lus après la
            // réponse de l'extension, au plus près de l'inventaire. Les couches intermédiaires des
            // plans exécutés sortent de sa vue, et les résultats y perdent leurs valeurs (5.22) ; elles
            // sont déduites du journal du projet, relu à chaque proposition, et reconnues par leur
            // fichier : la règle tient après une relance du noyau ou un rejeu (5.34). Au-dessus du
            // seuil, la vue est triée par la demande et le dernier plan exécuté, lu lui aussi dans
            // le journal (5.46, 5.48).
            let raw = Channel { link: link.clone() }
                .execute("project.inventory", json!({}))
                .map_err(ErrorReport::from)?;
            let now = Read {
                learned: agent_inventory(&raw),
                files: work_dir.as_deref().map(work_files).unwrap_or_default(),
                intermediates: journal
                    .as_deref()
                    .map(|j| intermediate_files(registry, j))
                    .unwrap_or_default(),
                last: journal
                    .as_deref()
                    .map(|j| last_plan(registry, j))
                    .unwrap_or_default(),
            };
            let view = now.view(request, &[]);
            *read = Some(now);
            let model_view = view.0.clone();
            *shown = Some(view);
            Ok(model_view)
        };
        composer
            .compose(state.registry, request, model, &mut inventory)
            .map_err(RpcError::business)?
    };
    // Redemande élargie (ticket 5.48, AD-23) : un plan qui fait porter un paramètre de champ sur
    // une couche montrée réduite est recomposé une fois, la même demande avec ces couches
    // détaillées. Ce n'est pas une réparation : le plan rendu est jugé comme le premier, et rien
    // n'est recomposé une seconde fois.
    let wanted = match (&shown, &first) {
        (Some((view, ids)), Plan::Calls { calls }) => {
            reduced_with_fields(&view.inventory, ids, calls)
        }
        _ => Vec::new(),
    };
    let mut plan = first;
    if let (false, Some(now)) = (wanted.is_empty(), read.as_ref()) {
        let shown = &mut shown;
        let mut inventory = || {
            let view = now.view(request, &wanted);
            let model_view = view.0.clone();
            *shown = Some(view);
            Ok(model_view)
        };
        plan = composer
            .compose(state.registry, request, model, &mut inventory)
            .map_err(RpcError::business)?;
    }
    let (short_ids, left_out) = match shown {
        Some((view, ids)) => (ids, view.left_out),
        None => (ShortIds::default(), Vec::new()),
    };
    // Un refus ou une question qui cite un identifiant court le montre avec celui de QGIS.
    if let Plan::Refusal { reason } = &plan {
        let reason = short_ids.restore_text(reason);
        return Err(RpcError::coded(
            "agent.refused",
            format!("L'agent ne peut pas répondre à cette demande : {reason}"),
        ));
    }
    // Une question est une réplique, pas un plan : rien n'est gardé, aucun `plan_id`.
    // Sa provenance est toujours explicite : `model`, ou `guard` (AD-23, coordonnées).
    if let Plan::Question { question, source } = &plan {
        let question = short_ids.restore_text(question);
        let mut answer = json!({"question": question, "source": source.as_str()});
        with_left_out(&mut answer, left_out);
        return Ok(answer);
    }
    // Les identifiants courts du plan redeviennent ceux de QGIS, avant toute vérification : le
    // plan vérifié, gardé, montré et journalisé ne porte que les vrais identifiants.
    if let Plan::Calls { calls } = &mut plan {
        for call in calls {
            short_ids.restore(&mut call.params);
        }
    }
    verify(state, &plan).map_err(RpcError::business)?;
    // Le plan vérifié du modèle, garde-fou d'AD-23 compris, est complété par les conversions de
    // points (amendement d'AD-19, ticket 11.22) si l'extension annonce le geste, puis revérifié :
    // le plan complété est gardé, montré, exécuté et journalisé comme un plan ordinaire.
    if is_available(state, TRANSFORM_POINT, Some(RunsIn::Qgis)) {
        let session = state.session.as_ref();
        // Le système du raster est lu dans l'en-tête du GeoTIFF, par la règle de chemin d'un
        // appel (AD-9, AD-20) ; illisible ou introuvable, il est inconnu et rien n'est ajouté.
        let raster_epsg = |raw: &str| {
            session
                .and_then(|s| s.input_path(raw))
                .and_then(|path| epsg_of(&path).ok())
        };
        plan = complete_conversions(state.registry, plan, &raster_epsg);
        verify(state, &plan).map_err(RpcError::business)?;
    }
    state.next_plan += 1;
    let id = format!("plan-{}", state.next_plan);
    let shown = serde_json::to_value(&plan).expect("un plan se sérialise");
    state.pending = Some(Pending {
        id: id.clone(),
        plan,
    });
    let mut answer = json!({"plan_id": id, "plan": shown});
    with_left_out(&mut answer, left_out);
    Ok(answer)
}

/// Ce que la vue du modèle lit d'un état du projet : l'inventaire dans la forme apprise, les
/// fichiers du dossier de travail, les intermédiaires et le dernier plan déduits du journal.
struct Read {
    learned: Value,
    files: Vec<WorkFile>,
    intermediates: std::collections::HashSet<String>,
    last: LastPlan,
}

impl Read {
    /// La vue de `request`, `wanted` (identifiants QGIS) détaillées en plus.
    fn view(&self, request: &str, wanted: &[String]) -> (ModelInventory, ShortIds) {
        sorted_view_learned(
            self.learned.clone(),
            &self.files,
            &self.intermediates,
            &self.last,
            request,
            wanted,
            &ViewLimits::default(),
        )
    }
}

/// Les couches montrées réduites (dans `index`, pas dans `layers`) sur lesquelles un appel du plan
/// fait porter un paramètre de champ (`field`, `fields`, ou une clé finie par `_field`), en
/// identifiants QGIS, dans l'ordre du plan, sans doublon. La couche d'un appel est son `layer_id` ;
/// `step:k` et `step:k.layer_id` mènent à celle de l'étape k, dont la sortie porte les champs.
/// Une vue sans `index` n'a aucune couche réduite. Le plan est celui du modèle, aux identifiants
/// courts.
fn reduced_with_fields(
    view: &Value,
    ids: &ShortIds,
    calls: &[crate::plan::PlannedCall],
) -> Vec<String> {
    let listed = |key: &str| -> std::collections::HashSet<&str> {
        view.get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|l| l.get("layer_id").and_then(Value::as_str))
            .collect()
    };
    let (index, detailed) = (listed("index"), listed("layers"));
    // La couche d'entrée de l'étape `step` (à partir de 1), en suivant les `step:k`.
    let layer_of = |step: usize| -> Option<&str> {
        let mut at = step;
        for _ in 0..calls.len() {
            let id = calls
                .get(at.checked_sub(1)?)?
                .params
                .get("layer_id")?
                .as_str()?;
            match id.strip_prefix("step:") {
                Some(rest) => at = rest.split('.').next()?.parse().ok()?,
                None => return Some(id),
            }
        }
        None
    };
    let mut wanted: Vec<String> = Vec::new();
    for (i, call) in calls.iter().enumerate() {
        let has_field = call.params.as_object().is_some_and(|m| {
            m.iter().any(|(k, v)| {
                (k == "field" || k == "fields" || k.ends_with("_field"))
                    && !v.is_null()
                    && v.as_str() != Some("")
            })
        });
        let Some(short) = layer_of(i + 1).filter(|_| has_field) else {
            continue;
        };
        if index.contains(short) && !detailed.contains(short) {
            let mut id = Value::String(short.to_string());
            ids.restore(&mut id);
            if let Some(real) = id.as_str().map(str::to_string) {
                if !wanted.contains(&real) {
                    wanted.push(real);
                }
            }
        }
    }
    wanted
}

/// Ajoute à la réponse de `plan.propose` la clé `left_out` (ticket 5.33), seulement si le budget a
/// retiré au moins une couche : sous le budget, la réponse reste celle d'avant.
fn with_left_out(answer: &mut Value, left_out: Vec<Value>) {
    if !left_out.is_empty() {
        answer["left_out"] = Value::Array(left_out);
    }
}

/// Les fichiers du dossier de travail et de ses dossiers de rejeu (`replay/`, où un rejeu écrit
/// ses résultats, et ses replis `replay-<n>/` quand `replay/` est tenu par QGIS), avec leur
/// date. Un dossier illisible ou absent n'en donne aucun (rien n'est alors reconnu comme
/// résultat, rien n'est retiré) ; un fichier illisible est ignoré.
fn work_files(dir: &Path) -> Vec<WorkFile> {
    let mut files = files_in(dir);
    files.extend(files_in(&dir.join("replay")));
    let fallbacks = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|entry| {
            entry.file_name().to_str().is_some_and(|name| {
                name.strip_prefix("replay-")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
        });
    for entry in fallbacks {
        files.extend(files_in(&entry.path()));
    }
    files
}

/// Les fichiers d'un dossier (sans ses sous-dossiers), avec leur date.
fn files_in(dir: &Path) -> Vec<WorkFile> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let meta = entry.metadata().ok().filter(|m| m.is_file())?;
            Some(WorkFile {
                name: entry.file_name().into_string().ok()?,
                modified: meta.modified().ok()?,
            })
        })
        .collect()
}

/// `plan.reject {plan_id}` : oublie le plan en attente.
pub(super) fn reject(state: &mut State<'_>, params: Option<&Value>) -> Result<Value, RpcError> {
    let id = plan_id(params)?;
    match state.pending.take() {
        Some(p) if p.id == id => Ok(json!({"plan_id": id, "rejected": true})),
        other => {
            state.pending = other;
            Err(plan_unknown())
        }
    }
}

/// `plan.execute {plan_id}` : la validation de l'utilisateur. Le plan gardé est revérifié,
/// puis exécuté une étape à la fois ; la première erreur ou annulation arrête tout.
pub(super) fn execute(state: &mut State<'_>, params: Option<&Value>) -> Result<Value, RpcError> {
    let id = plan_id(params)?;
    let pending = match state.pending.take() {
        Some(p) if p.id == id => p,
        other => {
            state.pending = other;
            return Err(plan_unknown());
        }
    };
    verify(state, &pending.plan).map_err(RpcError::business)?;
    let Plan::Calls { calls } = &pending.plan else {
        return Err(plan_unknown());
    };
    let mut outputs: Vec<Value> = Vec::new();
    // Ligne du journal de chaque étape déjà exécutée du plan : un raster cité par `step:k` y est
    // journalisé par le repère de cette ligne, jamais par son chemin (amendement d'AD-24).
    let mut journal_steps: Vec<usize> = Vec::new();
    let mut steps = Vec::new();
    for (index, call) in calls.iter().enumerate() {
        let step = index + 1;
        let marked = match state.registry.spec(&call.command) {
            Some(spec) => raster_markers(spec, &call.params, &journal_steps),
            None => call.params.clone(),
        };
        let params = resolve_references(&marked, &outputs, step)
            .map_err(|e| RpcError::business(ErrorReport::from(e)))?;
        let notice = json!({"jsonrpc": "2.0", "method": "plan.step",
            "params": {"plan_id": pending.id, "step": step, "command": call.command}});
        if !state.link.send_or_fail(&notice) {
            return Err(RpcError::business(
                ErrorReport {
                    code: "session.io".to_string(),
                    message: "Le canal est interrompu : l'étape n'est pas lancée.".to_string(),
                    step: None,
                }
                .at_step(step),
            ));
        }
        let session = state.session.as_mut().expect("session vérifiée");
        journal_steps.push(session.next_step());
        // La ligne porte sa place dans le plan : le journal suffit à retrouver les couches
        // intermédiaires (`intermediate_files`, ticket 5.34).
        let value = session
            .call_in_plan(&call.command, params, step)
            .map_err(|e| RpcError::business(e.at_step(step)))?;
        // Résultat brut, chemins absolus compris, vers l'extension (AD-9, 2026-11-04). Un raster
        // du noyau, lui, est ouvert par l'étape `layer.open_raster` que la session dicte
        // (ticket 4.9), un GeoJSON du noyau par `layer.open_vector` (AD-30). Le journal reste
        // relatif.
        steps.push(json!({"step": step, "command": call.command, "result": value.clone()}));
        outputs.push(value);
    }
    Ok(json!({"plan_id": pending.id, "steps": steps}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_files_include_the_replay_folder() {
        let dir = std::env::temp_dir().join(format!("cassini-work-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // Dossier absent : aucun fichier.
        assert!(work_files(&dir).is_empty());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("3-analysis.buffer.gpkg"), b"x").unwrap();
        // Sans `replay/` : les fichiers du dossier de travail seuls.
        let names = |d: &Path| {
            let mut n: Vec<String> = work_files(d).into_iter().map(|f| f.name).collect();
            n.sort();
            n
        };
        assert_eq!(names(&dir), ["3-analysis.buffer.gpkg"]);
        std::fs::create_dir_all(dir.join("replay")).unwrap();
        std::fs::write(dir.join("replay").join("1-analysis.intersect.gpkg"), b"x").unwrap();
        assert_eq!(
            names(&dir),
            ["1-analysis.intersect.gpkg", "3-analysis.buffer.gpkg"]
        );
        // Repli d'un rejeu dont `replay/` était tenu ; un dossier au nom voisin est ignoré.
        std::fs::create_dir_all(dir.join("replay-2")).unwrap();
        std::fs::write(dir.join("replay-2").join("2-analysis.buffer.gpkg"), b"x").unwrap();
        std::fs::create_dir_all(dir.join("replay-x")).unwrap();
        std::fs::write(dir.join("replay-x").join("9-autre.gpkg"), b"x").unwrap();
        assert_eq!(
            names(&dir),
            [
                "1-analysis.intersect.gpkg",
                "2-analysis.buffer.gpkg",
                "3-analysis.buffer.gpkg"
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
