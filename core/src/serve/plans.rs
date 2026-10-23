//! Plans de l'agent sur le canal (AD-5, AD-23) : `plan.propose`, `plan.execute`,
//! `plan.reject`, et la notification `plan.step`.
//!
//! Le noyau garde le plan : l'extension ne renvoie que son `plan_id`, si bien que le plan
//! montré est le plan exécuté. Un seul plan attend à la fois ; une nouvelle proposition
//! l'efface, `plan.execute` et `plan.reject` le consomment. Chaque étape passe par
//! `Session::call` (journal, chemins, annulation), une à la fois, les références `step:k`
//! étant résolues depuis les sorties déjà obtenues : le journal garde les valeurs résolues.

use serde_json::{json, Value};

use super::{invalid_params, is_available, Channel, RpcError, State};
use crate::agent_inventory::agent_inventory;
use crate::error::ErrorReport;
use crate::exec::Executor;
use crate::journal::relativize;
use crate::plan::{resolve_references, Plan};
use crate::registry::{Registry, RegistryError};

/// Ce qui compose un plan depuis une demande en langage naturel (port du noyau ; son
/// implémentation par le modèle local vit hors de `core`, qui ne dépend pas de l'IA).
pub trait Composer {
    /// Compose le plan de `request`. `inventory` demande l'inventaire du projet à
    /// l'extension (`project.inventory`, hors journal) : à appeler seulement une fois prêt à
    /// composer, pour qu'un modèle absent ne coûte aucun ordre. Le plan rendu est vérifié
    /// ensuite par le noyau ; un refus du modèle se rend en erreur `agent.refused`, une
    /// question (`Plan::Question`) en résultat `{"question"}`, sans `plan_id` : elle n'est
    /// jamais gardée, donc jamais exécutable.
    fn compose(
        &self,
        registry: &Registry,
        request: &str,
        inventory: &mut dyn FnMut() -> Result<Value, ErrorReport>,
    ) -> Result<Plan, ErrorReport>;
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

/// `plan.propose {request}` : inventaire, composition, vérification ; le plan est gardé.
/// Une question du modèle est rendue telle quelle (`{"question"}`) et rien n'attend.
pub(super) fn propose(state: &mut State<'_>, params: Option<&Value>) -> Result<Value, RpcError> {
    // Une nouvelle proposition remplace l'ancienne, même si elle échoue (paramètres compris).
    state.pending = None;
    let request = params
        .and_then(|p| p.get("request"))
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("request doit être un texte."))?;
    let Some(composer) = state.composer else {
        return Err(RpcError::coded(
            "agent.unavailable",
            "L'agent n'est pas disponible dans ce noyau : aucune composition de plan n'y est branchée."
                .to_string(),
        ));
    };
    let plan = {
        let link = state.link.clone();
        let announced = &state.announced;
        let mut inventory = || {
            if !announced.contains("project.inventory") {
                return Err(ErrorReport::from(RegistryError::Unavailable(
                    "project.inventory".to_string(),
                )));
            }
            // Directement par le canal, hors `Session` : l'inventaire n'est pas journalisé.
            // Le modèle voit la forme de son entraînement, traduite ici (ticket 5.10, t. 4a).
            Channel { link: link.clone() }
                .execute("project.inventory", json!({}))
                .map(|raw| agent_inventory(&raw))
                .map_err(ErrorReport::from)
        };
        composer
            .compose(state.registry, request, &mut inventory)
            .map_err(RpcError::business)?
    };
    if let Plan::Refusal { reason } = &plan {
        return Err(RpcError::coded(
            "agent.refused",
            format!("L'agent ne peut pas répondre à cette demande : {reason}"),
        ));
    }
    // Une question est une réplique, pas un plan : rien n'est gardé, aucun `plan_id`.
    if let Plan::Question { question } = &plan {
        return Ok(json!({"question": question}));
    }
    verify(state, &plan).map_err(RpcError::business)?;
    state.next_plan += 1;
    let id = format!("plan-{}", state.next_plan);
    let shown = serde_json::to_value(&plan).expect("un plan se sérialise");
    state.pending = Some(Pending {
        id: id.clone(),
        plan,
    });
    Ok(json!({"plan_id": id, "plan": shown}))
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
        let shown = relativize(&value, session.work_dir());
        steps.push(json!({"step": step, "command": call.command, "result": shown}));
        outputs.push(value);
    }
    Ok(json!({"plan_id": pending.id, "steps": steps}))
}
