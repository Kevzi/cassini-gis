//! Exécutants : qui fait réellement tourner une commande, selon `runs_in`.
//!
//! Le routeur valide les paramètres par les types du registre, puis confie
//! l'appel à l'exécutant du bon côté. QGIS n'est pas branché ici : `QgisStub`
//! répond `registry.unavailable` en nommant la commande.

use serde_json::Value;

use crate::registry::{Registry, RegistryError, RunsIn};

/// Quelqu'un qui sait exécuter une commande dont les paramètres sont déjà valides.
pub trait Executor {
    fn execute(&self, id: &str, params: Value) -> Result<Value, RegistryError>;
}

/// Exécute les commandes Rust du registre, dans le processus.
pub struct InProcess<'a> {
    registry: &'a Registry,
}

impl<'a> InProcess<'a> {
    pub fn new(registry: &'a Registry) -> Self {
        InProcess { registry }
    }
}

impl Executor for InProcess<'_> {
    fn execute(&self, id: &str, params: Value) -> Result<Value, RegistryError> {
        self.registry.call(id, params)
    }
}

/// Bouchon tenant la place de QGIS : il n'exécute rien.
pub struct QgisStub;

impl Executor for QgisStub {
    fn execute(&self, id: &str, _params: Value) -> Result<Value, RegistryError> {
        Err(RegistryError::Unavailable(id.to_string()))
    }
}

/// Aiguille un appel : id connu, paramètres valides, puis exécutant selon `runs_in`.
pub struct Router<'a> {
    registry: &'a Registry,
    core: InProcess<'a>,
    qgis: Box<dyn Executor + 'a>,
}

impl<'a> Router<'a> {
    /// Routeur dont le côté QGIS est le bouchon.
    pub fn new(registry: &'a Registry) -> Self {
        Router {
            registry,
            core: InProcess::new(registry),
            qgis: Box::new(QgisStub),
        }
    }

    /// Branche un exécutant QGIS à la place du bouchon.
    pub fn with_qgis(mut self, qgis: impl Executor + 'a) -> Self {
        self.qgis = Box::new(qgis);
        self
    }

    pub fn call(&self, id: &str, params: Value) -> Result<Value, RegistryError> {
        let spec = self
            .registry
            .spec(id)
            .ok_or_else(|| RegistryError::UnknownCommand(id.to_string()))?;
        self.registry.validate_params(id, &params)?;
        match spec.runs_in {
            Some(RunsIn::Qgis) => self.qgis.execute(id, params),
            _ => self.core.execute(id, params),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::*;
    use crate::registry::{Command, CommandSpec, Delegated};

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct Path {
        path: String,
    }

    #[derive(Debug, Serialize, JsonSchema)]
    struct Done {
        ok: bool,
    }

    struct Local;
    impl Command for Local {
        type Params = Path;
        type Output = Done;
        fn spec() -> CommandSpec {
            CommandSpec::new::<Path, Done>("test.local", "Local.", "Local.")
                .runs_in(RunsIn::Core)
                .cancellable(false)
                .tests(&["t"])
        }
        fn run(&self, _p: Path) -> Done {
            Done { ok: true }
        }
    }

    struct Remote;
    impl Delegated for Remote {
        type Params = Path;
        type Output = Done;
        fn spec() -> CommandSpec {
            CommandSpec::new::<Path, Done>("test.remote", "Distante.", "Remote.")
                .runs_in(RunsIn::Qgis)
                .cancellable(true)
                .tests(&["t"])
        }
    }

    fn registry() -> Registry {
        let mut r = Registry::new();
        r.register(Local);
        r.register_delegated::<Remote>();
        r
    }

    struct Counting(Arc<AtomicUsize>);
    impl Executor for Counting {
        fn execute(&self, _id: &str, _p: Value) -> Result<Value, RegistryError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(json!({"ok": true, "by": "qgis"}))
        }
    }

    #[test]
    fn invalid_params_execute_nothing() {
        let r = registry();
        let count = Arc::new(AtomicUsize::new(0));
        let router = Router::new(&r).with_qgis(Counting(count.clone()));
        for bad in [json!({"extra": 1}), json!({"path": 3}), json!({})] {
            let err = router.call("test.remote", bad).unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params");
            assert!(err.to_string().starts_with("Paramètres invalides"));
        }
        assert_eq!(count.load(Ordering::SeqCst), 0);
        let err = router.call("test.local", json!({"x": 1})).unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
    }

    #[test]
    fn qgis_command_with_the_stub_is_unavailable_and_named() {
        let r = registry();
        let err = Router::new(&r)
            .call("test.remote", json!({"path": "a"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
        assert!(err.to_string().contains("test.remote"));
    }

    #[test]
    fn qgis_command_with_an_executor_returns_its_result() {
        let r = registry();
        let count = Arc::new(AtomicUsize::new(0));
        let router = Router::new(&r).with_qgis(Counting(count.clone()));
        let out = router.call("test.remote", json!({"path": "a"})).unwrap();
        assert_eq!(out, json!({"ok": true, "by": "qgis"}));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn core_command_runs_in_process_and_unknown_id_is_refused() {
        let r = registry();
        let router = Router::new(&r);
        assert_eq!(
            router.call("test.local", json!({"path": "a"})).unwrap(),
            json!({"ok": true})
        );
        assert_eq!(
            router.call("no.such", json!({})).unwrap_err().code(),
            "registry.unknown_command"
        );
    }
}
