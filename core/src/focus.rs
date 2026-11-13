//! Mise en évidence du résultat d'un plan dans son PDF (entrée 7.9, changement de cap
//! `change-mise-en-evidence-du-resultat-dans-le-pdf`, 2026-11-12).
//!
//! Quand un plan finit par `layout.export_pdf`, le noyau remplit le paramètre caché `focus`
//! d'après les lignes du plan en cours au journal : quelles couches sont la réponse, leur nom en
//! clair, leur style de mise en évidence, les couches intermédiaires à cacher et les couches sur
//! lesquelles cadrer. Le geste de l'extension l'applique à la seule mise en page ; rien n'y est
//! décidé (AD-5). `focus` est journalisé avec les paramètres et relu au rejeu, jamais recalculé
//! (AD-11). Tout ici est une fonction pure du journal, testable sans QGIS.
//!
//! Règle :
//! - une **couche produite** est une sortie de Cassini posée par une étape réussie du plan :
//!   la couche rendue par une commande qui écrit une sortie (`out`), ou une couche ouverte par
//!   `layer.open_raster` / `layer.open_vector` sur un repère `out:k.n` (même règle que
//!   `intermediate_files`). Une couche ouverte sur un fichier de l'utilisateur n'en est pas une ;
//! - une couche produite est **intermédiaire** si une étape suivante du plan, qui écrit elle-même
//!   une sortie, la consomme (par son id de couche ou par le repère de son fichier) ; sinon c'est
//!   un **résultat**, dans l'ordre du plan ;
//! - un seul résultat est nommé par le titre de l'export, coupé avant un complément de lieu final
//!   (« à Annecy ») et borné à [`MAX_LABEL`] caractères ; plusieurs résultats le sont par le titre
//!   court de leur commande, « — », et le nom de leur première couche d'entrée (inventaire pour
//!   une couche de l'utilisateur, titre court de l'étape pour une sortie du plan) ;
//! - le cadre est l'union des résultats non vides ; tous vides, celle des couches de
//!   l'utilisateur que le plan a consommées ; sans elles, le cadrage d'avant.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::commands::layer_open_raster::OPEN_RASTER;
use crate::commands::layer_open_vector::OPEN_VECTOR;

/// Retrait d'une couche du projet.
const REMOVE: &str = "layer.remove";
use crate::raster_path::RASTER_EXTENSIONS;
use crate::registry::{CommandSpec, Registry};
use crate::style::spec::StyleSpec;

/// Longueur maximale, en caractères, du nom d'un résultat unique tiré du titre (décision 1 de la
/// party d'approbation du 2026-11-12).
pub const MAX_LABEL: usize = 60;

/// Longueur maximale du titre court d'une commande, comme `summary.short_title` du panneau.
const MAX_SHORT_TITLE: usize = 80;

/// Remplissage du résultat mis en évidence : un magenta plein, qui ne se confond ni avec les
/// routes, ni avec l'eau, ni avec la végétation d'un fond topographique.
pub const RESULT_FILL: &str = "#c2187a";
/// Contour du résultat mis en évidence : un magenta très foncé.
pub const RESULT_STROKE: &str = "#3d0026";
/// Épaisseur du contour, en millimètres de carte.
pub const RESULT_WIDTH_MM: f64 = 0.5;

/// Sur quoi la carte du PDF est cadrée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(inline)]
pub enum FramedOn {
    Results,
    Inputs,
    VisibleLayers,
}

/// Une couche de résultat : son id, son nom en clair dans la légende, et son style de mise en
/// évidence (vecteurs seulement).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FocusResult {
    /// Id de la couche de résultat dans le projet.
    pub layer_id: String,
    /// Nom imprimé dans la légende du PDF, à la place du nom de la couche.
    pub label: String,
    /// Style posé sur un clone en mémoire de la couche, dans la mise en page seulement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<StyleSpec>,
}

/// Mise en évidence du résultat d'un plan dans son PDF : rempli par le noyau, jamais par
/// l'appelant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Focus {
    /// Couches de résultat, dans l'ordre du plan : en tête de la carte et de la légende.
    pub results: Vec<FocusResult>,
    /// Couches intermédiaires du plan, ôtées de la carte, de la légende et des sources.
    #[serde(default)]
    pub hidden: Vec<String>,
    /// Couches sur lesquelles cadrer la carte (export sans échelle) ; vide : cadrage d'avant.
    #[serde(default)]
    pub frame: Vec<String>,
    /// Sur quoi `frame` cadre : `results`, `inputs` (résultats vides) ou `visible_layers`.
    pub framed_on: FramedOn,
    /// Vrai : les intitulés de bande des rasters (« Band 1 (Gray) ») sont ôtés de la légende.
    #[serde(default)]
    pub hide_band_labels: bool,
}

/// Style de mise en évidence d'un résultat vecteur : une constante du noyau.
pub fn result_style() -> StyleSpec {
    StyleSpec::Single {
        stroke: Some(RESULT_STROKE.to_string()),
        fill: Some(RESULT_FILL.to_string()),
        width_mm: Some(RESULT_WIDTH_MM),
        opacity: None,
        max_scale: None,
    }
}

/// Code d'un défaut du noyau à l'écriture de `focus` (jamais une faute de l'appelant, AD-21).
pub const FOCUS_INTERNAL: &str = "core.internal";

