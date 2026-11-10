//! Inventaire vu par l'agent (ticket 5.10, tâche 4a).
//!
//! L'extension rend, par `project.inventory`, des faits bruts de QGIS : SCR en code explicite,
//! type de champ nommé par le fournisseur, nom WKB de la géométrie. Le modèle ajusté, lui, a
//! appris sur des inventaires d'une autre forme, celle du générateur de `training/` :
//!
//! ```json
//! {"project": {"name": "...", "epsg": 2154, "utm_epsg": 32631},
//!  "layers": [{"layer_id": "...", "name": "...", "kind": "vector", "geometry": "MultiPolygon",
//!              "epsg": 2154, "feature_count": 120, "source": "plu.gpkg",
//!              "fields": [{"name": "typezone", "type": "string", "values": ["N", "U"]}]}],
//!  "unloaded_files": []}
//! ```
//!
//! La traduction vit ici, dans le noyau, pour que l'extension garde un contrat stable de faits
//! QGIS et que cette forme, qui bouge avec un ré-entraînement, soit confrontée au générateur
//! par un test (`training/tests/forme_inventaire.rs`). Une clé brute absente (extension
//! antérieure) donne une clé absente, jamais une erreur.
//!
//! **Budget d'inventaire** (ticket 5.18). Chaque plan exécuté ajoute ses couches de résultat au
//! projet, donc à l'inventaire lu par le modèle : enchaînées dans une session, les phrases finissent
//! par ne plus tenir dans le contexte. `within_budget_by_batches` borne l'inventaire vu par le
//! modèle : toutes les couches de l'utilisateur, et des couches de résultat les plus récentes ;
//! au dépassement du budget, les plus anciennes sortent par paquets (ticket 5.19). Rien n'est
//! retiré de QGIS, et la forme d'une couche gardée ne change pas.
//!
//! **Ordre stable et identifiants courts** (ticket 5.19). La lecture de l'inventaire domine la
//! durée d'une phrase quand il change ; le moteur reprend un instantané pris à une frontière de couches
//! si tout ce qui la précède est inchangé. `stable_order` range donc les couches de l'utilisateur
//! d'abord (dans l'ordre de QGIS), puis les résultats du plus ancien au plus récent : un plan
//! exécuté n'ajoute ses couches qu'à la fin. L'entraînement mélange l'ordre des couches, cet ordre
//! est donc un ordre déjà vu. `ShortIds` montre au modèle un identifiant court `nom_<4 hex>`
//! (forme apprise) pour un identifiant QGIS `nom_<uuid>`, que le modèle recopie bien plus vite ;
//! le noyau retraduit le plan rendu avant de le vérifier et de le garder : le journal, le plan
//! gardé et l'extension ne voient que les vrais identifiants.
//!
//! **Résultats allégés** (ticket 5.22). Une couche de résultat coûte cher à relire : un plan de
//! démo en produit quatre, et une sélection porte tous les champs de sa couche, valeurs comprises.
//! `without_intermediates` retire de la vue les sorties qu'une étape exécutée du même plan a
//! consommées, reconnues par leur fichier source (le noyau les déduit du journal du projet, ticket
//! 5.34 : elles restent cachées après une relance ou un rejeu, sous de nouveaux ids), et
//! `results_without_values` ôte la clé `values` des champs des résultats. Les couches de
//! l'utilisateur ne changent pas ; QGIS, le journal et le plan gardé non plus.
//!
//! **Vue triée d'un gros projet** (ticket 5.46, changement de cap du 2026-11-10). Le budget ne
//! retire que des résultats : un projet d'une soixantaine de couches de l'utilisateur ne tient
//! jamais. `sorted_view` garde la vue de `model_inventory` tant que les couches de l'utilisateur
//! tiennent sous un seuil ; au-dessus, elle rend un `index` réduit (`{layer_id, name,
//! geometry}`) de toutes les couches, stable d'une demande à l'autre pour un même état du
//! projet, et dans `layers` le détail
//! des couches que la demande désigne, puis des résultats et des couches cités par le dernier
//! plan exécuté, puis des rasters, jusqu'au budget. Elle n'est pas encore branchée à la
//! composition (ticket 5.48).

mod sorted_view;

pub use sorted_view::{
    sorted_view, LastPlan, ViewLimits, DETAIL_MAX_CHARS, DETAIL_MAX_FIELDS, INDEX_BUDGET_CHARS,
    INDEX_LOW_CHARS, SORT_THRESHOLD_CHARS,
};

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::SystemTime;

use serde_json::{json, Map, Value};

/// Plafond de valeurs d'un champ texte : le jeu tenu à l'écart en montre 10 au plus,
/// l'entraînement 9. Au-delà, le champ n'a pas de clé `values`.
pub const MAX_VALUES: usize = 10;

/// Budget de l'inventaire vu par le modèle, en caractères de son texte JSON compact (celui que
/// le prompt court recopie). Mesuré le 2026-10-24 avec le tokeniseur du modèle livré
/// (`docs/entrainement-du-modele.md`, « Budget d'inventaire ») : l'inventaire de Villeneuve
/// compte 0,364 jeton par caractère, une couche d'intersection à 34 champs 0,362. Au budget, la
/// redemande au pire cas (sortie recopiée à son plafond, huit écarts, longue phrase) laisse
/// encore les 1 024 jetons de la réponse dans les 8 192 du contexte, avec une marge. Le noyau
/// ne tokenise pas (il ne dépend pas de l'IA) : le contrôle exact de `cassini-ai` reste le
/// dernier filet, et le test `the_inventory_budget_fits_the_context_with_the_real_tokenizer`
/// le vérifie au pire cas.
pub const INVENTORY_BUDGET_CHARS: usize = 14_000;

/// Seuil bas du retrait par paquets (ticket 5.19, décision du 2026-10-29) : quand l'inventaire vu
/// dépasse `INVENTORY_BUDGET_CHARS`, les résultats les plus anciens sortent jusqu'à descendre sous
/// ce seuil, la moitié du budget. La phrase du retrait relit les résultats gardés ; les suivantes
/// n'ajoutent que leurs couches, jusqu'au prochain dépassement.
pub const INVENTORY_LOW_CHARS: usize = 7_000;

/// Un fichier du dossier de travail Cassini du projet (`<projet>.cassini/`, ou de ses dossiers
/// de rejeu `replay/` et `replay-<n>/`) et sa date de
/// modification : une couche dont la source porte ce nom est une couche de résultat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkFile {
    /// Nom du fichier, sans dossier (le dernier segment de `source` dans l'inventaire).
    pub name: String,
    pub modified: SystemTime,
}

/// Nombre de caractères du texte JSON compact de `value`.
fn json_chars(value: &Value) -> usize {
    value.to_string().chars().count()
}

/// L'inventaire traduit, borné par paquets (ticket 5.19) : sous `budget` caractères pour le modèle,
/// en retirant les résultats les plus anciens jusqu'à `low` à chaque dépassement.
///
/// Le noyau ne garde rien d'une phrase à l'autre : la décision est rejouée sur les résultats
/// présents, du plus ancien au plus récent (même ordre que `stable_order`). Chaque résultat est
/// ajouté à la vue ; si elle dépasse alors `budget`, les plus anciens de la vue en sortent jusqu'à
/// ce qu'elle tienne sous `low`, sauf le résultat qui vient d'entrer : il ne sort que s'il ne tient
/// pas sous `budget` à côté des couches de l'utilisateur. Un résultat plus récent ne fait que
/// prolonger ce rejeu : tant que la vue ne redépasse pas le budget, les résultats gardés restent
/// les mêmes, et l'invite d'avant reste un préfixe de la nouvelle. Les couches de l'utilisateur ne sortent jamais (si elles seules
/// dépassent, le contrôle du contexte de l'IA tranche). Ordre d'entrée gardé, couches gardées
/// intactes.
pub fn within_budget_by_batches(
    inventory: Value,
    work_files: &[WorkFile],
    budget: usize,
    low: usize,
) -> Value {
    let Some(layers) = inventory.get("layers").and_then(Value::as_array) else {
        return inventory;
    };
    // Taille de la vue : l'inventaire sans couche, plus chaque couche et une virgule entre deux.
    let mut empty = inventory.clone();
    empty["layers"] = json!([]);
    let sizes: Vec<usize> = layers.iter().map(json_chars).collect();
    let keep = kept_by_batches(
        json_chars(&empty),
        &sizes,
        &results_oldest_first(layers, work_files),
        budget,
        low,
    );
    let mut out = inventory.clone();
    out["layers"] = layers
        .iter()
        .zip(&keep)
        .filter(|(_, &k)| k)
        .map(|(l, _)| l.clone())
        .collect();
    out
}

/// La règle des paquets (ticket 5.19), sur des tailles seulement : `sizes[i]` est la taille de
/// l'élément `i` d'une liste JSON, `base` celle du conteneur vide, `results` les rangs des
/// résultats du plus ancien au plus récent (tout autre rang est une couche de l'utilisateur,
/// toujours gardée). Rend, pour chaque rang, s'il est gardé.
pub(crate) fn kept_by_batches(
    base: usize,
    sizes: &[usize],
    results: &[usize],
    budget: usize,
    low: usize,
) -> Vec<bool> {
    let mut is_result = vec![false; sizes.len()];
    for &index in results {
        is_result[index] = true;
    }
    let users: Vec<usize> = (0..sizes.len()).filter(|&i| !is_result[i]).collect();
    let mut count = users.len();
    let mut size = base + users.iter().map(|&i| sizes[i]).sum::<usize>() + count.saturating_sub(1);
    let mut kept: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for &index in results {
        size += sizes[index] + usize::from(count > 0);
        count += 1;
        kept.push_back(index);
        if size > budget {
            // Le plus récent reste, même si les couches de l'utilisateur dépassent déjà `low`.
            while size > low && kept.len() > 1 {
                let oldest = kept.pop_front().expect("au moins deux résultats gardés");
                count -= 1;
                size -= sizes[oldest] + usize::from(count > 0);
            }
            if size > budget {
                kept.pop_back();
                count -= 1;
                size -= sizes[index] + usize::from(count > 0);
            }
        }
    }
    let mut keep = vec![false; sizes.len()];
    for &i in users.iter().chain(&kept) {
        keep[i] = true;
    }
    keep
}

