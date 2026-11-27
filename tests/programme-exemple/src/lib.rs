//! Programme noyau d'exemple (entrée 2.14) : le registre officiel plus une commande ajoutée
//! fictive, `exemple.echo`. C'est le modèle qu'un intégrateur recopie : une bibliothèque qui
//! compose le programme, un binaire de trois lignes qui le passe aux quatre points d'entrée.

use cassini::{AddedCommands, Program, ProgramInfo};
use cassini_core::registry::{Command, CommandFailure, CommandSpec, RunsIn};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Nom du programme d'exemple, annoncé par `system.about` et `session.hello`.
pub const NAME: &str = "programme-exemple";

/// Paramètres de `exemple.echo`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EchoParams {
    /// Texte à rendre.
    pub text: String,
}

/// Résultat de `exemple.echo`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct EchoOutput {
    /// Le texte reçu, inchangé.
    pub text: String,
    /// Nombre de caractères du texte.
    pub length: u64,
}

/// Commande ajoutée fictive : rend son texte et sa longueur. Elle calcule dans le noyau.
pub struct Echo;

impl Command for Echo {
    type Params = EchoParams;
    type Output = EchoOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<EchoParams, EchoOutput>(
            "exemple.echo",
            "Rend le texte reçu et sa longueur (commande ajoutée d'exemple).",
            "Returns the given text and its length (example added command).",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .example(serde_json::json!({"text": "bonjour"}))
        .tests(&["echo_returns_text_and_length"])
    }

    fn run(&self, params: EchoParams) -> Result<EchoOutput, CommandFailure> {
        Ok(EchoOutput {
            length: params.text.chars().count() as u64,
            text: params.text,
        })
    }
}

/// Les commandes que le programme d'exemple ajoute.
pub fn added() -> AddedCommands {
    AddedCommands::new().with(Echo)
}

/// Le programme d'exemple : le registre officiel plus `exemple.echo`.
pub fn program() -> Program {
    Program::with_added(ProgramInfo::new(NAME, env!("CARGO_PKG_VERSION")), added)
}

/// Le catalogue des commandes ajoutées, au même format que le catalogue public.
pub const CATALOGUE: &str = include_str!("../catalogue.md");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_returns_text_and_length() {
        let out = Echo.run(EchoParams { text: "é!".into() }).unwrap();
        assert_eq!((out.text.as_str(), out.length), ("é!", 2));
    }
}
