//! Composition d'un plan : de la phrase de l'utilisateur à une suite d'appels
//! du registre (AD-23).
//!
//! Le modèle ne reçoit que le catalogue (tiré du registre réel) et l'inventaire
//! du projet, jamais les données. Sa sortie est contrainte par une grammaire à
//! la forme d'un plan, puis **validée** par `Registry::check_plan`. Un plan
//! invalide n'est jamais réparé : on redemande une fois en citant les écarts,
//! puis on rejette. Rien n'est montré ni exécuté avant cette validation.

use std::fmt;
use std::path::{Path, PathBuf};

use cassini_core::commands::model_list::PromptKind;
use cassini_core::plan::Plan;
use cassini_core::point::{guard_coordinates, guard_rejected};
use cassini_core::registry::{CommandSpec, Registry};
use serde_json::{json, Map, Value};

use crate::{generate_files, AiError, Completion, ModelFiles, Role, Turn};

/// Le prompt que le modèle attend (ticket 5.10, tâche 3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PromptStyle {
    /// Le long prompt : consignes, catalogue tiré du registre et exemples. Celui de tout GGUF
    /// sans déclaration.
    #[default]
    Full,
    /// Le prompt court appris par le modèle ajusté : quelques consignes, sans catalogue ni
    /// exemples. Texte identique, à l'octet près, à celui de l'entraînement.
    Short,
}

impl From<PromptKind> for PromptStyle {
    fn from(kind: PromptKind) -> Self {
        match kind {
            PromptKind::Full => PromptStyle::Full,
            PromptKind::Short => PromptStyle::Short,
        }
    }
}

/// Borne de la réponse du modèle, en jetons : de quoi écrire un plan de douze étapes.
pub const MAX_PLAN_TOKENS: usize = 1024;
/// Plafond d'étapes d'un plan (celui de l'épreuve du modèle local).
pub const MAX_STEPS: usize = 12;
/// Longueur maximale (en caractères) de la sortie rejetée recopiée dans la redemande.
const MAX_ECHO_CHARS: usize = 1500;
/// Nombre d'écarts rappelés au modèle lors de la redemande.
const MAX_REPORTED: usize = 8;

/// Ce qu'on demande au générateur : la conversation, la grammaire qui contraint
/// la sortie et la borne de jetons.
#[derive(Debug, Clone, Copy)]
pub struct GenerationRequest<'a> {
    pub turns: &'a [Turn],
    pub grammar: &'a str,
    pub max_new_tokens: usize,
}

/// Échecs de `compose_plan`, tous typés.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentError {
    /// Le modèle (ou son chargement) a échoué : l'erreur de la couche IA, telle quelle.
    Ai(AiError),
    /// Le modèle a refusé la demande, avec une raison.
    Refused(String),
    /// Deux sorties de suite rejetées : voici les écarts de la dernière.
    PlanInvalid(Vec<String>),
}

impl AgentError {
    /// Code stable de l'erreur, pour les sorties machine.
    pub fn code(&self) -> &'static str {
        match self {
            AgentError::Ai(e) => e.code(),
            AgentError::Refused(_) => "agent.refused",
            AgentError::PlanInvalid(_) => "agent.plan_invalid",
        }
    }
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::Ai(e) => write!(f, "{e}"),
            AgentError::Refused(reason) => {
                write!(f, "L'agent ne peut pas répondre à cette demande : {reason}")
            }
            AgentError::PlanInvalid(errors) => write!(
                f,
                "L'agent n'a pas su composer un plan valide, même après une nouvelle demande. Écarts : {}",
                errors.join(" ; ")
            ),
        }
    }
}

impl std::error::Error for AgentError {}

impl From<AiError> for AgentError {
    fn from(e: AiError) -> Self {
        AgentError::Ai(e)
    }
}

// -- grammaire -------------------------------------------------------------

/// Les commandes offertes au modèle : toutes, sauf celles que le noyau dicte seul
/// (`dictated_by_core`, ticket 4.9). Le prompt et la grammaire restent ceux d'avant à l'octet.
fn offered(registry: &Registry) -> impl Iterator<Item = &CommandSpec> {
    registry.specs().filter(|s| !s.dictated_by_core)
}

/// Grammaire GBNF d'un plan : `{"kind":"calls","calls":[{"command":<id du
/// registre>,"params":<objet JSON>}, ...]}`, `{"kind":"refusal","reason":<texte>}` ou
/// `{"kind":"question","question":<texte>}`.
/// Les identifiants de commande sont ceux du registre ; le contenu des
/// paramètres est contraint à du JSON bien formé, le reste (types, champs,
/// références) étant vérifié ensuite par `check_plan`. Format compact, sans
/// espaces : c'est celui des exemples du prompt.
pub fn plan_grammar(registry: &Registry) -> String {
    let commands = offered(registry)
        .map(|s| format!("\"\\\"{}\\\"\"", s.id))
        .collect::<Vec<_>>()
        .join(" | ");
    let hex = "[0-9a-fA-F]";
    format!(
        "root ::= calls | refusal | question\n\
         calls ::= \"{{\\\"kind\\\":\\\"calls\\\",\\\"calls\\\":[\" step (\",\" step)* \"]}}\"\n\
         step ::= \"{{\\\"command\\\":\" command \",\\\"params\\\":\" object \"}}\"\n\
         command ::= {commands}\n\
         refusal ::= \"{{\\\"kind\\\":\\\"refusal\\\",\\\"reason\\\":\" string \"}}\"\n\
         question ::= \"{{\\\"kind\\\":\\\"question\\\",\\\"question\\\":\" string \"}}\"\n\
         value ::= object | array | string | number | \"true\" | \"false\" | \"null\"\n\
         object ::= \"{{\" (pair (\",\" pair)*)? \"}}\"\n\
         pair ::= string \":\" value\n\
         array ::= \"[\" (value (\",\" value)*)? \"]\"\n\
         string ::= \"\\\"\" ([^\"\\\\\\x00-\\x1f] | \"\\\\\" ([\"\\\\/bfnrt] | \"u\" {hex} {hex} {hex} {hex}))* \"\\\"\"\n\
         number ::= \"-\"? (\"0\" | [1-9] [0-9]*) (\".\" [0-9]+)? ([eE] [-+]? [0-9]+)?\n"
    )
}

// -- prompt ----------------------------------------------------------------

const SYSTEM: &str = "Tu es l'agent d'un SIG hors ligne. Tu transformes la demande de l'utilisateur en un plan : \
une suite d'appels de commandes du catalogue, rien d'autre. Réponds uniquement par un objet JSON compact, sans aucun texte autour : \
{\"kind\":\"calls\",\"calls\":[{\"command\":id,\"params\":{...}}]} avec les appels dans l'ordre d'exécution.\n\
Règles : une couche existante se désigne par son identifiant exact de l'inventaire ; la couche produite par l'étape k (en comptant à partir de 1) se désigne par la chaîne \"step:k.layer_id\" ; un fichier à ouvrir se prend parmi les fichiers de l'inventaire ; une option facultative n'est posée que si la demande la réclame ; n'ajoute aucune étape que la demande ne réclame pas ; n'invente ni commande, ni couche, ni chemin ; \
n'écris jamais de chemin de sortie : le noyau les décide.\n\
Les étapes s'enchaînent : une étape peut prendre en entrée la couche produite par une étape précédente (\"step:k.layer_id\"). Une distance (\"à moins de X m de ...\") se traduit par analysis.buffer sur la couche de référence, puis analysis.intersect entre la couche cherchée et le tampon ; un filtre sur un champ (\"de la zone U\", \"de la ligne D\") se traduit par analysis.select_by_attribute, avant le tampon ou l'intersection. Le tampon se fait sur la couche de référence (celle après « de », « d'un », « des »), jamais sur la couche cherchée ni sur celle qu'on a filtrée pour une autre condition. Quand la demande combine plusieurs conditions (un filtre sur la couche cherchée, une distance à une autre couche), chaque condition produit sa couche, puis les couches se croisent par des analysis.intersect successifs, le résultat du premier devenant l'entrée du suivant ; la couche cherchée (ce que l'utilisateur veut trouver) est le layer_id du premier intersect, et le plan n'est complet que si elle y figure. Dans une demande, les espaces au milieu d'un nombre séparent les milliers (\"12 500\" vaut 12500). Un mode de déplacement se traduit par le paramètre mode de la commande. Réponds par une question {\"kind\":\"question\",\"question\":\"<question courte>\"} quand les commandes existent mais qu'un mot de la demande ne désigne rien de l'inventaire : une couche dont aucun nom de couche ni aucun fichier ne parle, ou un critère qu'aucun champ ni aucune valeur des couches ne porte. Ne devine jamais une couche ou un champ voisin à la place : demande lequel utiliser. Une couche ou un champ absent de l'inventaire n'est jamais une raison de refus : c'est toujours une question. Quand chaque mot de la demande se retrouve dans l'inventaire, réponds par le plan, jamais par une question.\n\
Réponds par un refus {\"kind\":\"refusal\",\"reason\":\"<raison courte>\"} uniquement si AUCUNE combinaison des commandes du catalogue ne sait faire l'opération demandée (envoyer, retirer, reprojeter, exporter...), quelles que soient les couches. N'invente jamais de contrainte ni de limite que le catalogue ne mentionne pas pour refuser ;.";

