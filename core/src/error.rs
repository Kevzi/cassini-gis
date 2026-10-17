//! Forme unique des erreurs rendues aux appelants (AD-21) : `{code, message, step?}`.

use serde::Serialize;

use crate::journal::JournalError;
use crate::plan::PlanError;
use crate::registry::RegistryError;

/// Erreur telle qu'elle sort du noyau : code stable `domaine.cause`, message en
/// français, et numéro d'étape (à partir de 1) quand l'erreur vient d'un plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorReport {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<usize>,
}

impl ErrorReport {
    /// Rattache l'erreur à une étape d'un plan.
    pub fn at_step(mut self, step: usize) -> Self {
        self.step = Some(step);
        self
    }
}

impl From<&RegistryError> for ErrorReport {
    fn from(e: &RegistryError) -> Self {
        ErrorReport {
            code: e.code().to_string(),
            message: e.to_string(),
            step: None,
        }
    }
}

impl From<RegistryError> for ErrorReport {
    fn from(e: RegistryError) -> Self {
        ErrorReport::from(&e)
    }
}

impl From<&PlanError> for ErrorReport {
    fn from(e: &PlanError) -> Self {
        ErrorReport {
            code: e.code.to_string(),
            message: e.message.clone(),
            step: Some(e.step),
        }
    }
}

impl From<PlanError> for ErrorReport {
    fn from(e: PlanError) -> Self {
        ErrorReport::from(&e)
    }
}

impl From<&JournalError> for ErrorReport {
    fn from(e: &JournalError) -> Self {
        ErrorReport {
            code: e.code().to_string(),
            message: e.to_string(),
            step: match e {
                JournalError::Diverged { step, .. } | JournalError::UnreadableLine { step } => {
                    Some(*step)
                }
                _ => None,
            },
        }
    }
}

impl From<JournalError> for ErrorReport {
    fn from(e: JournalError) -> Self {
        ErrorReport::from(&e)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_registry_error_has_no_step_in_its_json() {
        let report = ErrorReport::from(RegistryError::UnknownCommand("a.b".into()));
        let v = serde_json::to_value(&report).unwrap();
        assert_eq!(v["code"], "registry.unknown_command");
        assert!(v.get("step").is_none());
        assert!(v["message"].as_str().unwrap().contains("a.b"));
    }

    #[test]
    fn a_step_is_rendered_from_one() {
        let report = ErrorReport::from(RegistryError::InvalidParams("x".into())).at_step(2);
        assert_eq!(serde_json::to_value(&report).unwrap()["step"], json!(2));
    }
}
