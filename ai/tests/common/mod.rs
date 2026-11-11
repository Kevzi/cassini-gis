//! Partagé par les essais réels : le modèle désigné par `CASSINI_TEST_MODEL`, et l'inventaire
//! tel que le noyau le montre au modèle.

use std::path::PathBuf;

use cassini_ai::plan::{resolve_model, AgentError, PromptStyle};
use cassini_ai::ModelFiles;
use cassini_core::agent_inventory::ShortIds;
use cassini_core::plan::Plan;
use cassini_core::plan_completion::complete_conversions;
use cassini_core::registry::Registry;
use serde_json::Value;

/// Le modèle à essayer, lu de `CASSINI_TEST_MODEL` : un GGUF seul (long prompt) ou une
/// déclaration `*.modele.json` (base, adaptateur et prompt qu'elle nomme), résolus par
/// `resolve_model`, comme dans la CLI. `None` sans la variable. Une déclaration absente ou
/// illisible fait échouer l'essai en la nommant.
#[allow(dead_code)]
pub fn test_model() -> Option<(ModelFiles, PromptStyle)> {
    let path = PathBuf::from(std::env::var_os("CASSINI_TEST_MODEL")?);
    let resolved = resolve_model(&path)
        .unwrap_or_else(|e| panic!("modèle illisible : {} ({e})", path.display()));
    eprintln!(
        "Modèle : {} ; adaptateur : {} ; prompt : {:?}",
        resolved.0.model.display(),
        resolved
            .0
            .adapter
            .as_ref()
            .map_or_else(|| "aucun".to_string(), |a| a.display().to_string()),
        resolved.1
    );
    Some(resolved)
}

/// L'inventaire tel que le noyau le montre au modèle depuis le ticket 5.19 : chaque identifiant
/// QGIS `nom_<uuid>` raccourci en `nom_<4 hex>`, avec la table qui retraduit le plan. Avec
/// `CASSINI_TEST_FULL_IDS` non vide, les identifiants complets d'avant 5.19 (table vide).
#[allow(dead_code)]
pub fn shown_inventory(inventory: &serde_json::Value) -> (serde_json::Value, ShortIds) {
    if std::env::var_os("CASSINI_TEST_FULL_IDS").is_some_and(|v| !v.is_empty()) {
        eprintln!("Identifiants : complets");
        return (inventory.clone(), ShortIds::default());
    }
    let ids = ShortIds::of_inventory(inventory);
    eprintln!("Identifiants : courts");
    (ids.shorten(inventory.clone()), ids)
}

/// L'inventaire d'un essai dans la forme apprise : un fichier écrit dans cette forme est rendu
/// tel quel ; la sortie brute de `project.inventory` (reconnue à sa clé `project_crs`, prise dans
/// QGIS) est traduite comme le noyau la traduit.
#[allow(dead_code)]
pub fn learned(inventory: &Value) -> Value {
    if inventory.get("project_crs").is_some() {
        cassini_core::agent_inventory::agent_inventory(inventory)
    } else {
        inventory.clone()
    }
}

/// La vue que le noyau montre au modèle pour `request` (ticket 5.48), depuis `inventory` (forme
/// apprise ou sortie brute de QGIS, aux identifiants complets) : sous le seuil du tri, l'inventaire
/// aux identifiants courts de `shown_inventory`, à l'octet près ; au-dessus, la vue triée par la
/// demande. Rend aussi le nombre de couches de l'utilisateur en tête de la liste coupée (ancrage
/// du cache) et la table des identifiants. Avec `CASSINI_TEST_FULL_IDS` non vide : l'inventaire tel
/// quel, comme `shown_inventory`.
#[allow(dead_code)]
pub fn shown_view(inventory: &Value, request: &str) -> (Value, Option<usize>, ShortIds) {
    use cassini_core::agent_inventory::{sorted_view_learned, LastPlan, ViewLimits};
    if std::env::var_os("CASSINI_TEST_FULL_IDS").is_some_and(|v| !v.is_empty()) {
        return (learned(inventory), None, ShortIds::default());
    }
    let (view, ids) = sorted_view_learned(
        learned(inventory),
        &[],
        &std::collections::HashSet::new(),
        &LastPlan::default(),
        request,
        &[],
        &ViewLimits::default(),
    );
    (view.inventory, Some(view.user_layers), ids)
}

