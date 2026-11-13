//! Vue triée de l'inventaire d'un gros projet (ticket 5.46, changement de cap du 2026-11-10).
//!
//! Sous un seuil, la vue est exactement celle de `model_inventory`. Au-dessus, elle a deux
//! parties : `index`, toutes les couches dans l'ordre stable en forme réduite `{layer_id, name,
//! geometry}`, identique d'une demande à l'autre pour un même état du projet (des résultats qui
//! arrivent ou sortent par paquets le changent) ; et `layers`, dans la forme apprise et dans
//! l'ordre stable, le détail des couches choisies par priorité : celles que la demande désigne
//! (par leur nom, puis un nom de champ, puis une valeur), les résultats du dernier plan exécuté,
//! les couches qu'il cite, puis les rasters, jusqu'au budget. Une couche détaillée garde au plus
//! `max_fields` champs et `max_layer_chars` caractères. La règle est générique : elle ne connaît
//! aucun jeu de données. La fonction est pure : ni disque, ni journal (le dernier plan lui est
//! donné), et sa sortie ne dépend d'aucun ordre de table de hachage. `index` précède `layers` dans
//! le texte parce que `serde_json`, sans sa fonction `preserve_order`, range les clés par ordre
//! alphabétique : activer cette fonction changerait cet ordre.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde_json::{json, Map, Value};

use super::{
    agent_inventory, json_chars, kept_by_batches, left_out_by_budget, model_inventory_learned,
    qgis_uuid_id, result_date, results_oldest_first, results_without_values, stable_order,
    without_intermediates, ModelInventory, ShortIds, WorkFile, INVENTORY_BUDGET_CHARS,
    INVENTORY_LOW_CHARS,
};

/// Seuil du tri : tant que la vue des couches de l'utilisateur tient en ce nombre de caractères,
/// la vue reste celle d'aujourd'hui. Décision du propriétaire du 2026-11-10 (≈ 9 000, à revoir en
/// 5.48), revue sur la mesure (tranchée en party le 2026-11-10, à confirmer par le propriétaire) :
/// le projet d'Annecy (démo filmée, 12 couches) fait 16 597 caractères dans QGIS ; 17 000 le laisse
/// inchangé à l'octet, et reste le plus haut seuil sûr pour le contexte de 8 192 jetons (≈ 6 200
/// jetons d'inventaire, consignes et réponse de 1 024 jetons comprises).
pub const SORT_THRESHOLD_CHARS: usize = 17_000;

/// Budget de toute la vue triée (ticket 5.48) : sous la borne des prompts de l'entraînement
/// (`MAX_PROMPT_CHARS` de `training/`, 12 900 caractères avec consignes et demande en 5.48, 19 000
/// depuis 5.49), pour que le modèle ne voie jamais une vue plus longue que celles qu'il a apprises.
pub const SORTED_BUDGET_CHARS: usize = 11_000;

/// Budget de l'index réduit, en caractères de sa liste JSON. Au dépassement, les résultats les
/// plus anciens en sortent par paquets jusqu'à `INDEX_LOW_CHARS`. Réglage, non une décision du
/// propriétaire : à revoir en 5.48 sur les mesures.
pub const INDEX_BUDGET_CHARS: usize = 7_000;

/// Seuil bas du retrait par paquets de l'index.
pub const INDEX_LOW_CHARS: usize = 3_500;

/// Plafond de champs d'une couche détaillée (décision du 2026-11-10, à revoir en 5.48).
pub const DETAIL_MAX_FIELDS: usize = 30;

/// Plafond de caractères d'une couche détaillée (décision du 2026-11-10, à revoir en 5.48).
pub const DETAIL_MAX_CHARS: usize = 3_000;

/// Le dernier plan exécuté, tel que le noyau le lit dans le journal (ticket 5.48).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LastPlan {
    /// Noms des fichiers de résultat de ses étapes (dernier segment de `source`).
    pub results: HashSet<String>,
    /// Identifiants QGIS complets des couches qu'il cite.
    pub cited: HashSet<String>,
}

/// Les réglages de la vue triée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewLimits {
    /// Seuil du tri, en caractères de la vue des seules couches de l'utilisateur.
    pub threshold: usize,
    /// Budget de la vue sous le seuil (celui de `model_inventory`).
    pub budget: usize,
    /// Budget de toute la vue triée, au-dessus du seuil.
    pub sorted_budget: usize,
    /// Seuil bas du budget, passé à `model_inventory` sous le seuil du tri.
    pub low: usize,
    /// Budget de l'index et son seuil bas.
    pub index_budget: usize,
    pub index_low: usize,
    /// Plafonds d'une couche détaillée.
    pub max_fields: usize,
    pub max_layer_chars: usize,
}

impl Default for ViewLimits {
    fn default() -> Self {
        ViewLimits {
            threshold: SORT_THRESHOLD_CHARS,
            budget: INVENTORY_BUDGET_CHARS,
            sorted_budget: SORTED_BUDGET_CHARS,
            low: INVENTORY_LOW_CHARS,
            index_budget: INDEX_BUDGET_CHARS,
            index_low: INDEX_LOW_CHARS,
            max_fields: DETAIL_MAX_FIELDS,
            max_layer_chars: DETAIL_MAX_CHARS,
        }
    }
}

/// La vue du modèle pour `request`, et la table des identifiants courts (faite sur toutes les
/// couches, comme `model_inventory`), depuis `raw`, la sortie de `project.inventory`. Voir
/// `sorted_view_learned`, ici sans couche réclamée.
pub fn sorted_view(
    raw: &Value,
    work_files: &[WorkFile],
    intermediates: &HashSet<String>,
    last_plan: &LastPlan,
    request: &str,
    limits: &ViewLimits,
) -> (ModelInventory, ShortIds) {
    sorted_view_learned(
        agent_inventory(raw),
        work_files,
        intermediates,
        last_plan,
        request,
        &[],
        limits,
    )
}