/// Les exemples, chacun avec son petit inventaire : ils montrent la
/// correspondance phrase, inventaire, plan, y compris une question et un refus.
fn examples() -> Vec<(Value, &'static str, Value)> {
    vec![
        (
            json!({"layers": [{"layer_id": "gares_7f3a", "name": "Gares", "geometry": "Point",
                "fields": [{"name": "ligne", "type": "string", "values": ["A", "D"]}]}]}),
            "Montre-moi les gares de la ligne D.",
            json!({"kind": "calls", "calls": [{"command": "analysis.select_by_attribute",
                "params": {"layer_id": "gares_7f3a", "field": "ligne", "operator": "eq", "value": "D"}}]}),
        ),
        (
            json!({"layers": [{"layer_id": "routes_2c91", "name": "Routes", "geometry": "LineString",
                "fields": [{"name": "classe", "type": "string", "values": ["nationale", "communale"]}]}]}),
            "Fais une zone tampon de 200 m autour des routes.",
            json!({"kind": "calls", "calls": [{"command": "analysis.buffer",
                "params": {"layer_id": "routes_2c91", "distance": {"value": 200, "unit": "m"}}}]}),
        ),
        (
            json!({"layers": [{"layer_id": "routes_2c91", "name": "Routes", "geometry": "LineString",
                "fields": [{"name": "classe", "type": "string", "values": ["nationale", "communale"]}]}]}),
            "Fais une zone tampon de 200 m autour des routes nationales.",
            json!({"kind": "calls", "calls": [
                {"command": "analysis.select_by_attribute",
                 "params": {"layer_id": "routes_2c91", "field": "classe", "operator": "eq", "value": "nationale"}},
                {"command": "analysis.buffer",
                 "params": {"layer_id": "step:1.layer_id", "distance": {"value": 200, "unit": "m"}}}]}),
        ),
        (
            json!({"layers": [
                {"layer_id": "arrets_4d2e", "name": "Arrêts de bus", "geometry": "Point",
                 "fields": [{"name": "ligne", "type": "string", "values": ["12", "34"]}]},
                {"layer_id": "commerces_9a07", "name": "Commerces", "geometry": "Point", "fields": []}]}),
            "Quels commerces sont à moins de 150 m des arrêts de la ligne 12 ?",
            json!({"kind": "calls", "calls": [
                {"command": "analysis.select_by_attribute",
                 "params": {"layer_id": "arrets_4d2e", "field": "ligne", "operator": "eq", "value": "12"}},
                {"command": "analysis.buffer",
                 "params": {"layer_id": "step:1.layer_id", "distance": {"value": 150, "unit": "m"}}},
                {"command": "analysis.intersect",
                 "params": {"layer_id": "commerces_9a07", "overlay_layer_id": "step:2.layer_id"}}]}),
        ),
        (
            json!({"layers": [
                {"layer_id": "zonage_8b15", "name": "Zonage", "geometry": "MultiPolygon",
                 "fields": [{"name": "typezone", "type": "string", "values": ["U", "N"]}]},
                {"layer_id": "rivieres_20c4", "name": "Rivières", "geometry": "LineString", "fields": []},
                {"layer_id": "ecoles_51be", "name": "Écoles", "geometry": "Point", "fields": []}]}),
            "Trouve les écoles en zone N à moins de 300 m d'une rivière.",
            json!({"kind": "calls", "calls": [
                {"command": "analysis.select_by_attribute",
                 "params": {"layer_id": "zonage_8b15", "field": "typezone", "operator": "eq", "value": "N"}},
                {"command": "analysis.buffer",
                 "params": {"layer_id": "rivieres_20c4", "distance": {"value": 300, "unit": "m"}}},
                {"command": "analysis.intersect",
                 "params": {"layer_id": "ecoles_51be", "overlay_layer_id": "step:1.layer_id"}},
                {"command": "analysis.intersect",
                 "params": {"layer_id": "step:3.layer_id", "overlay_layer_id": "step:2.layer_id"}}]}),
        ),
        (
            json!({"layers": [{"layer_id": "reseau_6e90", "name": "Réseau", "geometry": "LineString", "fields": []}]}),
            "Quelle zone atteint-on en 20 minutes en voiture depuis le point 512 300 / 6 901 450 (Lambert-93) ?",
            json!({"kind": "calls", "calls": [{"command": "analysis.isochrone",
                "params": {"layer_id": "reseau_6e90", "origin": {"x": 512300, "y": 6901450, "epsg": 2154},
                           "mode": "car", "duration": {"value": 20, "unit": "min"}}}]}),
        ),
        (
            json!({"layers": [{"layer_id": "gares_7f3a", "name": "Gares", "geometry": "Point",
                "fields": [{"name": "ligne", "type": "string", "values": ["A", "D"]}]}]}),
            "Fais une zone tampon de 100 m autour des pharmacies.",
            json!({"kind": "question", "question": "Aucune couche de pharmacies dans le projet : laquelle faut-il utiliser ?"}),
        ),
        (
            json!({"layers": [{"layer_id": "ecoles_51be", "name": "Écoles", "geometry": "Point", "fields": []}]}),
            "Envoie le résultat par courriel au maire.",
            json!({"kind": "refusal", "reason": "Aucune commande du catalogue n'envoie de courriel."}),
        ),
    ]
}

/// Le catalogue montré au modèle : identifiant, description française et
/// schéma des paramètres, tirés du registre. Les paramètres de sortie sont
/// ôtés : le noyau les décide (AD-10), le modèle n'a pas à les connaître. De même pour le
/// paramètre que seul le rejeu remplit (ticket 5.36).
fn catalog_entry(spec: &CommandSpec) -> Value {
    let mut params = spec.params_schema.clone();
    let hidden: Vec<&String> = spec
        .output_params
        .iter()
        .chain(spec.leave_out_param.as_ref())
        .chain(spec.replay_param.as_ref().map(|(name, _)| name))
        .collect();
    if let Value::Object(map) = &mut params {
        map.remove("$schema");
        map.remove("title");
        if let Some(Value::Object(props)) = map.get_mut("properties") {
            for name in &hidden {
                props.remove(name.as_str());
            }
        }
        if let Some(Value::Array(required)) = map.get_mut("required") {
            required.retain(|r| r.as_str().is_none_or(|n| !hidden.iter().any(|o| *o == n)));
        }
        // Un `RasterPath` est montré en simple chaîne, avec la description du champ : le long
        // prompt reste celui d'avant le type (ticket 11.5) ; la définition devenue inutile est
        // ôtée ci-dessous.
        map.values_mut().for_each(inline_raster_paths);
        // Une définition que seul un paramètre caché employait (le `Point` du centre d'un
        // export) n'a rien à faire dans le long prompt.
        if let Some(Value::Object(defs)) = map.remove("$defs") {
            let rest = Value::Object(map.clone()).to_string();
            let kept: Map<String, Value> = defs
                .iter()
                .filter(|(key, _)| {
                    let target = format!("\"#/$defs/{key}\"");
                    rest.contains(&target)
                        || defs
                            .iter()
                            .any(|(other, def)| other != *key && def.to_string().contains(&target))
                })
                .map(|(key, def)| (key.clone(), def.clone()))
                .collect();
            if !kept.is_empty() {
                map.insert("$defs".into(), Value::Object(kept));
            }
        }
    }
    strip_marks(&mut params);
    json!({"id": spec.id, "description": spec.description_fr, "params": params})
}

/// Remplace toute référence à la définition `RasterPath` par `"type": "string"`, en gardant
/// les autres clés (la description du champ).
fn inline_raster_paths(node: &mut Value) {
    match node {
        Value::Object(map) => {
            if map.get("$ref").and_then(Value::as_str) == Some("#/$defs/RasterPath") {
                map.remove("$ref");
                map.insert("type".into(), Value::String("string".into()));
            }
            map.values_mut().for_each(inline_raster_paths);
        }
        Value::Array(items) => items.iter_mut().for_each(inline_raster_paths),
        _ => {}
    }
}

/// Ôte du schéma montré au modèle toute marque `x-cassini` (`$defs` compris) : la marque sert
/// au noyau (garde-fou de AD-23), le long prompt reste celui d'avant.
fn strip_marks(node: &mut Value) {
    match node {
        Value::Object(map) => {
            map.remove(cassini_core::point::MARK_KEY);
            map.values_mut().for_each(strip_marks);
        }
        Value::Array(items) => items.iter_mut().for_each(strip_marks),
        _ => {}
    }
}

/// La partie fixe du message système : consignes, catalogue et exemples. Elle ne dépend
/// que du registre : c'est elle que le moteur garde lue d'une demande à l'autre.
fn fixed_system_text(registry: &Registry) -> String {
    let catalog: Vec<Value> = offered(registry).map(catalog_entry).collect();
    let mut text = format!(
        "{SYSTEM}\n\nCATALOGUE :\n{}\n\nEXEMPLES :\n",
        Value::Array(catalog)
    );
    for (inv, sentence, plan) in examples() {
        text.push_str(&format!(
            "Inventaire : {inv}\nDemande : {sentence}\nPlan : {plan}\n"
        ));
    }
    text
}

/// Ce qui précède l'inventaire dans le long prompt.
const FULL_INVENTORY_LEAD: &str = "\nINVENTAIRE DU PROJET :\n";
/// Ce qui précède l'inventaire dans le prompt court.
const SHORT_INVENTORY_LEAD: &str = "\n\nINVENTAIRE DU PROJET :\n";

/// La partie du message système qui suit la partie fixe : l'inventaire du projet.
fn inventory_text(inventory: &Value) -> String {
    format!("{FULL_INVENTORY_LEAD}{inventory}\n")
}

/// Position, dans le texte JSON compact d'une couche `text`, de sa dernière clé (juste après le
/// guillemet qui l'ouvre) : `None` si la couche n'est pas un objet non vide ou si cette clé ne
/// suit pas `,"` ou `{"` et ne commence pas par une lettre.
///
/// Couper là ne change aucun jeton du découpeur de Qwen (BPE à pré-découpe par classes de
/// caractères) : la ponctuation qui précède (`","`, `],"`, `},"`…) forme un morceau qui finit au
/// guillemet, et les lettres de la clé en commencent un autre. Prouvé par le test au vrai
/// tokeniseur sur les inventaires de l'entraînement.
fn last_key_cut(layer: &Value, text: &str) -> Option<usize> {
    let (key, value) = layer.as_object()?.iter().next_back()?;
    let key = serde_json::to_string(key).ok()?;
    let entry = format!("{key}:{value}}}");
    let start = text.strip_suffix(&entry).map(str::len)?;
    let opener = text[..start].chars().last()?;
    let first = key[1..].chars().next()?;
    (matches!(opener, ',' | '{') && first.is_ascii_alphabetic()).then_some(start + 1)
}

/// Coupes du texte JSON compact de `inventory` : une par couche, juste avant sa dernière clé, en
/// octets depuis le début du texte. Toutes ou aucune : une seule couche qui ne se coupe pas, ou un
/// texte d'une autre forme que `{"layers":[…],…}`, et l'inventaire se lit d'un tenant.
fn layer_cuts(inventory: &Value) -> Vec<usize> {
    let Some(layers) = inventory.get("layers").and_then(Value::as_array) else {
        return Vec::new();
    };
    let text = inventory.to_string();
    const OPEN: &str = "{\"layers\":[";
    if !text.starts_with(OPEN) {
        return Vec::new();
    }
    let mut pos = OPEN.len();
    let mut cuts = Vec::with_capacity(layers.len());
    for (i, layer) in layers.iter().enumerate() {
        if i > 0 {
            if !text[pos..].starts_with(',') {
                return Vec::new();
            }
            pos += 1;
        }
        let one = layer.to_string();
        if !text[pos..].starts_with(&one) {
            return Vec::new();
        }
        let Some(cut) = last_key_cut(layer, &one) else {
            return Vec::new();
        };
        cuts.push(pos + cut);
        pos += one.len();
    }
    cuts
}