/// La vue de `shown_view`, seule.
#[allow(dead_code)]
pub fn view(inventory: &Value, request: &str) -> Value {
    shown_view(inventory, request).0
}

/// Le plan composé, ses identifiants courts retraduits par `ids`, comme le fait le noyau avant de
/// le vérifier.
#[allow(dead_code)]
pub fn restored(composed: Result<Plan, AgentError>, ids: &ShortIds) -> Result<Plan, AgentError> {
    composed.map(|mut plan| {
        if let Plan::Calls { calls } = &mut plan {
            for call in calls {
                ids.restore(&mut call.params);
            }
        }
        plan
    })
}

/// Le plan composé, complété par les conversions de points comme le fait `plan.propose` avant de
/// le montrer (amendement d'AD-19, 11.22) : depuis 11.23, les essais réels de terrain mesurent le
/// plan complété, le modèle recopiant un point tel que la phrase le donne. Le système d'un raster
/// est celui que `inventory` donne à sa `source` (dans le produit, l'en-tête du GeoTIFF) ; une
/// question ou un refus passent inchangés.
#[allow(dead_code)]
pub fn completed(
    registry: &Registry,
    composed: Result<Plan, AgentError>,
    inventory: &Value,
) -> Result<Plan, AgentError> {
    composed.map(|plan| complete_with(registry, plan, inventory))
}

/// La complétion d'un plan sur `inventory`, comme `completed`.
#[allow(dead_code)]
pub fn complete_with(registry: &Registry, plan: Plan, inventory: &Value) -> Plan {
    let rasters: Vec<(String, u32)> = inventory["layers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| l["kind"] == "raster")
        .filter_map(|l| {
            let source = l["source"].as_str()?.to_string();
            let epsg = u32::try_from(l["epsg"].as_u64()?).ok()?;
            Some((source, epsg))
        })
        .collect();
    let raster_epsg = |raw: &str| {
        rasters
            .iter()
            .find(|(source, _)| source == raw)
            .map(|(_, epsg)| *epsg)
    };
    complete_conversions(registry, plan, &raster_epsg)
}

// --- Phrases écrites à la main et contrôle de leurs attendus (5.53, livrable 3) ---
//
// Un seul contrôle pour toutes les barres : la barre par commande, `plan_reel` et
// `question_reel` lisent leurs phrases `{id, phrase, expected}` dans un `phrases.json` et
// passent leurs attendus par `invalid`.

/// Les phrases `{id, phrase, expected, …}` d'un `phrases.json`, nommé par `name` en cas d'échec.
#[allow(dead_code)]
pub fn read_phrases(text: &str, name: &str) -> Vec<Value> {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{name} illisible : {e}"))
}

/// Les appels attendus d'une phrase, si elle attend un plan.
#[allow(dead_code)]
pub fn expected_calls(phrase: &Value) -> Option<&Vec<Value>> {
    (phrase["expected"]["kind"] == "calls")
        .then(|| phrase["expected"]["calls"].as_array())
        .flatten()
}

/// Le plan attendu d'une phrase, prêt pour `check_plan` : un export sans titre en reçoit un
/// (le titre est libre et retiré à la comparaison).
#[allow(dead_code)]
pub fn expected_plan(calls: &[Value]) -> Plan {
    let mut calls = calls.to_vec();
    for call in &mut calls {
        if call["command"] == "layout.export_pdf" && call["params"].get("title").is_none() {
            call["params"]["title"] = serde_json::json!("Carte");
        }
    }
    serde_json::from_value(serde_json::json!({"kind": "calls", "calls": calls})).unwrap()
}

/// Ce que l'inventaire d'une barre connaît : les couches (`layer_id`) et leurs champs, les
/// rasters (`source`) et les fichiers non chargés (`unloaded_files`).
#[allow(dead_code)]
pub struct Known {
    pub fields: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    pub rasters: std::collections::BTreeSet<String>,
    pub unloaded: std::collections::BTreeSet<String>,
    /// Les valeurs montrées d'un champ (`values`), par couche puis par champ.
    pub values: std::collections::BTreeMap<(String, String), Vec<String>>,
}

#[allow(dead_code)]
pub fn known(inventory: &Value) -> Known {
    let mut fields = std::collections::BTreeMap::new();
    let mut rasters = std::collections::BTreeSet::new();
    let mut values = std::collections::BTreeMap::new();
    for layer in inventory["layers"].as_array().unwrap() {
        let layer_id = layer["layer_id"].as_str().unwrap().to_string();
        for f in layer["fields"].as_array().into_iter().flatten() {
            if let (Some(name), Some(shown)) = (f["name"].as_str(), f["values"].as_array()) {
                let shown = shown
                    .iter()
                    .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_string))
                    .collect();
                values.insert((layer_id.clone(), name.to_string()), shown);
            }
        }
        let names = layer["fields"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["name"].as_str().map(str::to_string))
            .collect();
        fields.insert(layer["layer_id"].as_str().unwrap().to_string(), names);
        if layer["kind"] == "raster" {
            rasters.insert(layer["source"].as_str().unwrap().to_string());
        }
    }
    let unloaded = inventory["unloaded_files"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f["path"].as_str().map(str::to_string))
        .collect();
    Known {
        fields,
        rasters,
        unloaded,
        values,
    }
}