/// La vue du modèle pour `request`, depuis un inventaire déjà dans la forme apprise (`learned`,
/// sortie d'`agent_inventory`, ou fichier d'essai écrit dans cette forme).
///
/// Sous `limits.threshold` (taille de l'inventaire réduit aux couches de l'utilisateur, forme
/// apprise, identifiants courts), c'est `model_inventory` tel quel. Au-dessus : `index` réduit de
/// toutes les couches (intermédiaires retirés), dont les résultats les plus anciens sortent par
/// paquets au-delà de `limits.index_budget` (rendus dans `left_out`) ; puis `layers`, les couches
/// détaillées par priorité tant que toute la vue tient sous `limits.sorted_budget` (une couche qui
/// ne tient pas est sautée, la suivante essayée), rangées dans l'ordre stable. `wanted` : les
/// identifiants QGIS des couches que la redemande élargie réclame (ticket 5.48), détaillées avant
/// toute autre. `user_layers` compte les couches de l'utilisateur en tête de l'**index**, c'est-à-dire
/// toutes (l'index n'en retire jamais) : c'est là que le moteur pose son ancrage. Pour un même état
/// du projet, l'index ne dépend pas de la demande ; il vient avant `layers` dans le texte par
/// l'ordre alphabétique des clés de `serde_json` (sans `preserve_order`).
pub fn sorted_view_learned(
    learned: Value,
    work_files: &[WorkFile],
    intermediates: &HashSet<String>,
    last_plan: &LastPlan,
    request: &str,
    wanted: &[String],
    limits: &ViewLimits,
) -> (ModelInventory, ShortIds) {
    let original = learned.clone();
    let (ordered, users) = stable_order(learned, work_files);
    let ids = ShortIds::of_inventory(&ordered);
    let lighter = results_without_values(
        without_intermediates(ordered, work_files, intermediates),
        work_files,
    );
    let shown = ids.shorten(lighter.clone());
    let empty: Vec<Value> = Vec::new();
    let real = lighter
        .get("layers")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let short = shown
        .get("layers")
        .and_then(Value::as_array)
        .unwrap_or(&empty);

    let mut users_only = shown.clone();
    users_only["layers"] = Value::Array(short[..users.min(short.len())].to_vec());
    if real.is_empty() || json_chars(&users_only) <= limits.threshold {
        return model_inventory_learned(
            original,
            work_files,
            intermediates,
            limits.budget,
            limits.low,
        );
    }

    // L'index : forme réduite de chaque couche, résultats les plus anciens sortis par paquets.
    let reduced: Vec<Value> = short.iter().map(reduce).collect();
    let sizes: Vec<usize> = reduced.iter().map(json_chars).collect();
    let in_index = kept_by_batches(
        2,
        &sizes,
        &results_oldest_first(real, work_files),
        limits.index_budget,
        limits.index_low,
    );
    let index: Vec<Value> = reduced
        .into_iter()
        .zip(&in_index)
        .filter(|(_, &k)| k)
        .map(|(l, _)| l)
        .collect();
    let left_out = left_out_by_budget(&lighter, &json!({ "layers": index }), &ids, work_files);

    // Les couches détaillées, par priorité, jusqu'au budget de toute la vue.
    let request = Request::new(request);
    let mut view = shown.clone();
    view["index"] = Value::Array(index);
    view["layers"] = json!([]);
    let mut size = json_chars(&view);
    let mut chosen: Vec<(usize, Value)> = Vec::new();
    for i in detail_order(real, &in_index, work_files, last_plan, wanted, &request) {
        let layer = capped(&short[i], &request, limits);
        let more = json_chars(&layer) + usize::from(!chosen.is_empty());
        if size + more > limits.sorted_budget {
            continue;
        }
        size += more;
        chosen.push((i, layer));
    }
    chosen.sort_by_key(|(i, _)| *i);
    let user_layers = in_index[..users.min(in_index.len())]
        .iter()
        .filter(|&&k| k)
        .count();
    view["layers"] = Value::Array(chosen.into_iter().map(|(_, l)| l).collect());
    (
        ModelInventory {
            inventory: view,
            user_layers,
            left_out,
        },
        ids,
    )
}

/// Forme réduite d'une couche : `layer_id`, `name` et `geometry`, quand elle les a.
fn reduce(layer: &Value) -> Value {
    let mut out = Map::new();
    for key in ["layer_id", "name", "geometry"] {
        if let Some(v) = layer.get(key) {
            out.insert(key.into(), v.clone());
        }
    }
    Value::Object(out)
}

/// Les rangs des couches à détailler, dans l'ordre de priorité, sans doublon : réclamées par la
/// redemande (`wanted`), désignées par la demande (`layer_score` décroissant ; à égalité, l'ordre
/// stable), résultats du dernier plan, couches qu'il cite, rasters. Seules les couches de l'index
/// sont candidates.
fn detail_order(
    layers: &[Value],
    in_index: &[bool],
    work_files: &[WorkFile],
    last_plan: &LastPlan,
    wanted: &[String],
    request: &Request,
) -> Vec<usize> {
    let candidates = || (0..layers.len()).filter(|&i| in_index[i]);
    let id_of = |i: usize| layers[i].get("layer_id").and_then(Value::as_str);
    let claimed =
        candidates().filter(|&i| id_of(i).is_some_and(|id| wanted.iter().any(|w| w == id)));
    let mut designated: Vec<(Reverse<Score>, usize)> = candidates()
        .filter_map(|i| layer_score(&layers[i], request).map(|score| (Reverse(score), i)))
        .collect();
    designated.sort_unstable();
    let of_plan = candidates().filter(|&i| {
        result_date(&layers[i], work_files)
            .is_some_and(|(_, name)| last_plan.results.contains(name))
    });
    let cited = candidates().filter(|&i| {
        layers[i]
            .get("layer_id")
            .and_then(Value::as_str)
            .is_some_and(|id| last_plan.cited.contains(id))
    });
    let rasters =
        candidates().filter(|&i| layers[i].get("kind").and_then(Value::as_str) == Some("raster"));
    let mut seen = vec![false; layers.len()];
    let mut order = Vec::new();
    for i in claimed
        .chain(designated.into_iter().map(|(_, i)| i))
        .chain(of_plan)
        .chain(cited)
        .chain(rasters)
    {
        if !seen[i] {
            seen[i] = true;
            order.push(i);
        }
    }
    order
}

/// Score de désignation d'une couche par la demande, comparé dans cet ordre : le nombre de mots
/// distincts de la demande qui la désignent par son nom (nom affiché, radical de l'identifiant,
/// radical du fichier), puis le nombre de mots distincts qui la désignent de quelque façon (nom,
/// nom de champ ou valeur), puis le nombre de ceux qui la désignent par un nom de champ au moins.
/// Chaque mot compte une fois, à sa meilleure place. `None` si aucun mot ne la désigne.
fn layer_score(layer: &Value, request: &Request) -> Option<Score> {
    let text = |key: &str| layer.get(key).and_then(Value::as_str).unwrap_or_default();
    let id = text("layer_id");
    let id_stem = qgis_uuid_id(id).map_or(id, |(name, _)| name);
    // Meilleure place de chaque mot : 0 nom, 1 champ, 2 valeur.
    let mut best: BTreeMap<String, u8> = BTreeMap::new();
    let mut note = |words: BTreeSet<String>, place: u8| {
        for w in words {
            let slot = best.entry(w).or_insert(place);
            *slot = (*slot).min(place);
        }
    };
    for t in [text("name"), id_stem, file_stem(text("source"))] {
        note(request.matched(t), 0);
    }
    for f in layer
        .get("fields")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        note(request.matched(field_name(f)), 1);
        note(request.value_words(f), 2);
    }
    if best.is_empty() {
        return None;
    }
    let count = |max: u8| best.values().filter(|&&p| p <= max).count();
    Some(Score {
        by_name: count(0),
        all: best.len(),
        by_field: count(1),
    })
}

/// Score de désignation (`layer_score`) : l'ordre des champs est l'ordre de comparaison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Score {
    by_name: usize,
    all: usize,
    by_field: usize,
}

