//! Commande `test.echo`, réservée aux tests : preuve qu'un fichier neuf
//! s'inscrit sans modifier un fichier partagé.

#![cfg(test)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{Command, CommandSpec, Registration};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EchoParams {
    pub text: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct EchoOutput {
    pub text: String,
}

pub struct TestEcho;

impl Command for TestEcho {
    type Params = EchoParams;
    type Output = EchoOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<EchoParams, EchoOutput>(
            "test.echo",
            "Renvoie le texte reçu (tests seulement).",
            "Returns the given text (tests only).",
        )
    }

    fn run(&self, params: EchoParams) -> EchoOutput {
        EchoOutput { text: params.text }
    }
}

inventory::submit! { Registration(|r| r.register(TestEcho)) }

#[cfg(test)]
mod tests {
    use crate::registry::Registry;
    use serde_json::json;

    #[test]
    fn a_new_file_registers_its_command() {
        let registry = Registry::standard();
        assert!(registry.spec("test.echo").is_some());
        let out = registry
            .call("test.echo", json!({"text": "salut"}))
            .unwrap();
        assert_eq!(out, json!({"text": "salut"}));
    }
}