/// Les identifiants de couche et les chemins de raster que nomme un appel.
#[allow(dead_code)]
pub fn references(call: &Value) -> (Vec<String>, Vec<String>) {
    let params = &call["params"];
    let text = |v: &Value| v.as_str().map(str::to_string);
    let mut layers: Vec<String> = ["layer_id", "overlay_layer_id", "points_layer_id"]
        .iter()
        .filter_map(|k| text(&params[*k]))
        .collect();
    for key in ["leave_out", "layers"] {
        layers.extend(
            params[key]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(text),
        );
    }
    let mut rasters: Vec<String> = ["dem", "cost", "raster"]
        .iter()
        .filter_map(|k| text(&params[*k]))
        .collect();
    for criterion in params["criteria"].as_array().into_iter().flatten() {
        rasters.extend(text(&criterion["raster"]));
    }
    rasters.extend(
        params["exclusions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(text),
    );
    (layers, rasters)
}

/// Les identifiants de commande du registre.
#[allow(dead_code)]
pub fn registry_ids(registry: &Registry) -> Vec<String> {
    registry.specs().map(|s| s.id.to_string()).collect()
}

/// Les refus devenus faux : une phrase dont l'attendu est un refus et dont un mot de `missing`
/// (ce que le registre n'a pas) figure dans un identifiant de `ids`. Le jour où la commande
/// arrive au registre, le refus ne tient plus et la phrase est nommée.
#[allow(dead_code)]
pub fn refusals_now_wrong(ids: &[String], phrases: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    for p in phrases
        .iter()
        .filter(|p| p["expected"]["kind"] == "refusal")
    {
        let id = p["id"].as_str().unwrap_or("?");
        for word in p["expected"]["missing"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            // Un mot vide (`contains("")` toujours vrai) ou mal écrit est nommé par
            // `malformed_refusals`, pas ici.
            .filter(|w| is_keyword(w))
        {
            if let Some(found) = ids.iter().find(|c| c.contains(word)) {
                out.push(format!(
                    "{id} : refus devenu faux, « {word} » est dans {found}"
                ));
            }
        }
    }
    out
}

/// Un mot de `missing` est un mot-clé cherché tel quel dans les identifiants du registre (jamais
/// un synonyme) : non vide, en minuscules ASCII (lettres, chiffres, `.`, `_`).
fn is_keyword(word: &str) -> bool {
    !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_')
}

/// Les refus mal écrits ou devenus faux du côté des options : un refus sans `missing` ni
/// `missing_params`, un mot qui n'est pas un mot-clé, une option `commande:paramètre` de
/// `missing_params` que le schéma de la commande porte désormais.
#[allow(dead_code)]
pub fn malformed_refusals(registry: &Registry, phrases: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    for p in phrases
        .iter()
        .filter(|p| p["expected"]["kind"] == "refusal")
    {
        let id = p["id"].as_str().unwrap_or("?");
        let words = p["expected"]["missing"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let options = p["expected"]["missing_params"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if words.is_empty() && options.is_empty() {
            out.push(format!("{id} : refus sans `missing` ni `missing_params`"));
        }
        for word in &words {
            if !word.as_str().is_some_and(is_keyword) {
                out.push(format!("{id} : mot `missing` invalide {word}"));
            }
        }
        for option in &options {
            let Some((command, param)) = option.as_str().and_then(|o| o.split_once(':')) else {
                out.push(format!("{id} : option `missing_params` invalide {option}"));
                continue;
            };
            match registry.spec(command) {
                None => out.push(format!("{id} : commande inconnue {command}")),
                Some(spec) if spec.params_schema["properties"].get(param).is_some() => out.push(
                    format!("{id} : refus devenu faux, {command} porte l'option {param}"),
                ),
                Some(_) => {}
            }
        }
    }
    out
}

/// Les phrases dont l'attendu est invalide, avec la raison : sorte inconnue ; refus mal écrit,
/// ou dont un mot de `missing` ou une option de `missing_params` est au registre ; plan qui échoue `check_plan`, le garde-fou des coordonnées ou
/// que la complétion change ; couche, raster, champ ou fichier non chargé absents de l'inventaire.
#[allow(dead_code)]
pub fn invalid(registry: &Registry, inventory: &Value, phrases: &[Value]) -> Vec<String> {
    use cassini_core::point::guard_coordinates;
    let known = known(inventory);
    let mut out = refusals_now_wrong(&registry_ids(registry), phrases);
    out.extend(malformed_refusals(registry, phrases));
    for p in phrases {
        let id = p["id"].as_str().unwrap_or("?");
        let text = p["phrase"].as_str().unwrap_or_default();
        match p["expected"]["kind"].as_str() {
            Some("question" | "refusal") => continue,
            Some("calls") => {}
            other => {
                out.push(format!("{id} : sorte attendue inconnue {other:?}"));
                continue;
            }
        }
        let Some(calls) = expected_calls(p) else {
            out.push(format!("{id} : plan attendu sans appels"));
            continue;
        };
        let plan = expected_plan(calls);
        if let Err(e) = registry.check_plan(&plan) {
            out.push(format!("{id} : check_plan refuse ({e:?})"));
            continue;
        }
        if guard_coordinates(registry, plan.clone(), text) != plan {
            out.push(format!(
                "{id} : le garde-fou des coordonnées refuse le plan attendu"
            ));
        }
        if complete_with(registry, plan.clone(), inventory) != plan {
            out.push(format!(
                "{id} : la complétion ajoute une conversion au plan attendu"
            ));
        }
        for call in calls {
            let (named_layers, named_rasters) = references(call);
            for layer in named_layers.iter().filter(|l| !l.starts_with("step:")) {
                if !known.fields.contains_key(layer) {
                    out.push(format!("{id} : couche absente de l'inventaire {layer}"));
                }
            }
            for raster in named_rasters.iter().filter(|r| !r.starts_with("step:")) {
                if !known.rasters.contains(raster) {
                    out.push(format!("{id} : raster absent de l'inventaire {raster}"));
                }
            }
            // Un champ nommé doit exister dans la couche qu'il désigne directement.
            let params = &call["params"];
            if let Some(own) = params["layer_id"]
                .as_str()
                .and_then(|l| known.fields.get(l))
            {
                for key in ["field", "label_field"] {
                    if let Some(field) = params[key].as_str() {
                        if !own.contains(field) {
                            out.push(format!("{id} : champ absent de la couche {field}"));
                        }
                    }
                }
            }
            // Une égalité sur un champ aux valeurs montrées vise une de ces valeurs.
            if call["command"] == "analysis.select_by_attribute"
                && matches!(params["operator"].as_str(), Some("eq" | "ne"))
            {
                let key = (
                    params["layer_id"].as_str().unwrap_or_default().to_string(),
                    params["field"].as_str().unwrap_or_default().to_string(),
                );
                if let (Some(shown), Some(value)) =
                    (known.values.get(&key), params["value"].as_str())
                {
                    if !shown.iter().any(|v| v == value) {
                        out.push(format!(
                            "{id} : valeur absente des valeurs du champ {value}"
                        ));
                    }
                }
            }
            if call["command"] == "layer.add" {
                let path = params["path"].as_str().unwrap_or_default();
                if !known.unloaded.contains(path) {
                    out.push(format!(
                        "{id} : fichier absent des fichiers non chargés {path}"
                    ));
                }
            }
        }
    }
    out
}

/// Un nombre entier écrit en flottant (`2.0`) vaut l'entier (`2`) : serde_json les distingue.
#[allow(dead_code)]
pub fn normalize_numbers(value: Value) -> Value {
    match value {
        Value::Number(n) => match n.as_f64() {
            Some(f) if n.is_f64() && f.fract() == 0.0 && f.abs() < 9.0e15 => Value::from(f as i64),
            _ => Value::Number(n),
        },
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_numbers).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, normalize_numbers(v)))
                .collect(),
        ),
        other => other,
    }
}