/// Radical du fichier d'une source : la table `layername=<t>` quand la source en nomme une (un
/// GeoPackage à plusieurs tables) ; sinon avant `|`, dernier segment, sans extension.
fn file_stem(source: &str) -> &str {
    let mut parts = source.split('|');
    let path = parts.next().unwrap_or_default();
    if let Some(table) = parts
        .filter_map(|p| p.strip_prefix("layername="))
        .find(|t| !t.is_empty())
    {
        return table;
    }
    let file = path.rsplit(['/', '\\']).next().unwrap_or_default();
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

fn field_name(field: &Value) -> &str {
    field
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

/// La couche détaillée, plafonnée à `max_fields` champs et `max_layer_chars` caractères.
///
/// Une couche sous les deux plafonds reste intacte. Sinon, les champs sont pris par rang :
/// désignés par la demande (nom ou valeur), texte à valeurs, numériques, autres ; dans l'ordre de
/// QGIS à chaque rang ; jusqu'au premier qui dépasse un plafond. Ils sont rendus dans l'ordre de
/// QGIS.
fn capped(layer: &Value, request: &Request, limits: &ViewLimits) -> Value {
    let Some(fields) = layer.get("fields").and_then(Value::as_array) else {
        return layer.clone();
    };
    if fields.len() <= limits.max_fields && json_chars(layer) <= limits.max_layer_chars {
        return layer.clone();
    }
    let rank = |f: &Value| -> u8 {
        if request.names(field_name(f)) || request.has_value(f) {
            0
        } else if f.get("values").is_some() {
            1
        } else if matches!(
            f.get("type").and_then(Value::as_str),
            Some("integer" | "number")
        ) {
            2
        } else {
            3
        }
    };
    let mut by_rank: Vec<(u8, usize)> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| (rank(f), i))
        .collect();
    by_rank.sort_unstable();
    let mut out = layer.clone();
    out["fields"] = json!([]);
    let mut size = json_chars(&out);
    let mut kept: Vec<usize> = Vec::new();
    for (_, i) in by_rank {
        let more = json_chars(&fields[i]) + usize::from(!kept.is_empty());
        if kept.len() + 1 > limits.max_fields || size + more > limits.max_layer_chars {
            break;
        }
        size += more;
        kept.push(i);
    }
    kept.sort_unstable();
    out["fields"] = Value::Array(kept.into_iter().map(|i| fields[i].clone()).collect());
    out
}

/// La demande, prête à la correspondance : ses mots normalisés d'au moins 3 lettres, ses mots
/// bruts (pour les valeurs de 1 ou 2 signes, égales casse comprise), et ses sigles (mots écrits
/// en capitales, d'au moins 3 signes dont 2 lettres), qui désignent aussi un mot qu'ils
/// commencent, plus long d'au plus `SIGLE_SLACK` signe (« PLU » trouve « PLUi »).
struct Request {
    words: BTreeSet<String>,
    raw: BTreeSet<String>,
    sigles: BTreeSet<String>,
}

/// Signes de plus admis derrière un sigle de la demande (« PLU » → « PLUi », « PPR » → « PPRI ») ;
/// deux laisseraient passer des mots courants (« plu » → « pluie », « plume »).
const SIGLE_SLACK: usize = 1;

impl Request {
    fn new(text: &str) -> Request {
        let words: BTreeSet<String> = words(text)
            .into_iter()
            .filter(|w| w.chars().count() >= 3 && w.chars().any(char::is_alphabetic))
            .collect();
        let sigles = text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| {
                w.chars().count() >= 3
                    && w.chars().filter(|c| c.is_alphabetic()).count() >= 2
                    && w.chars().all(|c| !c.is_alphabetic() || c.is_uppercase())
            })
            .map(fold)
            .filter(|w| words.contains(w))
            .collect();
        Request {
            words,
            raw: text
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| !w.is_empty() && !STOP_WORDS.contains(w))
                .map(str::to_string)
                .collect(),
            sigles,
        }
    }

    /// Les mots de la demande qui désignent un mot de `text` : égaux, ou sigle qui le commence.
    fn matched(&self, text: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for w in words(text) {
            if self.words.contains(&w) {
                out.insert(w);
                continue;
            }
            for s in &self.sigles {
                if w.starts_with(s.as_str()) && w.len() <= s.len() + SIGLE_SLACK {
                    out.insert(s.clone());
                }
            }
        }
        out
    }

    /// Un mot de la demande égale ou commence (sigle) un mot de `text`.
    fn names(&self, text: &str) -> bool {
        !self.matched(text).is_empty()
    }

    /// Les mots de la demande qui désignent une valeur du champ : une valeur de 1 ou 2 signes
    /// égale à un mot brut de la demande (ce mot compte) ; plus longue, ses mots désignés.
    fn value_words(&self, field: &Value) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let values = field.get("values").and_then(Value::as_array);
        for v in values.into_iter().flatten().filter_map(Value::as_str) {
            if v.chars().count() <= 2 {
                if self.raw.contains(v) {
                    out.insert(v.to_string());
                }
            } else {
                out.extend(self.matched(v));
            }
        }
        out
    }

    /// Une valeur du champ est désignée.
    fn has_value(&self, field: &Value) -> bool {
        !self.value_words(field).is_empty()
    }
}

/// Petite liste de mots vides, au singulier comme au pluriel (comparés avant le pluriel ôté).
const STOP_WORDS: &[&str] = &[
    "le", "la", "les", "l", "de", "des", "du", "d", "un", "une", "et", "ou", "a", "au", "aux",
    "en", "dans", "sur", "sous", "pour", "par", "avec", "sans", "moins", "plus", "que", "qui",
    "quoi", "dont", "est", "sont", "ce", "ces", "cet", "cette", "son", "sa", "ses", "mon", "ma",
    "mes", "ton", "ta", "tes", "nos", "vos", "leur", "leurs", "tout", "tous", "toute", "toutes",
    "moi", "toi", "nous", "vous", "il", "elle", "ils", "elles", "on", "the", "of", "and",
];

/// Les mots normalisés de `text` : minuscules, accents ôtés, découpe sur tout ce qui n'est ni
/// lettre ni chiffre, mots vides retirés, `s` ou `x` final ôté au-delà de 3 lettres.
fn words(text: &str) -> Vec<String> {
    let folded = fold(text);
    folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !STOP_WORDS.contains(w))
        .map(|w| {
            if w.chars().count() > 3 && (w.ends_with('s') || w.ends_with('x')) {
                w[..w.len() - 1].to_string()
            } else {
                w.to_string()
            }
        })
        .collect()
}