/// La suite du message système, `lead`, l'inventaire et un retour à la ligne, découpée aux
/// frontières de couches (ticket 5.19) : les parties mises en cache (une par couche, chacune
/// finie juste avant la dernière clé de sa couche), le rang de la partie d'ancrage (celle de la
/// dernière couche de l'utilisateur, si `user_layers` est connu et non nul), et le reste. Mis bout
/// à bout, c'est le texte d'un tenant, à l'octet près.
fn inventory_parts(
    lead: &str,
    inventory: &Value,
    user_layers: Option<usize>,
) -> (Vec<String>, Option<usize>, String) {
    let whole = format!("{lead}{inventory}\n");
    let cuts = layer_cuts(inventory);
    if cuts.is_empty() {
        return (Vec::new(), None, whole);
    }
    let mut parts = Vec::with_capacity(cuts.len());
    let mut from = 0;
    for cut in cuts {
        let to = lead.len() + cut;
        parts.push(whole[from..to].to_string());
        from = to;
    }
    let anchor = user_layers
        .and_then(|n| n.checked_sub(1))
        .map(|i| i.min(parts.len() - 1));
    (parts, anchor, whole[from..].to_string())
}

/// Le message système : consignes, catalogue, exemples, puis inventaire (la partie fixe
/// d'abord, ce qui change ensuite). Tout y est du texte (jamais lu comme balise de structure).
pub fn system_text(registry: &Registry, inventory: &Value) -> String {
    fixed_system_text(registry) + &inventory_text(inventory)
}

/// Consignes du prompt court. Copie, à l'octet près, de `SYSTEM` dans `training/src/prompt.rs`,
/// le texte sur lequel le modèle ajusté a été entraîné : `cassini-ai` ne dépend pas de la chaîne
/// d'entraînement, et un test compare les deux textes sur tout le jeu engendré. N'en changer
/// rien sans ré-entraîner.
const SHORT_SYSTEM: &str = "Tu es l'agent d'un SIG hors ligne. Tu traduis la demande de l'utilisateur en un plan de commandes du registre. \
Réponds uniquement par un objet JSON compact, sans texte autour, de l'une de ces trois formes : \
{\"kind\":\"calls\",\"calls\":[{\"command\":id,\"params\":{...}}]}, \
{\"kind\":\"question\",\"question\":\"<question courte>\"} ou \
{\"kind\":\"refusal\",\"reason\":\"<raison courte>\"}.\n\
Une couche se désigne par son identifiant exact de l'inventaire, la couche produite par l'étape k (en comptant à partir de 1) par \"step:k.layer_id\". \
Un filtre sur un champ vient avant le tampon ou l'intersection. Une distance se traduit par un tampon sur la couche de référence, \
puis une intersection dont la couche cherchée est l'entrée ; plusieurs conditions se croisent par intersections successives. \
N'ajoute ni étape, ni option, ni chemin de sortie que la demande ne réclame pas.\n\
Si un mot de la demande ne désigne aucune couche ni aucun champ de l'inventaire, pose une question au lieu de deviner. \
Refuse seulement si aucune commande ne sait faire l'opération demandée.";

/// Les tours de la première tentative, dans le prompt `style`, sans savoir où finissent les
/// couches de l'utilisateur.
#[cfg(test)]
fn first_turns(
    style: PromptStyle,
    registry: &Registry,
    inventory: &Value,
    request: &str,
) -> Vec<Turn> {
    first_turns_anchored(style, registry, inventory, None, request)
}

/// Les tours de la première tentative, dans le prompt `style`. La fin de la partie fixe est un
/// point d'instantané, pour les deux prompts ; l'inventaire qui la suit est coupé aux frontières de
/// ses couches (ticket 5.19), avec un instantané juste avant la dernière clé de la dernière des
/// `user_layers` premières couches, et juste avant la dernière clé de la dernière couche.
fn first_turns_anchored(
    style: PromptStyle,
    registry: &Registry,
    inventory: &Value,
    user_layers: Option<usize>,
    request: &str,
) -> Vec<Turn> {
    let (head, lead) = match style {
        PromptStyle::Full => (fixed_system_text(registry), FULL_INVENTORY_LEAD),
        PromptStyle::Short => (SHORT_SYSTEM.to_string(), SHORT_INVENTORY_LEAD),
    };
    let (parts, anchor, rest) = inventory_parts(lead, inventory, user_layers);
    let system = Turn::with_cached_parts(Role::System, head, parts, anchor, rest);
    vec![system, Turn::new(Role::User, request)]
}

/// Les tours de la redemande : la sortie rejetée, puis la liste des écarts.
fn retry_turns(mut turns: Vec<Turn>, previous: &str, errors: &[String]) -> Vec<Turn> {
    // Bornée : une sortie rejetée peut être très longue, et la redemande doit tenir au contexte.
    let echoed: String = previous.chars().take(MAX_ECHO_CHARS).collect();
    turns.push(Turn::new(Role::Assistant, echoed));
    let shown: Vec<&str> = errors
        .iter()
        .take(MAX_REPORTED)
        .map(String::as_str)
        .collect();
    turns.push(Turn::new(
        Role::User,
        format!(
            "Ta réponse n'est pas conforme : {}. Redonne le plan corrigé, uniquement le JSON.",
            shown.join(" ; ")
        ),
    ));
    turns
}

// -- validation ------------------------------------------------------------

/// Juge une sortie du modèle. `Ok(plan)` seulement si elle est exactement du
/// JSON de plan et passe `check_plan` ; sinon la liste des écarts. Rien n'est
/// corrigé ici : AD-23.
fn judge(registry: &Registry, completion: &Completion) -> Result<Plan, Vec<String>> {
    if completion.truncated {
        return Err(vec![
            "la réponse a été coupée avant la fin : elle est trop longue".to_string(),
        ]);
    }
    let plan: Plan = serde_json::from_str(&completion.text).map_err(|e| {
        vec![format!(
            "la sortie n'est pas un plan JSON de la forme attendue ({e})"
        )]
    })?;
    match &plan {
        Plan::Refusal { reason } if reason.trim().is_empty() => {
            Err(vec!["le refus ne donne aucune raison".to_string()])
        }
        Plan::Refusal { .. } => Ok(plan),
        Plan::Question { question, .. } if question.trim().is_empty() => {
            Err(vec!["la question est vide".to_string()])
        }
        Plan::Question { .. } => Ok(plan),
        Plan::Calls { calls } if calls.is_empty() => {
            Err(vec!["le plan ne contient aucune étape".to_string()])
        }
        Plan::Calls { calls } if calls.len() > MAX_STEPS => Err(vec![format!(
            "le plan compte {} étapes, le plafond est de {MAX_STEPS}",
            calls.len()
        )]),
        Plan::Calls { calls } => {
            // Les chemins de sortie sont décidés par le noyau (AD-10) : un plan
            // qui en nomme un est rejeté, pas corrigé.
            let mut errors = Vec::new();
            for (index, call) in calls.iter().enumerate() {
                let Some(spec) = registry.spec(&call.command) else {
                    continue; // `check_plan` le signale
                };
                if let Value::Object(params) = &call.params {
                    let replay = spec.replay_param.as_ref().map(|(name, _)| name);
                    for name in spec
                        .output_params
                        .iter()
                        .chain(&spec.leave_out_param)
                        .chain(replay)
                    {
                        if params.contains_key(name) {
                            errors.push(format!(
                                "étape {} : le paramètre « {name} » est décidé par le noyau, ne l'écris pas",
                                index + 1
                            ));
                        }
                    }
                }
            }
            if !errors.is_empty() {
                return Err(errors);
            }
            registry
                .check_plan(&plan)
                .map(|()| plan.clone())
                .map_err(|e| vec![format!("étape {} : {}", e.step, e.message)])
        }
    }
}

/// Compose le plan de `request` pour le projet décrit par `inventory`.
///
/// `generate` produit la sortie du modèle (injectable pour tester sans modèle).
/// Au plus deux appels : la première sortie, puis, si elle est rejetée, une
/// seule redemande qui cite les écarts. Un refus du modèle rend `agent.refused`
/// sans redemande ; une question (un mot de la demande ne désigne rien de
/// l'inventaire) est un résultat, `Ok(Plan::Question)`, rendue sans redemande ; un plan dont
/// un point ne vient ni de `request` ni d'une étape devient la question du garde-fou
/// (`cassini_core::point::guard_coordinates`), et une sortie rejetée qui porte un tel point
/// rend cette question tout de suite, sans redemande (`guard_rejected`) ;
/// sinon deux sorties rejetées rendent `agent.plan_invalid`. Aucun plan invalide n'est
/// jamais rendu.
pub fn compose_plan(
    registry: &Registry,
    inventory: &Value,
    request: &str,
    generate: &mut dyn FnMut(&GenerationRequest) -> Result<Completion, AiError>,
) -> Result<Plan, AgentError> {
    compose_plan_with(PromptStyle::Full, registry, inventory, request, generate)
}

/// Même composition que `compose_plan`, dans le prompt `style` que le modèle attend. Seuls les
/// tours de la première tentative changent : même grammaire, même jugement, même redemande
/// unique (AD-23).
pub fn compose_plan_with(
    style: PromptStyle,
    registry: &Registry,
    inventory: &Value,
    request: &str,
    generate: &mut dyn FnMut(&GenerationRequest) -> Result<Completion, AiError>,
) -> Result<Plan, AgentError> {
    compose_plan_anchored(style, registry, inventory, None, request, generate)
}

/// Même composition que `compose_plan_with`, quand on sait que les `user_layers` premières couches
/// de `inventory` sont celles de l'utilisateur (ticket 5.19) : le moteur garde alors un instantané
/// juste avant la dernière clé de la dernière d'entre elles, repris quand seules les couches de
/// résultat qui les suivent changent. Même invite,
/// donc même plan : seuls les instantanés gardés changent.
pub fn compose_plan_anchored(
    style: PromptStyle,
    registry: &Registry,
    inventory: &Value,
    user_layers: Option<usize>,
    request: &str,
    generate: &mut dyn FnMut(&GenerationRequest) -> Result<Completion, AiError>,
) -> Result<Plan, AgentError> {
    if request.trim().is_empty() {
        return Err(AiError::EmptyPrompt.into());
    }
    let grammar = plan_grammar(registry);
    let mut turns = first_turns_anchored(style, registry, inventory, user_layers, request);
    let mut last_errors = Vec::new();
    for attempt in 0..2 {
        let completion = match generate(&GenerationRequest {
            turns: &turns,
            grammar: &grammar,
            max_new_tokens: MAX_PLAN_TOKENS,
        }) {
            Ok(c) => c,
            // La redemande ne tient pas au contexte : on s'en tient aux écarts de la première sortie.
            Err(AiError::PromptTooLong) if attempt > 0 => break,
            Err(e) => return Err(e.into()),
        };
        match judge(registry, &completion) {
            Ok(Plan::Refusal { reason }) => return Err(AgentError::Refused(reason)),
            // Garde-fou de AD-23 sur les coordonnées, sur le plan validé : un point qui ne vient
            // ni de la demande ni d'une étape devient une question à provenance `guard`.
            Ok(plan) => return Ok(guard_coordinates(registry, plan, request)),
            Err(errors) => {
                // Garde-fou de AD-23 sur une sortie rejetée : un point inventé, même mal formé,
                // rend la question tout de suite, sans redemande.
                if !completion.truncated {
                    if let Ok(rejected) = serde_json::from_str::<Plan>(&completion.text) {
                        if let Some(question) = guard_rejected(registry, &rejected, request) {
                            return Ok(question);
                        }
                    }
                }
                if attempt == 0 {
                    turns = retry_turns(turns, &completion.text, &errors);
                }
                last_errors = errors;
            }
        }
    }
    Err(AgentError::PlanInvalid(last_errors))
}

