//! Plans de l'agent sur le canal (AD-5, AD-23) : `plan.propose`, `plan.execute`,
//! `plan.reject`, la notification `plan.step`, et la liste des modèles `agent.models`.
//!
//! Le noyau garde le plan : l'extension ne renvoie que son `plan_id`, si bien que le plan
//! montré est le plan exécuté. Un seul plan attend à la fois ; une nouvelle proposition
//! l'efface, `plan.execute` et `plan.reject` le consomment. Chaque étape passe par
//! `Session::call` (journal, chemins, annulation), une à la fois, les références `step:k`
//! étant résolues depuis les sorties déjà obtenues : le journal garde les valeurs résolues.

use std::path::Path;

use serde_json::{json, Value};

use super::{invalid_params, is_available, Channel, RpcError, State};
use crate::agent_inventory::{
    model_inventory, ModelInventory, ShortIds, WorkFile, INVENTORY_BUDGET_CHARS,
    INVENTORY_LOW_CHARS,
};
use crate::error::ErrorReport;
use crate::exec::Executor;
use crate::journal::relativize;
use crate::plan::{parse_reference, resolve_references, Plan};
use crate::registry::{Registry, RegistryError};

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
    // La table des identifiants courts montrés au modèle, pour retraduire son plan (ticket 5.19).
    let mut short_ids = ShortIds::default();
    // Les couches de résultat que le budget a laissées hors de la vue du modèle (ticket 5.33) ;
    // le dernier inventaire lu fait foi, comme pour la table.
    let mut left_out: Vec<Value> = Vec::new();
    let mut plan = {
        let short_ids = &mut short_ids;
        let left_out = &mut left_out;
        let link = state.link.clone();
        let announced = &state.announced;
        let intermediates = &state.intermediates;
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
            // plans exécutés sortent de sa vue, et les résultats y perdent leurs valeurs (5.22).
            let raw = Channel { link: link.clone() }
                .execute("project.inventory", json!({}))
                .map_err(ErrorReport::from)?;
            let files = work_dir.as_deref().map(work_files).unwrap_or_default();
            let (view, ids) = model_inventory(
                &raw,
                &files,
                intermediates,
                INVENTORY_BUDGET_CHARS,
                INVENTORY_LOW_CHARS,
            );
            *short_ids = ids;
            left_out.clone_from(&view.left_out);
            Ok(view)
        };
        composer
            .compose(state.registry, request, model, &mut inventory)
            .map_err(RpcError::business)?
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
    let mut steps = Vec::new();
    for (index, call) in calls.iter().enumerate() {
        let step = index + 1;
        let params = resolve_references(&call.params, &outputs, step)
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
        let value = session
            .call(&call.command, params)
            .map_err(|e| RpcError::business(e.at_step(step)))?;
        // L'étape a abouti et a produit une couche : chaque couche qu'elle a consommée (`step:k`)
        // devient intermédiaire. Une étape sans couche (description, PDF) ne cache rien.
        if value.get("layer_id").and_then(Value::as_str).is_some() {
            state
                .intermediates
                .extend(consumed_layers(&call.params, &outputs));
        }
        let shown = relativize(&value, session.work_dir());
        steps.push(json!({"step": step, "command": call.command, "result": shown}));
        outputs.push(value);
    }
    Ok(json!({"plan_id": pending.id, "steps": steps}))
}

/// Les identifiants des couches de sortie que les paramètres non résolus `params` consomment :
/// pour chaque référence de premier niveau `step:k` ou `step:k.layer_id`, le `layer_id` de la
/// sortie de l'étape `k`, s'il en a un (ticket 5.22). Une référence à un autre champ
/// (`step:k.feature_count`) ne consomme pas la couche.
fn consumed_layers(params: &Value, outputs: &[Value]) -> Vec<String> {
    let Value::Object(map) = params else {
        return Vec::new();
    };
    map.values()
        .filter_map(|v| parse_reference(v)?.ok())
        .filter(|(_, field)| field.as_deref().is_none_or(|f| f == "layer_id"))
        .filter_map(|(k, _)| outputs.get(k.checked_sub(1)?)?.get("layer_id")?.as_str())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_outputs_a_step_refers_to_are_consumed() {
        let outputs = [
            json!({"layer_id": "r1", "feature_count": 3}),
            json!({"layer_id": "r2"}),
            json!({"path": "carte.pdf"}),
        ];
        let params = json!({"layer_id": "step:1.layer_id", "overlay_layer_id": "step:2",
            "pdf": "step:3", "bad": "step:0", "far": "step:9", "plain": "r2", "list": ["step:2"]});
        let mut got = consumed_layers(&params, &outputs);
        got.sort();
        assert_eq!(got, ["r1", "r2"]);
        // Un champ scalaire de la sortie ne consomme pas la couche.
        let scalar = json!({"count": "step:1.feature_count", "name": "step:2.name"});
        assert!(consumed_layers(&scalar, &outputs).is_empty());
        assert!(consumed_layers(&json!({"layer_id": "r1"}), &outputs).is_empty());
        assert!(consumed_layers(&json!(null), &outputs).is_empty());
    }

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