/// Les appels obtenus valent-ils les appels attendus d'une phrase écrite à la main, entiers
/// écrits en flottant ramenés à l'entier. Titre d'export : comparé exactement quand l'attendu
/// en porte un ; sinon libre, mais non vide.
#[allow(dead_code)]
pub fn same_calls(got: &[cassini_core::plan::PlannedCall], want: &Value) -> bool {
    let mut got = serde_json::to_value(got).unwrap();
    let wants = want.as_array();
    for (i, call) in got.as_array_mut().unwrap().iter_mut().enumerate() {
        if call["command"] != "layout.export_pdf" {
            continue;
        }
        let title_wanted = wants
            .and_then(|w| w.get(i))
            .is_some_and(|w| w["params"].get("title").is_some());
        if title_wanted {
            continue;
        }
        let Some(params) = call.get_mut("params").and_then(Value::as_object_mut) else {
            return false;
        };
        match params.remove("title") {
            Some(Value::String(t)) if !t.trim().is_empty() => {}
            _ => return false,
        }
    }
    normalize_numbers(got) == normalize_numbers(want.clone())
}

/// Le nombre de réponses justes exigé sur `total` phrases, à la proportion `num/den` d'une
/// barre fixée sur un total plus court, arrondi vers le bas (D4 du plan 5.53, livrable 3).
#[allow(dead_code)]
pub fn proportional(total: usize, num: usize, den: usize) -> usize {
    total * num / den
}