/// Erreur rendue quand `focus` ne peut pas être écrit : un défaut du noyau, nommé.
pub fn internal_error(why: &str) -> crate::registry::RegistryError {
    crate::registry::RegistryError::Failed {
        code: FOCUS_INTERNAL.to_string(),
        message: format!(
            "Défaut du noyau : la mise en évidence du résultat n'a pas pu être écrite ({why})."
        ),
    }
}

/// `focus` en JSON, ou un défaut du noyau nommé.
pub fn focus_value(focus: &Focus) -> Result<Value, crate::registry::RegistryError> {
    serde_json::to_value(focus).map_err(|e| internal_error(&e.to_string()))
}

/// Une couche produite par le plan.
struct Produced {
    layer_id: String,
    /// Place dans les lignes du plan de la première ligne qui la pose.
    at: usize,
    /// Étape du journal de la commande qui l'a calculée (la sienne, ou celle du repère ouvert).
    source_step: usize,
    raster: bool,
    /// `Some(true)` si le journal dit 0 entité ; illisible ou absent : `None` (non vide).
    empty: Option<bool>,
    /// Ce par quoi une étape suivante la désigne : son id, et les repères de son fichier.
    tokens: Vec<String>,
}

fn failed(line: &Value) -> bool {
    line.get("error").is_some() || line.get("cancelled").is_some()
}

fn line_id(line: &Value) -> &str {
    line.get("id").and_then(Value::as_str).unwrap_or_default()
}

/// Étape `k` d'un repère `out:<k>.<n>`.
fn reference_step(reference: &str) -> Option<usize> {
    let (k, n) = reference.strip_prefix("out:")?.split_once('.')?;
    let ok = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !(ok(k) && ok(n)) {
        return None;
    }
    k.parse::<usize>().ok().filter(|k| *k > 0)
}

/// Toutes les chaînes d'une valeur, à toute profondeur.
fn strings<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
    match value {
        Value::String(s) => out.push(s),
        Value::Array(items) => items.iter().for_each(|v| strings(v, out)),
        Value::Object(map) => map.values().for_each(|v| strings(v, out)),
        _ => {}
    }
}

/// Nombre d'entités d'un résultat journalisé : `Some(true)` pour 0, `Some(false)` pour plus ;
/// `None` s'il n'est pas lisible (résultat résumé, champ absent).
fn emptiness(line: &Value) -> Option<bool> {
    line.get("result")
        .and_then(|r| r.get("feature_count"))
        .and_then(Value::as_u64)
        .map(|n| n == 0)
}

/// Ligne du journal à l'étape `step` parmi les lignes du plan.
fn line_at(lines: &[(usize, Value)], step: usize) -> Option<&Value> {
    lines.iter().find(|(s, _)| *s == step).map(|(_, l)| l)
}

fn produced_layers(registry: &Registry, lines: &[(usize, Value)]) -> Vec<Produced> {
    let mut produced: Vec<Produced> = Vec::new();
    for (at, (step, line)) in lines.iter().enumerate() {
        if failed(line) {
            continue;
        }
        let id = line_id(line);
        let Some(spec) = registry.spec(id) else {
            continue;
        };
        let Some(layer_id) = line.get("layer_id").and_then(Value::as_str) else {
            continue;
        };
        if spec.output_none {
            continue;
        }
        let params = line.get("params").unwrap_or(&Value::Null);
        let (source_step, raster, tokens) = if matches!(id, OPEN_RASTER | OPEN_VECTOR) {
            // Une ouverture n'est une couche produite que sur la sortie d'une étape.
            let mut refs = Vec::new();
            strings(params, &mut refs);
            let Some((reference, k)) = refs
                .into_iter()
                .find_map(|r| reference_step(r).map(|k| (r, k)))
            else {
                continue;
            };
            (
                k,
                id == OPEN_RASTER,
                vec![layer_id.to_string(), reference.to_string()],
            )
        } else if !spec.output_params.is_empty() {
            let ext = spec.output_ext_for(params);
            let mut tokens = vec![layer_id.to_string()];
            tokens.extend((1..=spec.output_params.len()).map(|n| format!("out:{step}.{n}")));
            (*step, RASTER_EXTENSIONS.contains(&ext), tokens)
        } else {
            continue;
        };
        let empty = emptiness(line).or_else(|| {
            line_at(lines, source_step)
                .filter(|_| source_step != *step)
                .and_then(emptiness)
        });
        // Une même couche posée deux fois (sa commande, puis l'ouverture de sa sortie) ne compte
        // qu'une fois : ses repères sont réunis.
        if let Some(known) = produced.iter_mut().find(|p| p.layer_id == layer_id) {
            for t in tokens {
                if !known.tokens.contains(&t) {
                    known.tokens.push(t);
                }
            }
            known.empty = known.empty.or(empty);
            continue;
        }
        produced.push(Produced {
            layer_id: layer_id.to_string(),
            at,
            source_step,
            raster,
            empty,
            tokens,
        });
    }
    produced
}

/// Chaînes que la ligne `line` consomme : ses paramètres, hors ceux que le noyau remplit pour la
/// mise en page (`focus`, `leave_out`), si c'est une étape réussie qui écrit une sortie et n'est
/// pas une ouverture dictée. Une mise en forme (`layer.set_style`…) ne consomme rien.
fn consumed<'a>(registry: &Registry, line: &'a Value) -> Vec<&'a str> {
    let mut out = Vec::new();
    let id = line_id(line);
    let Some(spec) = registry.spec(id) else {
        return out;
    };
    if failed(line) || matches!(id, OPEN_RASTER | OPEN_VECTOR) || spec.output_params.is_empty() {
        return out;
    }
    if let Some(Value::Object(params)) = line.get("params") {
        for (key, value) in params {
            if spec.focus_param.as_deref() == Some(key.as_str())
                || spec.leave_out_param.as_deref() == Some(key.as_str())
            {
                continue;
            }
            strings(value, &mut out);
        }
    }
    out
}