/// `text` en minuscules, accents ôtés par une table écrite à la main (pas de dépendance).
fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => out.push('a'),
            'ç' => out.push('c'),
            'é' | 'è' | 'ê' | 'ë' => out.push('e'),
            'î' | 'ï' | 'í' | 'ì' => out.push('i'),
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => out.push('o'),
            'ù' | 'û' | 'ü' | 'ú' => out.push('u'),
            'ÿ' | 'ý' => out.push('y'),
            'ñ' => out.push('n'),
            'œ' => out.push_str("oe"),
            'æ' => out.push_str("ae"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_inventory::model_inventory;
    use std::time::{Duration, UNIX_EPOCH};

    /// Une couche brute de QGIS, d'identifiant QGIS `<name>_<uuid>` (uuid tiré de `seed`).
    fn qgis_layer(name: &str, seed: usize, source: &str, fields: Vec<Value>) -> Value {
        json!({"layer_id": qgis_id(name, seed), "name": name, "source": source,
            "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiPolygon",
            "feature_count": 10, "fields": fields})
    }

    fn qgis_id(name: &str, seed: usize) -> String {
        format!("{name}_{seed:08x}_1111_2222_3333_444455556666")
    }

    fn raster(name: &str, seed: usize) -> Value {
        json!({"layer_id": qgis_id(name, seed), "name": name, "source": format!("{name}.tif"),
            "crs": "EPSG:2154", "kind": "raster"})
    }

    fn text_field(name: &str, values: &[&str]) -> Value {
        json!({"name": name, "type": "String", "values": values})
    }

    fn real_field(name: &str) -> Value {
        json!({"name": name, "type": "Real"})
    }

    fn project(layers: Vec<Value>) -> Value {
        json!({"project_name": "p", "project_crs": "EPSG:2154", "layers": layers})
    }

    /// Réglages qui trient toujours (seuil nul), plafonds et budgets par défaut.
    fn always() -> ViewLimits {
        ViewLimits {
            threshold: 0,
            ..ViewLimits::default()
        }
    }

    fn view(raw: &Value, request: &str, limits: &ViewLimits) -> ModelInventory {
        sorted_view(
            raw,
            &[],
            &HashSet::new(),
            &LastPlan::default(),
            request,
            limits,
        )
        .0
    }

    fn names(list: &Value) -> Vec<String> {
        list.as_array()
            .into_iter()
            .flatten()
            .map(|l| l["name"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    /// Quelques couches neutres, qu'aucune demande des tests ne désigne.
    fn neutral(count: usize) -> Vec<Value> {
        (0..count)
            .map(|k| {
                qgis_layer(
                    &format!("objet_{k}"),
                    100 + k,
                    "fond.gpkg",
                    vec![real_field("hauteur")],
                )
            })
            .collect()
    }

    #[test]
    fn under_the_threshold_the_view_is_the_model_inventory_byte_for_byte() {
        let mut layers = neutral(3);
        layers.push(qgis_layer(
            "gare_sncf",
            1,
            "gares.gpkg",
            vec![text_field("nature", &["Gare"])],
        ));
        let raw = project(layers);
        let files: Vec<WorkFile> = Vec::new();
        let expected = model_inventory(
            &raw,
            &files,
            &HashSet::new(),
            INVENTORY_BUDGET_CHARS,
            INVENTORY_LOW_CHARS,
        );
        let got = sorted_view(
            &raw,
            &files,
            &HashSet::new(),
            &LastPlan::default(),
            "les gares",
            &ViewLimits::default(),
        );
        assert_eq!(
            got.0.inventory.to_string(),
            expected.0.inventory.to_string()
        );
        assert_eq!(got, expected);
    }

    #[test]
    fn a_plural_word_finds_the_layer_by_its_name() {
        let mut layers = neutral(3);
        layers.push(qgis_layer("gare_sncf", 1, "fer.gpkg", vec![]));
        let out = view(&project(layers), "Trouve les gares", &always());
        assert_eq!(names(&out.inventory["layers"]), ["gare_sncf"]);
        // Les quatre couches de l'utilisateur en tête de l'index : l'ancrage du cache.
        assert_eq!(out.user_layers, 4);
        assert_eq!(out.inventory["index"].as_array().map(Vec::len), Some(4));
    }

    #[test]
    fn accents_and_plurals_in_x_are_folded() {
        let layers = vec![
            qgis_layer("Réseau des eaux", 1, "a.gpkg", vec![]),
            qgis_layer("Écoles", 2, "b.gpkg", vec![]),
            qgis_layer("autre", 3, "c.gpkg", vec![]),
        ];
        let out = view(
            &project(layers),
            "le reseau d'eau près des ÉCOLE",
            &always(),
        );
        assert_eq!(
            names(&out.inventory["layers"]),
            ["Réseau des eaux", "Écoles"]
        );
    }

    #[test]
    fn the_id_stem_and_the_file_stem_designate_a_layer_too() {
        let layers = vec![
            json!({"layer_id": qgis_id("troncon_route", 1), "name": "Couche 1",
                "source": "x.gpkg", "kind": "vector", "fields": []}),
            json!({"layer_id": "c2", "name": "Couche 2",
                "source": "C:/data/Communes_74.shp|layername=communes_74", "kind": "vector",
                "fields": []}),
            json!({"layer_id": "c3", "name": "Couche 3", "source": "x.gpkg|layername=troncon",
                "kind": "vector", "fields": []}),
        ];
        let out = view(
            &project(layers.clone()),
            "les routes des communes",
            &always(),
        );
        assert_eq!(names(&out.inventory["layers"]), ["Couche 1", "Couche 2"]);
        // La table `layername=` d'un GeoPackage désigne sa couche, pas le fichier partagé.
        let out = view(&project(layers.clone()), "le troncon", &always());
        assert_eq!(names(&out.inventory["layers"]), ["Couche 1", "Couche 3"]);
        let out = view(&project(layers), "le fichier x", &always());
        assert!(names(&out.inventory["layers"]).is_empty());
    }

    #[test]
    fn a_short_value_designates_only_the_layer_with_that_exact_value() {
        let layers = vec![
            qgis_layer(
                "plu_a",
                1,
                "a.shp",
                vec![text_field("typezone", &["AU", "N"])],
            ),
            qgis_layer(
                "plu_b",
                2,
                "b.shp",
                vec![text_field("typezone", &["N", "U"])],
            ),
            qgis_layer(
                "plu_c",
                3,
                "c.shp",
                vec![text_field("typezone", &["Ua", "u"])],
            ),
        ];
        let out = view(&project(layers), "les zones U", &always());
        assert_eq!(names(&out.inventory["layers"]), ["plu_b"]);
        // Un mot vide de la demande ne désigne pas une valeur courte qui l'égale.
        let layers = vec![
            qgis_layer("plu_d", 4, "d.shp", vec![text_field("particule", &["de"])]),
            qgis_layer("plu_e", 5, "e.shp", vec![text_field("typezone", &["A"])]),
        ];
        let out = view(&project(layers), "les zones de type A", &always());
        assert_eq!(names(&out.inventory["layers"]), ["plu_e"]);
    }

    #[test]
    fn the_threshold_measures_the_user_layers_only() {
        let (raw, files) = with_results();
        let (ordered, users) = stable_order(agent_inventory(&raw), &files);
        let ids = ShortIds::of_inventory(&ordered);
        let all = ids.shorten(results_without_values(ordered, &files));
        let mut user_only = all.clone();
        user_only["layers"] = json!(all["layers"].as_array().unwrap()[..users].to_vec());
        let (small, big) = (json_chars(&user_only), json_chars(&all));
        assert!(small < big);
        let limits = ViewLimits {
            threshold: (small + big) / 2,
            ..ViewLimits::default()
        };
        let got = sorted_view(
            &raw,
            &files,
            &HashSet::new(),
            &LastPlan::default(),
            "",
            &limits,
        );
        let expected = model_inventory(&raw, &files, &HashSet::new(), limits.budget, limits.low);
        assert_eq!(
            got.0.inventory.to_string(),
            expected.0.inventory.to_string()
        );
        assert!(got.0.inventory.get("index").is_none());
        assert_eq!(got, expected);
    }

    #[test]
    fn a_field_designated_by_a_value_is_kept_in_a_capped_layer() {
        let fields: Vec<Value> = (0..12)
            .map(|k| {
                let values: Vec<String> = (0..8).map(|i| format!("valeur_{k}_{i}")).collect();
                let values: Vec<&str> = values.iter().map(String::as_str).collect();
                let mut f = text_field(&format!("champ_{k}"), &values);
                if k == 11 {
                    f = text_field("champ_11", &["Viaduc", "Pont"]);
                }
                f
            })
            .collect();
        let raw = project(vec![qgis_layer("ouvrages", 1, "o.gpkg", fields)]);
        let limits = ViewLimits {
            max_layer_chars: 600,
            ..always()
        };
        let out = view(&raw, "les ouvrages de type viaduc", &limits);
        let layer = &out.inventory["layers"][0];
        assert!(json_chars(layer) <= 600);
        let kept: Vec<&str> = layer["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert!(kept.contains(&"champ_11"), "{kept:?}");
        assert!(kept.len() < 12);
    }

    #[test]
    fn name_comes_before_field_then_value_and_ties_keep_the_stable_order() {
        let layers = vec![
            qgis_layer(
                "par_valeur",
                1,
                "a.gpkg",
                vec![text_field("t", &["Gare TGV"])],
            ),
            qgis_layer("par_champ", 2, "b.gpkg", vec![real_field("dist_gare")]),
            qgis_layer("gare_b", 3, "c.gpkg", vec![]),
            qgis_layer("gare_a", 4, "d.gpkg", vec![]),
        ];
        let raw = project(layers);
        let request = Request::new("la gare");
        let real = stable_order(agent_inventory(&raw), &[]).0;
        let real = real["layers"].as_array().unwrap().clone();
        let order = detail_order(&real, &[true; 4], &[], &LastPlan::default(), &[], &request);
        assert_eq!(order, [2, 3, 1, 0]);
        // Rendues dans l'ordre stable, quelle que soit la priorité.
        let out = view(&raw, "la gare", &always());
        assert_eq!(
            names(&out.inventory["layers"]),
            ["par_valeur", "par_champ", "gare_b", "gare_a"]
        );
    }

    #[test]
    fn two_close_layers_are_both_detailed() {
        let mut layers = neutral(4);
        layers.insert(1, qgis_layer("gare_sncf", 1, "a.gpkg", vec![]));
        layers.push(qgis_layer("gares_routieres", 2, "b.gpkg", vec![]));
        let out = view(&project(layers), "à moins de 500 m des gares", &always());
        assert_eq!(
            names(&out.inventory["layers"]),
            ["gare_sncf", "gares_routieres"]
        );
    }

    /// Une couche de 88 champs : 4 textes à valeurs, 20 numériques, 64 textes sans valeurs, mêlés.
    fn wide_layer() -> Value {
        let fields: Vec<Value> = (0..88)
            .map(|k| match k % 22 {
                _ if k == 87 => json!({"name": "pente_87", "type": "String"}),
                0 => text_field(&format!("nature_{k}"), &["Autoroute", "Route", "Chemin"]),
                1..=5 => json!({"name": format!("largeur_{k}"), "type": "Integer"}),
                _ => json!({"name": format!("commentaire_{k}"), "type": "String"}),
            })
            .collect();
        qgis_layer("troncon_large", 9, "large.gpkg", fields)
    }

    #[test]
    fn a_wide_layer_keeps_designated_then_valued_fields_under_both_caps() {
        let raw = project(vec![wide_layer(), qgis_layer("autre", 1, "a.gpkg", vec![])]);
        let out = view(&raw, "troncon large, pente", &always());
        let layers = out.inventory["layers"].as_array().unwrap();
        assert_eq!(names(&out.inventory["layers"]), ["troncon_large"]);
        let layer = &layers[0];
        let fields: Vec<&str> = layer["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert!(fields.len() <= DETAIL_MAX_FIELDS, "{}", fields.len());
        assert!(
            json_chars(layer) <= DETAIL_MAX_CHARS,
            "{}",
            json_chars(layer)
        );
        // Le champ désigné, puis les quatre champs à valeurs, puis des numériques.
        assert!(fields.contains(&"pente_87"));
        for k in [0, 22, 44, 66] {
            assert!(
                fields.contains(&format!("nature_{k}").as_str()),
                "{fields:?}"
            );
        }
        assert!(fields.contains(&"largeur_1"));
        // Rendus dans l'ordre de QGIS.
        let position = |n: &str| n.rsplit('_').next().unwrap().parse::<usize>().unwrap();
        assert!(fields.windows(2).all(|w| position(w[0]) < position(w[1])));
        // Tous les numériques, puis des champs du dernier rang jusqu'au plafond de 30.
        let numeric = fields.iter().filter(|f| f.starts_with("largeur")).count();
        assert_eq!(numeric, 20);
        assert_eq!(fields.len(), DETAIL_MAX_FIELDS);
    }

    #[test]
    fn the_character_cap_stops_at_the_first_field_that_does_not_fit() {
        let limits = ViewLimits {
            max_layer_chars: 600,
            ..always()
        };
        let raw = project(vec![wide_layer()]);
        let out = view(&raw, "troncon", &limits);
        let layer = &out.inventory["layers"][0];
        assert!(json_chars(layer) <= 600);
        let fields = layer["fields"].as_array().unwrap();
        assert!(!fields.is_empty() && fields.len() < 30);
    }

    #[test]
    fn the_index_is_the_same_for_two_different_requests() {
        let mut layers = neutral(5);
        layers.push(qgis_layer("gare_sncf", 1, "a.gpkg", vec![]));
        layers.push(qgis_layer(
            "plu",
            2,
            "b.gpkg",
            vec![text_field("typezone", &["U"])],
        ));
        let raw = project(layers);
        let a = view(&raw, "les gares", &always());
        let b = view(&raw, "zones U du plu", &always());
        assert_ne!(a.inventory["layers"], b.inventory["layers"]);
        assert_eq!(
            a.inventory["index"].to_string(),
            b.inventory["index"].to_string()
        );
        // Toutes les couches y figurent, en forme réduite, aux identifiants courts.
        let index = a.inventory["index"].as_array().unwrap();
        assert_eq!(index.len(), 7);
        assert_eq!(
            index[5],
            json!({"layer_id": "gare_sncf_0000", "name": "gare_sncf", "geometry": "MultiPolygon"})
        );
        // `index` vient avant `layers` dans le texte.
        let text = a.inventory.to_string();
        assert!(text.find("\"index\"").unwrap() < text.find("\"layers\"").unwrap());
    }

    #[test]
    fn the_view_is_deterministic() {
        let raw = project(vec![
            wide_layer(),
            qgis_layer("gare_sncf", 1, "a.gpkg", vec![]),
            raster("mnt", 2),
        ]);
        let plan = LastPlan {
            results: HashSet::new(),
            cited: [qgis_id("troncon_large", 9), qgis_id("gare_sncf", 1)]
                .into_iter()
                .collect(),
        };
        let first = sorted_view(&raw, &[], &HashSet::new(), &plan, "gares", &always());
        for _ in 0..20 {
            let again = sorted_view(&raw, &[], &HashSet::new(), &plan, "gares", &always());
            assert_eq!(again.0.inventory.to_string(), first.0.inventory.to_string());
            assert_eq!(again, first);
        }
    }

    /// Projet à résultats : trois couches de l'utilisateur (PLU, gares, MNT), puis les résultats
    /// des étapes 1 à 4, posés par QGIS en tête de légende ; l'étape 4 est du dernier plan.
    fn with_results() -> (Value, Vec<WorkFile>) {
        let step = |k: usize| format!("{k}-analysis.buffer.gpkg");
        let mut layers: Vec<Value> = (1..=4)
            .rev()
            .map(|k| {
                qgis_layer(
                    &format!("tampon_{k}"),
                    10 + k,
                    &step(k),
                    vec![text_field("nature", &["Gare"])],
                )
            })
            .collect();
        layers.push(qgis_layer(
            "plu",
            1,
            "plu.gpkg",
            vec![text_field("typezone", &["U"])],
        ));
        layers.push(qgis_layer("gare_sncf", 2, "gares.gpkg", vec![]));
        layers.push(raster("mnt", 3));
        let files = (1..=4)
            .map(|k| WorkFile {
                name: step(k),
                modified: UNIX_EPOCH + Duration::from_secs(k as u64),
            })
            .collect();
        (project(layers), files)
    }

    #[test]
    fn the_last_plan_results_and_cited_layers_come_after_the_designated_then_rasters() {
        let (raw, files) = with_results();
        let plan = LastPlan {
            results: ["4-analysis.buffer.gpkg".to_string()].into_iter().collect(),
            cited: [qgis_id("plu", 1)].into_iter().collect(),
        };
        let (ordered, _) = stable_order(agent_inventory(&raw), &files);
        let real = results_without_values(ordered, &files)["layers"]
            .as_array()
            .unwrap()
            .clone();
        let names_of = |order: Vec<usize>| -> Vec<String> {
            order
                .into_iter()
                .map(|i| real[i]["name"].as_str().unwrap().to_string())
                .collect()
        };
        let order = detail_order(
            &real,
            &[true; 7],
            &files,
            &plan,
            &[],
            &Request::new("gares"),
        );
        assert_eq!(names_of(order), ["gare_sncf", "tampon_4", "plu", "mnt"]);
        // Une couche réclamée par la redemande passe avant toute autre.
        let wanted = [qgis_id("tampon_1", 11)];
        let order = detail_order(
            &real,
            &[true; 7],
            &files,
            &plan,
            &wanted,
            &Request::new("gares"),
        );
        assert_eq!(
            names_of(order),
            ["tampon_1", "gare_sncf", "tampon_4", "plu", "mnt"]
        );

        let (out, ids) = sorted_view(&raw, &files, &HashSet::new(), &plan, "gares", &always());
        // Ordre stable : l'utilisateur d'abord, puis les résultats du plus ancien au plus récent.
        assert_eq!(
            names(&out.inventory["layers"]),
            ["plu", "gare_sncf", "mnt", "tampon_4"]
        );
        assert_eq!(out.user_layers, 3);
        assert_eq!(
            names(&out.inventory["index"]),
            [
                "plu",
                "gare_sncf",
                "mnt",
                "tampon_1",
                "tampon_2",
                "tampon_3",
                "tampon_4"
            ]
        );
        // Le résultat est montré sans valeurs, la couche de l'utilisateur avec.
        let layers = out.inventory["layers"].as_array().unwrap();
        assert!(layers[3]["fields"][0].get("values").is_none());
        assert_eq!(layers[0]["fields"][0]["values"], json!(["U"]));
        // Identifiants courts partout, retraduits par la table.
        for layer in layers
            .iter()
            .chain(out.inventory["index"].as_array().unwrap())
        {
            let id = layer["layer_id"].as_str().unwrap();
            assert!(id.len() < 20, "{id}");
            let mut back = json!(id);
            ids.restore(&mut back);
            assert_ne!(back, json!(id));
        }
        assert!(out.left_out.is_empty());
    }

    #[test]
    fn a_layer_that_does_not_fit_is_skipped_and_the_next_one_tried() {
        let (raw, files) = with_results();
        let plan = LastPlan {
            results: ["4-analysis.buffer.gpkg".to_string()].into_iter().collect(),
            cited: [qgis_id("plu", 1)].into_iter().collect(),
        };
        let full = sorted_view(&raw, &files, &HashSet::new(), &plan, "gares", &always()).0;
        // Budget qui tient la vue sans `plu` : le PLU (3e) ne tient pas, il est sauté ; le MNT
        // (4e), plus court, tient encore.
        let plu = json_chars(&full.inventory["layers"][0]) + 1;
        let limits = ViewLimits {
            sorted_budget: json_chars(&full.inventory) - plu,
            ..always()
        };
        let out = sorted_view(&raw, &files, &HashSet::new(), &plan, "gares", &limits).0;
        assert_eq!(
            names(&out.inventory["layers"]),
            ["gare_sncf", "mnt", "tampon_4"]
        );
        assert!(json_chars(&out.inventory) <= limits.sorted_budget);
        assert_eq!(out.user_layers, 3);
    }

    #[test]
    fn the_sorted_budget_bounds_the_view_and_the_under_threshold_budget_does_not() {
        let (raw, files) = with_results();
        let wide = sorted_view(
            &raw,
            &files,
            &HashSet::new(),
            &LastPlan::default(),
            "gares plu",
            &always(),
        )
        .0;
        let limits = ViewLimits {
            sorted_budget: json_chars(&wide.inventory) - 1,
            budget: 1_000_000,
            ..always()
        };
        let out = sorted_view(
            &raw,
            &files,
            &HashSet::new(),
            &LastPlan::default(),
            "gares plu",
            &limits,
        )
        .0;
        assert!(json_chars(&out.inventory) <= limits.sorted_budget);
        assert!(names(&out.inventory["layers"]).len() < names(&wide.inventory["layers"]).len());
    }

    #[test]
    fn a_word_counts_once_and_name_words_come_first() {
        // « bâtiments par commune » : la couche des bâtiments (un mot par son nom) passe avant une
        // couche désignée par deux noms de champ.
        let layers = vec![
            qgis_layer(
                "erp",
                1,
                "a.gpkg",
                vec![
                    real_field("insee_commune"),
                    real_field("liens_vers_batiment"),
                ],
            ),
            qgis_layer("batiment", 2, "b.gpkg", vec![real_field("hauteur")]),
            qgis_layer("commune", 3, "c.gpkg", vec![real_field("population")]),
            qgis_layer(
                "valeurs",
                4,
                "d.gpkg",
                vec![text_field("t", &["Bâtiment", "Commune"])],
            ),
        ];
        let raw = project(layers);
        let real = stable_order(agent_inventory(&raw), &[]).0;
        let real = real["layers"].as_array().unwrap().clone();
        let request = Request::new("Combien de bâtiments par commune ?");
        let order = detail_order(&real, &[true; 4], &[], &LastPlan::default(), &[], &request);
        // Deux couches à un mot par leur nom ; puis deux mots par champ, puis deux par valeur.
        assert_eq!(order, [1, 2, 0, 3]);
        // Un même mot trouvé par trois champs d'une couche ne compte qu'une fois.
        let one = Request::new("la commune");
        let thrice = qgis_layer(
            "x",
            5,
            "x.gpkg",
            vec![
                real_field("insee_commune"),
                real_field("nom_commune"),
                real_field("code_commune"),
            ],
        );
        let layer = &agent_inventory(&project(vec![thrice]))["layers"][0];
        assert_eq!(
            layer_score(layer, &one),
            Some(Score {
                by_name: 0,
                all: 1,
                by_field: 1
            })
        );
    }

    #[test]
    fn a_sigle_finds_a_longer_word_it_begins() {
        let layers = vec![
            qgis_layer(
                "zone_urba",
                1,
                "z.shp",
                vec![text_field("idurba", &["200066793_PLUI_20260326"])],
            ),
            qgis_layer("pluviometre", 2, "p.shp", vec![text_field("t", &["Pluie"])]),
            qgis_layer("autre", 3, "a.shp", vec![text_field("t", &["plume"])]),
        ];
        let raw = project(layers);
        // « PLU » en capitales trouve « PLUI » (un signe de plus) ; ni « pluviometre » ni
        // « pluie » ni « plume » (deux signes de plus ou davantage).
        let out = view(&raw, "le zonage du PLU", &always());
        assert_eq!(names(&out.inventory["layers"]), ["zone_urba"]);
        // En minuscules, « plu » n'est pas un sigle : rien.
        let out = view(&raw, "le zonage du plu", &always());
        assert!(names(&out.inventory["layers"]).is_empty());
        // Un sigle trouve aussi un mot égal (et, d'un signe de plus, « pluie »), jamais « plume ».
        let out = view(&raw, "les zones du PLUI", &always());
        assert_eq!(
            names(&out.inventory["layers"]),
            ["zone_urba", "pluviometre"]
        );
        // Un mot à moins de deux lettres n'est pas un sigle (« A3 », « 500 »).
        assert!(Request::new("format A3, 500 m, MNT").sigles.contains("mnt"));
        assert_eq!(Request::new("format A3, 500 m, MNT").sigles.len(), 1);
    }

    #[test]
    fn a_wanted_layer_is_detailed_first_even_if_nothing_designates_it() {
        let mut layers = neutral(3);
        layers.push(qgis_layer(
            "zonage",
            7,
            "z.gpkg",
            vec![text_field("typezone", &["U", "N"])],
        ));
        let raw = project(layers);
        let wanted = [qgis_id("zonage", 7)];
        let (out, _) = sorted_view_learned(
            agent_inventory(&raw),
            &[],
            &HashSet::new(),
            &LastPlan::default(),
            "rien ici",
            &wanted,
            &always(),
        );
        assert_eq!(names(&out.inventory["layers"]), ["zonage"]);
        // Sans réclamation, rien n'est détaillé.
        assert!(names(&view(&raw, "rien ici", &always()).inventory["layers"]).is_empty());
    }

    #[test]
    fn the_learned_entry_is_the_raw_entry_after_translation() {
        let (raw, files) = with_results();
        for request in ["gares", "", "le plu"] {
            for limits in [always(), ViewLimits::default()] {
                let a = sorted_view(
                    &raw,
                    &files,
                    &HashSet::new(),
                    &LastPlan::default(),
                    request,
                    &limits,
                );
                let b = sorted_view_learned(
                    agent_inventory(&raw),
                    &files,
                    &HashSet::new(),
                    &LastPlan::default(),
                    request,
                    &[],
                    &limits,
                );
                assert_eq!(a, b, "{request:?}");
            }
        }
    }

    #[test]
    fn an_index_too_long_drops_the_oldest_results_by_batches_never_a_user_layer() {
        let (raw, files) = with_results();
        let full = sorted_view(
            &raw,
            &files,
            &HashSet::new(),
            &LastPlan::default(),
            "",
            &always(),
        )
        .0;
        let index = full.inventory["index"].as_array().unwrap();
        let users: usize = index[..3].iter().map(json_chars).sum::<usize>() + 2 + 2;
        let one = json_chars(&index[3]) + 1;
        // L'index tient l'utilisateur et trois résultats, pas quatre ; le seuil bas, un seul.
        let limits = ViewLimits {
            index_budget: users + 3 * one,
            index_low: users + one,
            ..always()
        };
        let (out, _) = sorted_view(
            &raw,
            &files,
            &HashSet::new(),
            &LastPlan::default(),
            "les tampons",
            &limits,
        );
        assert_eq!(
            names(&out.inventory["index"]),
            ["plu", "gare_sncf", "mnt", "tampon_4"]
        );
        assert_eq!(
            out.left_out,
            vec![
                json!({"layer_id": qgis_id("tampon_1", 11), "name": "tampon_1"}),
                json!({"layer_id": qgis_id("tampon_2", 12), "name": "tampon_2"}),
                json!({"layer_id": qgis_id("tampon_3", 13), "name": "tampon_3"}),
            ]
        );
        // Une couche sortie de l'index n'est pas détaillée ; le raster l'est.
        assert_eq!(names(&out.inventory["layers"]), ["mnt", "tampon_4"]);
        // Même un index minuscule garde toutes les couches de l'utilisateur.
        let tiny = ViewLimits {
            index_budget: 0,
            index_low: 0,
            ..always()
        };
        let (out, _) = sorted_view(
            &raw,
            &files,
            &HashSet::new(),
            &LastPlan::default(),
            "",
            &tiny,
        );
        assert_eq!(names(&out.inventory["index"]), ["plu", "gare_sncf", "mnt"]);
        assert_eq!(out.left_out.len(), 4);
    }

    #[test]
    fn intermediates_leave_the_index_as_they_leave_the_view() {
        let (raw, files) = with_results();
        let hidden: HashSet<String> = ["2-analysis.buffer.gpkg".to_string()].into_iter().collect();
        let (out, _) = sorted_view(&raw, &files, &hidden, &LastPlan::default(), "", &always());
        assert_eq!(
            names(&out.inventory["index"]),
            [
                "plu",
                "gare_sncf",
                "mnt",
                "tampon_1",
                "tampon_3",
                "tampon_4"
            ]
        );
        assert!(out.left_out.is_empty());
    }

    // -- gros projet synthétique, de même forme que la BD TOPO 74 + PLUi + MNT --

    /// Forme de chaque couche du projet réel de 63 couches, mesurée le 2026-11-10 hors du dépôt
    /// (nombres seulement, aucun nom ni valeur réels) : géométrie, champs, dont entiers, dont
    /// réels, dont textes à valeurs, et nombre de valeurs de la couche. Les 54 premières sont
    /// d'un GeoPackage, les 7 suivantes de fichiers Shapefile, les 2 dernières des MNT.
    const SHAPES: [(&str, usize, usize, usize, usize, usize); 63] = [
        ("Point", 20, 1, 0, 2, 12),
        ("MultiPolygon", 19, 0, 2, 9, 27),
        ("MultiPolygon", 11, 0, 0, 7, 44),
        ("MultiPolygon", 20, 0, 1, 4, 4),
        ("MultiPolygon", 28, 2, 7, 6, 39),
        ("Point", 6, 0, 0, 0, 0),
        ("LineString", 14, 0, 2, 5, 10),
        ("MultiPolygon", 12, 0, 0, 4, 16),
        ("MultiPolygon", 17, 0, 2, 8, 26),
        ("MultiPolygon", 10, 0, 0, 6, 16),
        ("MultiPolygon", 26, 2, 0, 5, 17),
        ("MultiPolygon", 14, 1, 0, 3, 22),
        ("LineString", 17, 0, 2, 7, 37),
        ("Point", 18, 0, 3, 7, 37),
        ("MultiPolygon", 17, 0, 2, 7, 26),
        ("MultiLineString", 15, 0, 0, 4, 9),
        ("MultiPolygon", 10, 0, 0, 6, 16),
        ("Point", 19, 0, 1, 7, 16),
        ("Point", 17, 0, 1, 5, 14),
        ("MultiPolygon", 11, 0, 0, 1, 2),
        ("MultiPolygon", 25, 0, 2, 5, 22),
        ("Point", 34, 2, 1, 7, 36),
        ("MultiPolygon", 13, 0, 1, 5, 8),
        ("LineString", 11, 2, 1, 2, 2),
        ("MultiLineString", 13, 0, 0, 4, 10),
        ("LineString", 15, 1, 0, 3, 13),
        ("Point", 16, 0, 1, 3, 10),
        ("LineString", 15, 0, 2, 7, 22),
        ("LineString", 15, 0, 2, 4, 17),
        ("Point", 22, 0, 2, 6, 19),
        ("Point", 7, 0, 0, 0, 0),
        ("MultiPolygon", 15, 0, 1, 6, 28),
        ("MultiPolygon", 14, 0, 2, 5, 8),
        ("MultiPolygon", 22, 0, 2, 4, 15),
        ("Point", 19, 1, 1, 6, 28),
        ("Point", 20, 0, 2, 7, 21),
        ("MultiPolygon", 15, 0, 2, 5, 12),
        ("Point", 14, 0, 3, 4, 15),
        ("MultiPolygon", 9, 0, 0, 5, 5),
        ("MultiPolygon", 20, 0, 8, 6, 20),
        ("MultiLineString", 12, 0, 0, 4, 21),
        ("LineString", 11, 0, 0, 4, 16),
        ("MultiPolygon", 29, 0, 2, 9, 28),
        ("MultiPolygon", 14, 0, 2, 6, 32),
        ("Point", 9, 0, 0, 1, 4),
        ("LineString", 16, 0, 2, 7, 29),
        ("LineString", 86, 5, 8, 22, 104),
        ("LineString", 21, 2, 2, 8, 34),
        ("LineString", 44, 0, 2, 14, 50),
        ("MultiLineString", 9, 0, 0, 5, 16),
        ("MultiLineString", 17, 0, 0, 1, 2),
        ("MultiPolygon", 23, 0, 1, 5, 20),
        ("MultiPolygon", 20, 0, 1, 6, 27),
        ("MultiPolygon", 10, 0, 1, 2, 12),
        ("MultiPolygon", 19, 0, 0, 9, 12),
        ("MultiPolygon", 13, 0, 0, 8, 8),
        ("MultiPolygon", 19, 0, 0, 9, 24),
        ("MultiPolygon", 19, 0, 0, 14, 48),
        ("MultiPolygon", 19, 0, 0, 13, 25),
        ("MultiPolygon", 19, 0, 0, 9, 27),
        ("MultiPolygon", 13, 0, 0, 5, 9),
        ("raster", 0, 0, 0, 0, 0),
        ("raster", 0, 0, 0, 0, 0),
    ];

    /// Taille de l'inventaire complet du projet réel, forme de l'entraînement (2026-11-10).
    const REAL_FULL_CHARS: usize = 84_463;

    /// Le projet synthétique : noms, champs et valeurs inventés, de longueurs moyennes proches
    /// du projet réel (nom de couche ≈ 18 signes, de champ ≈ 16, valeur ≈ 11).
    fn big_project() -> Value {
        let layers: Vec<Value> = SHAPES
            .iter()
            .enumerate()
            .map(|(k, &(geometry, count, ints, reals, valued, values))| {
                let name = format!("couche{k:02}_objet_synth");
                let id = format!("{name}_{k:08x}_1111_2222_3333_444455556666");
                if geometry == "raster" {
                    return json!({"layer_id": id, "name": name, "source": format!("mnt_{k}.tif"),
                        "crs": "EPSG:2154", "kind": "raster"});
                }
                let source = if k < 54 {
                    format!("referentiel_synthetique.gpkg|layername={name}")
                } else {
                    format!("plan_local_synth_{k}.shp")
                };
                let fields: Vec<Value> = (0..count)
                    .map(|j| {
                        let field = format!("attribut_{j:02}_synth");
                        if j < ints {
                            json!({"name": field, "type": "Integer"})
                        } else if j < ints + reals {
                            json!({"name": field, "type": "Real"})
                        } else if j < ints + reals + valued {
                            let v = j - ints - reals;
                            let n = values / valued + usize::from(v < values % valued);
                            let vals: Vec<String> =
                                (0..n).map(|i| format!("valeur_{i:02}_x")).collect();
                            json!({"name": field, "type": "String", "values": vals})
                        } else {
                            json!({"name": field, "type": "String"})
                        }
                    })
                    .collect();
                json!({"layer_id": id, "name": name, "source": source, "crs": "EPSG:2154",
                    "kind": "vector", "geometry": geometry, "feature_count": 12345,
                    "fields": fields})
            })
            .collect();
        json!({"project_name": "gros-projet", "project_crs": "EPSG:2154", "layers": layers})
    }

    #[test]
    fn the_synthetic_big_project_has_the_size_of_the_real_one() {
        let raw = big_project();
        let full = ShortIds::of_inventory(&agent_inventory(&raw)).shorten(agent_inventory(&raw));
        let size = json_chars(&full);
        assert!(
            size.abs_diff(REAL_FULL_CHARS) * 10 <= REAL_FULL_CHARS,
            "{size} au lieu de {REAL_FULL_CHARS} à 10 % près"
        );
        // Six fois le budget : sans tri, ni la vue d'aujourd'hui ni le budget n'y peuvent rien.
        let today = model_inventory(
            &raw,
            &[],
            &HashSet::new(),
            INVENTORY_BUDGET_CHARS,
            INVENTORY_LOW_CHARS,
        );
        assert!(json_chars(&today.0.inventory) > 6 * INVENTORY_BUDGET_CHARS);
    }

    #[test]
    fn the_big_project_fits_the_budget_whatever_the_request() {
        let raw = big_project();
        for request in [
            "la couche12 et la couche30",
            "tous les objets synthétiques",
            "attribut_03_synth vaut valeur_01_x",
            "",
        ] {
            let (out, _) = sorted_view(
                &raw,
                &[],
                &HashSet::new(),
                &LastPlan::default(),
                request,
                &ViewLimits::default(),
            );
            let size = json_chars(&out.inventory);
            assert!(size <= SORTED_BUDGET_CHARS, "{request:?} : {size}");
            assert_eq!(out.inventory["index"].as_array().map(Vec::len), Some(63));
            assert!(out.left_out.is_empty());
            // Chaque couche détaillée tient sous les deux plafonds.
            let layers = out.inventory["layers"].as_array().unwrap();
            assert!(!layers.is_empty(), "{request:?}");
            for layer in layers {
                assert!(json_chars(layer) <= DETAIL_MAX_CHARS);
                assert!(layer["fields"].as_array().unwrap().len() <= DETAIL_MAX_FIELDS);
            }
        }
        // La couche de 86 champs désignée est détaillée, plafonnée.
        let (out, _) = sorted_view(
            &raw,
            &[],
            &HashSet::new(),
            &LastPlan::default(),
            "la couche46",
            &ViewLimits::default(),
        );
        let wide = &out.inventory["layers"][0];
        assert_eq!(wide["name"], "couche46_objet_synth");
        assert_eq!(wide["layer_id"], "couche46_objet_synth_0000");
        assert!(json_chars(wide) <= DETAIL_MAX_CHARS);
    }
}