/// Les rangs des couches de résultat de `layers`, du plus ancien au plus récent.
pub(crate) fn results_oldest_first(layers: &[Value], work_files: &[WorkFile]) -> Vec<usize> {
    let mut results = results_of(layers, work_files);
    results.sort_by(|a, b| newest_first(b, a));
    results.into_iter().map(|(index, _, _)| index).collect()
}

/// Une couche de résultat : rang dans l'inventaire, date et nom de son fichier.
type ResultKey<'a> = (usize, SystemTime, &'a str);

/// Les couches de résultat de `layers` (source nommée dans `work_files`), dans l'ordre d'entrée.
fn results_of<'a>(layers: &'a [Value], work_files: &[WorkFile]) -> Vec<ResultKey<'a>> {
    layers
        .iter()
        .enumerate()
        .filter_map(|(index, layer)| {
            result_date(layer, work_files).map(|(date, name)| (index, date, name))
        })
        .collect()
}

/// Les plus récentes d'abord : date du fichier décroissante, puis numéro d'étape décroissant (un
/// nom sans numéro après ceux qui en ont un), puis nom décroissant, puis rang dans l'inventaire
/// (le plus haut dans la légende de QGIS, où les nouveaux arrivent en tête, d'abord).
fn newest_first(a: &ResultKey<'_>, b: &ResultKey<'_>) -> Ordering {
    b.1.cmp(&a.1)
        .then_with(|| step_number(b.2).cmp(&step_number(a.2)))
        .then_with(|| b.2.cmp(a.2))
        .then(a.0.cmp(&b.0))
}

/// L'inventaire dans un ordre stable d'une phrase à l'autre, et le nombre de couches de
/// l'utilisateur, qui viennent en tête.
///
/// Les couches de l'utilisateur d'abord, dans l'ordre d'entrée (celui de QGIS) ; puis les couches
/// de résultat (source nommée dans `work_files`), de la plus ancienne à la plus récente, l'ordre
/// exactement inverse de `newest_first`. Seul l'ordre change : chaque couche
/// reste celle d'entrée, à l'octet près, et le reste de l'inventaire aussi.
pub fn stable_order(inventory: Value, work_files: &[WorkFile]) -> (Value, usize) {
    let Some(layers) = inventory.get("layers").and_then(Value::as_array) else {
        return (inventory, 0);
    };
    let mut results = results_of(layers, work_files);
    results.sort_by(|a, b| newest_first(b, a));
    let mut is_result = vec![false; layers.len()];
    for (index, _, _) in &results {
        is_result[*index] = true;
    }
    let mut ordered: Vec<Value> = layers
        .iter()
        .zip(&is_result)
        .filter(|(_, &r)| !r)
        .map(|(l, _)| l.clone())
        .collect();
    let users = ordered.len();
    ordered.extend(results.iter().map(|(index, _, _)| layers[*index].clone()));
    let mut out = inventory.clone();
    out["layers"] = Value::Array(ordered);
    (out, users)
}

/// Longueur de l'identifiant court de départ, en chiffres hexadécimaux : celle de l'entraînement.
const SHORT_HEX: usize = 4;

/// Un identifiant de couche QGIS `nom_<uuid>` (uuid aux tirets remplacés par des soulignés,
/// groupes de 8, 4, 4, 4 et 12 chiffres hexadécimaux) : le nom et les 32 chiffres. `None` pour
/// toute autre forme, et pour un uuid sans nom devant.
fn qgis_uuid_id(id: &str) -> Option<(&str, String)> {
    const UUID: usize = 36;
    let cut = id.len().checked_sub(UUID)?;
    if !id.is_char_boundary(cut) {
        return None;
    }
    let (head, tail) = id.split_at(cut);
    let name = head.strip_suffix('_').filter(|n| !n.is_empty())?;
    let groups: Vec<&str> = tail.split('_').collect();
    let lengths: Vec<usize> = groups.iter().map(|g| g.len()).collect();
    if lengths != [8, 4, 4, 4, 12]
        || !groups
            .iter()
            .all(|g| g.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return None;
    }
    Some((name, groups.concat()))
}

/// La table des identifiants courts montrés au modèle (ticket 5.19).
///
/// Un identifiant QGIS `nom_<uuid>` est montré `nom_<4 premiers chiffres de l'uuid>`, la forme de
/// l'entraînement ; tout autre identifiant est montré tel quel. Deux couches dont les courts se
/// confondent (entre eux, ou avec un identifiant montré tel quel) reçoivent un chiffre de plus,
/// ensemble, jusqu'à être uniques ; au-delà des 32 chiffres, l'identifiant complet. La table ne
/// dépend que de l'ensemble des identifiants, jamais de leur ordre.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShortIds {
    /// Identifiant complet de chaque court qui en diffère.
    full: BTreeMap<String, String>,
    /// Court de chaque identifiant complet qui en a un.
    short: BTreeMap<String, String>,
}

impl ShortIds {
    /// La table des identifiants `ids` (ceux des couches de l'inventaire).
    pub fn of<'a>(ids: impl IntoIterator<Item = &'a str>) -> ShortIds {
        let ids: Vec<&str> = ids.into_iter().collect();
        let split: Vec<Option<(&str, String)>> = ids.iter().map(|id| qgis_uuid_id(id)).collect();
        let mut digits = vec![SHORT_HEX; ids.len()];
        let shown = |digits: &[usize]| -> Vec<String> {
            ids.iter()
                .zip(&split)
                .zip(digits)
                .map(|((id, parts), &n)| match parts {
                    Some((name, hex)) if n <= hex.len() => format!("{name}_{}", &hex[..n]),
                    _ => (*id).to_string(),
                })
                .collect()
        };
        let mut shorts = shown(&digits);
        loop {
            let mut count: HashMap<&str, usize> = HashMap::new();
            for s in &shorts {
                *count.entry(s.as_str()).or_default() += 1;
            }
            let mut longer = false;
            for (i, s) in shorts.iter().enumerate() {
                if count[s.as_str()] > 1 && split[i].is_some() && digits[i] <= 32 {
                    digits[i] += 1;
                    longer = true;
                }
            }
            if !longer {
                break;
            }
            shorts = shown(&digits);
        }
        let mut table = ShortIds::default();
        for (id, s) in ids.iter().zip(shorts) {
            if s != *id {
                table.full.insert(s.clone(), (*id).to_string());
                table.short.insert((*id).to_string(), s);
            }
        }
        table
    }

    /// La table des couches de `inventory`.
    pub fn of_inventory(inventory: &Value) -> ShortIds {
        let layers = inventory.get("layers").and_then(Value::as_array);
        ShortIds::of(
            layers
                .into_iter()
                .flatten()
                .filter_map(|l| l.get("layer_id").and_then(Value::as_str)),
        )
    }

    /// L'inventaire vu par le modèle : chaque `layer_id` remplacé par son court, rien d'autre.
    pub fn shorten(&self, mut inventory: Value) -> Value {
        if let Some(layers) = inventory.get_mut("layers").and_then(Value::as_array_mut) {
            for layer in layers {
                let id = layer.get("layer_id").and_then(Value::as_str);
                if let Some(short) = id.and_then(|id| self.short.get(id)) {
                    layer["layer_id"] = Value::String(short.clone());
                }
            }
        }
        inventory
    }

    /// `text` (une question, une raison de refus) où chaque court de la table, écrit comme un mot
    /// entier (ni lettre, ni chiffre, ni souligné juste avant ou juste après), est remplacé par son
    /// identifiant complet, les plus longs courts d'abord. Un identifiant complet déjà écrit, qui
    /// commence par son court, n'est donc jamais touché.
    pub fn restore_text(&self, text: &str) -> String {
        let is_id = |c: char| c.is_alphanumeric() || c == '_';
        let mut shorts: Vec<(&String, &String)> = self.full.iter().collect();
        shorts.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then(a.0.cmp(b.0)));
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        let mut prev: Option<char> = None;
        'scan: while let Some(c) = rest.chars().next() {
            if !prev.is_some_and(is_id) {
                for (short, full) in &shorts {
                    if let Some(after) = rest.strip_prefix(short.as_str()) {
                        if !after.chars().next().is_some_and(is_id) {
                            out.push_str(full);
                            rest = after;
                            prev = short.chars().last();
                            continue 'scan;
                        }
                    }
                }
            }
            out.push(c);
            rest = &rest[c.len_utf8()..];
            prev = Some(c);
        }
        out
    }

    /// Retraduit vers les identifiants complets tout texte de `value` égal à un court de la table,
    /// à toute profondeur. Un texte qui n'est pas un court de la table reste tel quel (un
    /// identifiant inconnu est refusé ensuite, comme sans traduction).
    pub fn restore(&self, value: &mut Value) {
        match value {
            Value::String(text) => {
                if let Some(full) = self.full.get(text.as_str()) {
                    *text = full.clone();
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|v| self.restore(v)),
            Value::Object(map) => map.values_mut().for_each(|v| self.restore(v)),
            _ => {}
        }
    }
}