/// Minuscules, sans accents (`œ` en `oe`), chaque suite de caractères non alphanumériques
/// réduite à une espace.
#[allow(dead_code)]
pub fn folded(text: &str) -> String {
    let mut out = String::new();
    for c in text.to_lowercase().chars() {
        let mapped: &str = match c {
            'à' | 'â' | 'ä' | 'á' => "a",
            'é' | 'è' | 'ê' | 'ë' => "e",
            'î' | 'ï' => "i",
            'ô' | 'ö' => "o",
            'ù' | 'û' | 'ü' => "u",
            'ç' => "c",
            'œ' => "oe",
            'æ' => "ae",
            c if c.is_alphanumeric() => {
                out.push(c);
                continue;
            }
            _ => " ",
        };
        if mapped != " " || !out.ends_with(' ') {
            out.push_str(mapped);
        }
    }
    out.trim().to_string()
}

/// Les phrases qui nomment un fichier non ouvert de l'inventaire : le nom du fichier, sans
/// extension ni mot `mnt`, au singulier, replié comme la phrase (`ecole`, `hydrographie`,
/// `zone nord`).
#[allow(dead_code)]
pub fn naming_unloaded_files(inventory: &Value, phrases: &[Value]) -> Vec<String> {
    let stems: Vec<String> = known(inventory)
        .unloaded
        .iter()
        .map(|path| {
            let file = path.rsplit('/').next().unwrap_or(path);
            let stem = file.split('.').next().unwrap_or(file);
            let words: Vec<String> = folded(stem)
                .split(' ')
                .filter(|w| *w != "mnt")
                .map(|w| w.strip_suffix('s').unwrap_or(w).to_string())
                .collect();
            words.join(" ")
        })
        .collect();
    phrases
        .iter()
        .filter(|p| {
            let text = folded(p["phrase"].as_str().unwrap_or_default());
            stems.iter().any(|s| text.contains(s.as_str()))
        })
        .map(|p| p["id"].as_str().unwrap_or("?").to_string())
        .collect()
}
