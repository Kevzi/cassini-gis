//! Registre de commandes : le contrat de AD-6, à la taille minimale.
//!
//! Chaque commande décrit ses paramètres et son résultat par des types Rust ;
//! les schémas JSON (2020-12) en sont engendrés par `schemars`, jamais écrits
//! à la main. Le registre garde une version à types effacés des commandes.

use std::collections::BTreeMap;
use std::fmt;

use schemars::{schema_for, JsonSchema};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Version du protocole du noyau.
pub const PROTOCOL_VERSION: u32 = 1;

/// Où s'exécute une commande.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunsIn {
    /// Implémentée en Rust dans le noyau.
    Core,
}

/// Description d'une commande, lue par l'IA et les points d'entrée.
#[derive(Debug, Clone, Serialize)]
pub struct CommandSpec {
    pub id: String,
    pub description_fr: String,
    pub description_en: String,
    pub runs_in: RunsIn,
    /// Schéma JSON des paramètres, engendré depuis le type Rust.
    pub params_schema: Value,
    /// Schéma JSON du résultat, engendré depuis le type Rust.
    pub output_schema: Value,
}

impl CommandSpec {
    /// Construit une description dont les schémas viennent des types `P` et `O`.
    pub fn new<P: JsonSchema, O: JsonSchema>(
        id: &str,
        description_fr: &str,
        description_en: &str,
    ) -> Self {
        CommandSpec {
            id: id.to_string(),
            description_fr: description_fr.to_string(),
            description_en: description_en.to_string(),
            runs_in: RunsIn::Core,
            params_schema: schema_value::<P>(),
            output_schema: schema_value::<O>(),
        }
    }
}

fn schema_value<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).expect("un schéma engendré se sérialise toujours")
}

/// Une commande Rust du noyau.
pub trait Command {
    type Params: DeserializeOwned + JsonSchema;
    type Output: Serialize + JsonSchema;

    fn spec() -> CommandSpec;
    fn run(&self, params: Self::Params) -> Self::Output;
}

/// Erreur du registre, avec un code stable `domaine.cause`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    UnknownCommand(String),
    InvalidParams(String),
    /// Le résultat d'une commande n'a pas pu être mis en JSON : un défaut du noyau, pas de l'appelant.
    ResultNotSerializable(String),
}

impl RegistryError {
    /// Code stable, destiné aux machines.
    pub fn code(&self) -> &'static str {
        match self {
            RegistryError::UnknownCommand(_) => "registry.unknown_command",
            RegistryError::InvalidParams(_) => "registry.invalid_params",
            RegistryError::ResultNotSerializable(_) => "registry.result_not_serializable",
        }
    }
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::UnknownCommand(id) => write!(f, "Commande inconnue : « {id} »."),
            RegistryError::InvalidParams(why) => {
                write!(f, "Paramètres invalides (détail technique : {why}).")
            }
            RegistryError::ResultNotSerializable(why) => {
                write!(
                    f,
                    "Le résultat n'a pas pu être produit (détail technique : {why})."
                )
            }
        }
    }
}

impl std::error::Error for RegistryError {}

type Handler = Box<dyn Fn(Value) -> Result<Value, RegistryError> + Send + Sync>;

struct Entry {
    spec: CommandSpec,
    handler: Handler,
}

/// Le registre : id de commande vers description et exécution.
#[derive(Default)]
pub struct Registry {
    entries: BTreeMap<String, Entry>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inscrit une commande ; une commande de même id est remplacée.
    pub fn register<C: Command + Send + Sync + 'static>(&mut self, command: C) {
        let spec = C::spec();
        let handler: Handler = Box::new(move |json| {
            let params: C::Params = serde_json::from_value(json)
                .map_err(|e| RegistryError::InvalidParams(e.to_string()))?;
            serde_json::to_value(command.run(params))
                .map_err(|e| RegistryError::ResultNotSerializable(e.to_string()))
        });
        self.entries
            .insert(spec.id.clone(), Entry { spec, handler });
    }

    pub fn spec(&self, id: &str) -> Option<&CommandSpec> {
        self.entries.get(id).map(|e| &e.spec)
    }

    /// Appelle une commande avec ses paramètres en JSON.
    pub fn call(&self, id: &str, params: Value) -> Result<Value, RegistryError> {
        let entry = self
            .entries
            .get(id)
            .ok_or_else(|| RegistryError::UnknownCommand(id.to_string()))?;
        (entry.handler)(params)
    }

    /// Le registre standard du noyau.
    pub fn standard() -> Self {
        let mut registry = Self::new();
        registry.register(SystemAbout);
        registry
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

    #[test]
    fn schemas_are_generated_in_2020_12() {
        let registry = Registry::standard();
        let spec = registry.spec("system.about").unwrap();
        assert_eq!(spec.params_schema["$schema"], DIALECT);
        assert_eq!(spec.output_schema["$schema"], DIALECT);
        assert!(spec.output_schema["properties"]["core_version"].is_object());
        assert_eq!(spec.params_schema["additionalProperties"], false);
        assert_eq!(spec.runs_in, RunsIn::Core);
    }

    #[test]
    fn system_about_returns_the_versions() {
        let out = Registry::standard()
            .call("system.about", json!({}))
            .unwrap();
        assert_eq!(
            out,
            json!({"core_version": crate::VERSION, "protocol_version": 1})
        );
    }

    #[test]
    fn unknown_id_gives_unknown_command() {
        let err = Registry::standard().call("foo.bar", json!({})).unwrap_err();
        assert_eq!(err.code(), "registry.unknown_command");
        assert!(err.to_string().contains("foo.bar"));
    }

    #[test]
    fn unknown_field_gives_invalid_params() {
        let err = Registry::standard()
            .call("system.about", json!({"x": 1}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
    }
}