/// L'inventaire montré au modèle, et le nombre de couches de l'utilisateur, qui viennent en tête
/// (le moteur garde un instantané juste avant la dernière clé de la dernière d'entre elles).
/// `left_out` : les couches de résultat que le budget a laissées hors de sa vue (ticket 5.33),
/// `{"layer_id", "name"}` aux vrais identifiants QGIS, de la plus ancienne à la plus récente ;
/// jamais montré au modèle, seulement rendu à l'utilisateur.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelInventory {
    pub inventory: Value,
    pub user_layers: usize,
    pub left_out: Vec<Value>,
}

/// De `project.inventory` à l'inventaire montré au modèle, avec la table pour retraduire son plan :
/// forme de l'entraînement, ordre stable, identifiants courts (table faite sur toutes les couches,
/// pour qu'un court ne change pas quand une couche sort de la vue), couches intermédiaires
/// (`intermediates`, noms de fichier de sortie, ticket 5.34) retirées et résultats sans valeurs
/// (ticket 5.22), puis
/// budget par paquets (`budget`, seuil bas `low`).
pub fn model_inventory(
    raw: &Value,
    work_files: &[WorkFile],
    intermediates: &HashSet<String>,
    budget: usize,
    low: usize,
) -> (ModelInventory, ShortIds) {
    let (ordered, user_layers) = stable_order(agent_inventory(raw), work_files);
    let ids = ShortIds::of_inventory(&ordered);
    let lighter = results_without_values(
        without_intermediates(ordered, work_files, intermediates),
        work_files,
    );
    let inventory = within_budget_by_batches(ids.shorten(lighter.clone()), work_files, budget, low);
    let left_out = left_out_by_budget(&lighter, &inventory, &ids, work_files);
    (
        ModelInventory {
            inventory,
            user_layers,
            left_out,
        },
        ids,
    )
}

/// Les couches de résultat de `lighter` (identifiants complets, avant le budget) absentes de
/// `kept` (identifiants courts, après le budget), dans l'ordre de `lighter` : de la plus ancienne
/// à la plus récente (ticket 5.33). Les intermédiaires de 5.22 sont déjà hors de `lighter`, elles
/// ne comptent donc pas.
fn left_out_by_budget(
    lighter: &Value,
    kept: &Value,
    ids: &ShortIds,
    work_files: &[WorkFile],
) -> Vec<Value> {
    fn layers(v: &Value) -> impl Iterator<Item = &Value> {
        v.get("layers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
    }
    let shown: HashSet<&str> = layers(kept)
        .filter_map(|l| l.get("layer_id").and_then(Value::as_str))
        .collect();
    layers(lighter)
        .filter(|layer| result_date(layer, work_files).is_some())
        .filter_map(|layer| {
            let id = layer.get("layer_id").and_then(Value::as_str)?;
            let short = ids.short.get(id).map_or(id, String::as_str);
            if shown.contains(short) {
                return None;
            }
            // Un nom absent ou blanc serait ignoré par l'extension : l'identifiant le remplace.
            let name = layer
                .get("name")
                .and_then(Value::as_str)
                .filter(|n| !n.trim().is_empty())
                .unwrap_or(id);
            Some(json!({"layer_id": id, "name": name}))
        })
        .collect()
}

/// L'inventaire sans les couches de résultat dont le fichier source est dans `intermediates`
/// (tickets 5.22 et 5.34) : les sorties d'une étape qu'une étape exécutée du même plan a
/// consommées (`step:k`). Le lien passe par le fichier, jamais par l'id, que QGIS renouvelle à
/// chaque ouverture ; une copie du rejeu (`replay/`, même nom) est reconnue de même. Une couche de
/// l'utilisateur (source hors des fichiers de travail) n'est jamais retirée, même citée par
/// l'ensemble. Ordre et couches gardées intacts.
pub fn without_intermediates(
    mut inventory: Value,
    work_files: &[WorkFile],
    intermediates: &HashSet<String>,
) -> Value {
    if intermediates.is_empty() {
        return inventory;
    }
    if let Some(layers) = inventory.get_mut("layers").and_then(Value::as_array_mut) {
        layers.retain(|layer| {
            !result_date(layer, work_files)
                .is_some_and(|(_, source)| intermediates.contains(source))
        });
    }
    inventory
}

/// L'inventaire où les champs des couches de résultat n'ont plus de clé `values` (ticket 5.22) :
/// la forme d'un champ texte trop varié, déjà apprise. Les couches de l'utilisateur gardent leurs
/// valeurs ; rien d'autre ne change.
pub fn results_without_values(mut inventory: Value, work_files: &[WorkFile]) -> Value {
    if let Some(layers) = inventory.get_mut("layers").and_then(Value::as_array_mut) {
        for layer in layers {
            if result_date(layer, work_files).is_none() {
                continue;
            }
            if let Some(fields) = layer.get_mut("fields").and_then(Value::as_array_mut) {
                for field in fields {
                    if let Some(map) = field.as_object_mut() {
                        map.remove("values");
                    }
                }
            }
        }
    }
    inventory
}

/// Numéro d'étape en tête d'un nom de fichier de résultat (`21-analysis.intersect.gpkg` donne
/// 21) ; `None` sans numéro, qui se range avant tout numéro (donc après en ordre décroissant).
fn step_number(name: &str) -> Option<u64> {
    let digits: &str = &name[..name.bytes().take_while(u8::is_ascii_digit).count()];
    digits.parse().ok()
}

/// Date et nom du fichier d'une couche de résultat ; `None` pour une couche de l'utilisateur.
///
/// Le lien passe par le nom du fichier, dernier segment de `source` : celle d'un vecteur est un
/// nom seul, celle d'un raster un chemin relatif au projet (`p.cassini/1-terrain.slope.tif`,
/// ticket 4.3).
fn result_date<'a>(layer: &'a Value, work_files: &[WorkFile]) -> Option<(SystemTime, &'a str)> {
    let source = layer.get("source").and_then(Value::as_str)?;
    let name = source.rsplit('/').next().unwrap_or(source);
    let file = work_files.iter().find(|f| f.name == name)?;
    Some((file.modified, name))
}

/// L'inventaire de `project.inventory` dans la forme de l'entraînement.
pub fn agent_inventory(raw: &Value) -> Value {
    let layers: Vec<Value> = raw
        .get("layers")
        .and_then(Value::as_array)
        .map(|layers| layers.iter().map(layer).collect())
        .unwrap_or_default();
    let mut project = Map::new();
    if let Some(name) = text(raw, "project_name").filter(|n| !n.is_empty()) {
        project.insert("name".into(), json!(name));
    }
    if let Some(code) = text(raw, "project_crs").and_then(epsg) {
        project.insert("epsg".into(), json!(code));
    }
    if let Some(code) = raw.get("extent").and_then(utm_epsg) {
        project.insert("utm_epsg".into(), json!(code));
    }
    // L'extension ne parcourt pas le disque : aucun fichier non ouvert n'est connu.
    json!({"project": project, "layers": layers, "unloaded_files": []})
}

/// Code EPSG du fuseau UTM normal du centre de l'emprise en degrés WGS84 (`extent.lonlat`,
/// reprojetée par QGIS, `crs` = `EPSG:4326`) : 326xx au nord, 327xx au sud. Le noyau ne
/// convertit aucune coordonnée (AD-19) : il lit une longitude et une latitude déjà en degrés et
/// en tire un numéro de fuseau. Le centre décide quand l'emprise chevauche deux fuseaux. Rien
/// quand l'emprise manque, n'a pas le code `EPSG:4326`, est illisible (valeur non finie, hors
/// bornes, ouest après est, sud après nord), couvre plus de deux fuseaux (plus de 12° de
/// longitude : une couche du monde, ou une emprise qui passe l'antiméridien et s'étend de -180 à
/// 180) ou que son centre sort du domaine de l'UTM (80° S à 84° N).
/// Largeur en longitude au-delà de laquelle l'emprise couvre plus de deux fuseaux UTM.
const MAX_UTM_SPAN_DEGREES: f64 = 12.0;