/// Vrai si une étape réussie du plan, après la ligne `at`, retire du projet la couche `p`
/// (`layer.remove`).
fn removed(lines: &[(usize, Value)], p: &Produced) -> bool {
    lines[p.at + 1..].iter().any(|(_, line)| {
        if failed(line) || line_id(line) != REMOVE {
            return false;
        }
        let mut found = Vec::new();
        strings(line.get("params").unwrap_or(&Value::Null), &mut found);
        found.iter().any(|s| p.tokens.iter().any(|t| t == s))
    })
}

/// Les couches produites par le plan, séparées en résultats et intermédiaires (indices dans
/// `produced`, dans l'ordre du plan). Une couche retirée du projet par le plan n'est ni l'un ni
/// l'autre.
fn split(
    registry: &Registry,
    lines: &[(usize, Value)],
    produced: &[Produced],
) -> (Vec<usize>, Vec<usize>) {
    let (mut results, mut intermediates) = (Vec::new(), Vec::new());
    for (i, p) in produced.iter().enumerate() {
        let used = lines[p.at + 1..].iter().any(|(_, line)| {
            consumed(registry, line)
                .iter()
                .any(|s| p.tokens.iter().any(|t| t == s))
        });
        if used {
            intermediates.push(i);
        } else if !removed(lines, p) {
            results.push(i);
        }
    }
    (results, intermediates)
}

/// Clés des paramètres qui désignent une couche d'entrée : `layer_id`, les `…_layer_id` et les
/// chemins déclarés.
fn layer_key(spec: &CommandSpec, key: &str) -> bool {
    key == "layer_id"
        || key.ends_with("_layer_id")
        || spec
            .path_params
            .iter()
            .any(|p| p.split('.').next() == Some(key))
}

/// D'où vient la première couche d'entrée d'une étape.
enum Input<'a> {
    /// Une sortie du plan, calculée à cette étape du journal.
    Step(usize),
    /// Une couche de l'utilisateur, par son id.
    UserLayer(&'a str),
    /// Un fichier de l'utilisateur, par son chemin.
    UserFile(&'a str),
}

/// Première couche d'entrée de la commande calculée à l'étape `step` : la valeur de sa première
/// clé de couche (`layer_id`, puis les chemins dans l'ordre déclaré, puis les autres
/// `…_layer_id`), et rien d'autre.
fn first_input<'a>(
    registry: &Registry,
    lines: &'a [(usize, Value)],
    produced: &[Produced],
    step: usize,
) -> Option<Input<'a>> {
    let line = line_at(lines, step)?;
    let spec = registry.spec(line_id(line))?;
    let params = line.get("params")?.as_object()?;
    let mut keys: Vec<&str> = vec!["layer_id"];
    keys.extend(
        spec.path_params
            .iter()
            .map(|p| p.split('.').next().unwrap_or(p)),
    );
    keys.extend(
        params
            .keys()
            .map(String::as_str)
            .filter(|k| k.ends_with("_layer_id")),
    );
    let key = keys
        .into_iter()
        .find(|k| layer_key(spec, k) && params.contains_key(*k))?;
    let mut found = Vec::new();
    strings(&params[key], &mut found);
    let v = *found.first()?;
    if let Some(p) = produced.iter().find(|p| p.tokens.iter().any(|t| t == v)) {
        return Some(Input::Step(p.source_step));
    }
    if let Some(k) = reference_step(v) {
        return Some(Input::Step(k));
    }
    if key == "layer_id" || key.ends_with("_layer_id") {
        Some(Input::UserLayer(v))
    } else {
        Some(Input::UserFile(v))
    }
}

/// Vrai si le nom des résultats demande l'inventaire : plusieurs résultats, dont l'un a pour
/// première entrée une couche de l'utilisateur.
pub fn needs_inventory(registry: &Registry, lines: &[(usize, Value)]) -> bool {
    let produced = produced_layers(registry, lines);
    let (results, _) = split(registry, lines, &produced);
    results.len() > 1
        && results.iter().any(|&i| {
            matches!(
                first_input(registry, lines, &produced, produced[i].source_step),
                Some(Input::UserLayer(_))
            )
        })
}

/// Couches de l'utilisateur que le plan a consommées : valeurs des paramètres `layer_id` et
/// `…_layer_id` (texte, ou sortie entière qui porte `layer_id`) des étapes réussies qui écrivent
/// une sortie, hors couches produites, dans l'ordre du plan, sans doublon.
fn input_layers(
    registry: &Registry,
    lines: &[(usize, Value)],
    produced: &[Produced],
) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (_, line) in lines {
        let id = line_id(line);
        let writes = registry
            .spec(id)
            .is_some_and(|spec| !spec.output_params.is_empty());
        if failed(line) || !writes || matches!(id, OPEN_RASTER | OPEN_VECTOR) {
            continue;
        }
        let Some(Value::Object(params)) = line.get("params") else {
            continue;
        };
        for (key, value) in params {
            if key != "layer_id" && !key.ends_with("_layer_id") {
                continue;
            }
            let id = match value {
                Value::String(s) => Some(s.as_str()),
                Value::Object(o) => o.get("layer_id").and_then(Value::as_str),
                _ => None,
            };
            let Some(id) = id else { continue };
            if produced.iter().any(|p| p.tokens.iter().any(|t| t == id)) {
                continue;
            }
            if seen.insert(id.to_string()) {
                out.push(id.to_string());
            }
        }
    }
    out
}