/// Résout le fichier `path` d'un modèle : une déclaration `*.modele.json` (lue par le noyau,
/// qui seul sait ce qu'est un modèle, AD-15) nomme la base, l'adaptateur et le prompt, dans
/// son propre dossier ; tout autre fichier est un GGUF seul, au long prompt. Seule façon de
/// lire un modèle, partagée par la CLI et les essais réels.
///
/// Une déclaration absente rend `ModelNotFound` à son seul nom de fichier, comme un GGUF
/// absent ; une déclaration illisible rend `BadDeclaration`, sans recopier son dossier.
pub fn resolve_model(path: &Path) -> Result<(ModelFiles, PromptStyle), AiError> {
    use cassini_core::commands::model_list::{is_declaration, read_declaration};
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if !is_declaration(&name) {
        return Ok((ModelFiles::single(path), PromptStyle::Full));
    }
    if !path.is_file() {
        return Err(AiError::ModelNotFound(name.into()));
    }
    let declaration = read_declaration(path)
        .map_err(|m| AiError::BadDeclaration(format!("« {name} » : {m}.")))?;
    let folder = path.parent().unwrap_or(Path::new(""));
    Ok((
        ModelFiles {
            model: folder.join(&declaration.model),
            adapter: declaration.adapter.as_ref().map(|a| folder.join(a)),
        },
        declaration.prompt.into(),
    ))
}

/// Générateur branché sur le modèle local `model` (contraint par la grammaire).
pub fn model_generator(
    model: PathBuf,
) -> impl FnMut(&GenerationRequest) -> Result<Completion, AiError> {
    model_files_generator(ModelFiles::single(model))
}

