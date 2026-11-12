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

/// Commande qui lit l'inventaire du projet auprès de QGIS.
pub const INVENTORY: &str = "project.inventory";

/// Champ caché du résultat d'une commande qui dicte des ordres (`CommandSpec::dictates`) : une
/// liste de `{command, params}`, ôtée du résultat avant toute journalisation ou comparaison.
pub const ORDERS_FIELD: &str = "orders";

/// Version du protocole du noyau.
pub const PROTOCOL_VERSION: u32 = 2;

/// Où s'exécute une commande.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunsIn {
    /// Implémentée en Rust dans le noyau.
    Core,
    /// Déléguée à QGIS, déclarée comme telle.
    Qgis,
}

/// Noms des paramètres d'un filtre par attribut neutre (AD-22) : couche, champ, opérateur et
/// valeur. L'opérateur parle le vocabulaire fermé du filtre (`eq`, `ne`, `lt`, `lte`, `gt`,
/// `gte`, `contains`, `begins_with`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeFilter {
    pub layer: String,
    pub field: String,
    pub operator: String,
    pub value: String,
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
    /// Paramètres qui désignent un fichier d'entrée, relatif au projet (AD-9) ; un nom
    /// `liste.champ` désigne le champ `champ` de chaque objet du tableau `liste`.
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
    /// Extension tirée d'un paramètre énuméré : (paramètre, [(valeur, extension)]), pour une
    /// commande dont le format de sortie est un paramètre (`data.convert`, AD-10, AD-24). Une
    /// valeur absente ou hors de la table rend `output_ext`. Métadonnée interne : jamais
    /// sérialisée.
    #[serde(skip)]
    pub output_ext_by: Option<(String, Vec<(String, String)>)>,
    /// Paramètre qui reçoit, au rejeu seulement, les ids des couches posées par une exécution
    /// précédente du même journal, à laisser hors du résultat (ticket 5.36). Rempli par le
    /// noyau, jamais par l'appelant. Métadonnée interne : jamais sérialisée.
    #[serde(skip)]
    pub leave_out_param: Option<String>,
    /// Paramètre que le rejeu remplit depuis le résultat journalisé : (paramètre, champ du
    /// résultat). Ôté à l'appel ; le champ est journalisé sous la clé `replay` de la ligne, qui
    /// survit au résumé du résultat, et recopié au rejeu quand il y est (ticket 5.38 : le
    /// centre d'un export à l'échelle). Rempli par le noyau, jamais par l'appelant.
    /// Métadonnée interne : jamais sérialisée.
    #[serde(skip)]
    pub replay_param: Option<(String, String)>,
    /// Paramètre facultatif que la commande remplit elle-même quand l'appelant le tait, et rend
    /// au champ `field` de son résultat : (paramètre, champ). À la différence de
    /// `replay_param`, la valeur de l'appelant est gardée ; la valeur rendue est journalisée
    /// sous la clé `replay` de la ligne et repassée au rejeu, qui ne redécide pas (décision M4 de
    /// l'entrée 10.29 : la date d'un azimut magnétique). Une ligne sans clé `replay` se rejoue
    /// telle quelle. Métadonnée interne : jamais sérialisée.
    #[serde(skip)]
    pub filled_param: Option<(String, String)>,
    /// Vrai pour une commande que seul le noyau appelle, de lui-même, après une autre étape
    /// (ticket 4.9 : `layer.open_raster`). Elle n'est offerte ni au modèle, ni au client MCP, ni
    /// au formulaire du panneau, et `command.run` la refuse. Sérialisé seulement quand il est
    /// vrai : le catalogue des autres commandes reste celui d'avant.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub dictated_by_core: bool,
    /// Paramètre de carroyage que le noyau complète quand sa forme est `mgrs` (epic 10,
    /// entrée 31) : `epsg` et `squares`, calculés d'après l'emprise du projet lue auprès de QGIS,
    /// ôtés des paramètres de l'appelant et journalisés pour le rejeu. Métadonnée interne :
    /// jamais sérialisée.
    #[serde(skip)]
    pub mgrs_grid_param: Option<String>,
    /// Paramètre qui reçoit l'inventaire du projet, lu par le noyau auprès de QGIS juste avant
    /// l'exécution, hors du journal (amendement d'AD-30, entrée 11.20). Ôté des paramètres de
    /// l'appelant, jamais journalisé ; même lecture au rejeu. Métadonnée interne : jamais sérialisée.
    #[serde(skip)]
    pub inventory_param: Option<String>,
    /// Commandes `dictated_by_core` que cette commande peut demander au noyau de dicter après sa
    /// ligne, sous le champ caché [`ORDERS_FIELD`] de son résultat, ôté avant journalisation
    /// (amendement d'AD-30). Métadonnée interne : jamais sérialisée.
    #[serde(skip)]
    pub dictates: Vec<String>,
    /// Vrai pour une commande du genre de sortie `none` (amendement d'AD-24) : elle change l'état
    /// du projet sans produire ni couche ni fichier. Le `layer_id` de son résultat désigne une
    /// couche existante : il n'est ni journalisé comme couche posée, ni lié au rejeu.
    /// Métadonnée interne : jamais sérialisée.
    #[serde(skip)]
    pub output_none: bool,
    /// Style que la session dicte (`layer.set_style`) juste après l'ouverture dictée du raster
    /// écrit (`layer.open_raster`), composé d'après le résultat de la commande, si l'extension
    /// annonce `layer.set_style` (bogue du fond noir, 2026-11-09). Sans le geste, le raster est
    /// ouvert sans style, comme avant : ce n'est pas un geste requis. Métadonnée interne :
    /// jamais sérialisée.
    #[serde(skip)]
    pub result_style: Option<crate::style::ResultStyle>,
    /// Filtre par attribut que porte la commande (ticket 5.57) : les noms de ses paramètres
    /// couche, champ, opérateur et valeur, que le garde-fou d'AD-23 (`attribute_guard`) contrôle
    /// contre l'inventaire montré au modèle. Métadonnée interne : jamais sérialisée, le
    /// catalogue et le prompt ne changent pas.
    #[serde(skip)]
    pub attribute_filter: Option<AttributeFilter>,
    /// Système de calcul en mètres que le noyau choisit (changement de cap du 2026-11-12,
    /// entrée 9.16) : (paramètre rempli, paramètre qui désigne la couche d'entrée). Le noyau ôte
    /// la valeur de l'appelant, lit l'inventaire auprès de QGIS hors du journal, y choisit le
    /// code EPSG (`crate::work_crs`) et le journalise avec les paramètres ; le rejeu le relit.
    /// Caché au modèle, au client MCP et au formulaire, comme `out`. Métadonnée interne : jamais
    /// sérialisée.
    #[serde(skip)]
    pub work_crs_param: Option<(String, String)>,
    /// Mise en évidence du résultat d'un plan dans son PDF (changement de cap du 2026-11-12,
    /// entrée 7.9) : paramètre que le noyau remplit, pour un export lancé dans un plan, d'après
    /// les lignes du plan en cours au journal (`crate::focus`). Ôté des paramètres de l'appelant,
    /// journalisé, relu au rejeu sans être recalculé. Caché au modèle, au client MCP et au
    /// formulaire, comme `out`. Métadonnée interne : jamais sérialisée.
    #[serde(skip)]
    pub focus_param: Option<String>,
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
            output_ext_by: None,
            leave_out_param: None,
            replay_param: None,
            filled_param: None,
            dictated_by_core: false,
            inventory_param: None,
            mgrs_grid_param: None,
            dictates: Vec::new(),
            output_none: false,
            result_style: None,
            attribute_filter: None,
            work_crs_param: None,
            focus_param: None,
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

    /// Déclare les paramètres qui sont des chemins d'entrée relatifs au projet : une chaîne ou
    /// un tableau de chaînes (`dem`, `exclusions`), ou, sous la forme `liste.champ`, le champ
    /// `champ` de chaque objet du tableau `liste` (`criteria.raster`). Un seul niveau : ni
    /// `a.b.c`, ni objet hors d'un tableau.
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

    /// Déclare l'extension des fichiers de sortie tirée de la valeur du paramètre `param` :
    /// `table` associe chaque valeur permise à son extension (point compris, ex. `.shp`).
    /// Limite : un repère `out:<k>.<n>` ne peut pas désigner une telle sortie (il ne porte pas
    /// les paramètres) ; le journal le refuse au lieu de deviner l'extension.
    pub fn output_ext_from(mut self, param: &str, table: &[(&str, &str)]) -> Self {
        self.output_ext_by = Some((
            param.to_string(),
            table
                .iter()
                .map(|(value, ext)| (value.to_string(), ext.to_string()))
                .collect(),
        ));
        self
    }

    /// Extension des fichiers de sortie pour les paramètres `params` d'un appel : celle que la
    /// valeur du paramètre déclaré par `output_ext_from` désigne, sinon `output_ext`.
    pub fn output_ext_for(&self, params: &Value) -> &str {
        self.output_ext_by
            .as_ref()
            .and_then(|(param, table)| {
                let value = params.get(param)?.as_str()?;
                table
                    .iter()
                    .find(|(v, _)| v == value)
                    .map(|(_, ext)| ext.as_str())
            })
            .unwrap_or(&self.output_ext)
    }

    /// Déclare le paramètre que le rejeu remplit des ids de couche à laisser hors du résultat.
    pub fn leave_out_param(mut self, name: &str) -> Self {
        self.leave_out_param = Some(name.to_string());
        self
    }

    /// Déclare le paramètre que le rejeu recopie depuis le champ `field` du résultat journalisé.
    pub fn replay_param(mut self, name: &str, field: &str) -> Self {
        self.replay_param = Some((name.to_string(), field.to_string()));
        self
    }

    /// Déclare le paramètre facultatif que la commande remplit elle-même et rend au champ
    /// `field` de son résultat ; le rejeu le repasse depuis la ligne journalisée (M4, 10.29).
    pub fn filled_param(mut self, name: &str, field: &str) -> Self {
        self.filled_param = Some((name.to_string(), field.to_string()));
        self
    }

    /// Déclare la commande dictée par le noyau seul (ticket 4.9).
    pub fn dictated_by_core(mut self) -> Self {
        self.dictated_by_core = true;
        self
    }

    /// Déclare le paramètre de carroyage que le noyau complète pour la forme `mgrs`.
    pub fn mgrs_grid_param(mut self, name: &str) -> Self {
        self.mgrs_grid_param = Some(name.to_string());
        self
    }

    /// Déclare le paramètre qui reçoit l'inventaire du projet, lu hors du journal (entrée 11.20).
    pub fn inventory_param(mut self, name: &str) -> Self {
        self.inventory_param = Some(name.to_string());
        self
    }

    /// Déclare les commandes dictées que la commande peut demander après sa ligne (entrée 11.20).
    pub fn dictates(mut self, ids: &[&str]) -> Self {
        self.dictates = ids.iter().map(|n| n.to_string()).collect();
        self
    }

    /// Déclare le genre de sortie `none` (amendement d'AD-24).
    pub fn output_none(mut self) -> Self {
        self.output_none = true;
        self
    }

    /// Déclare le style dicté après l'ouverture du raster écrit (bogue du fond noir).
    pub fn result_style(mut self, style: crate::style::ResultStyle) -> Self {
        self.result_style = Some(style);
        self
    }

    /// Déclare le paramètre `name` que le noyau remplit du système de calcul en mètres choisi
    /// pour la couche désignée par le paramètre `layer` (entrée 9.16).
    pub fn work_crs_param(mut self, name: &str, layer: &str) -> Self {
        self.work_crs_param = Some((name.to_string(), layer.to_string()));
        self
    }

    /// Déclare le paramètre `name` de mise en évidence du résultat d'un plan, que le noyau
    /// remplit pour un export lancé dans un plan (entrée 7.9).
    pub fn focus_param(mut self, name: &str) -> Self {
        self.focus_param = Some(name.to_string());
        self
    }

    /// Déclare le filtre par attribut de la commande : les noms de ses paramètres couche, champ,
    /// opérateur et valeur (ticket 5.57).
    pub fn attribute_filter(
        mut self,
        layer: &str,
        field: &str,
        operator: &str,
        value: &str,
    ) -> Self {
        self.attribute_filter = Some(AttributeFilter {
            layer: layer.to_string(),
            field: field.to_string(),
            operator: operator.to_string(),
            value: value.to_string(),
        });
        self
    }

    /// Les gestes que l'extension doit annoncer pour que la commande soit offerte (AD-25) :
    /// `project.inventory` si elle lit l'inventaire, puis chaque commande qu'elle dicte. Vide pour
    /// toute autre commande.
    pub fn required_gestures(&self) -> Vec<&str> {
        self.inventory_param
            .as_ref()
            .map(|_| INVENTORY)
            .into_iter()
            .chain(self.dictates.iter().map(String::as_str))
            .collect()
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

    /// Retire une commande du registre ; vrai si elle y était. N'existe que pour le test
    /// d'égalité à l'octet du prompt et de la grammaire du modèle (`cassini-ai`, ticket 4.9) :
    /// aucun code du produit ne doit retirer une commande du registre unique.
    #[doc(hidden)]
    pub fn remove(&mut self, id: &str) -> bool {
        self.entries.remove(id).is_some()
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

/// Ôte d'un schéma les définitions (`$defs`) que rien de visible n'emploie plus, directement ou
/// par une autre définition gardée.
pub fn prune_defs(schema: &mut Value) {
    let Some(map) = schema.as_object_mut() else {
        return;
    };
    let Some(Value::Object(defs)) = map.remove("$defs") else {
        return;
    };
    let mut reached = vec![Value::Object(map.clone()).to_string()];
    let mut kept: Vec<String> = Vec::new();
    loop {
        let before = kept.len();
        for (key, def) in &defs {
            let target = format!("\"#/$defs/{key}\"");
            if !kept.contains(key) && reached.iter().any(|t| t.contains(&target)) {
                kept.push(key.clone());
                reached.push(def.to_string());
            }
        }
        if kept.len() == before {
            break;
        }
    }
    let defs: serde_json::Map<String, Value> =
        defs.into_iter().filter(|(k, _)| kept.contains(k)).collect();
    if !defs.is_empty() {
        map.insert("$defs".into(), Value::Object(defs));
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
            json!({"core_version": crate::VERSION, "protocol_version": 2})
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
