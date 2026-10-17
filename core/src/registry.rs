//! Registre de commandes : le contrat de AD-6, à la taille minimale.
//!
//! Chaque commande décrit ses paramètres et son résultat par des types Rust ;
//! les schémas JSON (2020-12) en sont engendrés par `schemars`, jamais écrits
//! à la main. Le registre garde une version à types effacés des commandes.

use std::collections::BTreeMap;
use std::fmt;

use schemars::{schema_for, JsonSchema};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

/// Version du protocole du noyau.
pub const PROTOCOL_VERSION: u32 = 1;

/// Où s'exécute une commande.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunsIn {
    /// Implémentée en Rust dans le noyau.
    Core,
    /// Déléguée à QGIS, déclarée comme telle.
    Qgis,
}

/// Description d'une commande, lue par l'IA et les points d'entrée.
#[derive(Debug, Clone, Serialize)]
pub struct CommandSpec {
    pub id: String,
    pub description_fr: String,
    pub description_en: String,
    /// Où la commande s'exécute. `None` = non déclaré, une violation du contrat (2.3).
    pub runs_in: Option<RunsIn>,
    /// Annulable ou non. `None` = non déclaré, une violation du contrat (2.3).
    pub cancellable: Option<bool>,
    /// Noms des fonctions `#[test]` de `core/src/commands/` qui prouvent la commande.
    /// Métadonnée interne : jamais sérialisée dans le catalogue (CLAUDE.md §2.3).
    #[serde(skip)]
    pub tests: Vec<String>,
    /// Paramètres qui désignent un fichier d'entrée, relatif au projet (AD-9).
    /// Métadonnée interne : jamais sérialisée dans le catalogue.
    #[serde(skip)]
    pub path_params: Vec<String>,
    /// Paramètres de sortie, dont la valeur est donnée par le noyau dans le
    /// dossier de travail (AD-10). Métadonnée interne : jamais sérialisée.
    #[serde(skip)]
    pub output_params: Vec<String>,
    /// Extension des fichiers de sortie, point compris (défaut `.out`). Une commande dont la
    /// sortie est une couche déclare `.gpkg`. Métadonnée interne : jamais sérialisée.
    #[serde(skip)]
    pub output_ext: String,
    /// Schéma JSON des paramètres, engendré depuis le type Rust.
    pub params_schema: Value,
    /// Schéma JSON du résultat, engendré depuis le type Rust.
    pub output_schema: Value,
    /// Exemples de paramètres valides, lus par l'IA.
    pub examples: Vec<Value>,
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
            runs_in: None,
            cancellable: None,
            tests: Vec::new(),
            path_params: Vec::new(),
            output_params: Vec::new(),
            output_ext: ".out".to_string(),
            params_schema: schema_value::<P>(),
            output_schema: schema_value::<O>(),
            examples: Vec::new(),
        }
    }

    /// Ajoute un exemple de paramètres valides.
    pub fn example(mut self, params: Value) -> Self {
        self.examples.push(params);
        self
    }

    /// Déclare où la commande s'exécute.
    pub fn runs_in(mut self, runs_in: RunsIn) -> Self {
        self.runs_in = Some(runs_in);
        self
    }

    /// Déclare si la commande est annulable.
    pub fn cancellable(mut self, cancellable: bool) -> Self {
        self.cancellable = Some(cancellable);
        self
    }

    /// Déclare les paramètres qui sont des chemins d'entrée relatifs au projet.
    pub fn path_params(mut self, names: &[&str]) -> Self {
        self.path_params = names.iter().map(|n| n.to_string()).collect();
        self
    }

    /// Déclare les paramètres de sortie, remplis par le noyau (AD-10).
    pub fn output_params(mut self, names: &[&str]) -> Self {
        self.output_params = names.iter().map(|n| n.to_string()).collect();
        self
    }

    /// Déclare l'extension des fichiers de sortie (point compris, ex. `.gpkg`).
    pub fn output_ext(mut self, ext: &str) -> Self {
        self.output_ext = ext.to_string();
        self
    }

    /// Déclare les noms des tests qui prouvent la commande.
    pub fn tests(mut self, names: &[&str]) -> Self {
        self.tests = names.iter().map(|n| n.to_string()).collect();
        self
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
    fn run(&self, params: Self::Params) -> Result<Self::Output, CommandFailure>;
}

/// Échec d'une commande du noyau : un code stable `domaine.cause` (AD-21) et un message français.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFailure {
    pub code: String,
    pub message: String,
}

impl CommandFailure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Erreur du registre, avec un code stable `domaine.cause`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    UnknownCommand(String),
    InvalidParams(String),
    /// Le résultat d'une commande n'a pas pu être mis en JSON : un défaut du noyau, pas de l'appelant.
    ResultNotSerializable(String),
    /// La commande est déclarée mais aucun exécutant n'est branché pour elle (QGIS absent).
    Unavailable(String),
    /// La commande a été annulée avant la fin (AD-12) ; porte son id.
    Cancelled(String),
    /// L'extension a répondu à l'ordre par une erreur : son code et son message sont gardés tels quels.
    Delegated {
        code: String,
        message: String,
    },
    /// La commande du noyau s'est exécutée et a échoué, avec son code typé (AD-21).
    Failed {
        code: String,
        message: String,
    },
}