/// Générateur branché sur le modèle local `files` : une base et son adaptateur, ou un seul
/// fichier (contraint par la grammaire).
pub fn model_files_generator(
    files: ModelFiles,
) -> impl FnMut(&GenerationRequest) -> Result<Completion, AiError> {
    move |request| {
        generate_files(
            &files,
            request.turns,
            Some(request.grammar),
            request.max_new_tokens,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        Registry::standard()
    }

    fn inventory() -> Value {
        json!({"layers": [{"layer_id": "gares_1", "name": "Gares"}]})
    }

    fn done(text: &str) -> Completion {
        Completion {
            text: text.to_string(),
            truncated: false,
        }
    }

    const GOOD: &str = r#"{"kind":"calls","calls":[{"command":"analysis.buffer","params":{"layer_id":"gares_1","distance":{"value":300,"unit":"m"}}}]}"#;
    const BAD: &str = r#"{"kind":"calls","calls":[{"command":"analysis.buffer","params":{"layer_id":"gares_1","distance":{"value":-3,"unit":"m"}}}]}"#;

    /// Générateur scripté : rend les sorties dans l'ordre et note les tours reçus.
    fn compose(outputs: Vec<Completion>) -> (Result<Plan, AgentError>, Vec<Vec<Turn>>) {
        let mut seen: Vec<Vec<Turn>> = Vec::new();
        let mut queue = outputs.into_iter();
        let result = compose_plan(&registry(), &inventory(), "Tampon de 300 m.", &mut |r| {
            seen.push(r.turns.to_vec());
            Ok(queue.next().expect("plus de sortie scriptée"))
        });
        (result, seen)
    }

    #[test]
    fn a_valid_plan_is_returned_after_one_call() {
        let (result, seen) = compose(vec![done(GOOD)]);
        let plan = result.unwrap();
        assert_eq!(seen.len(), 1);
        assert!(matches!(&plan, Plan::Calls { calls } if calls.len() == 1));
        assert_eq!(registry().check_plan(&plan), Ok(()));
    }

    #[test]
    fn a_refusal_carries_its_reason_without_a_second_call() {
        let (result, seen) = compose(vec![done(
            r#"{"kind":"refusal","reason":"Hors catalogue."}"#,
        )]);
        let err = result.unwrap_err();
        assert_eq!(err, AgentError::Refused("Hors catalogue.".to_string()));
        assert_eq!(err.code(), "agent.refused");
        assert_eq!(seen.len(), 1);
    }

    const QUESTION: &str =
        r#"{"kind":"question","question":"Aucune couche de pharmacies : laquelle utiliser ?"}"#;

    #[test]
    fn a_question_is_returned_as_a_result_after_one_call() {
        let (result, seen) = compose(vec![done(QUESTION)]);
        assert_eq!(
            result.unwrap(),
            Plan::Question {
                question: "Aucune couche de pharmacies : laquelle utiliser ?".to_string(),
                source: cassini_core::plan::QuestionSource::Model,
            }
        );
        assert_eq!(seen.len(), 1);
    }

    const ISOCHRONE: &str = r#"{"kind":"calls","calls":[{"command":"analysis.isochrone","params":{"layer_id":"routes_1","origin":{"x":652000,"y":6850000,"epsg":2154},"mode":"walk","duration":{"value":10,"unit":"min"}}}]}"#;

    /// Compose `request` sur un réseau, avec une sortie scriptée du modèle.
    fn compose_request(request: &str, output: &str) -> Result<Plan, AgentError> {
        let inventory = json!({"layers": [{"layer_id": "routes_1", "name": "Routes"}]});
        let mut calls = 0;
        compose_plan(&registry(), &inventory, request, &mut |_| {
            calls += 1;
            assert_eq!(calls, 1, "le garde-fou ne redemande pas");
            Ok(done(output))
        })
    }

    #[test]
    fn a_point_absent_from_the_request_becomes_the_guard_question() {
        let got = compose_request("Zone à 10 min à pied depuis la mairie.", ISOCHRONE).unwrap();
        assert_eq!(
            got,
            Plan::Question {
                question: cassini_core::point::GUARD_QUESTION.to_string(),
                source: cassini_core::plan::QuestionSource::Guard,
            }
        );
    }

    #[test]
    fn a_rejected_output_with_an_invented_point_is_guarded_without_a_retry() {
        // Le cas de la recette dans QGIS : une origine inventée sans `epsg`.
        let partial = r#"{"kind":"calls","calls":[{"command":"analysis.isochrone","params":{"layer_id":"routes_1","origin":{"x":2.458,"y":48.658},"mode":"walk","duration":{"value":10,"unit":"min"}}}]}"#;
        let got = compose_request(
            "Trouve les parcelles à 10 minutes à pied de la gare.",
            partial,
        );
        assert_eq!(
            got.unwrap(),
            Plan::Question {
                question: cassini_core::point::GUARD_QUESTION.to_string(),
                source: cassini_core::plan::QuestionSource::Guard,
            }
        );
    }

    #[test]
    fn a_point_given_in_the_request_leaves_the_plan_unchanged() {
        let request = "Zone à 10 min à pied depuis x 652 000, y 6 850 000 en EPSG:2154.";
        let got = compose_request(request, ISOCHRONE).unwrap();
        assert_eq!(got, serde_json::from_str::<Plan>(ISOCHRONE).unwrap());
    }

    #[test]
    fn an_empty_question_is_asked_again_then_the_valid_plan_is_returned() {
        let empty = r#"{"kind":"question","question":"  "}"#;
        let (result, seen) = compose(vec![done(empty), done(GOOD)]);
        let expected: Plan = serde_json::from_str(GOOD).unwrap();
        assert_eq!(result.unwrap(), expected);
        assert_eq!(seen.len(), 2);
        assert!(
            seen[1][3].text.contains("question est vide"),
            "{}",
            seen[1][3].text
        );
        let (result, seen) = compose(vec![done(empty), done(empty)]);
        assert_eq!(result.unwrap_err().code(), "agent.plan_invalid");
        assert_eq!(seen.len(), 2);
    }

    #[test]
    fn invalid_then_valid_returns_the_second_plan_as_is() {
        let (result, seen) = compose(vec![done(BAD), done(GOOD)]);
        let expected: Plan = serde_json::from_str(GOOD).unwrap();
        assert_eq!(result.unwrap(), expected);
        assert_eq!(seen.len(), 2);
        // La redemande cite la sortie rejetée telle quelle, puis les écarts.
        let retry = &seen[1];
        assert_eq!(retry.len(), 4);
        assert_eq!(retry[2], Turn::new(Role::Assistant, BAD));
        assert_eq!(retry[3].role, Role::User);
        assert!(retry[3].text.contains("étape 1"), "{}", retry[3].text);
        assert_eq!(&retry[..2], &seen[0][..]);
    }

    #[test]
    fn a_retry_that_overflows_the_context_gives_plan_invalid_with_the_first_deviations() {
        let mut calls = 0;
        let err = compose_plan(&registry(), &inventory(), "x", &mut |_| {
            calls += 1;
            if calls == 1 {
                Ok(done(BAD))
            } else {
                Err(AiError::PromptTooLong)
            }
        })
        .unwrap_err();
        assert_eq!(err.code(), "agent.plan_invalid");
        assert!(matches!(&err, AgentError::PlanInvalid(e) if !e.is_empty()));
        // Dès la première tentative, un dépassement reste une erreur de la couche IA.
        let first = compose_plan(&registry(), &inventory(), "x", &mut |_| {
            Err(AiError::PromptTooLong)
        })
        .unwrap_err();
        assert_eq!(first.code(), "ai.prompt_too_long");
    }

    #[test]
    fn the_rejected_output_echoed_in_the_retry_is_bounded() {
        let long = format!("{}{}", "x".repeat(10_000), BAD);
        let (_, seen) = compose(vec![done(&long), done(GOOD)]);
        assert!(seen[1][2].text.chars().count() <= MAX_ECHO_CHARS);
    }

    #[test]
    fn invalid_twice_returns_no_plan() {
        let (result, seen) = compose(vec![done(BAD), done(BAD)]);
        let err = result.unwrap_err();
        assert_eq!(err.code(), "agent.plan_invalid");
        assert!(matches!(&err, AgentError::PlanInvalid(e) if !e.is_empty()));
        assert_eq!(seen.len(), 2, "une seule redemande");
    }

    #[test]
    fn unreadable_or_truncated_output_is_invalid_not_repaired() {
        for first in [
            done("voici le plan : {}"),
            done(r#"{"kind":"calls","calls":[{"command":"analysis.bu"#),
            Completion {
                text: GOOD.to_string(),
                truncated: true,
            },
        ] {
            let (result, seen) = compose(vec![first, done(GOOD)]);
            assert!(result.is_ok());
            assert_eq!(seen.len(), 2);
        }
        let (result, _) = compose(vec![done("n'importe quoi"), done("{")]);
        assert_eq!(result.unwrap_err().code(), "agent.plan_invalid");
    }

    #[test]
    fn a_refusal_without_reason_and_an_empty_plan_are_invalid() {
        for bad in [
            r#"{"kind":"refusal","reason":"  "}"#,
            r#"{"kind":"calls","calls":[]}"#,
        ] {
            let (result, _) = compose(vec![done(bad), done(bad)]);
            assert_eq!(result.unwrap_err().code(), "agent.plan_invalid", "{bad}");
        }
    }

    #[test]
    fn an_unknown_command_is_invalid() {
        let bad = r#"{"kind":"calls","calls":[{"command":"no.such","params":{}}]}"#;
        let (result, _) = compose(vec![done(bad), done(bad)]);
        let AgentError::PlanInvalid(errors) = result.unwrap_err() else {
            panic!("agent.plan_invalid attendu");
        };
        assert!(errors[0].contains("no.such"), "{errors:?}");
    }

    #[test]
    fn an_output_path_chosen_by_the_model_is_rejected() {
        let bad = r#"{"kind":"calls","calls":[{"command":"analysis.buffer","params":{"layer_id":"gares_1","distance":{"value":300,"unit":"m"},"out":"C:/Windows/x.gpkg"}}]}"#;
        let (result, seen) = compose(vec![done(bad), done(GOOD)]);
        assert!(result.is_ok());
        assert!(seen[1][3].text.contains("« out »"), "{}", seen[1][3].text);
    }

    #[test]
    fn a_leave_out_chosen_by_the_model_is_rejected() {
        let bad = r#"{"kind":"calls","calls":[{"command":"layout.export_pdf","params":{"title":"Carte","leave_out":["gares_1"]}}]}"#;
        let (result, seen) = compose(vec![done(bad), done(GOOD)]);
        assert!(result.is_ok());
        assert!(
            seen[1][3].text.contains("« leave_out »"),
            "{}",
            seen[1][3].text
        );
        assert!(seen[1][3].text.contains("décidé par le noyau"));
    }

    #[test]
    fn a_center_chosen_by_the_model_is_rejected() {
        let bad = r#"{"kind":"calls","calls":[{"command":"layout.export_pdf","params":{"title":"Carte","scale":25000,"center":{"x":650000,"y":6850000,"epsg":2154}}}]}"#;
        let errors = judge(&registry(), &done(bad)).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("« center »") && e.contains("décidé par le noyau")),
            "{errors:?}"
        );
        // Le centre est un `Point` que la demande n'écrit pas : le garde-fou pose sa question
        // au lieu d'une redemande (AD-23), et le plan n'est jamais rendu.
        let (result, _) = compose(vec![done(bad), done(GOOD)]);
        assert!(matches!(result, Ok(Plan::Question { .. })), "{result:?}");
    }

    #[test]
    fn too_many_steps_are_invalid() {
        let step = r#"{"command":"project.inventory","params":{}}"#;
        let many = format!(
            r#"{{"kind":"calls","calls":[{}]}}"#,
            vec![step; MAX_STEPS + 1].join(",")
        );
        let (result, _) = compose(vec![done(&many), done(&many)]);
        assert_eq!(result.unwrap_err().code(), "agent.plan_invalid");
    }

    #[test]
    fn an_empty_request_is_refused_before_any_call() {
        let mut called = false;
        let err = compose_plan(&registry(), &inventory(), "  ", &mut |_| {
            called = true;
            Ok(done(GOOD))
        })
        .unwrap_err();
        assert_eq!(err, AgentError::Ai(AiError::EmptyPrompt));
        assert_eq!(err.code(), "ai.empty_prompt");
        assert!(!called);
    }

    #[test]
    fn a_model_error_is_propagated_with_its_ai_code() {
        let err = compose_plan(&registry(), &inventory(), "x", &mut |_| {
            Err(AiError::ModelNotFound(PathBuf::from("absent.gguf")))
        })
        .unwrap_err();
        assert_eq!(err.code(), "ai.model_not_found");
    }

    #[test]
    fn the_catalog_comes_from_the_real_registry_and_hides_output_paths() {
        let text = system_text(&registry(), &inventory());
        let reg = registry();
        for spec in offered(&reg) {
            assert!(
                text.contains(&format!("\"id\":\"{}\"", spec.id)),
                "{}",
                spec.id
            );
            assert!(text.contains(&spec.description_fr), "{}", spec.id);
        }
        let buffer = catalog_entry(reg.spec("analysis.buffer").unwrap());
        assert!(buffer["params"]["properties"].get("out").is_none());
        assert!(buffer["params"]["properties"].get("layer_id").is_some());
        // Le paramètre que seul le rejeu remplit reste caché au modèle (ticket 5.36).
        let export = catalog_entry(reg.spec("layout.export_pdf").unwrap());
        assert!(export["params"]["properties"].get("leave_out").is_none());
        // Le centre d'un export à l'échelle, rempli au rejeu seulement, aussi (ticket 5.38).
        assert!(export["params"]["properties"].get("center").is_none());
        assert!(export["params"]["properties"].get("scale").is_some());
        assert!(!export.to_string().contains("\"center\""), "{export}");
        assert!(export["params"].get("$defs").is_none(), "{export}");
        assert!(export["params"]["properties"].get("title").is_some());
        // La marque du `Point` reste au noyau : le modèle ne la voit pas.
        assert!(!text.contains("x-cassini"));
        // Un `RasterPath` reste une chaîne pour le modèle, sa description gardée (ticket 11.5).
        for id in [
            "terrain.slope",
            "terrain.viewshed",
            "terrain.line_of_sight",
            "terrain.aspect",
            "terrain.hillshade",
            "terrain.viewshed_cumulative",
        ] {
            let spec = reg.spec(id).unwrap();
            let entry = catalog_entry(spec);
            let dem = &entry["params"]["properties"]["dem"];
            let described = spec.params_schema["properties"]["dem"]["description"].clone();
            assert_eq!(
                dem,
                &json!({"type": "string", "description": described}),
                "{id}"
            );
            assert!(entry["params"]
                .get("$defs")
                .is_none_or(|d| d.get("RasterPath").is_none()));
        }
        assert!(!text.contains("RasterPath"));
        // La liste d'observateurs du champ de vision cumulé reste bornée pour le modèle (AD-8).
        let cumulative = catalog_entry(reg.spec("terrain.viewshed_cumulative").unwrap());
        let observers = &cumulative["params"]["properties"]["observers"];
        assert_eq!(observers["maxItems"], json!(32), "{cumulative}");
        assert_eq!(observers["minItems"], json!(1), "{cumulative}");
        let isochrone = catalog_entry(reg.spec("analysis.isochrone").unwrap());
        assert!(isochrone["params"]["$defs"]["Point"].is_object());
        assert!(!isochrone.to_string().contains("x-cassini"));
        assert!(text.contains("gares_1"), "l'inventaire est fourni");
        assert!(text.contains("\"kind\":\"refusal\""), "un exemple de refus");
        assert!(
            text.contains("\"kind\":\"question\""),
            "un exemple de question"
        );
    }

    /// Ce qu'un tour lit après sa tête : ses parties mises en cache, puis son reste.
    fn after_head(turn: &Turn) -> String {
        turn.cached_parts.concat() + &turn.text
    }

    #[test]
    fn the_fixed_head_is_the_same_for_two_inventories_and_the_inventory_comes_last() {
        let other = json!({"layers": [{"layer_id": "routes_9", "name": "Routes"}]});
        let a = first_turns(PromptStyle::Full, &registry(), &inventory(), "x");
        let b = first_turns(PromptStyle::Full, &registry(), &other, "x");
        assert_eq!(a[0].role, Role::System);
        assert!(!a[0].cached_head.is_empty());
        assert_eq!(a[0].cached_head, b[0].cached_head);
        assert_ne!(a[0].text, b[0].text);
        assert!(!a[0].cached_head.contains("gares_1"));
        assert!(after_head(&a[0]).contains("gares_1"));
        // Le tour se lit comme le message système entier.
        let whole = system_text(&registry(), &inventory());
        assert_eq!(format!("{}{}", a[0].cached_head, after_head(&a[0])), whole);
        let examples = whole.find("EXEMPLES :").unwrap();
        let inventory = whole.find("INVENTAIRE DU PROJET :").unwrap();
        let catalog = whole.find("CATALOGUE :").unwrap();
        assert!(catalog < examples && examples < inventory, "{whole}");
        // La redemande garde la même tête.
        let retry = retry_turns(a.clone(), "{}", &["écart".to_string()]);
        assert_eq!(retry[0], a[0]);
    }

    #[test]
    fn the_examples_are_valid_plans_of_the_real_registry() {
        let reg = registry();
        for (inv, sentence, plan) in examples() {
            let parsed: Plan = serde_json::from_value(plan.clone()).unwrap();
            assert_eq!(reg.check_plan(&parsed), Ok(()), "{sentence}");
            // Chaque identifiant de couche cité vient de l'inventaire de l'exemple.
            let ids: Vec<&str> = inv["layers"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|l| l["layer_id"].as_str())
                .collect();
            if let Plan::Calls { calls } = &parsed {
                for call in calls {
                    if let Some(id) = call.params["layer_id"].as_str() {
                        assert!(id.starts_with("step:") || ids.contains(&id), "{id}");
                    }
                }
            }
        }
    }

    /// Un plan à deux étapes enchaînées par `step:<k>.<champ>` passe `check_plan`.
    #[test]
    fn a_chained_plan_of_the_real_registry_is_valid() {
        let chained: Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "analysis.select_by_attribute",
             "params": {"layer_id": "routes_2c91", "field": "classe", "operator": "eq", "value": "nationale"}},
            {"command": "analysis.buffer",
             "params": {"layer_id": "step:1.layer_id", "distance": {"value": 200, "unit": "m"}}}]}))
        .unwrap();
        assert_eq!(registry().check_plan(&chained), Ok(()));
    }

    #[test]
    fn the_grammar_lists_every_command_and_the_three_plan_shapes() {
        let reg = registry();
        let g = plan_grammar(&reg);
        for spec in offered(&reg) {
            assert!(
                g.contains(&format!("\"\\\"{}\\\"\"", spec.id)),
                "{}",
                spec.id
            );
        }
        assert!(g.starts_with("root ::= calls | refusal | question\n"));
        assert!(g.contains("{\\\"kind\\\":\\\"calls\\\""));
        // Les règles entières, telles qu'émises : une fermeture cassée ne passe pas.
        let lines: Vec<&str> = g.lines().collect();
        assert!(
            lines.contains(&r#"refusal ::= "{\"kind\":\"refusal\",\"reason\":" string "}""#),
            "{g}"
        );
        assert!(
            lines.contains(&r#"question ::= "{\"kind\":\"question\",\"question\":" string "}""#),
            "{g}"
        );
        // Toute règle citée est définie.
        for rule in [
            "calls", "step", "command", "refusal", "question", "value", "object", "pair", "array",
            "string", "number",
        ] {
            assert!(
                g.contains(&format!("\n{rule} ::= ")) || g.starts_with(&format!("{rule} ::= ")),
                "{rule}"
            );
        }
    }

    /// Une commande que le noyau dicte seul (ticket 4.9) n'est jamais offerte au modèle : le
    /// prompt et la grammaire sont, à l'octet, ceux d'un registre qui ne la connaît pas. Le modèle
    /// installé reste donc valide.
    #[test]
    fn a_dictated_command_leaves_the_prompt_and_the_grammar_unchanged() {
        let full = registry();
        let dictated: Vec<String> = full
            .specs()
            .filter(|s| s.dictated_by_core)
            .map(|s| s.id.clone())
            .collect();
        assert_eq!(dictated, ["layer.open_raster"]);
        let mut bare = registry();
        for id in &dictated {
            assert!(bare.remove(id));
        }
        assert_eq!(plan_grammar(&full), plan_grammar(&bare));
        assert_eq!(fixed_system_text(&full), fixed_system_text(&bare));
        assert_eq!(
            system_text(&full, &inventory()),
            system_text(&bare, &inventory())
        );
        assert!(!plan_grammar(&full).contains("layer.open_raster"));
        assert!(!fixed_system_text(&full).contains("layer.open_raster"));
    }
    // -- prompt court (ticket 5.10, tâche 3) ---------------------------------

    /// Le texte exact des tours, tel que `chat_tokens` l'assemble (un « jeton » par segment).
    fn chat_text(turns: &[Turn]) -> String {
        crate::chat_tokens_with(turns, |text, _special| vec![text.to_string()])
            .0
            .concat()
    }

    #[test]
    fn the_short_instructions_are_those_of_the_training() {
        assert_eq!(SHORT_SYSTEM, cassini_training::prompt::SYSTEM);
    }

    #[test]
    fn the_short_prompt_is_the_training_prompt_byte_for_byte_on_the_whole_generated_set() {
        let (examples, _) = cassini_training::generate(
            cassini_training::DEFAULT_SEED,
            cassini_training::DEFAULT_COUNT,
        );
        assert!(!examples.is_empty());
        let reg = registry();
        for e in &examples {
            let turns = first_turns(PromptStyle::Short, &reg, &e.inventory, &e.request);
            assert_eq!(
                chat_text(&turns),
                cassini_training::prompt::render_prompt(&e.inventory, &e.request),
                "{}",
                e.request
            );
        }
    }

    /// La redemande vue à l'entraînement est celle de `ai` : pour chaque exemple de redemande du
    /// jeu engendré, la sortie rejetée est jugée ici avec les mêmes écarts, et les tours de
    /// `retry_turns` valent le prompt d'entraînement, octet pour octet.
    #[test]
    fn the_retry_prompt_is_the_training_retry_prompt_byte_for_byte() {
        let (examples, _) = cassini_training::generate(
            cassini_training::DEFAULT_SEED,
            cassini_training::DEFAULT_COUNT,
        );
        let reg = registry();
        let mut checked = 0;
        for e in &examples {
            let Some(retry) = &e.retry else {
                continue;
            };
            let errors = judge(&reg, &done(&retry.rejected)).unwrap_err();
            assert_eq!(errors, retry.errors, "{}", e.request);
            let first = first_turns(PromptStyle::Short, &reg, &e.inventory, &e.request);
            let turns = retry_turns(first, &retry.rejected, &errors);
            assert_eq!(
                chat_text(&turns),
                cassini_training::prompt_text(e),
                "{}",
                e.request
            );
            checked += 1;
        }
        assert!(checked * 25 >= examples.len(), "{checked} redemandes");
        assert_eq!(cassini_training::prompt::MAX_ECHO_CHARS, MAX_ECHO_CHARS);
        assert_eq!(cassini_training::prompt::MAX_REPORTED, MAX_REPORTED);
        // Bornes et plafond : le texte de redemande des deux côtés, sur des écarts nombreux.
        let many: Vec<String> = (0..20).map(|i| format!("étape {i} : écart")).collect();
        let long = "x".repeat(MAX_ECHO_CHARS + 50);
        let inv = inventory();
        let turns = retry_turns(
            first_turns(PromptStyle::Short, &reg, &inv, "x"),
            &long,
            &many,
        );
        assert_eq!(
            chat_text(&turns),
            cassini_training::prompt::render_retry_prompt(&inv, "x", &long, &many)
        );
    }

    #[test]
    fn the_short_head_is_fixed_holds_no_catalog_and_the_retry_keeps_it() {
        let other = json!({"layers": [{"layer_id": "routes_9", "name": "Routes"}]});
        let a = first_turns(PromptStyle::Short, &registry(), &inventory(), "x");
        let b = first_turns(PromptStyle::Short, &registry(), &other, "x");
        assert_eq!(a[0].role, Role::System);
        assert_eq!(a[0].cached_head, b[0].cached_head);
        assert_ne!(a[0].text, b[0].text);
        assert!(after_head(&a[0]).contains("gares_1"));
        assert!(!a[0].cached_head.contains("CATALOGUE") && !a[0].cached_head.contains("EXEMPLES"));
        assert_eq!(a[1], Turn::new(Role::User, "x"));
        let full = first_turns(PromptStyle::Full, &registry(), &inventory(), "x");
        assert!(
            a[0].cached_head.len() * 5 < full[0].cached_head.len(),
            "court"
        );
        let retry = retry_turns(a.clone(), "{}", &["écart".to_string()]);
        assert_eq!(retry[0], a[0]);
        assert_eq!(PromptStyle::default(), PromptStyle::Full);
        assert_eq!(PromptStyle::from(PromptKind::Short), PromptStyle::Short);
        assert_eq!(PromptStyle::from(PromptKind::Full), PromptStyle::Full);
    }

    #[test]
    fn the_short_prompt_keeps_the_grammar_the_judge_and_the_single_retry() {
        let mut seen: Vec<(Vec<Turn>, String)> = Vec::new();
        let mut queue = vec![done(BAD), done(GOOD)].into_iter();
        let result = compose_plan_with(
            PromptStyle::Short,
            &registry(),
            &inventory(),
            "Tampon de 300 m.",
            &mut |r| {
                seen.push((r.turns.to_vec(), r.grammar.to_string()));
                Ok(queue.next().expect("plus de sortie scriptée"))
            },
        );
        let expected: Plan = serde_json::from_str(GOOD).unwrap();
        assert_eq!(result.unwrap(), expected);
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].1, plan_grammar(&registry()));
        assert_eq!(seen[1].1, seen[0].1);
        assert_eq!(
            seen[0].0,
            first_turns(
                PromptStyle::Short,
                &registry(),
                &inventory(),
                "Tampon de 300 m."
            )
        );
        assert_eq!(seen[1].0[2], Turn::new(Role::Assistant, BAD));
        assert!(seen[1].0[3].text.contains("étape 1"));
        // Le long prompt reste celui de `compose_plan`.
        let mut full = Vec::new();
        let _ = compose_plan(&registry(), &inventory(), "Tampon de 300 m.", &mut |r| {
            full.push(r.turns.to_vec());
            Ok(done(GOOD))
        });
        assert_eq!(
            full[0],
            first_turns(
                PromptStyle::Full,
                &registry(),
                &inventory(),
                "Tampon de 300 m."
            )
        );
    }

    // -- coupes aux frontières de couches (ticket 5.19) ----------------------

    fn epreuve() -> Value {
        serde_json::from_str(include_str!(
            "../../tests/epreuve-modele-local/inventory.json"
        ))
        .unwrap()
    }

    /// L'inventaire de l'épreuve réduit à ses `n` premières couches.
    fn first_layers(n: usize) -> Value {
        let mut inv = epreuve();
        inv["layers"].as_array_mut().unwrap().truncate(n);
        inv
    }

    #[test]
    fn the_inventory_is_cut_once_per_layer_just_before_its_last_key() {
        let inv = epreuve();
        let n = inv["layers"].as_array().unwrap().len();
        let (parts, anchor, rest) = inventory_parts(SHORT_INVENTORY_LEAD, &inv, Some(3));
        assert_eq!(parts.len(), n);
        assert_eq!(anchor, Some(2));
        // Bout à bout : le texte d'un tenant, à l'octet près.
        assert_eq!(
            parts.concat() + &rest,
            format!("{SHORT_INVENTORY_LEAD}{inv}\n")
        );
        // Chaque partie finit au guillemet qui ouvre la dernière clé d'une couche ; la suivante
        // commence par les lettres de cette clé.
        for part in &parts {
            assert!(part.ends_with(",\"") || part.ends_with("{\""), "{part}");
        }
        for next in parts[1..].iter().chain([&rest]) {
            assert!(next.starts_with("source\":"), "{next}");
        }
        // Ancrage : aucun sans le nombre de couches de l'utilisateur, ni à zéro ; borné à la
        // dernière partie au-delà.
        assert_eq!(inventory_parts(SHORT_INVENTORY_LEAD, &inv, None).1, None);
        assert_eq!(inventory_parts(SHORT_INVENTORY_LEAD, &inv, Some(0)).1, None);
        assert_eq!(
            inventory_parts(SHORT_INVENTORY_LEAD, &inv, Some(99)).1,
            Some(n - 1)
        );
    }

    #[test]
    fn one_more_layer_keeps_every_part_of_the_previous_inventory() {
        let n = epreuve()["layers"].as_array().unwrap().len();
        let (before, _, _) = inventory_parts(SHORT_INVENTORY_LEAD, &first_layers(n - 1), Some(2));
        let (after, _, _) = inventory_parts(SHORT_INVENTORY_LEAD, &first_layers(n), Some(2));
        assert_eq!(&after[..before.len()], &before[..]);
        // Une couche de résultat retirée en tête des résultats : les parties de l'utilisateur
        // restent, jusqu'à l'ancrage compris.
        let mut removed = epreuve();
        removed["layers"].as_array_mut().unwrap().remove(2);
        let (full, anchor, _) = inventory_parts(SHORT_INVENTORY_LEAD, &epreuve(), Some(2));
        let (less, _, _) = inventory_parts(SHORT_INVENTORY_LEAD, &removed, Some(2));
        let anchor = anchor.unwrap();
        assert_eq!(&less[..=anchor], &full[..=anchor]);
        assert_ne!(less[anchor + 1], full[anchor + 1]);
    }

    #[test]
    fn an_inventory_of_another_shape_is_read_in_one_piece() {
        for inv in [
            json!({"layers": []}),
            json!({"project": {}}),
            json!({"layers": [{}]}),
            json!({"layers": [{"layer_id": "a", "~x": 1}]}),
            json!({"layers": [{"layer_id": "a"}, "pas une couche"]}),
        ] {
            let (parts, anchor, rest) = inventory_parts(FULL_INVENTORY_LEAD, &inv, Some(1));
            assert!(parts.is_empty(), "{inv}");
            assert_eq!(anchor, None);
            assert_eq!(rest, inventory_text(&inv));
        }
        // Une seule clé : la coupe suit `{"`.
        let one = json!({"layers": [{"layer_id": "a"}]});
        let (parts, _, rest) = inventory_parts(FULL_INVENTORY_LEAD, &one, None);
        assert_eq!(parts, [format!("{FULL_INVENTORY_LEAD}{{\"layers\":[{{\"")]);
        assert_eq!(rest, "layer_id\":\"a\"}]}\n");
    }

    #[test]
    fn the_whole_generated_set_is_cut_at_every_layer() {
        let (examples, _) = cassini_training::generate(
            cassini_training::DEFAULT_SEED,
            cassini_training::DEFAULT_COUNT,
        );
        for e in &examples {
            let layers = e.inventory["layers"].as_array().map_or(0, Vec::len);
            assert_eq!(layer_cuts(&e.inventory).len(), layers, "{}", e.inventory);
        }
    }

    #[test]
    fn the_anchored_composition_reads_the_same_prompt_with_its_cuts() {
        let inv = epreuve();
        let mut seen = Vec::new();
        let _ = compose_plan_anchored(
            PromptStyle::Short,
            &registry(),
            &inv,
            Some(3),
            "Tampon de 300 m.",
            &mut |r| {
                seen.push(r.turns.to_vec());
                Ok(done(GOOD))
            },
        );
        let turns = &seen[0];
        assert_eq!(turns[0].anchor, Some(2));
        assert_eq!(turns[0].cached_parts.len(), 7);
        assert_eq!(
            chat_text(turns),
            chat_text(&first_turns(
                PromptStyle::Short,
                &registry(),
                &inv,
                "Tampon de 300 m."
            ))
        );
        assert_eq!(
            chat_text(turns),
            cassini_training::prompt::render_prompt(&inv, "Tampon de 300 m.")
        );
    }

    /// Avec le vrai modèle (ticket 5.19) : une couche de plus reprend l'instantané de la dernière
    /// frontière de l'invite d'avant ; à invite égale, la sortie est la même sans instantané ; un
    /// résultat retiré en tête reprend l'instantané d'ancrage. Ignoré sans `CASSINI_TEST_MODEL`
    /// (GGUF ou déclaration) :
    ///
    ///   cargo test -p cassini-ai --release --features cuda --lib -- --ignored --nocapture reprise_reel
    #[test]
    #[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
    fn reprise_reel_one_more_layer_resumes_at_the_last_boundary() {
        let Some(path) = std::env::var_os("CASSINI_TEST_MODEL") else {
            eprintln!("CASSINI_TEST_MODEL absent : essai ignoré.");
            return;
        };
        let (files, style) = resolve_model(Path::new(&path)).unwrap();
        let reg = registry();
        let grammar = plan_grammar(&reg);
        let request = "Fais une zone tampon de 300 m autour des gares.";
        let users = 3;
        let n = epreuve()["layers"].as_array().unwrap().len();
        let before = first_layers(n - 1);
        let after = epreuve();
        let mut removed = epreuve();
        removed["layers"].as_array_mut().unwrap().remove(users);
        let turns = |inv: &Value| first_turns_anchored(style, &reg, inv, Some(users), request);
        let run = |inv: &Value| {
            crate::generate_files_measured(
                &files,
                &turns(inv),
                Some(&grammar),
                MAX_PLAN_TOKENS,
                None,
            )
            .unwrap()
        };
        let engine = || {
            crate::engine()
                .unwrap()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
        };
        engine().loaded = None;
        // Points de coupe, avec le vrai découpeur.
        let cut = |inv: &Value, slot: usize| {
            let guard = engine();
            let vocab = guard.loaded.as_ref().unwrap().model.vocab();
            let (_, cuts) = crate::chat_tokens(&vocab, &turns(inv));
            cuts.iter().find(|c| c.slot == Some(slot)).unwrap().at
        };
        let (c1, t1) = run(&before);
        eprintln!("avant : {t1:?}\n  {}", c1.text);
        let (c2, t2) = run(&after);
        eprintln!("une couche de plus : {t2:?}\n  {}", c2.text);
        assert_eq!(t2.cached_tokens, cut(&before, crate::SLOT_LAST));
        // Sans instantané, même invite : même sortie, tout est relu.
        engine().loaded.as_mut().unwrap().checkpoints.clear();
        let (c3, t3) = run(&after);
        eprintln!("sans instantané : {t3:?}");
        assert_eq!(t3.cached_tokens, 0);
        assert_eq!(c3, c2, "à invite égale, la sortie gloutonne est la même");
        // Le plus ancien résultat retiré : reprise à l'ancrage (avant la dernière clé de la
        // dernière couche de l'utilisateur).
        let (_, t4) = run(&removed);
        eprintln!("un résultat retiré : {t4:?}");
        assert_eq!(t4.cached_tokens, cut(&after, crate::SLOT_ANCHOR));
        assert!(t4.cached_tokens < t2.cached_tokens);
        engine().loaded = None;
    }

    /// Avec le vrai modèle (la base du modèle ajusté) : les jetons que `ai` lit pour le prompt
    /// court, découpé par segments, sont ceux du prompt d'entraînement découpé d'un tenant, balises
    /// comprises : pour 300 premiers essais, puis (depuis 5.15) pour chaque redemande du jeu
    /// entier (`DEFAULT_COUNT`). Ignoré sans `CASSINI_TEST_MODEL`.
    #[test]
    fn the_short_prompt_tokens_are_those_of_the_training_prompt_read_in_one_piece() {
        use llama_cpp_2::model::params::LlamaModelParams;
        use llama_cpp_2::model::LlamaModel;
        let Some(path) = std::env::var_os("CASSINI_TEST_MODEL") else {
            return;
        };
        let mut guard = crate::engine()
            .unwrap()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // Un seul modèle en mémoire : celui que le moteur garde est libéré d'abord.
        guard.loaded = None;
        let llama = LlamaModel::load_from_file(
            &guard.backend,
            std::path::Path::new(&path),
            &LlamaModelParams::default(),
        )
        .unwrap();
        let vocab = llama.vocab();
        let (examples, _) = cassini_training::generate(cassini_training::DEFAULT_SEED, 300);
        let reg = registry();
        // Le point d'instantané de la tête fixe : la fin de `turn_head("system")` et des consignes,
        // découpées comme `chat_tokens` les découpe.
        let head_end = vocab
            .tokenize(crate::turn_head("system").as_bytes(), false, true)
            .len()
            + vocab.tokenize(SHORT_SYSTEM.as_bytes(), false, false).len();
        let mut checked = 0;
        let mut cut_layers = 0;
        for e in &examples {
            let turns = first_turns(PromptStyle::Short, &reg, &e.inventory, &e.request);
            let (ours, points) = crate::chat_tokens(&vocab, &turns);
            let text = cassini_training::prompt::render_prompt(&e.inventory, &e.request);
            let theirs = vocab.tokenize(text.as_bytes(), false, true);
            assert_eq!(ours, theirs, "{}", e.request);
            // La tête fixe se découpe toujours de même : son point d'instantané est commun.
            assert_eq!(points[0].at, head_end, "{}", e.request);
            // Ticket 5.19 : une coupe par couche, entre la tête et la fin du tour système ; les
            // jetons lus par parties sont ceux du prompt lu d'un tenant (ci-dessus).
            let layers = e.inventory["layers"].as_array().map_or(0, Vec::len);
            assert_eq!(points.len(), layers + 2, "{}", e.request);
            cut_layers += layers;
            checked += 1;
        }
        eprintln!("parité au jeton : {checked} prompts identiques, {cut_layers} coupes de couches");
        // La redemande : chaque exemple de redemande du jeu entier, tours de `retry_turns`
        // découpés par segments contre le prompt de redemande d'entraînement lu d'un tenant.
        let (all, _) = cassini_training::generate(
            cassini_training::DEFAULT_SEED,
            cassini_training::DEFAULT_COUNT,
        );
        let mut retries = 0;
        for e in &all {
            let Some(retry) = &e.retry else {
                continue;
            };
            let errors = judge(&reg, &done(&retry.rejected)).expect_err(&e.request);
            assert_eq!(errors, retry.errors, "{}", e.request);
            let first = first_turns(PromptStyle::Short, &reg, &e.inventory, &e.request);
            let turns = retry_turns(first, &retry.rejected, &errors);
            let (ours, points) = crate::chat_tokens(&vocab, &turns);
            let text = cassini_training::prompt_text(e);
            let theirs = vocab.tokenize(text.as_bytes(), false, true);
            assert_eq!(ours, theirs, "{}", e.request);
            // La redemande garde la tête fixe : même point d'instantané qu'au premier essai.
            assert_eq!(points[0].at, head_end, "{}", e.request);
            retries += 1;
        }
        assert!(retries * 25 >= all.len(), "{retries} redemandes");
        eprintln!("parité au jeton : {retries} redemandes identiques");
    }
    /// Avec le vrai modèle et le vrai adaptateur : l'adaptateur est bien posé (la base seule
    /// répond autrement), et une reprise d'instantané rend la même réponse. Ignoré sans
    /// `CASSINI_TEST_MODEL` (la base) et `CASSINI_TEST_ADAPTER` (l'adaptateur GGUF).
    #[test]
    fn a_real_adapter_changes_the_answer_and_a_resumed_checkpoint_keeps_it() {
        let (Some(base), Some(lora)) = (
            std::env::var_os("CASSINI_TEST_MODEL"),
            std::env::var_os("CASSINI_TEST_ADAPTER"),
        ) else {
            return;
        };
        let (examples, _) = cassini_training::generate(cassini_training::DEFAULT_SEED, 50);
        let e = examples
            .iter()
            .find(|e| e.expected["kind"] == "calls")
            .expect("un exemple qui attend un plan d'appels");
        let reg = registry();
        let turns = first_turns(PromptStyle::Short, &reg, &e.inventory, &e.request);
        let grammar = plan_grammar(&reg);
        let run = |files: &ModelFiles| {
            crate::generate_files_measured(files, &turns, Some(&grammar), MAX_PLAN_TOKENS, None)
                .unwrap()
        };
        let (alone, _) = run(&ModelFiles::single(PathBuf::from(&base)));
        let adapted = ModelFiles::with_adapter(PathBuf::from(&base), PathBuf::from(&lora));
        let (first, _) = run(&adapted);
        let (second, timings) = run(&adapted);
        // Libère le modèle avant les assertions : un échec ne le laisse pas en mémoire.
        crate::engine()
            .unwrap()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .loaded = None;
        eprintln!("base seule : {}", alone.text);
        eprintln!("base + adaptateur : {}", first.text);
        assert_ne!(alone, first, "l'adaptateur change la réponse");
        assert!(
            timings.cached_tokens > 0,
            "la seconde lecture reprend un instantané"
        );
        assert_eq!(first, second, "même réponse après la reprise");
    }

    #[test]
    fn a_declaration_resolves_to_its_files_in_its_folder_with_its_prompt() {
        let dir = std::env::temp_dir().join(format!("cassini-ai-decl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let decl = dir.join("ajuste.modele.json");
        std::fs::write(
            &decl,
            r#"{"model":"base.gguf","adapter":"lora.gguf","prompt":"short"}"#,
        )
        .unwrap();
        assert_eq!(
            resolve_model(&decl).unwrap(),
            (
                ModelFiles::with_adapter(dir.join("base.gguf"), dir.join("lora.gguf")),
                PromptStyle::Short
            )
        );
        // Un modèle fusionné : un seul fichier, prompt court ; sans `prompt`, le long.
        std::fs::write(&decl, r#"{"model":"fusion.gguf","prompt":"short"}"#).unwrap();
        assert_eq!(
            resolve_model(&decl).unwrap(),
            (
                ModelFiles::single(dir.join("fusion.gguf")),
                PromptStyle::Short
            )
        );
        std::fs::write(&decl, r#"{"model":"fusion.gguf"}"#).unwrap();
        assert_eq!(resolve_model(&decl).unwrap().1, PromptStyle::Full);
        // Un GGUF seul (même absent : son chargement le dira) garde le long prompt.
        let gguf = dir.join("Qwen3.5-9B-Q4_K_M.gguf");
        assert_eq!(
            resolve_model(&gguf).unwrap(),
            (ModelFiles::single(&gguf), PromptStyle::Full)
        );
        // Une déclaration absente est un modèle introuvable, nommé par son seul fichier.
        let e = resolve_model(&dir.join("absente.modele.json")).unwrap_err();
        assert_eq!(
            e,
            AiError::ModelNotFound(PathBuf::from("absente.modele.json"))
        );
        // Une déclaration illisible ou qui nomme un chemin est refusée, sans recopier le dossier.
        for bad in [
            r#"{"model":"../base.gguf"}"#,
            "pas du json",
            r#"{"model":"a.gguf","x":1}"#,
        ] {
            std::fs::write(&decl, bad).unwrap();
            let e = resolve_model(&decl).unwrap_err();
            assert_eq!(e.code(), "models.bad_declaration", "{bad}");
            let message = e.to_string();
            assert!(message.contains("ajuste.modele.json"), "{message}");
            assert!(!message.contains(&*dir.to_string_lossy()), "{message}");
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Ticket 5.18, sans modèle : le budget d'inventaire du noyau (14 000 caractères) a été mesuré
    /// pour ces valeurs-là. Avant d'en changer une, relancer avec le modèle
    /// `the_inventory_budget_fits_the_context_with_the_real_tokenizer`, puis mettre à jour ce test.
    #[test]
    fn the_inventory_budget_was_measured_for_these_values() {
        const RERUN: &str =
            "relancer the_inventory_budget_fits_the_context_with_the_real_tokenizer \
            avec le modèle avant de changer cette valeur";
        assert_eq!(crate::N_CTX, 8192, "N_CTX : {RERUN}");
        assert_eq!(MAX_PLAN_TOKENS, 1024, "MAX_PLAN_TOKENS : {RERUN}");
        assert_eq!(MAX_ECHO_CHARS, 1500, "MAX_ECHO_CHARS : {RERUN}");
        assert_eq!(MAX_REPORTED, 8, "MAX_REPORTED : {RERUN}");
        assert_eq!(
            cassini_core::agent_inventory::INVENTORY_BUDGET_CHARS,
            14_000,
            "INVENTORY_BUDGET_CHARS : {RERUN}"
        );
        assert_eq!(SHORT_SYSTEM.chars().count(), 1041, "SHORT_SYSTEM : {RERUN}");
    }

    /// Mesure du ticket 5.18, avec le vrai tokeniseur du modèle livré : jetons par caractère de
    /// l'inventaire de Villeneuve et d'une couche d'intersection à 34 champs (parcelles × zones du
    /// PLU), et place prise par tout ce qui n'est pas l'inventaire (consignes, armature, phrase,
    /// redemande). Le noyau ne tokenise pas : son budget d'inventaire, en caractères, se vérifie
    /// ici au pire cas. `CASSINI_TEST_INVENTORY` : inventaire traduit du projet réel (hors dépôt),
    /// par défaut `../donnees-test-cassini/inventaire-villeneuve-traduit-4b.json`, chemin relatif
    /// à la racine du dépôt.
    #[test]
    #[ignore = "charge le vrai modèle ; lancer avec CASSINI_TEST_MODEL et --ignored"]
    fn the_inventory_budget_fits_the_context_with_the_real_tokenizer() {
        use cassini_core::agent_inventory::INVENTORY_BUDGET_CHARS;
        use llama_cpp_2::model::params::LlamaModelParams;
        use llama_cpp_2::model::LlamaModel;
        let Some(path) = std::env::var_os("CASSINI_TEST_MODEL") else {
            eprintln!("CASSINI_TEST_MODEL absent : mesure ignorée.");
            return;
        };
        let inventory_path = std::env::var_os("CASSINI_TEST_INVENTORY")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../donnees-test-cassini/inventaire-villeneuve-traduit-4b.json")
            });
        let text = std::fs::read_to_string(&inventory_path).expect("inventaire de Villeneuve");
        let villeneuve: Value = serde_json::from_str(&text).unwrap();
        let mut guard = crate::engine()
            .unwrap()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        guard.loaded = None;
        let llama = LlamaModel::load_from_file(
            &guard.backend,
            Path::new(&path),
            &LlamaModelParams::default(),
        )
        .unwrap();
        let vocab = llama.vocab();
        let count = |s: &str| vocab.tokenize(s.as_bytes(), false, false).len();
        let chars = |v: &Value| v.to_string().chars().count();

        // L'inventaire de Villeneuve, tel que le prompt court l'écrit.
        let inv_chars = chars(&villeneuve);
        let inv_tokens = count(&villeneuve.to_string());
        // Une couche d'intersection : les 10 champs des parcelles puis les 24 du PLU.
        let layers = villeneuve["layers"].as_array().unwrap();
        let by_name = |n: &str| layers.iter().find(|l| l["name"] == n).unwrap().clone();
        let (parcelles, plu) = (by_name("parcelles"), by_name("plu_zones"));
        let mut fields = parcelles["fields"].as_array().unwrap().clone();
        fields.extend(plu["fields"].as_array().unwrap().iter().cloned());
        assert_eq!(fields.len(), 34);
        let intersect = |k: usize| {
            json!({"layer_id": format!("Intersection_{k:02}_8d2f0c55_7a1e_4b8e_9b0f_2c6d5e4a3b1c"),
                "name": "Intersection", "kind": "vector", "geometry": "MultiPolygon",
                "epsg": 2154, "feature_count": 2429,
                "source": format!("{}-analysis.intersect.gpkg", 20 + k), "fields": fields})
        };
        let layer_chars = chars(&intersect(1));
        let layer_tokens = count(&intersect(1).to_string());

        // Tout le reste : consignes, armature, une longue phrase, l'inventaire vide.
        let request = "Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à \
            Villeneuve-le-Roi, hors des zones inondables, et sors-moi la carte en PDF avec un titre.";
        let empty = json!({"project": villeneuve["project"], "layers": [], "unloaded_files": []});
        let fixed = |inv: &Value| {
            let turns = first_turns(PromptStyle::Short, &registry(), inv, request);
            crate::chat_tokens(&vocab, &turns).0.len()
        };
        let base = fixed(&empty);
        // La redemande au pire : la sortie recopiée à son plafond, et le plafond d'écarts.
        let echo: String = r#"{"command":"analysis.intersect","params":{"layer_id":"step:3.layer_id","overlay_layer_id":"plu_zones_5a071b18_e012_4e1f_9c1c_250bbfb7550c"}},"#
            .repeat(20)
            .chars()
        .take(MAX_ECHO_CHARS)
        .collect();
        let errors: Vec<String> = (1..=MAX_REPORTED)
            .map(|k| format!("étape {k} : la couche « plu_zones_5a071b18_e012_4e1f_9c1c_250bbfb7550c_bis » n'est pas dans l'inventaire du projet"))
            .collect();
        let retry = |inv: &Value| {
            let turns = retry_turns(
                first_turns(PromptStyle::Short, &registry(), inv, request),
                &echo,
                &errors,
            );
            crate::chat_tokens(&vocab, &turns).0.len()
        };
        let retry_extra = retry(&empty) - base;

        // Un inventaire au budget : Villeneuve, puis des intersections tant qu'elles tiennent.
        let mut full = villeneuve.clone();
        let mut k = 1;
        loop {
            let mut next = full.clone();
            next["layers"].as_array_mut().unwrap().push(intersect(k));
            if chars(&next) > INVENTORY_BUDGET_CHARS {
                break;
            }
            full = next;
            k += 1;
        }
        // Puis une dernière intersection, aux champs coupés, pour arriver au plus près du budget.
        for n in (0..fields.len()).rev() {
            let mut last = intersect(k);
            last["fields"] = json!(fields[..n]);
            let mut next = full.clone();
            next["layers"].as_array_mut().unwrap().push(last);
            if chars(&next) <= INVENTORY_BUDGET_CHARS {
                full = next;
                break;
            }
        }
        let full_tokens = retry(&full);
        eprintln!(
            "Villeneuve : {inv_chars} caractères, {inv_tokens} jetons ({:.3} jeton/caractère)",
            inv_tokens as f64 / inv_chars as f64
        );
        eprintln!(
            "intersection à 34 champs : {layer_chars} caractères, {layer_tokens} jetons ({:.3} jeton/caractère)",
            layer_tokens as f64 / layer_chars as f64
        );
        eprintln!("hors inventaire : {base} jetons au premier essai, {retry_extra} de plus à la redemande");
        eprintln!(
            "budget {INVENTORY_BUDGET_CHARS} caractères : {} intersections et une coupée, {} caractères, redemande de {full_tokens} jetons + {MAX_PLAN_TOKENS} de réponse, contexte {}",
            k - 1,
            chars(&full),
            crate::N_CTX
        );
        assert!(
            full_tokens + MAX_PLAN_TOKENS <= crate::N_CTX as usize,
            "le budget ne laisse pas la place à la réponse"
        );
    }
}
