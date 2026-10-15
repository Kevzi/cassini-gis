//! Commande `system.about` : versions du noyau et du protocole.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{Command, CommandSpec, Registration, PROTOCOL_VERSION};

/// Paramètres de `system.about` : aucun, et aucun champ inconnu accepté.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AboutParams {}

/// Résultat de `system.about`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct AboutOutput {
    pub core_version: String,
    pub protocol_version: u32,
}

/// Commande `system.about` : versions du noyau et du protocole.
pub struct SystemAbout;

impl Command for SystemAbout {
    type Params = AboutParams;
    type Output = AboutOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<AboutParams, AboutOutput>(
            "system.about",
            "Donne la version du noyau et celle du protocole.",
            "Reports the core version and the protocol version.",
        )
    }

    fn run(&self, _params: AboutParams) -> AboutOutput {
        AboutOutput {
            core_version: crate::VERSION.to_string(),
            protocol_version: PROTOCOL_VERSION,
        }
    }
}

inventory::submit! { Registration(|r| r.register(SystemAbout)) }