/// Titre court d'une commande, comme `summary.short_title` du panneau : sa description française
/// avant « : » (sinon sa première phrase), sans parenthèse ni point-virgule, sans point final,
/// 80 caractères au plus (coupé au mot, puis « … »).
pub fn short_title(spec: &CommandSpec) -> String {
    let text = spec.description_fr.trim();
    if text.is_empty() {
        return spec.id.clone();
    }
    // Première phrase : jusqu'au premier point suivi d'un blanc ou de la fin.
    let chars: Vec<char> = text.chars().collect();
    let mut end = chars.len();
    for i in 0..chars.len() {
        if chars[i] == '.' && (i + 1 == chars.len() || chars[i + 1].is_whitespace()) {
            end = i;
            break;
        }
    }
    let first: String = chars[..end].iter().collect();
    let head = first
        .split_once(": ")
        .map(|(h, _)| h.trim())
        .filter(|h| !h.is_empty())
        .unwrap_or(first.trim());
    let head = head
        .split(['(', ';'])
        .next()
        .unwrap_or(head)
        .trim_end()
        .trim_end_matches(['.', ',', ':', ' '])
        .trim();
    let mut title = head.to_string();
    if title.chars().count() > MAX_SHORT_TITLE {
        let cut: String = title.chars().take(MAX_SHORT_TITLE).collect();
        let cut = cut.rsplit_once(' ').map(|(a, _)| a).unwrap_or(&cut);
        title = format!("{}…", cut.trim_end_matches([',', ';']));
    }
    if title.is_empty() {
        spec.id.clone()
    } else {
        title
    }
}

/// Mot d'un nom propre : une majuscule en tête, et au moins une minuscule (un sigle comme
/// « PLU » n'en est pas un).
fn proper(word: &str) -> bool {
    word.chars().next().is_some_and(char::is_uppercase) && word.chars().any(char::is_lowercase)
}

/// « d'Abeau », « l’Isle » : un nom propre précédé d'un article ou d'une préposition élidés.
fn elided_proper(word: &str) -> bool {
    ["d'", "l'", "d’", "l’"]
        .iter()
        .any(|p| word.strip_prefix(p).is_some_and(proper))
}

/// Mots de liaison admis à l'intérieur d'un nom de lieu de plusieurs mots (« Saint Jean de Luz »).
const PARTICLES: [&str; 9] = ["de", "du", "des", "la", "le", "les", "sur", "sous", "en"];

/// Nom d'un résultat unique tiré du titre de l'export : le titre sans complément de lieu final
/// (« à », « au », « aux » ou « en » suivi d'un nom propre jusqu'à la fin), puis coupé au dernier
/// mot entier à [`MAX_LABEL`] caractères au plus. Rien de reconnu : le titre, borné de même.
pub fn result_label(title: &str) -> String {
    let words: Vec<&str> = title.split_whitespace().collect();
    let mut kept = words.len();
    for i in 1..words.len().saturating_sub(1) {
        if !matches!(words[i], "à" | "au" | "aux" | "en") {
            continue;
        }
        let rest = &words[i + 1..];
        if proper(rest[0])
            && rest.iter().all(|w| {
                proper(w) || PARTICLES.contains(w) || w.ends_with(['\'', '’']) || elided_proper(w)
            })
        {
            kept = i;
            break;
        }
    }
    bounded(&words[..kept].join(" "))
}

/// Texte coupé au dernier mot entier à [`MAX_LABEL`] caractères au plus, sans ponctuation finale.
fn bounded(text: &str) -> String {
    bounded_to(text, MAX_LABEL)
}

/// Texte coupé au dernier mot entier à `max` caractères au plus, sans ponctuation finale.
fn bounded_to(text: &str, max: usize) -> String {
    let mut label = String::new();
    for word in text.split_whitespace() {
        let longer = if label.is_empty() {
            word.to_string()
        } else {
            format!("{label} {word}")
        };
        if longer.chars().count() > max {
            if label.is_empty() {
                label = word.chars().take(max).collect();
            }
            break;
        }
        label = longer;
    }
    label
        .trim_end_matches([',', ';', ':', '—', '-', ' '])
        .to_string()
}