fn utm_epsg(extent: &Value) -> Option<u32> {
    use crate::geodesy::{utm_zone, MAX_LATITUDE, MIN_LATITUDE};
    let lonlat = extent.get("lonlat")?;
    if !lonlat
        .get("crs")
        .and_then(Value::as_str)
        .is_some_and(|c| c.eq_ignore_ascii_case("EPSG:4326"))
    {
        return None;
    }
    let get = |key: &str| {
        lonlat
            .get(key)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
    };
    let (west, south, east, north) = (get("west")?, get("south")?, get("east")?, get("north")?);
    let lon_ok = |v: f64| (-180.0..=180.0).contains(&v);
    let lat_ok = |v: f64| (-90.0..=90.0).contains(&v);
    if !(lon_ok(west) && lon_ok(east) && lat_ok(south) && lat_ok(north)) {
        return None;
    }
    if west > east || south > north || east - west > MAX_UTM_SPAN_DEGREES {
        return None;
    }
    let (lon, lat) = ((west + east) / 2.0, (south + north) / 2.0);
    if !(MIN_LATITUDE..=MAX_LATITUDE).contains(&lat) {
        return None;
    }
    let base = if lat >= 0.0 { 32600 } else { 32700 };
    Some(base + u32::from(utm_zone(lat, lon)))
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// `EPSG:2154` donne `2154` ; un autre SCR (`IGNF:LAMB93`) ne donne rien.
fn epsg(authid: &str) -> Option<u64> {
    let (authority, code) = authid.split_once(':')?;
    if !authority.eq_ignore_ascii_case("EPSG") || code.is_empty() {
        return None;
    }
    if !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    code.parse().ok()
}

fn layer(raw: &Value) -> Value {
    let mut out = Map::new();
    for key in ["layer_id", "name", "source"] {
        if let Some(v) = text(raw, key) {
            out.insert(key.into(), json!(v));
        }
    }
    if let Some(code) = text(raw, "crs").and_then(epsg) {
        out.insert("epsg".into(), json!(code));
    }
    let kind = text(raw, "kind");
    if let Some(k @ ("vector" | "raster")) = kind {
        out.insert("kind".into(), json!(k));
    }
    if kind == Some("raster") {
        // Un raster n'a ni géométrie ni champs, quoi que dise la sortie brute.
        out.insert("geometry".into(), Value::Null);
        out.insert("fields".into(), json!([]));
        return Value::Object(out);
    }
    if let Some(g) = raw.get("geometry") {
        let shape = g.as_str().and_then(geometry);
        out.insert("geometry".into(), shape.map_or(Value::Null, |s| json!(s)));
    }
    if let Some(n) = raw.get("feature_count").and_then(Value::as_u64) {
        out.insert("feature_count".into(), json!(n));
    }
    if let Some(fields) = raw.get("fields").and_then(Value::as_array) {
        let fields: Vec<Value> = fields.iter().filter_map(field).collect();
        out.insert("fields".into(), json!(fields));
    }
    Value::Object(out)
}

/// Nom WKB de QGIS ramené aux six géométries de l'entraînement ; `None` pour une couche
/// sans géométrie ou d'une géométrie inconnue.
pub(crate) fn geometry(wkb: &str) -> Option<&'static str> {
    let mut base = wkb.trim();
    for suffix in ["25D", "ZM", "Z", "M"] {
        if let Some(stripped) = base.strip_suffix(suffix) {
            base = stripped;
            break;
        }
    }
    Some(match base {
        "Point" => "Point",
        "MultiPoint" => "MultiPoint",
        "LineString" | "CircularString" | "CompoundCurve" => "LineString",
        "MultiLineString" | "MultiCurve" => "MultiLineString",
        "Polygon" | "CurvePolygon" | "Triangle" => "Polygon",
        "MultiPolygon" | "MultiSurface" | "PolyhedralSurface" | "TIN" => "MultiPolygon",
        _ => return None,
    })
}

/// Type QGIS sans précision de longueur, en minuscules : `VARCHAR(80)` donne `varchar`.
fn base_type(qgis_type: &str) -> String {
    qgis_type
        .split('(')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

/// Type de l'entraînement : `integer`, `number`, ou `string` (tout le reste, dates comprises).
pub(crate) fn field_type(qgis_type: &str) -> &'static str {
    match base_type(qgis_type).as_str() {
        "integer" | "integer64" | "int" | "int2" | "int4" | "int8" | "int16" | "int32"
        | "tinyint" | "smallint" | "mediumint" | "bigint" | "serial" | "bigserial" | "long"
        | "longlong" | "short" | "uint" | "qlonglong" | "qulonglong" => "integer",
        "real" | "double" | "double precision" | "float" | "float4" | "float8" | "numeric"
        | "decimal" => "number",
        _ => "string",
    }
}

/// Seuls les champs texte portent des valeurs (une date, un booléen n'en portent pas).
pub(crate) fn is_text(qgis_type: &str) -> bool {
    matches!(
        base_type(qgis_type).as_str(),
        "string"
            | "text"
            | "varchar"
            | "char"
            | "character"
            | "character varying"
            | "nvarchar"
            | "bpchar"
            | "citext"
            | "qstring"
    )
}