impl RegistryError {
    /// Code stable, destiné aux machines.
    pub fn code(&self) -> &str {
        match self {
            RegistryError::UnknownCommand(_) => "registry.unknown_command",
            RegistryError::InvalidParams(_) => "registry.invalid_params",
            RegistryError::ResultNotSerializable(_) => "registry.result_not_serializable",
            RegistryError::Unavailable(_) => "registry.unavailable",
            RegistryError::Cancelled(_) => "registry.cancelled",
            RegistryError::Delegated { code, .. } | RegistryError::Failed { code, .. } => code,
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
            RegistryError::Unavailable(id) => write!(
                f,
                "La commande « {id} » est déléguée à QGIS, qui n'est pas branché : elle ne peut pas s'exécuter."
            ),
            RegistryError::Cancelled(id) => write!(f, "La commande « {id} » a été annulée."),
            RegistryError::Delegated { message, .. } | RegistryError::Failed { message, .. } => {
                write!(f, "{message}")
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

type Validator = Box<dyn Fn(&Value) -> Result<(), RegistryError> + Send + Sync>;

/// Une commande déléguée à QGIS : ses types et sa description, sans implémentation Rust.
pub trait Delegated {
    type Params: DeserializeOwned + JsonSchema;
    type Output: Serialize + JsonSchema;

    fn spec() -> CommandSpec;
}

struct Entry {
    spec: CommandSpec,
    /// Valide les paramètres par le type Rust qui a engendré le schéma.
    validate: Validator,
    /// `None` pour une commande déléguée : il n'y a rien à exécuter en processus.
    handler: Option<Handler>,
}

/// Inscription d'une commande, faite par la commande elle-même dans son fichier :
/// `inventory::submit! { Registration(|r| r.register(MaCommande)) }`.
pub struct Registration(pub fn(&mut Registry));

inventory::collect!(Registration);

/// Le registre : id de commande vers description et exécution.
#[derive(Default)]
pub struct Registry {
    entries: BTreeMap<String, Entry>,
    /// Ids inscrits plus d'une fois, une entrée par tentative écartée.
    duplicates: Vec<String>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inscrit une commande. Si l'id existe déjà, la première entrée est gardée
    /// et le doublon est noté : `check_contract` le rend (`contract.duplicate_id`).
    pub fn register<C: Command + Send + Sync + 'static>(&mut self, command: C) {
        let spec = C::spec();
        if self.entries.contains_key(&spec.id) {
            self.duplicates.push(spec.id);
            return;
        }
        let validate: Validator = Box::new(|json| {
            serde_json::from_value::<C::Params>(json.clone())
                .map(|_| ())
                .map_err(|e| RegistryError::InvalidParams(e.to_string()))
        });
        let handler: Handler = Box::new(move |json| {
            let params: C::Params = serde_json::from_value(json)
                .map_err(|e| RegistryError::InvalidParams(e.to_string()))?;
            let output = command.run(params).map_err(|f| RegistryError::Failed {
                code: f.code,
                message: f.message,
            })?;
            serde_json::to_value(output)
                .map_err(|e| RegistryError::ResultNotSerializable(e.to_string()))
        });
        self.entries.insert(
            spec.id.clone(),
            Entry {
                spec,
                validate,
                handler: Some(handler),
            },
        );
    }

    /// Inscrit une commande déléguée à QGIS : types et schémas, aucune implémentation.
    /// Même règle de doublon que `register`.
    pub fn register_delegated<D: Delegated>(&mut self) {
        let spec = D::spec();
        if self.entries.contains_key(&spec.id) {
            self.duplicates.push(spec.id);
            return;
        }
        let validate: Validator = Box::new(|json| {
            serde_json::from_value::<D::Params>(json.clone())
                .map(|_| ())
                .map_err(|e| RegistryError::InvalidParams(e.to_string()))
        });
        self.entries.insert(
            spec.id.clone(),
            Entry {
                spec,
                validate,
                handler: None,
            },
        );
    }

    /// Valide des paramètres contre le type de la commande, sans rien exécuter.
    pub fn validate_params(&self, id: &str, params: &Value) -> Result<(), RegistryError> {
        let entry = self
            .entries
            .get(id)
            .ok_or_else(|| RegistryError::UnknownCommand(id.to_string()))?;
        (entry.validate)(params)
    }

    /// Chaque description, avec un booléen vrai si la commande a une implémentation Rust.
    pub(crate) fn implementations(&self) -> impl Iterator<Item = (&CommandSpec, bool)> {
        self.entries
            .values()
            .map(|e| (&e.spec, e.handler.is_some()))
    }

    /// Les descriptions de toutes les commandes, dans l'ordre des ids.
    pub fn specs(&self) -> impl Iterator<Item = &CommandSpec> {
        self.entries.values().map(|e| &e.spec)
    }

    /// Les ids inscrits plus d'une fois.
    pub(crate) fn duplicate_ids(&self) -> &[String] {
        &self.duplicates
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
        match &entry.handler {
            Some(handler) => handler(params),
            None => Err(RegistryError::Unavailable(id.to_string())),
        }
    }

    /// Le registre standard du noyau : toutes les commandes qui se sont inscrites
    /// (voir `Registration`). Le registre ne connaît aucune commande par son nom.
    /// L'ordre vient de l'id (BTreeMap), jamais de l'ordre d'inscription.
    pub fn standard() -> Self {
        let mut registry = Self::new();
        for registration in inventory::iter::<Registration> {
            (registration.0)(&mut registry);
        }
        registry
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
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
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

    #[test]
    fn every_declared_example_validates_its_params() {
        let registry = Registry::standard();
        let mut seen = 0;
        for spec in registry.specs() {
            for example in &spec.examples {
                registry.validate_params(&spec.id, example).unwrap();
                seen += 1;
            }
        }
        assert!(seen >= 1);
    }

    #[test]
    fn standard_registry_is_sorted_by_id() {
        let ids: Vec<String> = Registry::standard().entries.keys().cloned().collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
        assert!(ids.contains(&"system.about".to_string()));
    }
}