/// `focus` d'un export de plan, d'après les lignes du plan en cours au journal (`lines` : étape
/// du journal et ligne, dans l'ordre), le titre de l'export et, s'il y a plusieurs résultats,
/// l'inventaire du projet. `None` si le plan n'a produit aucune couche : l'export garde son
/// comportement d'avant.
pub fn focus_of(
    registry: &Registry,
    lines: &[(usize, Value)],
    title: &str,
    inventory: Option<&Value>,
) -> Option<Focus> {
    let produced = produced_layers(registry, lines);
    let (results, intermediates) = split(registry, lines, &produced);
    // Rien de produit, ou tout consommé ou retiré : pas de réponse à montrer.
    if results.is_empty() {
        return None;
    }
    let single = results.len() == 1;
    let mut labels: Vec<String> = Vec::new();
    let mut focus_results = Vec::new();
    for &i in &results {
        let p = &produced[i];
        let mut label = if single {
            result_label(title)
        } else {
            let title_of = |k: usize| {
                line_at(lines, k)
                    .and_then(|l| registry.spec(line_id(l)))
                    .map(short_title)
            };
            let command = title_of(p.source_step).unwrap_or_else(|| p.layer_id.clone());
            let input = match first_input(registry, lines, &produced, p.source_step) {
                Some(Input::Step(k)) => title_of(k),
                Some(Input::UserLayer(v)) => inventory
                    .and_then(|inv| inv.get("layers"))
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .find(|l| l.get("layer_id").and_then(Value::as_str) == Some(v))
                    .and_then(|l| l.get("name").and_then(Value::as_str))
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .map(str::to_string),
                Some(Input::UserFile(v)) => {
                    let file = v.rsplit(['/', '\\']).next().unwrap_or(v);
                    let stem = file.rsplit_once('.').map(|(s, _)| s).unwrap_or(file);
                    (!stem.is_empty()).then(|| stem.to_string())
                }
                None => None,
            };
            // Le nom de l'entrée est gardé : le titre de la commande cède la place.
            match input {
                Some(input) => {
                    let input = bounded_to(&input, MAX_LABEL / 2);
                    let room = MAX_LABEL.saturating_sub(input.chars().count() + 3);
                    format!("{} — {input}", bounded_to(&command, room))
                }
                None => bounded(&command),
            }
        };
        if label.is_empty() {
            label = title.trim().to_string();
        }
        // Deux résultats ne portent jamais le même nom.
        let mut base = label.clone();
        let mut n = 2;
        if labels.contains(&label) {
            base = bounded_to(&base, MAX_LABEL - 5);
        }
        while labels.contains(&label) {
            label = format!("{base} ({n})");
            n += 1;
        }
        labels.push(label.clone());
        focus_results.push(FocusResult {
            layer_id: p.layer_id.clone(),
            label,
            style: (!p.raster).then(result_style),
        });
    }
    let hidden = intermediates
        .iter()
        .map(|&i| produced[i].layer_id.clone())
        .collect();
    let shown: Vec<String> = results
        .iter()
        .map(|&i| &produced[i])
        .filter(|p| p.empty != Some(true))
        .map(|p| p.layer_id.clone())
        .collect();
    let (frame, framed_on) = if !shown.is_empty() {
        (shown, FramedOn::Results)
    } else {
        let inputs = input_layers(registry, lines, &produced);
        if inputs.is_empty() {
            (Vec::new(), FramedOn::VisibleLayers)
        } else {
            (inputs, FramedOn::Inputs)
        }
    };
    Some(Focus {
        results: focus_results,
        hidden,
        frame,
        framed_on,
        hide_band_labels: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Une ligne réussie du journal, à la ligne `n` (son `step`), à la place `plan_step` du plan.
    fn ok(
        n: usize,
        plan_step: usize,
        id: &str,
        params: Value,
        layer: Option<&str>,
        count: Option<u64>,
    ) -> (usize, Value) {
        let mut line = json!({"step": n, "id": id, "params": params, "plan_step": plan_step});
        if let Some(layer) = layer {
            line["layer_id"] = json!(layer);
        }
        line["result"] = match (layer, count) {
            (Some(layer), Some(c)) => json!({"layer_id": layer, "feature_count": c}),
            (Some(layer), None) => json!({"layer_id": layer}),
            _ => json!({}),
        };
        (n, line)
    }

    /// Le plan de démo, commencé à la ligne 7 du journal : sélection (A), tampon (B),
    /// intersection (C), intersection C × B (D).
    fn demo(count: u64) -> Vec<(usize, Value)> {
        vec![
            ok(
                7,
                1,
                "analysis.select_by_attribute",
                json!({"layer_id": "plu", "field": "typezone", "operator": "eq", "value": "U",
                       "out": "7-analysis.select_by_attribute.gpkg"}),
                Some("A"),
                Some(40),
            ),
            ok(
                8,
                2,
                "analysis.buffer",
                json!({"layer_id": "gares", "distance": {"value": 500, "unit": "m"},
                       "work_epsg": 2154, "out": "8-analysis.buffer.gpkg"}),
                Some("B"),
                Some(3),
            ),
            ok(
                9,
                3,
                "analysis.intersect",
                json!({"layer_id": "parcelles", "overlay_layer_id": "A",
                       "out": "9-analysis.intersect.gpkg"}),
                Some("C"),
                Some(900),
            ),
            ok(
                10,
                4,
                "analysis.intersect",
                json!({"layer_id": "C", "overlay_layer_id": "B",
                       "out": "10-analysis.intersect.gpkg"}),
                Some("D"),
                Some(count),
            ),
        ]
    }

    const TITLE: &str = "Parcelles constructibles à moins de 500 m d'une gare à Annecy";

    #[test]
    fn the_demo_plan_highlights_its_last_intersection() {
        let registry = Registry::standard();
        let lines = demo(12);
        assert!(!needs_inventory(&registry, &lines));
        let focus = focus_of(&registry, &lines, TITLE, None).unwrap();
        assert_eq!(
            focus_value(&focus).unwrap(),
            json!({"results": [{"layer_id": "D",
                     "label": "Parcelles constructibles à moins de 500 m d'une gare",
                     "style": {"kind": "single", "stroke": "#3d0026", "fill": "#c2187a",
                               "width_mm": 0.5}}],
                   "hidden": ["A", "B", "C"], "frame": ["D"], "framed_on": "results",
                   "hide_band_labels": true})
        );
        let style = focus.results[0].style.as_ref().unwrap();
        assert!(style.validate().is_ok());
        assert_eq!(style.target(), crate::style::spec::StyleTarget::Vector);
        let again: Focus = serde_json::from_value(focus_value(&focus).unwrap()).unwrap();
        assert_eq!(again, focus);
    }

    #[test]
    fn a_plan_without_produced_layer_has_no_focus() {
        let registry = Registry::standard();
        let opened = vec![ok(
            1,
            1,
            "layer.open_vector",
            json!({"path": "gares.gpkg"}),
            Some("g"),
            Some(3),
        )];
        assert!(focus_of(&registry, &opened, "Gares", None).is_none());
        let added = vec![ok(
            1,
            1,
            "layer.add",
            json!({"path": "a.csv", "crs": "EPSG:2154"}),
            Some("a"),
            None,
        )];
        assert!(focus_of(&registry, &added, "Carte", None).is_none());
        assert!(focus_of(&registry, &[], "Carte", None).is_none());
        let mut failed = demo(1);
        failed.truncate(1);
        failed[0].1["error"] = json!("qgis.failed");
        assert!(focus_of(&registry, &failed, "Carte", None).is_none());
    }

    #[test]
    fn a_plan_whose_outputs_are_all_consumed_or_removed_has_no_focus() {
        let registry = Registry::standard();
        // Le résultat retiré du projet par une étape du plan n'est pas un résultat ; les autres
        // sorties sont consommées : rien à montrer.
        let mut lines = demo(4);
        lines.push(ok(
            11,
            5,
            "layer.remove",
            json!({"layer_id": "D"}),
            None,
            None,
        ));
        assert!(focus_of(&registry, &lines, TITLE, None).is_none());
        // Un retrait en échec ne retire rien.
        lines[4].1["error"] = json!("layer.unsaved");
        assert_eq!(
            focus_of(&registry, &lines, TITLE, None).unwrap().results[0].layer_id,
            "D"
        );
    }

    #[test]
    fn a_removed_branch_leaves_the_other_result() {
        let registry = Registry::standard();
        let mut lines = demo(4);
        lines.push(ok(
            11,
            5,
            "analysis.buffer",
            json!({"layer_id": "gares",
            "distance": {"value": 1, "unit": "km"}, "out": "11-analysis.buffer.gpkg"}),
            Some("E"),
            Some(2),
        ));
        lines.push(ok(
            12,
            6,
            "layer.remove",
            json!({"layer_id": "E"}),
            None,
            None,
        ));
        let focus = focus_of(&registry, &lines, TITLE, None).unwrap();
        assert_eq!(focus.results.len(), 1);
        assert_eq!(focus.results[0].layer_id, "D");
        assert!(!focus.hidden.contains(&"E".to_string()));
    }

    #[test]
    fn an_empty_result_is_framed_on_the_inputs() {
        let registry = Registry::standard();
        let focus = focus_of(&registry, &demo(0), TITLE, None).unwrap();
        assert_eq!(focus.results[0].layer_id, "D");
        assert_eq!(focus.hidden, ["A", "B", "C"]);
        assert_eq!(focus.framed_on, FramedOn::Inputs);
        assert_eq!(focus.frame, ["plu", "gares", "parcelles"]);
    }

    #[test]
    fn the_inputs_skip_failed_and_output_less_lines() {
        let registry = Registry::standard();
        let mut lines = demo(0);
        // Une mise en forme sur une couche de l'utilisateur, une étape en échec : ni l'une ni
        // l'autre ne donne une couche d'entrée.
        lines.insert(
            0,
            ok(
                6,
                1,
                "layer.set_style",
                json!({"layer_id": "fond",
            "style": {"kind": "single", "fill": "#ffffff"}}),
                None,
                None,
            ),
        );
        let mut bad = ok(
            11,
            5,
            "analysis.buffer",
            json!({"layer_id": "ailleurs",
            "distance": {"value": 1, "unit": "m"}, "out": "11-analysis.buffer.gpkg"}),
            None,
            None,
        );
        bad.1["error"] = json!("qgis.failed");
        lines.push(bad);
        let focus = focus_of(&registry, &lines, TITLE, None).unwrap();
        assert_eq!(focus.frame, ["plu", "gares", "parcelles"]);
    }

    #[test]
    fn an_empty_result_without_input_layer_keeps_the_former_framing() {
        let registry = Registry::standard();
        let lines = vec![
            ok(
                1,
                1,
                "terrain.peaks",
                json!({"dem": "dem/mnt.tif", "out": "1-terrain.peaks.geojson"}),
                None,
                None,
            ),
            ok(
                2,
                1,
                OPEN_VECTOR,
                json!({"path": "out:1.1"}),
                Some("v"),
                Some(0),
            ),
        ];
        let focus = focus_of(&registry, &lines, "Carte", None).unwrap();
        assert_eq!(focus.framed_on, FramedOn::VisibleLayers);
        assert!(focus.frame.is_empty());
    }

    #[test]
    fn a_layer_posed_twice_counts_once() {
        let registry = Registry::standard();
        // Une commande qui rend sa couche, puis l'ouverture de sa sortie qui rend la même.
        let lines = vec![
            ok(
                3,
                1,
                "terrain.peaks",
                json!({"dem": "dem/mnt.tif", "out": "3-terrain.peaks.geojson"}),
                Some("P"),
                None,
            ),
            ok(
                4,
                1,
                OPEN_VECTOR,
                json!({"path": "out:3.1"}),
                Some("P"),
                Some(5),
            ),
        ];
        let focus = focus_of(&registry, &lines, "Points hauts", None).unwrap();
        assert_eq!(focus.results.len(), 1);
        assert_eq!(focus.frame, ["P"]);
    }

    #[test]
    fn several_results_are_named_by_command_and_first_input() {
        let registry = Registry::standard();
        let lines = vec![
            ok(
                1,
                1,
                "analysis.buffer",
                json!({"layer_id": "gares", "distance": {"value": 1, "unit": "km"},
                "out": "1-analysis.buffer.gpkg"}),
                Some("B"),
                Some(3),
            ),
            ok(
                2,
                2,
                "analysis.select_by_attribute",
                json!({"layer_id": "plu", "field": "f",
                "operator": "eq", "value": "U", "out": "2-analysis.select_by_attribute.gpkg"}),
                Some("S"),
                None,
            ),
            ok(
                3,
                3,
                "analysis.intersect",
                json!({"layer_id": "S", "overlay_layer_id": "communes",
                "out": "3-analysis.intersect.gpkg"}),
                Some("I"),
                Some(5),
            ),
            ok(
                4,
                4,
                "analysis.buffer",
                json!({"layer_id": "gares", "distance": {"value": 1, "unit": "km"},
                "out": "4-analysis.buffer.gpkg"}),
                Some("B2"),
                None,
            ),
        ];
        assert!(needs_inventory(&registry, &lines));
        let inventory = json!({"layers": [
            {"layer_id": "gares", "name": "Gares SNCF"}, {"layer_id": "plu", "name": "Zonage du PLU"}
        ]});
        let focus = focus_of(&registry, &lines, "Carte", Some(&inventory)).unwrap();
        let buffer = short_title(registry.spec("analysis.buffer").unwrap());
        let intersect = short_title(registry.spec("analysis.intersect").unwrap());
        let select = short_title(registry.spec("analysis.select_by_attribute").unwrap());
        let labels: Vec<&str> = focus.results.iter().map(|r| r.label.as_str()).collect();
        assert!(labels[0].ends_with(" — Gares SNCF"), "{labels:?}");
        assert!(
            labels[1].ends_with(&format!(" — {}", bounded_to(&select, MAX_LABEL / 2))),
            "{labels:?}"
        );
        assert!(labels[2].ends_with(" (2)"), "{labels:?}");
        assert!(intersect.starts_with(labels[1].split(" — ").next().unwrap()));
        assert!(buffer.starts_with(labels[0].split(" — ").next().unwrap()));
        for l in &labels {
            assert!(l.chars().count() <= MAX_LABEL, "{l}");
        }
        assert_eq!(focus.hidden, ["S"]);
        assert_eq!(focus.frame, ["B", "I", "B2"]);
        // Sans inventaire lisible, une couche de l'utilisateur n'a pas de nom : le titre seul,
        // jamais un autre paramètre (la distance, son unité…).
        let focus = focus_of(&registry, &lines, "Carte", None).unwrap();
        assert_eq!(focus.results[0].label, bounded(&buffer));
    }

    #[test]
    fn multi_result_labels_are_bounded() {
        let registry = Registry::standard();
        let lines = vec![
            ok(
                1,
                1,
                "analysis.select_by_attribute",
                json!({"layer_id": "plu", "field": "f",
                "operator": "eq", "value": "U", "out": "1-analysis.select_by_attribute.gpkg"}),
                Some("S"),
                None,
            ),
            ok(
                2,
                2,
                "analysis.select_by_attribute",
                json!({"layer_id": "parcelles", "field": "f",
                "operator": "eq", "value": "U", "out": "2-analysis.select_by_attribute.gpkg"}),
                Some("T"),
                None,
            ),
        ];
        let inventory = json!({"layers": [{"layer_id": "plu", "name": "Zonage du PLU d'Annecy"},
            {"layer_id": "parcelles", "name": "Parcelles"}]});
        let focus = focus_of(&registry, &lines, "Carte", Some(&inventory)).unwrap();
        for r in &focus.results {
            assert!(r.label.chars().count() <= MAX_LABEL, "{}", r.label);
        }
        assert!(
            focus.results[0]
                .label
                .ends_with(" — Zonage du PLU d'Annecy"),
            "{:?}",
            focus.results
        );
        assert!(
            focus.results[1].label.ends_with(" — Parcelles"),
            "{:?}",
            focus.results
        );
    }

    #[test]
    fn the_inventory_is_read_only_for_a_result_from_a_user_layer() {
        let registry = Registry::standard();
        // Deux résultats tirés d'un MNT de l'utilisateur (chemin) : le nom vient du fichier.
        let lines = vec![
            ok(
                1,
                1,
                "terrain.slope",
                json!({"dem": "dem/mnt.tif", "out": "1-terrain.slope.tif"}),
                None,
                None,
            ),
            ok(
                2,
                1,
                OPEN_RASTER,
                json!({"path": "out:1.1"}),
                Some("P"),
                None,
            ),
            ok(
                3,
                2,
                "terrain.aspect",
                json!({"dem": "dem/mnt.tif", "out": "3-terrain.aspect.tif"}),
                None,
                None,
            ),
            ok(
                4,
                2,
                OPEN_RASTER,
                json!({"path": "out:3.1"}),
                Some("E"),
                None,
            ),
        ];
        assert!(!needs_inventory(&registry, &lines));
        let focus = focus_of(&registry, &lines, "Carte", None).unwrap();
        assert!(
            focus.results[0].label.ends_with("— mnt"),
            "{}",
            focus.results[0].label
        );
        // Une seule couche de l'utilisateur en entrée suffit à demander l'inventaire.
        let mut lines = lines;
        lines.push(ok(
            5,
            3,
            "analysis.buffer",
            json!({"layer_id": "gares",
            "distance": {"value": 1, "unit": "km"}, "out": "5-analysis.buffer.gpkg"}),
            Some("B"),
            Some(1),
        ));
        assert!(needs_inventory(&registry, &lines));
    }

    #[test]
    fn a_raster_result_is_named_but_not_restyled_and_its_dem_stays() {
        let registry = Registry::standard();
        let lines = vec![
            ok(
                1,
                1,
                "terrain.slope",
                json!({"dem": "dem/mnt.tif", "out": "1-terrain.slope.tif"}),
                None,
                None,
            ),
            ok(
                2,
                1,
                OPEN_RASTER,
                json!({"path": "out:1.1"}),
                Some("P"),
                None,
            ),
            ok(
                3,
                1,
                "layer.set_style",
                json!({"layer_id": "P", "style": {"kind": "hillshade"}}),
                None,
                None,
            ),
            ok(
                4,
                2,
                "raster.reclassify",
                json!({"raster": "out:1.1", "out": "4-raster.reclassify.tif"}),
                None,
                None,
            ),
            ok(
                5,
                2,
                OPEN_RASTER,
                json!({"path": "out:4.1"}),
                Some("R"),
                None,
            ),
        ];
        let focus = focus_of(&registry, &lines, "Pentes fortes", None).unwrap();
        assert_eq!(focus.hidden, ["P"]);
        assert_eq!(focus.results.len(), 1);
        assert_eq!(focus.results[0].layer_id, "R");
        assert_eq!(focus.results[0].label, "Pentes fortes");
        assert!(focus.results[0].style.is_none());
        assert_eq!(focus.frame, ["R"]);
        let focus = focus_of(&registry, &lines[..3], "Pente", None).unwrap();
        assert_eq!(focus.results[0].layer_id, "P");
        assert!(focus.hidden.is_empty());
    }

    #[test]
    fn an_earlier_export_of_the_plan_consumes_nothing() {
        let registry = Registry::standard();
        let mut lines = demo(3);
        lines.push(ok(
            11,
            5,
            "layout.export_pdf",
            json!({"title": "Avant", "out": "11-layout.export_pdf.pdf",
            "focus": {"results": [{"layer_id": "D", "label": "x"}], "hidden": ["A"], "frame": ["D"],
                      "framed_on": "results"}}),
            None,
            None,
        ));
        let focus = focus_of(&registry, &lines, "Après", None).unwrap();
        assert_eq!(focus.results[0].layer_id, "D");
    }

    #[test]
    fn the_label_drops_a_final_place_and_stays_short() {
        for (title, label) in [
            (
                TITLE,
                "Parcelles constructibles à moins de 500 m d'une gare",
            ),
            (
                "Réseau ferré de Villeneuve-le-Roi",
                "Réseau ferré de Villeneuve-le-Roi",
            ),
            ("Gares à Villeneuve-le-Roi", "Gares"),
            ("Zones humides en Haute-Savoie", "Zones humides"),
            ("Pistes au Grand-Bornand", "Pistes"),
            ("Écoles aux Contamines-Montjoie", "Écoles"),
            ("Parcelles à Saint Jean de Luz", "Parcelles"),
            ("Gares à L'Isle d'Abeau", "Gares"),
            ("Gares à L’Isle d’ Abeau", "Gares"),
            ("Zones en PLU", "Zones en PLU"),
            ("Parcelles à moins de 500 m", "Parcelles à moins de 500 m"),
            ("À Annecy", "À Annecy"),
            ("  Carte  ", "Carte"),
        ] {
            assert_eq!(result_label(title), label, "{title}");
        }
        let long = "Parcelles constructibles du plan local d'urbanisme situées à moins de cinq cents mètres d'une gare";
        let cut = result_label(long);
        assert!(cut.chars().count() <= MAX_LABEL, "{cut}");
        assert!(
            long.starts_with(&cut) && long[cut.len()..].starts_with(' '),
            "{cut}"
        );
        assert_eq!(result_label(&"x".repeat(80)).chars().count(), MAX_LABEL);
    }

    #[test]
    fn a_focus_that_cannot_be_written_is_an_internal_error() {
        let e = internal_error("essai");
        assert_eq!(e.code(), FOCUS_INTERNAL);
        assert_ne!(e.code(), "registry.invalid_params");
    }

    #[test]
    fn the_short_title_follows_the_panel() {
        let registry = Registry::standard();
        assert_eq!(
            short_title(registry.spec("analysis.intersect").unwrap()),
            "Calcule l'intersection de deux couches vecteur"
        );
        assert_eq!(
            short_title(registry.spec("analysis.select_by_attribute").unwrap()),
            "Garde les entités d'une couche vecteur dont un champ vérifie une comparaison"
        );
        for spec in registry.specs() {
            let t = short_title(spec);
            assert!(
                !t.is_empty() && t.chars().count() <= MAX_SHORT_TITLE + 1,
                "{}",
                spec.id
            );
        }
    }
}