fn field(raw: &Value) -> Option<Value> {
    let name = text(raw, "name")?;
    let qgis_type = text(raw, "type").unwrap_or_default();
    let mut out = json!({"name": name, "type": field_type(qgis_type)});
    if is_text(qgis_type) {
        if let Some(values) = raw.get("values").and_then(Value::as_array) {
            let mut kept: Vec<&str> = values
                .iter()
                .filter_map(Value::as_str)
                .filter(|v| !v.is_empty())
                .collect();
            kept.sort_unstable();
            kept.dedup();
            if !kept.is_empty() && kept.len() <= MAX_VALUES {
                out["values"] = json!(kept);
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plu() -> Value {
        json!({
            "layer_id": "plu_1", "name": "Zonage du PLU", "source": "plu.gpkg",
            "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiPolygon",
            "feature_count": 42,
            "fields": [
                {"name": "typezone", "type": "String", "values": ["U", "Nh", "N"]},
                {"name": "surface", "type": "Real"}
            ]
        })
    }

    #[test]
    fn a_vector_layer_takes_the_learned_shape() {
        let raw = json!({"project_name": "projet-test", "project_crs": "EPSG:2154",
            "extent": null, "layers": [plu()]});
        assert_eq!(
            agent_inventory(&raw),
            json!({
                "project": {"name": "projet-test", "epsg": 2154},
                "layers": [{
                    "layer_id": "plu_1", "name": "Zonage du PLU", "kind": "vector",
                    "geometry": "MultiPolygon", "epsg": 2154, "feature_count": 42,
                    "source": "plu.gpkg",
                    "fields": [
                        {"name": "typezone", "type": "string", "values": ["N", "Nh", "U"]},
                        {"name": "surface", "type": "number"}
                    ]
                }],
                "unloaded_files": []
            })
        );
    }

    fn with_lonlat(west: f64, south: f64, east: f64, north: f64) -> Value {
        json!({"project_crs": "EPSG:2154", "layers": [], "extent": {
            "xmin": 0.0, "ymin": 0.0, "xmax": 1.0, "ymax": 1.0, "crs": "EPSG:2154",
            "lonlat": {"west": west, "south": south, "east": east, "north": north,
                "crs": "EPSG:4326"}}})
    }

    #[test]
    fn the_project_utm_zone_comes_from_the_centre_of_the_wgs84_extent() {
        for (raw, code) in [
            (with_lonlat(5.9, 45.7, 6.4, 46.1), 32632),     // Annecy
            (with_lonlat(2.35, 48.68, 2.45, 48.75), 32631), // Villeneuve-le-Roi
            (with_lonlat(-1.8, 47.1, -1.4, 47.3), 32630),   // Nantes
            (with_lonlat(55.2, -21.4, 55.9, -20.8), 32740), // La Réunion, au sud
            (with_lonlat(5.0, 45.0, 7.4, 46.0), 32632),     // à cheval : le centre (6,2° E) décide
        ] {
            assert_eq!(
                agent_inventory(&raw)["project"]["utm_epsg"],
                json!(code),
                "{raw}"
            );
        }
        assert_eq!(
            agent_inventory(&with_lonlat(2.35, 48.68, 2.45, 48.75))["project"],
            json!({"epsg": 2154, "utm_epsg": 32631})
        );
    }

    #[test]
    fn without_a_readable_wgs84_extent_there_is_no_utm_zone() {
        let mut cases = vec![
            json!({"project_crs": "EPSG:2154", "layers": [], "extent": null}),
            json!({"project_crs": "EPSG:2154", "layers": []}),
            json!({"extent": {"xmin": 0.0, "ymin": 0.0, "xmax": 1.0, "ymax": 1.0,
                "crs": "EPSG:2154"}}),
            json!({"extent": {"lonlat": {"west": "6", "south": 45.0, "east": 6.2,
                "north": 46.0}}}),
            json!({"extent": {"lonlat": {"west": 6.0, "south": 45.0, "east": 6.2}}}),
            json!({"extent": {"lonlat": null}}),
            // Sans code, ou dans un autre système : rien n'est lu.
            json!({"extent": {"lonlat": {"west": 5.9, "south": 45.7, "east": 6.4,
                "north": 46.1}}}),
            json!({"extent": {"lonlat": {"west": 5.9, "south": 45.7, "east": 6.4,
                "north": 46.1, "crs": "EPSG:4258"}}}),
        ];
        for (w, s, e, n) in [
            (190.0, 45.0, 195.0, 46.0),       // longitude hors bornes
            (6.0, -95.0, 6.2, 46.0),          // latitude hors bornes
            (6.2, 45.0, 6.0, 46.0),           // ouest après est
            (6.0, 46.0, 6.2, 45.0),           // sud après nord
            (10.0, 85.0, 12.0, 89.0),         // au-delà de 84° N
            (10.0, -89.0, 12.0, -85.0),       // en deçà de 80° S
            (f64::MAX, 45.0, f64::MAX, 46.0), // hors bornes, fini
            (-180.0, -85.0, 180.0, 83.6),     // une couche du monde (Natural Earth)
            (-180.0, -21.0, 180.0, -20.0),    // antiméridien passé : -180 à 180
            (-1.0, 45.0, 11.5, 46.0),         // plus de 12° : trois fuseaux
        ] {
            cases.push(with_lonlat(w, s, e, n));
        }
        for raw in cases {
            assert!(
                agent_inventory(&raw)["project"].get("utm_epsg").is_none(),
                "{raw}"
            );
        }
    }

    #[test]
    fn a_very_varied_text_field_has_no_values() {
        let many: Vec<String> = (0..=MAX_VALUES).map(|i| format!("P{i:04}")).collect();
        let at_cap: Vec<String> = many[..MAX_VALUES].to_vec();
        let raw = json!({"layers": [{"name": "parcelles", "kind": "vector", "fields": [
            {"name": "id_parcelle", "type": "String", "values": many},
            {"name": "section", "type": "String", "values": at_cap},
            {"name": "vide", "type": "String", "values": [null, ""]}
        ]}]});
        let fields = &agent_inventory(&raw)["layers"][0]["fields"];
        assert_eq!(fields[0], json!({"name": "id_parcelle", "type": "string"}));
        assert_eq!(fields[1]["values"].as_array().unwrap().len(), MAX_VALUES);
        assert_eq!(fields[2], json!({"name": "vide", "type": "string"}));
    }

    #[test]
    fn qgis_types_become_the_three_learned_types() {
        for (qgis, learned) in [
            ("Integer", "integer"),
            ("Integer64", "integer"),
            ("Real", "number"),
            ("Double", "number"),
            ("String", "string"),
            ("Date", "string"),
            ("int4", "integer"),
            ("tinyint", "integer"),
            ("mediumint", "integer"),
            ("int16", "integer"),
            ("int32", "integer"),
            ("serial", "integer"),
            ("bigserial", "integer"),
            ("longlong", "integer"),
            ("bpchar", "string"),
            ("citext", "string"),
            ("VARCHAR(80)", "string"),
            ("JSON", "string"),
            ("", "string"),
        ] {
            assert_eq!(field_type(qgis), learned, "{qgis}");
        }
    }

    #[test]
    fn only_text_fields_keep_their_values() {
        let raw = json!({"layers": [{"name": "x", "fields": [
            {"name": "d", "type": "Date", "values": ["2024-01-01"]},
            {"name": "n", "type": "Integer", "values": ["1", "2"]},
            {"name": "t", "type": "text", "values": ["b", "a", "b"]},
            {"name": "p", "type": "bpchar", "values": ["x"]},
            {"name": "c", "type": "citext", "values": ["y"]}
        ]}]});
        let fields = &agent_inventory(&raw)["layers"][0]["fields"];
        assert!(fields[0].get("values").is_none());
        assert!(fields[1].get("values").is_none());
        assert_eq!(fields[2]["values"], json!(["a", "b"]));
        assert_eq!(fields[3]["values"], json!(["x"]));
        assert_eq!(fields[4]["values"], json!(["y"]));
    }

    #[test]
    fn another_kind_of_layer_has_no_kind_key() {
        let raw = json!({"layers": [{"layer_id": "m_1", "name": "Maillage", "crs": "EPSG:2154",
            "kind": "other", "geometry": null, "fields": []}]});
        assert_eq!(
            agent_inventory(&raw)["layers"][0],
            json!({"layer_id": "m_1", "name": "Maillage", "epsg": 2154,
                "geometry": null, "fields": []})
        );
    }

    #[test]
    fn a_non_epsg_crs_has_no_epsg_key() {
        let raw = json!({"project_crs": "IGNF:LAMB93", "layers": [
            {"name": "x", "crs": "IGNF:LAMB93"}, {"name": "y", "crs": "EPSG:abc"}, {"name": "z"}
        ]});
        let out = agent_inventory(&raw);
        assert_eq!(out["project"], json!({}));
        for l in out["layers"].as_array().unwrap() {
            assert!(l.get("epsg").is_none(), "{l}");
        }
        assert_eq!(epsg("epsg:4326"), Some(4326));
    }

    #[test]
    fn a_raster_has_no_geometry_and_no_fields() {
        let raw = json!({"layers": [{"layer_id": "mnt_1", "name": "MNT", "source": "mnt.tif",
            "crs": "EPSG:2154", "kind": "raster", "geometry": null, "fields": []}]});
        assert_eq!(
            agent_inventory(&raw)["layers"][0],
            json!({"layer_id": "mnt_1", "name": "MNT", "source": "mnt.tif", "epsg": 2154,
                "kind": "raster", "geometry": null, "fields": []})
        );
    }

    #[test]
    fn an_older_extension_output_translates_without_new_keys() {
        let raw = json!({"layers": [{"layer_id": "g_1", "name": "Gares", "source": null,
            "crs": "EPSG:2154"}], "project_crs": "EPSG:2154", "extent": null});
        assert_eq!(
            agent_inventory(&raw),
            json!({"project": {"epsg": 2154},
                "layers": [{"layer_id": "g_1", "name": "Gares", "epsg": 2154}],
                "unloaded_files": []})
        );
        assert_eq!(
            agent_inventory(&json!({})),
            json!({"project": {}, "layers": [], "unloaded_files": []})
        );
    }

    #[test]
    fn wkb_names_come_back_to_the_six_learned_geometries() {
        for (wkb, learned) in [
            ("MultiPolygon", Some("MultiPolygon")),
            ("PointZ", Some("Point")),
            ("MultiLineStringZM", Some("MultiLineString")),
            ("LineString25D", Some("LineString")),
            ("MultiSurface", Some("MultiPolygon")),
            ("NoGeometry", None),
            ("Unknown", None),
        ] {
            assert_eq!(geometry(wkb), learned, "{wkb}");
        }
        let raw = json!({"layers": [{"name": "t", "kind": "vector", "geometry": "NoGeometry"}]});
        assert_eq!(agent_inventory(&raw)["layers"][0]["geometry"], Value::Null);
    }

    // -- budget d'inventaire (ticket 5.18) --

    use std::time::{Duration, UNIX_EPOCH};

    fn user_layer(id: &str) -> Value {
        json!({"layer_id": id, "name": id, "kind": "vector", "geometry": "MultiPolygon",
            "epsg": 2154, "feature_count": 12, "source": "villeneuve.gpkg",
            "fields": [{"name": "typezone", "type": "string", "values": ["N", "U"]}]})
    }

    fn result_layer(step: usize) -> Value {
        json!({"layer_id": format!("Intersection_{step}"), "name": "Intersection",
            "kind": "vector", "geometry": "MultiPolygon", "epsg": 2154, "feature_count": 7,
            "source": format!("{step}-analysis.intersect.gpkg"),
            "fields": [{"name": "idu", "type": "string"}, {"name": "surface", "type": "number"}]})
    }

    fn work_file(step: usize, seconds: u64) -> WorkFile {
        WorkFile {
            name: format!("{step}-analysis.intersect.gpkg"),
            modified: UNIX_EPOCH + Duration::from_secs(seconds),
        }
    }

    /// Trois couches de l'utilisateur, puis dix résultats (étapes 1 à 10, la 10 la plus
    /// récente), mêlés comme QGIS les pose : les nouveaux en tête de légende.
    fn session() -> (Value, Vec<WorkFile>) {
        let mut layers = vec![user_layer("plu"), user_layer("gares")];
        layers.extend((1..=10).rev().map(result_layer));
        layers.insert(5, user_layer("parcelles"));
        let inventory = json!({"project": {"name": "p", "epsg": 2154}, "layers": layers,
            "unloaded_files": []});
        let files = (1..=10).map(|k| work_file(k, 1_000 + k as u64)).collect();
        (inventory, files)
    }

    fn ids(inventory: &Value) -> Vec<String> {
        inventory["layers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["layer_id"].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn the_newest_results_are_decided_by_file_date_then_step_number() {
        let (inventory, mut files) = session();
        // Le fichier de l'étape 2 est réécrit en dernier : il devient le plus récent.
        files[1].modified = UNIX_EPOCH + Duration::from_secs(5_000);
        let (ordered, _) = stable_order(inventory.clone(), &files);
        assert_eq!(ids(&ordered).last().unwrap(), "Intersection_2");
        // À date égale, le numéro d'étape tranche : l'étape 10 est la plus récente, pas « 9-… ».
        let same: Vec<WorkFile> = files
            .into_iter()
            .map(|f| WorkFile {
                modified: UNIX_EPOCH,
                ..f
            })
            .collect();
        let (ordered, _) = stable_order(inventory, &same);
        assert_eq!(ids(&ordered).last().unwrap(), "Intersection_10");
        assert_eq!(step_number("21-analysis.intersect.gpkg"), Some(21));
        assert_eq!(step_number("resultat.gpkg"), None);
        assert!(step_number("resultat.gpkg") < step_number("1-a.gpkg"));
    }

    #[test]
    fn a_work_file_the_layer_does_not_name_does_not_make_it_a_result() {
        let (inventory, _) = session();
        let other = [WorkFile {
            name: "journal.jsonl".to_string(),
            modified: UNIX_EPOCH,
        }];
        assert_eq!(
            within_budget_by_batches(inventory.clone(), &other, 10, 5),
            inventory
        );
    }

    // -- ordre stable et identifiants courts (ticket 5.19) --

    #[test]
    fn users_come_first_in_qgis_order_then_results_from_oldest_to_newest() {
        let (inventory, files) = session();
        let (ordered, users) = stable_order(inventory.clone(), &files);
        assert_eq!(users, 3);
        let mut want = vec!["plu".to_string(), "gares".into(), "parcelles".into()];
        want.extend((1..=10).map(|k| format!("Intersection_{k}")));
        assert_eq!(ids(&ordered), want);
        // Seul l'ordre change : mêmes couches à l'octet près, même reste.
        for layer in ordered["layers"].as_array().unwrap() {
            assert!(inventory["layers"].as_array().unwrap().contains(layer));
        }
        assert_eq!(ordered["project"], inventory["project"]);
        // Sans dossier de travail, rien n'est un résultat : l'ordre de QGIS est gardé.
        let (same, all) = stable_order(inventory.clone(), &[]);
        assert_eq!((same, all), (inventory, 13));
        assert_eq!(stable_order(json!({}), &files), (json!({}), 0));
    }

    #[test]
    fn one_more_result_only_adds_a_layer_at_the_end() {
        let (inventory, mut files) = session();
        let (before, _) = stable_order(inventory.clone(), &files);
        // Le plan suivant ajoute l'étape 11, que QGIS pose en tête de légende.
        let mut next = inventory.clone();
        next["layers"]
            .as_array_mut()
            .unwrap()
            .insert(0, result_layer(11));
        files.push(work_file(11, 2_000));
        let (after, users) = stable_order(next, &files);
        assert_eq!(users, 3);
        let before = before["layers"].as_array().unwrap();
        let after = after["layers"].as_array().unwrap();
        assert_eq!(&after[..before.len()], &before[..]);
        assert_eq!(after[before.len()], result_layer(11));
        assert_eq!(after.len(), before.len() + 1);
    }

    #[test]
    fn a_user_layer_added_in_qgis_changes_the_prefix_in_the_middle() {
        let (inventory, files) = session();
        let mut next = inventory.clone();
        next["layers"]
            .as_array_mut()
            .unwrap()
            .insert(0, user_layer("routes"));
        let (after, users) = stable_order(next, &files);
        assert_eq!(users, 4);
        assert_eq!(ids(&after)[..4], ["routes", "plu", "gares", "parcelles"]);
    }

    const PARCELLES: &str = "parcelles_e9ba733c_8a43_4717_ac73_04344c1b6bb5";
    const GARES: &str = "gares_83d935aa_ca33_431d_966f_dbbf0fb7ac0f";
    const RESULT: &str = "141_analysis_select_by_attribute_e63fa196_c4d5_4255_9a08_203c73df1f96";

    #[test]
    fn a_qgis_id_is_shown_with_the_first_four_hex_digits_of_its_uuid() {
        let ids = ShortIds::of([PARCELLES, GARES, RESULT, "plu_1", "Intersection_3"]);
        let mut inventory = json!({"layers": [
            {"layer_id": PARCELLES, "name": "Parcelles"},
            {"layer_id": GARES, "name": "Gares"},
            {"layer_id": RESULT, "name": "141-analysis.select_by_attribute"},
            {"layer_id": "plu_1", "name": "PLU"},
            {"layer_id": "Intersection_3", "name": "Intersection"}
        ], "project": {}});
        inventory = ids.shorten(inventory);
        assert_eq!(
            inventory["layers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|l| l["layer_id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "parcelles_e9ba",
                "gares_83d9",
                "141_analysis_select_by_attribute_e63f",
                "plu_1",
                "Intersection_3"
            ]
        );
        assert_eq!(inventory["layers"][0]["name"], "Parcelles");
        // Formes voisines qui ne sont pas un identifiant QGIS : montrées telles quelles.
        for id in [
            "e9ba733c_8a43_4717_ac73_04344c1b6bb5",
            "_e9ba733c_8a43_4717_ac73_04344c1b6bb5",
            "x_e9ba733c_8a43_4717_ac73_04344c1b6bbz",
            "x_e9ba733c-8a43-4717-ac73-04344c1b6bb5",
            "x_e9ba733c_8a43_4717_ac7_304344c1b6bb5",
            "é",
        ] {
            assert_eq!(qgis_uuid_id(id), None, "{id}");
        }
        assert_eq!(
            qgis_uuid_id("zoné_E9BA733C_8A43_4717_AC73_04344C1B6BB5").map(|p| p.0),
            Some("zoné")
        );
    }

    #[test]
    fn colliding_short_ids_are_lengthened_until_unique_whatever_the_order() {
        let a = "parcelles_e9ba733c_8a43_4717_ac73_04344c1b6bb5";
        let b = "parcelles_e9ba7f00_0000_4717_ac73_04344c1b6bb5";
        let c = "parcelles_e9bb0000_0000_4717_ac73_04344c1b6bb5";
        let ids = ShortIds::of([a, b, c]);
        assert_eq!(ids.short[a], "parcelles_e9ba73");
        assert_eq!(ids.short[b], "parcelles_e9ba7f");
        assert_eq!(ids.short[c], "parcelles_e9bb");
        assert_eq!(ShortIds::of([c, b, a]), ids);
        // Un identifiant montré tel quel qui vaut le court d'un autre : l'autre s'allonge.
        let ids = ShortIds::of(["parcelles_e9ba", a]);
        assert_eq!(ids.short[a], "parcelles_e9ba7");
        assert!(!ids.short.contains_key("parcelles_e9ba"));
        // Deux fois le même identifiant (impossible dans QGIS) : complet, sans boucler.
        let ids = ShortIds::of([a, a]);
        assert!(ids.short.is_empty());
    }

    #[test]
    fn a_plan_is_translated_back_to_the_full_ids_and_unknown_ids_stay() {
        let ids = ShortIds::of([PARCELLES, GARES, "plu_1"]);
        let mut params = json!({"layer_id": "parcelles_e9ba", "overlay_layer_id": "step:1.layer_id",
            "layers": ["gares_83d9", "plu_1", "routes_ab12"], "distance": {"value": 5, "unit": "m"},
            "title": "parcelles_e9ba près des gares"});
        ids.restore(&mut params);
        assert_eq!(
            params,
            json!({"layer_id": PARCELLES, "overlay_layer_id": "step:1.layer_id",
                "layers": [GARES, "plu_1", "routes_ab12"], "distance": {"value": 5, "unit": "m"},
                "title": "parcelles_e9ba près des gares"})
        );
        // Un identifiant complet recopié tel quel reste complet.
        let mut full = json!({"layer_id": PARCELLES});
        ids.restore(&mut full);
        assert_eq!(full, json!({"layer_id": PARCELLES}));
    }

    #[test]
    fn the_model_inventory_is_ordered_shortened_then_bounded() {
        let raw_layer = |id: &str, source: &str| {
            json!({"layer_id": id, "name": id, "source": source, "crs": "EPSG:2154",
                "kind": "vector", "geometry": "MultiPolygon", "feature_count": 3, "fields": []})
        };
        let result_id = |k: u64| format!("{k}_analysis_buffer_e63fa196_c4d5_4255_9a08_20{k:010}");
        let mut layers: Vec<Value> = (1..=3u64)
            .rev()
            .map(|k| raw_layer(&result_id(k), &format!("{k}-analysis.buffer.gpkg")))
            .collect();
        layers.push(raw_layer(PARCELLES, "villeneuve.gpkg"));
        let raw = json!({"project_name": "p", "project_crs": "EPSG:2154", "layers": layers});
        let files: Vec<WorkFile> = (1..=3u64)
            .map(|k| WorkFile {
                name: format!("{k}-analysis.buffer.gpkg"),
                modified: UNIX_EPOCH + Duration::from_secs(k),
            })
            .collect();
        let (view, table) = model_inventory(
            &raw,
            &files,
            &HashSet::new(),
            INVENTORY_BUDGET_CHARS,
            INVENTORY_LOW_CHARS,
        );
        assert_eq!(view.user_layers, 1);
        assert_eq!(
            ids(&view.inventory),
            [
                "parcelles_e9ba",
                "1_analysis_buffer_e63f",
                "2_analysis_buffer_e63f",
                "3_analysis_buffer_e63f"
            ]
        );
        let mut back = json!(["parcelles_e9ba", "3_analysis_buffer_e63f"]);
        table.restore(&mut back);
        assert_eq!(back, json!([PARCELLES, result_id(3)]));
        // Au-delà du budget, les plus anciens résultats sortent ; les courts ne changent pas.
        let one = {
            let (all, _) = model_inventory(&raw, &files, &HashSet::new(), 0, 0);
            json_chars(&all.inventory) + json_chars(&view.inventory["layers"][3]) + 1
        };
        let (small, same_table) = model_inventory(&raw, &files, &HashSet::new(), one, one);
        assert_eq!(
            ids(&small.inventory),
            ["parcelles_e9ba", "3_analysis_buffer_e63f"]
        );
        assert_eq!(same_table, table);
        assert_eq!(small.user_layers, 1);
    }

    // -- retrait par paquets (ticket 5.19, décision du 2026-10-29) --

    /// La session, rangée dans l'ordre stable, avec les étapes 1 à `last` seulement ; un budget qui
    /// tient six résultats et un seuil bas qui en tient trois.
    fn batches_up_to(last: usize) -> (Value, Vec<WorkFile>, usize, usize) {
        let mut layers = vec![
            user_layer("plu"),
            user_layer("gares"),
            user_layer("parcelles"),
        ];
        layers.extend((1..=last).map(result_layer));
        let inventory = json!({"project": {"name": "p", "epsg": 2154}, "layers": layers,
            "unloaded_files": []});
        let files = (1..=last).map(|k| work_file(k, 1_000 + k as u64)).collect();
        let users = json!({"project": {"name": "p", "epsg": 2154},
            "layers": [user_layer("plu"), user_layer("gares"), user_layer("parcelles")],
            "unloaded_files": []});
        let per = json_chars(&result_layer(10)) + 1;
        let base = json_chars(&users);
        (
            inventory,
            files,
            base + 6 * per + per / 2,
            base + 3 * per + per / 2,
        )
    }

    fn results_in(inventory: &Value) -> Vec<String> {
        ids(inventory)
            .into_iter()
            .filter(|id| id.starts_with("Intersection_"))
            .collect()
    }

    fn steps(range: std::ops::RangeInclusive<usize>) -> Vec<String> {
        range.map(|k| format!("Intersection_{k}")).collect()
    }

    #[test]
    fn under_the_budget_nothing_leaves_by_batches() {
        let (inventory, files, budget, low) = batches_up_to(6);
        assert_eq!(
            within_budget_by_batches(inventory.clone(), &files, budget, low),
            inventory
        );
    }

    #[test]
    fn over_the_budget_the_oldest_results_leave_down_to_the_low_mark() {
        // Sept résultats : le 7ᵉ fait dépasser, la vue retombe à trois (5, 6, 7).
        let (inventory, files, budget, low) = batches_up_to(7);
        let out = within_budget_by_batches(inventory.clone(), &files, budget, low);
        assert_eq!(results_in(&out), steps(5..=7));
        assert!(json_chars(&out) <= low);
        assert_eq!(ids(&out)[..3], ["plu", "gares", "parcelles"]);
        for layer in out["layers"].as_array().unwrap() {
            assert!(inventory["layers"].as_array().unwrap().contains(layer));
        }
    }

    #[test]
    fn after_a_batch_new_results_only_add_to_the_end_until_the_next_overflow() {
        let view = |last: usize| {
            let (inventory, files, budget, low) = batches_up_to(last);
            within_budget_by_batches(inventory, &files, budget, low)
        };
        // De 7 à 10 résultats, la vue garde 5, 6, 7 et ne fait qu'ajouter : chaque vue est le
        // début de la suivante, donc l'invite d'avant reste un préfixe.
        for last in 7..10 {
            let before = view(last)["layers"].as_array().unwrap().clone();
            let after = view(last + 1)["layers"].as_array().unwrap().clone();
            assert_eq!(&after[..before.len()], &before[..], "étape {}", last + 1);
            assert_eq!(after.len(), before.len() + 1);
        }
        assert_eq!(results_in(&view(10)), steps(5..=10));
        // Le 11ᵉ refait dépasser : nouveau paquet, la vue retombe à 9, 10, 11.
        assert_eq!(results_in(&view(11)), steps(9..=11));
        // Déterministe, et indépendant de l'ordre d'entrée (celui de QGIS ici, nouveaux en tête).
        let (mut inventory, files, budget, low) = batches_up_to(10);
        inventory["layers"].as_array_mut().unwrap()[3..].reverse();
        let out = within_budget_by_batches(inventory, &files, budget, low);
        let mut got = results_in(&out);
        got.sort_by_key(|id| id[13..].parse::<u32>().unwrap());
        assert_eq!(got, steps(5..=10));
    }

    #[test]
    fn user_layers_never_leave_by_batches() {
        let (inventory, files, _, _) = batches_up_to(4);
        let out = within_budget_by_batches(inventory, &files, 10, 5);
        assert_eq!(ids(&out), ["plu", "gares", "parcelles"]);
        let (inventory, _, _, _) = batches_up_to(4);
        // Sans dossier de travail, rien n'est un résultat : rien ne sort.
        assert_eq!(
            within_budget_by_batches(inventory.clone(), &[], 10, 5),
            inventory
        );
        assert_eq!(
            within_budget_by_batches(json!({}), &files, 10, 5),
            json!({})
        );
    }

    #[test]
    fn users_over_the_low_mark_still_see_the_newest_result() {
        // Couches de l'utilisateur au-dessus du seuil bas, sous le budget : à chaque dépassement,
        // le résultat qui vient d'entrer reste ; il ne sort que s'il ne tient pas sous le budget.
        let (inventory, files, _, _) = batches_up_to(4);
        let users = json!({"project": {"name": "p", "epsg": 2154},
            "layers": [user_layer("plu"), user_layer("gares"), user_layer("parcelles")],
            "unloaded_files": []});
        let per = json_chars(&result_layer(10)) + 1;
        let low = json_chars(&users) - 1;
        let budget = json_chars(&users) + per + per / 2;
        let out = within_budget_by_batches(inventory.clone(), &files, budget, low);
        assert_eq!(results_in(&out), steps(4..=4));
        assert!(json_chars(&out) <= budget);
        // Un budget qui ne laisse la place à aucun résultat : aucun ne reste.
        let tight = json_chars(&users) + per / 2;
        let out = within_budget_by_batches(inventory, &files, tight, low);
        assert!(results_in(&out).is_empty());
    }

    #[test]
    fn a_question_or_a_reason_quoting_short_ids_gets_the_full_ids() {
        let longer = "parcelles_e9ba7f00_0000_4717_ac73_04344c1b6bb5";
        let ids = ShortIds::of([PARCELLES, longer, GARES]);
        // `parcelles_e9ba73` et `parcelles_e9ba7f` : le plus long d'abord, aucun n'est coupé.
        assert_eq!(
            ids.restore_text(
                "Laquelle : parcelles_e9ba73 ou parcelles_e9ba7f, près de gares_83d9 ?"
            ),
            format!("Laquelle : {PARCELLES} ou {longer}, près de {GARES} ?")
        );
        // Un identifiant complet déjà écrit n'est pas touché ; un texte sans court non plus.
        assert_eq!(ids.restore_text(GARES), GARES);
        assert_eq!(ids.restore_text("Aucune couche."), "Aucune couche.");
    }

    // -- résultats allégés (ticket 5.22) --

    fn raw_layer(id: &str, source: &str) -> Value {
        json!({"layer_id": id, "name": id, "source": source, "crs": "EPSG:2154",
            "kind": "vector", "geometry": "MultiPolygon", "feature_count": 3,
            "fields": [{"name": "typezone", "type": "String", "values": ["U", "N"]},
                {"name": "surface", "type": "Real"}]})
    }

    /// Le plan de démo exécuté : PLU et gares de l'utilisateur, puis sélection (1), tampon (2) et
    /// deux intersections (3, 4), posés par QGIS en tête de légende.
    fn demo() -> (Value, Vec<WorkFile>) {
        let steps = [
            "1-analysis.select_by_attribute.gpkg",
            "2-analysis.buffer.gpkg",
            "3-analysis.intersect.gpkg",
            "4-analysis.intersect.gpkg",
        ];
        let mut layers: Vec<Value> = steps
            .iter()
            .enumerate()
            .rev()
            .map(|(i, f)| raw_layer(&format!("r{}", i + 1), f))
            .collect();
        layers.push(raw_layer("plu", "plu.gpkg"));
        layers.push(raw_layer("gares", "gares.gpkg"));
        let raw = json!({"project_name": "p", "project_crs": "EPSG:2154", "layers": layers});
        let files = steps
            .iter()
            .enumerate()
            .map(|(i, f)| WorkFile {
                name: (*f).to_string(),
                modified: UNIX_EPOCH + Duration::from_secs(10 + i as u64),
            })
            .collect();
        (raw, files)
    }

    fn set(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn a_chained_plan_shows_only_its_last_result_without_values() {
        let (raw, files) = demo();
        let hidden = set(&[
            "1-analysis.select_by_attribute.gpkg",
            "2-analysis.buffer.gpkg",
            "3-analysis.intersect.gpkg",
        ]);
        let (view, table) = model_inventory(
            &raw,
            &files,
            &hidden,
            INVENTORY_BUDGET_CHARS,
            INVENTORY_LOW_CHARS,
        );
        assert_eq!(ids(&view.inventory), ["plu", "gares", "r4"]);
        assert_eq!(view.user_layers, 2);
        let layers = view.inventory["layers"].as_array().unwrap();
        // L'utilisateur garde ses valeurs ; le résultat a la forme d'un champ trop varié.
        assert_eq!(layers[0]["fields"][0]["values"], json!(["N", "U"]));
        assert_eq!(
            layers[2]["fields"],
            json!([{"name": "typezone", "type": "string"}, {"name": "surface", "type": "number"}])
        );
        // La table est faite sur toutes les couches, cachées comprises.
        assert_eq!(
            table,
            ShortIds::of(["r1", "r2", "r3", "r4", "plu", "gares"])
        );
    }

    #[test]
    fn a_one_step_plan_keeps_its_result_without_values() {
        let (raw, files) = demo();
        let (view, _) = model_inventory(&raw, &files, &HashSet::new(), 100_000, 50_000);
        assert_eq!(
            ids(&view.inventory),
            ["plu", "gares", "r1", "r2", "r3", "r4"]
        );
        for layer in &view.inventory["layers"].as_array().unwrap()[2..] {
            for field in layer["fields"].as_array().unwrap() {
                assert!(field.get("values").is_none(), "{field}");
            }
        }
    }

    #[test]
    fn a_raster_result_is_known_by_the_file_name_of_its_relative_source() {
        // Ticket 4.3 : la source d'un raster est un chemin relatif au projet ; le dernier
        // segment le relie au fichier de travail. Un raster de l'utilisateur n'est jamais retiré.
        let files = [WorkFile {
            name: "1-terrain.slope.tif".to_string(),
            modified: UNIX_EPOCH + Duration::from_secs(5),
        }];
        let inventory = json!({"project": {}, "unloaded_files": [], "layers": [
            {"layer_id": "pente", "kind": "raster", "source": "p.cassini/1-terrain.slope.tif",
             "fields": [{"name": "v", "values": ["a"]}]},
            {"layer_id": "mnt", "kind": "raster", "source": "dem/mnt.tif",
             "fields": [{"name": "v", "values": ["a"]}]}]});
        let (ordered, users) = stable_order(inventory.clone(), &files);
        assert_eq!(users, 1);
        assert_eq!(ids(&ordered), ["mnt", "pente"]);
        let hidden = set(&["1-terrain.slope.tif", "mnt.tif"]);
        assert_eq!(
            ids(&without_intermediates(inventory.clone(), &files, &hidden)),
            ["mnt"]
        );
        let bare = results_without_values(inventory, &files);
        assert!(bare["layers"][0]["fields"][0].get("values").is_none());
        assert_eq!(bare["layers"][1]["fields"][0]["values"], json!(["a"]));
    }

    #[test]
    fn a_user_layer_named_as_intermediate_stays_whole() {
        // `layer.add` puis une étape qui reprend la couche ouverte : elle reste vue, valeurs comprises.
        let (raw, files) = demo();
        let hidden = set(&["plu.gpkg", "2-analysis.buffer.gpkg"]);
        let shaped = stable_order(agent_inventory(&raw), &files).0;
        let out = without_intermediates(shaped.clone(), &files, &hidden);
        assert_eq!(ids(&out), ["plu", "gares", "r1", "r3", "r4"]);
        assert_eq!(out["layers"][0], shaped["layers"][0]);
        // Ensemble vide ou sans dossier de travail : rien ne sort.
        assert_eq!(
            without_intermediates(shaped.clone(), &files, &HashSet::new()),
            shaped
        );
        assert_eq!(without_intermediates(shaped.clone(), &[], &hidden), shaped);
        assert_eq!(results_without_values(shaped.clone(), &[]), shaped);
    }

    #[test]
    fn with_the_same_results_the_view_does_not_change_and_a_new_plan_only_adds() {
        let (raw, mut files) = demo();
        let mut hidden = set(&[
            "1-analysis.select_by_attribute.gpkg",
            "2-analysis.buffer.gpkg",
            "3-analysis.intersect.gpkg",
        ]);
        let view = |raw: &Value, files: &[WorkFile], hidden: &HashSet<String>| {
            model_inventory(
                raw,
                files,
                hidden,
                INVENTORY_BUDGET_CHARS,
                INVENTORY_LOW_CHARS,
            )
            .0
            .inventory
        };
        let before = view(&raw, &files, &hidden);
        assert_eq!(view(&raw, &files, &hidden), before);
        // Un plan de deux étapes chaînées ajoute r5 (consommée) et r6.
        let mut next = raw.clone();
        let layers = next["layers"].as_array_mut().unwrap();
        layers.insert(0, raw_layer("r5", "5-analysis.buffer.gpkg"));
        layers.insert(0, raw_layer("r6", "6-analysis.intersect.gpkg"));
        for (k, name) in [
            (5, "5-analysis.buffer.gpkg"),
            (6, "6-analysis.intersect.gpkg"),
        ] {
            files.push(WorkFile {
                name: name.to_string(),
                modified: UNIX_EPOCH + Duration::from_secs(100 + k),
            });
        }
        hidden.insert("5-analysis.buffer.gpkg".to_string());
        let after = view(&next, &files, &hidden);
        let before = before["layers"].as_array().unwrap();
        let after = after["layers"].as_array().unwrap();
        assert_eq!(&after[..before.len()], &before[..]);
        assert_eq!(ids(&json!({"layers": after})), ["plu", "gares", "r4", "r6"]);
    }

    #[test]
    fn the_lightening_runs_before_the_budget() {
        // Douze résultats à vingt champs texte de dix valeurs : la vue brute dépasse le budget ;
        // huit sont intermédiaires, et les quatre autres, sans valeurs, tiennent tous.
        let fields: Vec<Value> = (0..20)
            .map(|i| {
                let values: Vec<String> = (0..10).map(|v| format!("valeur_{i:02}_{v}")).collect();
                json!({"name": format!("champ_{i:02}"), "type": "String", "values": values})
            })
            .collect();
        let mut layers: Vec<Value> = (1..=12)
            .rev()
            .map(|k| {
                let mut l = raw_layer(&format!("r{k}"), &format!("{k}-analysis.intersect.gpkg"));
                l["fields"] = json!(fields);
                l
            })
            .collect();
        layers.push(raw_layer("plu", "plu.gpkg"));
        let raw = json!({"project_name": "p", "project_crs": "EPSG:2154", "layers": layers});
        let files: Vec<WorkFile> = (1..=12u64)
            .map(|k| WorkFile {
                name: format!("{k}-analysis.intersect.gpkg"),
                modified: UNIX_EPOCH + Duration::from_secs(k),
            })
            .collect();
        assert!(json_chars(&agent_inventory(&raw)) > INVENTORY_BUDGET_CHARS);
        let hidden: HashSet<String> = [1, 2, 4, 5, 7, 8, 10, 11]
            .iter()
            .map(|k| format!("{k}-analysis.intersect.gpkg"))
            .collect();
        let (view, _) = model_inventory(
            &raw,
            &files,
            &hidden,
            INVENTORY_BUDGET_CHARS,
            INVENTORY_LOW_CHARS,
        );
        assert_eq!(ids(&view.inventory), ["plu", "r3", "r6", "r9", "r12"]);
        assert!(json_chars(&view.inventory) <= INVENTORY_BUDGET_CHARS);
    }

    // -- couches retirées par le budget, signalées (ticket 5.33) --

    /// Douze résultats lourds aux identifiants QGIS (`intersection_<uuid>`, tous nommés
    /// « Intersection »), plus le PLU de l'utilisateur.
    fn heavy_results() -> (Value, Vec<WorkFile>) {
        let fields: Vec<Value> = (0..20)
            .map(|i| json!({"name": format!("champ_long_numero_{i:02}"), "type": "String"}))
            .collect();
        let uuid = |k: u64| format!("intersection_{k:08x}_aaaa_bbbb_cccc_{k:012x}");
        let mut layers: Vec<Value> = (1..=12u64)
            .rev()
            .map(|k| {
                let mut l = raw_layer(&uuid(k), &format!("{k}-analysis.intersect.gpkg"));
                l["name"] = json!("Intersection");
                l["fields"] = json!(fields);
                l
            })
            .collect();
        layers.push(raw_layer("plu", "plu.gpkg"));
        let raw = json!({"project_name": "p", "project_crs": "EPSG:2154", "layers": layers});
        let files = (1..=12u64)
            .map(|k| WorkFile {
                name: format!("{k}-analysis.intersect.gpkg"),
                modified: UNIX_EPOCH + Duration::from_secs(k),
            })
            .collect();
        (raw, files)
    }

    #[test]
    fn the_results_left_out_by_the_budget_are_noted_with_their_real_ids() {
        let (raw, files) = heavy_results();
        let (view, table) = model_inventory(&raw, &files, &HashSet::new(), 6_000, 3_000);
        let shown = ids(&view.inventory);
        assert!(shown.len() < 13, "le budget doit retirer : {shown:?}");
        assert!(!view.left_out.is_empty());
        // Vrais identifiants QGIS, du plus ancien au plus récent, et rien de ce qui est vu.
        let expected: Vec<Value> = (1..=12u64)
            .map(|k| format!("intersection_{k:08x}_aaaa_bbbb_cccc_{k:012x}"))
            .filter(|id| !shown.contains(table.short.get(id).unwrap()))
            .map(|id| json!({"layer_id": id, "name": "Intersection"}))
            .collect();
        assert_eq!(view.left_out, expected);
        assert_eq!(view.left_out.len() + shown.len(), 13);
        // Un nom vide ou blanc est remplacé par l'identifiant.
        let mut blank = raw.clone();
        for (layer, name) in blank["layers"].as_array_mut().unwrap()[10..]
            .iter_mut()
            .zip(["", "  "])
        {
            layer["name"] = json!(name);
        }
        let (view, _) = model_inventory(&blank, &files, &HashSet::new(), 6_000, 3_000);
        for k in [1u64, 2] {
            let id = format!("intersection_{k:08x}_aaaa_bbbb_cccc_{k:012x}");
            assert!(
                view.left_out.contains(&json!({"layer_id": id, "name": id})),
                "{:?}",
                view.left_out
            );
        }
        assert_eq!(
            view.left_out[0]["layer_id"],
            "intersection_00000001_aaaa_bbbb_cccc_000000000001"
        );
    }

    #[test]
    fn under_the_budget_nothing_is_left_out_and_intermediates_do_not_count() {
        let (raw, files) = heavy_results();
        let (view, _) = model_inventory(&raw, &files, &HashSet::new(), 100_000, 50_000);
        assert_eq!(ids(&view.inventory).len(), 13);
        assert!(view.left_out.is_empty());
        // Résultats cachés par 5.22, le reste sous le budget : rien n'est signalé.
        let hidden: HashSet<String> = (1..=10u64)
            .map(|k| format!("{k}-analysis.intersect.gpkg"))
            .collect();
        let (view, _) = model_inventory(
            &raw,
            &files,
            &hidden,
            INVENTORY_BUDGET_CHARS,
            INVENTORY_LOW_CHARS,
        );
        assert_eq!(ids(&view.inventory).len(), 3);
        assert!(view.left_out.is_empty());
        // La démo, sous le budget : rien non plus.
        let (raw, files) = demo();
        let (view, _) = model_inventory(
            &raw,
            &files,
            &set(&[
                "1-analysis.select_by_attribute.gpkg",
                "2-analysis.buffer.gpkg",
                "3-analysis.intersect.gpkg",
            ]),
            INVENTORY_BUDGET_CHARS,
            INVENTORY_LOW_CHARS,
        );
        assert!(view.left_out.is_empty());
    }
}
