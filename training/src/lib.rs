//! Chaîne d'entraînement du modèle de composition (ticket 5.10, tâche 1) : un générateur
//! déterministe d'exemples « inventaire + demande française → plan attendu ».
//!
//! Chaque exemple est construit par une famille de tournures (`families`) sur un inventaire
//! synthétique (`inventories`), puis **gardé seulement s'il passe `Registry::check_plan`** du
//! registre réel, et quelques gardes de plus (couches citées présentes dans l'inventaire,
//! fichier ouvert parmi les fichiers non chargés, question et refus non vides). Les
//! enregistrements sont structurés ; leur mise en forme en prompt court (tâche 2) vit dans
//! `prompt`, seul endroit qui décide du texte vu par le modèle.

pub mod barres;
pub mod families;
pub mod inventories;
pub mod prompt;
pub mod rng;

use std::collections::{BTreeMap, HashSet};

use cassini_core::plan::Plan;
use cassini_core::registry::Registry;
use serde_json::json;

use families::{Example, Family, Retry, FAMILIES};
use rng::Rng;

/// Graine par défaut : celle de la commande documentée et du test de recouvrement (le jeu
/// engendré n'est pas versionné, il se régénère).
pub const DEFAULT_SEED: u64 = 1;
/// Nombre d'exemples par défaut : celui de l'entraînement (décision du propriétaire, 5.15 ;
/// 8 600 depuis le 2026-11-10, pour que les familles ajoutées ne diluent plus les autres).
pub const DEFAULT_COUNT: usize = 8600;
/// Plafond d'étapes d'un plan, celui de `cassini-ai` (`MAX_STEPS`).
const MAX_STEPS: usize = 12;
/// Borne du texte vu par le modèle, en caractères de `prompt_text` (5.47, décision 5 du
/// 2026-11-10) : un exemple plus long est écarté ici, pour que la chaîne d'entraînement, qui
/// écarte toute séquence de plus de `MAX_LENGTH` = 5 632 jetons (prompt et réponse), n'en écarte
/// aucun. 19 000 depuis 5.49 (tranchée en party le 2026-11-10, à confirmer par le propriétaire) :
/// les gros projets vus en entier, sous le seuil du tri du noyau (17 000 caractères, la taille
/// d'Annecy), ne sont plus écartés. Réglée sur la mesure au tokeniseur de Qwen3.5
/// (`docs/entrainement-du-modele.md`).
pub const MAX_PROMPT_CHARS: usize = 19_000;

/// Décompte d'une famille : exemples gardés, écartés par la validation, doublons.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Tally {
    pub kept: usize,
    pub rejected: usize,
    pub duplicates: usize,
}

/// Résumé d'une génération, par famille (ordre alphabétique).
#[derive(Debug, Default, Clone)]
pub struct Summary {
    pub families: BTreeMap<&'static str, Tally>,
    /// Premiers motifs d'écart, pour le diagnostic.
    pub reasons: Vec<String>,
}

/// Paramètres de couche qui désignent une couche existante (ou une étape précédente).
const LAYER_PARAMS: [&str; 3] = ["layer_id", "overlay_layer_id", "points_layer_id"];
/// Paramètres raster de premier niveau : un MNT, un raster d'entrée ou un raster de coût.
const RASTER_PARAMS: [&str; 3] = ["dem", "raster", "cost"];

/// Vérifie un exemple : plan lisible, `check_plan` du registre réel, couches et fichiers cités
/// présents dans l'inventaire, plafond d'étapes, question et refus non vides. Rend le motif
/// d'écart sinon.
pub fn validate(registry: &Registry, example: &Example) -> Result<(), String> {
    let plan: Plan = serde_json::from_value(example.expected.clone())
        .map_err(|e| format!("plan illisible : {e}"))?;
    registry
        .check_plan(&plan)
        .map_err(|e| format!("{} (étape {}) : {}", e.code, e.step, e.message))?;
    match &plan {
        Plan::Question { question, .. } => {
            if question.trim().is_empty() || !question.trim_end().ends_with('?') {
                return Err("question vide ou sans point d'interrogation".into());
            }
        }
        Plan::Refusal { reason } => {
            // Une question écrite dans un refus est le défaut à corriger, pas à apprendre.
            if reason.trim().is_empty() || reason.contains('?') {
                return Err("refus vide ou formulé en question".into());
            }
        }
        Plan::Calls { calls } => {
            if calls.is_empty() || calls.len() > MAX_STEPS {
                return Err(format!("{} étapes", calls.len()));
            }
            // Une couche de l'index d'un gros projet (5.47) existe, même montrée réduite.
            let ids: HashSet<&str> = example.inventory["layers"]
                .as_array()
                .into_iter()
                .flatten()
                .chain(example.inventory["index"].as_array().into_iter().flatten())
                .filter_map(|l| l["layer_id"].as_str())
                .collect();
            // Le MNT d'une commande de terrain est un chemin recopié de l'inventaire : la
            // `source` d'une couche raster, jamais un id de couche ni un chemin inventé.
            let dems: HashSet<&str> = example.inventory["layers"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|l| l["kind"] == "raster")
                .filter_map(|l| l["source"].as_str())
                .collect();
            let files: HashSet<&str> = example.inventory["unloaded_files"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|f| f["path"].as_str())
                .collect();
            // Un raster d'un autre format, non ouvert, que `data.convert_raster` cite par son
            // chemin (epic 10, entrée 30).
            let raster_files: HashSet<&str> = example.inventory["unloaded_files"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|f| f["kind"] == "raster")
                .filter_map(|f| f["path"].as_str())
                .collect();
            for (index, call) in calls.iter().enumerate() {
                for name in LAYER_PARAMS {
                    if let Some(id) = call.params[name].as_str() {
                        if !id.starts_with("step:") && !ids.contains(id) {
                            return Err(format!("étape {} : couche absente « {id} »", index + 1));
                        }
                    }
                }
                // Un paramètre raster de premier niveau est une `source` raster de l'inventaire
                // ou le résultat d'une étape précédente (`step:k`) ; `check_plan` a déjà vérifié
                // la référence et son type raster.
                for name in RASTER_PARAMS {
                    if let Some(raster) = call.params.get(name) {
                        let known = raster.as_str().is_some_and(|r| {
                            dems.contains(r)
                                || r.starts_with("step:")
                                || (call.command == "data.convert_raster"
                                    && raster_files.contains(r))
                        });
                        if !known {
                            return Err(format!(
                                "étape {} : raster absent « {raster} »",
                                index + 1
                            ));
                        }
                    }
                }
                // Les rasters en liste de `raster.weighted_overlay` (11.12, lot 2b-2) : chaque
                // critère et chaque masque est une `source` raster de l'inventaire, jamais une
                // étape (`step:k`) ni un chemin inventé.
                let criteria = call.params["criteria"].as_array().into_iter().flatten();
                let listed = criteria
                    .map(|c| &c["raster"])
                    .chain(call.params["exclusions"].as_array().into_iter().flatten());
                for raster in listed {
                    if !raster.as_str().is_some_and(|r| dems.contains(r)) {
                        return Err(format!("étape {} : raster absent « {raster} »", index + 1));
                    }
                }
                if let Some(path) = call.params["path"].as_str() {
                    if !files.contains(path) {
                        return Err(format!("étape {} : fichier absent « {path} »", index + 1));
                    }
                }
            }
        }
    }
    if example.request.trim().is_empty() {
        return Err("demande vide".into());
    }
    reduced_focus(example, &plan)?;
    if let Some(listed) = &example.listed {
        listed_shown(example, listed)?;
    }
    let chars = prompt_text(example).chars().count();
    if chars > MAX_PROMPT_CHARS {
        return Err(format!("prompt trop long : {chars} caractères"));
    }
    if let Some(retry) = &example.retry {
        validate_retry(registry, &plan, retry)?;
    }
    Ok(())
}

/// Dans un gros projet montré trié (clé `index`, 5.47), chaque couche utile de l'exemple est
/// détaillée dans `layers`, sauf une couche qu'un plan d'appels ne cite que comme géométrie : citée
/// par un paramètre de couche, et sans paramètre de champ (`field`, `fields`, `label_field`...)
/// dans aucun des appels qui la citent. Un raster cité par sa source n'a pas d'identifiant dans le
/// plan : sa source manquerait, ce que `validate` écarte déjà.
fn reduced_focus(example: &Example, plan: &Plan) -> Result<(), String> {
    if example.inventory.get("index").is_none() {
        return Ok(());
    }
    let detailed: HashSet<&str> = example.inventory["layers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| l["layer_id"].as_str())
        .collect();
    let geometry_only = |id: &str| -> bool {
        let Plan::Calls { calls } = plan else {
            return false;
        };
        let citing: Vec<_> = calls
            .iter()
            .filter(|c| {
                LAYER_PARAMS
                    .iter()
                    .any(|k| c.params[*k].as_str() == Some(id))
            })
            .collect();
        // Une reprojection dépend du système de la couche, que l'index ne montre pas.
        !citing.is_empty()
            && citing.iter().all(|c| {
                c.command != "data.reproject"
                    && c.params
                        .as_object()
                        .is_none_or(|m| m.keys().all(|k| !k.contains("field")))
            })
    };
    for id in &example.focus {
        if !detailed.contains(id.as_str()) && !geometry_only(id) {
            return Err(format!("couche utile « {id} » montrée réduite"));
        }
    }
    // Une couche détaillée plafonnée (30 champs ou 3 000 caractères, `sorted_view`) peut perdre
    // un champ : le champ que cite le plan doit rester visible.
    if let Plan::Calls { calls } = plan {
        for call in calls {
            let Some(id) = call.params["layer_id"].as_str() else {
                continue;
            };
            let Some(layer) = example.inventory["layers"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|l| l["layer_id"].as_str() == Some(id))
            else {
                continue;
            };
            let shown: HashSet<&str> = layer["fields"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|f| f["name"].as_str())
                .collect();
            for key in ["field", "label_field", "x_field", "y_field"] {
                if let Some(field) = call.params[key].as_str() {
                    if !shown.contains(field) {
                        return Err(format!("couche utile « {id} » : champ « {field} » coupé"));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Une question de valeur absente (5.54, séance 3) nomme les valeurs connues d'un champ : la
/// couche détaillée de l'inventaire montré porte ce champ avec exactement ces valeurs. La vue
/// triée d'un gros projet, qui plafonne une couche, peut ôter le champ : l'exemple est écarté.
fn listed_shown(example: &Example, listed: &families::Listed) -> Result<(), String> {
    let values = example.inventory["layers"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|l| l["layer_id"].as_str() == Some(listed.layer_id.as_str()))
        .and_then(|l| l["fields"].as_array())
        .into_iter()
        .flatten()
        .find(|f| f["name"].as_str() == Some(listed.field.as_str()))
        .and_then(|f| f["values"].as_array());
    let mut shown: Vec<&str> = values
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect();
    shown.sort_unstable();
    let mut wanted: Vec<&str> = listed.values.iter().map(String::as_str).collect();
    wanted.sort_unstable();
    if shown != wanted {
        return Err(format!(
            "couche « {} » : champ à valeurs « {} » coupé",
            listed.layer_id, listed.field
        ));
    }
    Ok(())
}

/// Un exemple de redemande : la sortie rejetée se relit comme un plan d'appels, `check_plan`
/// la rejette, les écarts sont ceux du jugement de `cassini-ai`, et la réponse attendue est un
/// plan d'appels qui diffère de la sortie rejetée.
fn validate_retry(registry: &Registry, expected: &Plan, retry: &Retry) -> Result<(), String> {
    let rejected: Plan = serde_json::from_str(&retry.rejected)
        .map_err(|e| format!("sortie rejetée illisible : {e}"))?;
    if !matches!(rejected, Plan::Calls { .. }) || !matches!(expected, Plan::Calls { .. }) {
        return Err("redemande hors d'un plan d'appels".into());
    }
    if rejected == *expected {
        return Err("sortie rejetée identique au plan attendu".into());
    }
    match registry.check_plan(&rejected) {
        Ok(()) => Err("corruption acceptée par check_plan".into()),
        Err(e) if retry.errors == [format!("étape {} : {}", e.step, e.message)] => Ok(()),
        Err(_) => Err("écarts différents de ceux du jugement".into()),
    }
}

/// Engendre `count` exemples valides et distincts avec la graine `seed`. Même graine, même
/// suite, à l'octet près. S'arrête plus tôt (et le résumé le montre) si les familles ne
/// rendent plus d'exemples neufs.
pub fn generate(seed: u64, count: usize) -> (Vec<Example>, Summary) {
    let all: Vec<&'static Family> = FAMILIES.iter().collect();
    generate_from(seed, count, &all)
}

/// Graine du jeu ciblé d'une séance d'ajustement continu (5.54) : autre que celle de
/// l'entraînement (1) et de la validation (2). `training/tests/recouvrement.rs` garde chaque
/// famille tirée seule à cette graine.
pub const FOCUS_SEED: u64 = 3;
/// Exemples ciblés d'une séance continue par défaut : 3 000 exemples, dont 1 200 de rappel
/// (décision D2 du propriétaire, 2026-11-11).
pub const FOCUS_COUNT: usize = 1800;
/// Familles qui, tirées seules à `FOCUS_SEED` jusqu'à `FOCUS_COUNT` exemples, engendrent une
/// phrase d'une barre à un nombre ou une commune près (relevé de
/// `training/tests/recouvrement.rs`) : le jeu complet ne la tire pas, un jeu ciblé le pourrait.
/// Refusées au jeu ciblé tant que leurs tournures ne sont pas reprises ; le test tient cette liste
/// exacte, dans les deux sens. Vide depuis 5.54, lot 2 : les cinq familles du premier relevé
/// (`filtre_numerique`, `pente`, `reprojection_couche`, `retrait_couche`, `carroyage_mgrs`) ont
/// perdu leur tournure fautive. La liste reste le contrôle des familles futures.
pub const FOCUS_EXCLUDED: &[&str] = &[];

/// Les familles du jeu ciblé de la première séance continue (5.54, lot 2) : les six familles
/// neuves ou renforcées d'après les leçons de la remesure du 2026-11-11 (format gardé avec
/// l'échelle, fonctions absentes du registre, options écrites, champ de vision cumulé, point en
/// Lambert-93 vers `geo.*` ou `coord.convert`, nord écrit avant l'est). Recopiée dans la commande
/// de la séance (`docs/entrainement-du-modele.md`), que le test
/// `the_session_command_of_the_doc_names_the_first_session_families` tient égale ; le rappel tient
/// le reste.
pub const FIRST_SESSION_FAMILIES: &[&str] = &[
    "export_pdf_echelle_format",
    "refus_fonction_absente",
    "options_ecrites",
    "champ_de_vision_cumule",
    "point_lambert_vers_geo",
    "ordre_nord_est",
];

/// Les familles du jeu ciblé de la seconde séance continue (5.54, séance 2) : celles de la
/// première (la séance repart de B, ses gains se réapprennent), plus la ligne de vue et le
/// recoupement, que le champ de vision et la reprojection vers 4326 attiraient (contraste).
/// Recopiée dans la commande de la séance (`docs/entrainement-du-modele.md`), que le test
/// `the_session_command_of_the_doc_names_the_second_session_families` tient égale.
pub const SECOND_SESSION_FAMILIES: &[&str] = &[
    "export_pdf_echelle_format",
    "refus_fonction_absente",
    "options_ecrites",
    "champ_de_vision_cumule",
    "point_lambert_vers_geo",
    "ordre_nord_est",
    "ligne_de_vue",
    "recoupement",
];

/// Les familles du jeu ciblé de la troisième séance continue (5.54, séance 3) : celles de la
/// seconde (la séance repart de B), plus les plans d'urbanisme à plusieurs conditions
/// (`conditions_multiples`, `constructible`), leur contraste à trois étapes
/// (`distance_reference_filtree`) et les questions d'un champ ou d'une valeur absents, que le
/// mélange de la séance 2 diluait. Recopiée dans la commande de la séance
/// (`docs/entrainement-du-modele.md`), que le test
/// `the_session_command_of_the_doc_names_the_third_session_families` tient égale.
pub const THIRD_SESSION_FAMILIES: &[&str] = &[
    "export_pdf_echelle_format",
    "refus_fonction_absente",
    "options_ecrites",
    "champ_de_vision_cumule",
    "point_lambert_vers_geo",
    "ordre_nord_est",
    "ligne_de_vue",
    "recoupement",
    "conditions_multiples",
    "constructible",
    "distance_reference_filtree",
    "question_champ_absent",
    "question_valeur_absente",
];

/// Plancher du rappel stratifié d'une séance continue (5.54, séance 2) : au moins autant
/// d'exemples de chaque famille (ou tous ceux du jeu complet, s'il en a moins). 15 garde les gros
/// projets vers 22 % d'un rappel de 1 500 ; 20 les ferait tomber vers 5 %. Lu par `pipeline.py`.
pub const RECALL_FLOOR: usize = 15;
/// Version de l'algorithme de `stratified_recall` : la lignée l'écrit (`rappel_version`) et
/// `pipeline.py` refuse de reprendre une séance commencée sous une autre. À relever à chaque
/// changement de l'algorithme (quotas, ordre rendu) ; lue par `pipeline.py`.
pub const RECALL_VERSION: usize = 1;

/// Le rappel stratifié d'une séance continue (5.54, séance 2) : `count` exemples du jeu graine 1
/// complet (`generate(DEFAULT_SEED, DEFAULT_COUNT)`, que `training/tests/recouvrement.rs` garde),
/// au moins `min(floor, n_f)` de chaque famille `f` (`n_f` ses exemples dans le jeu complet), le
/// reste au poids réel des familles dans le jeu complet (rejets de `gros_projet` compris).
///
/// Quota de `f` : `q_f = min(n_f, max(min(floor, n_f), ⌊λ·n_f⌋))`, λ le plus grand tel que
/// Σ q_f ≤ `count` ; chaque famille prend ses `q_f` premiers exemples (ordre du jeu complet), et le
/// reliquat se comble par les premiers exemples non pris. Ordre rendu : le plancher (les
/// `min(floor, n_f)` premiers de chaque famille, dans l'ordre du jeu complet), puis le surplus
/// (dans l'ordre du jeu complet) : un rappel coupé à la fin (marge de `pipeline.py`) ne perd que du
/// surplus. Refusé si `count` dépasse `DEFAULT_COUNT` ou si le plancher n'y tient pas.
pub fn stratified_recall(count: usize, floor: usize) -> Result<Vec<Example>, String> {
    stratified_recall_with_floor(count, floor).map(|(recall, _)| recall)
}

/// `stratified_recall`, plus le plancher de chaque famille, `min(floor, n_f)` : ce que la chaîne
/// vérifie encore après avoir écarté les exemples trop longs et coupé la marge.
pub fn stratified_recall_with_floor(
    count: usize,
    floor: usize,
) -> Result<(Vec<Example>, BTreeMap<&'static str, usize>), String> {
    if floor == 0 {
        return Err("rappel stratifié sans plancher : --rappel-stratifie vaut au moins 1".into());
    }
    if count > DEFAULT_COUNT {
        return Err(format!(
            "rappel stratifié de {count} exemples : au plus {DEFAULT_COUNT} (DEFAULT_COUNT, le jeu \
             graine {DEFAULT_SEED} gardé par recouvrement.rs)"
        ));
    }
    let (full, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
    if full.len() < count {
        return Err(format!(
            "jeu graine {DEFAULT_SEED} de {} exemples seulement, {count} demandés",
            full.len()
        ));
    }
    let mut sizes: BTreeMap<&'static str, usize> = BTreeMap::new();
    for e in &full {
        *sizes.entry(e.family).or_default() += 1;
    }
    let minimum: BTreeMap<&'static str, usize> =
        sizes.iter().map(|(f, &n)| (*f, n.min(floor))).collect();
    let quota = |lambda: f64, n: usize| -> usize {
        let share = (lambda * n as f64).floor() as usize;
        n.min(floor.min(n).max(share))
    };
    let total = |lambda: f64| -> usize { sizes.values().map(|&n| quota(lambda, n)).sum() };
    let base = total(0.0);
    if base > count {
        return Err(format!(
            "plancher de {floor} exemples par famille : {base} exemples, plus que les {count} \
             demandés"
        ));
    }
    // λ le plus grand tel que Σ q_f ≤ count, par dichotomie : Σ q_f croît avec λ, et vaut à
    // λ = 1 le jeu complet entier, au moins `count`.
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..64 {
        let mid = (lo + hi) / 2.0;
        if total(mid) <= count {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let quotas: BTreeMap<&str, usize> = sizes.iter().map(|(f, &n)| (*f, quota(lo, n))).collect();
    // Pris : les `q_f` premiers de chaque famille, puis le reliquat dans l'ordre du jeu complet.
    // Le plancher (`r < floor`) est toujours pris : `q_f ≥ min(floor, n_f)`.
    let mut taken = vec![false; full.len()];
    let mut in_floor = vec![false; full.len()];
    let mut rank: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, e) in full.iter().enumerate() {
        let r = rank.entry(e.family).or_default();
        taken[i] = *r < quotas[e.family];
        in_floor[i] = *r < floor;
        *r += 1;
    }
    let mut left = count - taken.iter().filter(|t| **t).count();
    for t in taken.iter_mut() {
        if left == 0 {
            break;
        }
        if !*t {
            *t = true;
            left -= 1;
        }
    }
    let (taken, in_floor, full) = (&taken, &in_floor, &full);
    let pick = |floor_part: bool| {
        full.iter()
            .enumerate()
            .filter(move |(i, _)| taken[*i] && in_floor[*i] == floor_part)
            .map(|(_, e)| e.clone())
    };
    Ok((pick(true).chain(pick(false)).collect(), minimum))
}

/// Les familles nommées, dans l'ordre fixe de `FAMILIES` (le tirage ne dépend pas de l'ordre
/// de la liste donnée). Un nom inconnu, un nom en double ou une liste vide sont refusés, avec
/// la liste des noms connus.
pub fn select_families(names: &[&str]) -> Result<Vec<&'static Family>, String> {
    let known = || {
        FAMILIES
            .iter()
            .map(|f| f.name)
            .collect::<Vec<_>>()
            .join(", ")
    };
    if names.is_empty() {
        return Err(format!(
            "aucune famille nommée ; familles connues : {}",
            known()
        ));
    }
    let mut asked = HashSet::new();
    for name in names {
        if !FAMILIES.iter().any(|f| f.name == *name) {
            return Err(format!(
                "famille inconnue : {name:?} ; familles connues : {}",
                known()
            ));
        }
        if !asked.insert(*name) {
            return Err(format!("famille nommée deux fois : {name:?}"));
        }
    }
    Ok(select_unchecked(&asked))
}

/// Les familles du jeu ciblé, refusées si l'une d'elles est dans `FOCUS_EXCLUDED`.
fn check_focus(families: Vec<&'static Family>) -> Result<Vec<&'static Family>, String> {
    check_focus_against(families, FOCUS_EXCLUDED)
}

/// `check_focus` contre une liste d'exclusion donnée (le test l'essaie sur une liste non vide).
fn check_focus_against(
    families: Vec<&'static Family>,
    excluded_names: &[&str],
) -> Result<Vec<&'static Family>, String> {
    let excluded: Vec<&str> = families
        .iter()
        .map(|f| f.name)
        .filter(|n| excluded_names.contains(n))
        .collect();
    if excluded.is_empty() {
        Ok(families)
    } else {
        Err(format!(
            "famille refusée au jeu ciblé : {} (tirée seule, elle engendre une phrase d'une \
             barre ; voir FOCUS_EXCLUDED)",
            excluded.join(", ")
        ))
    }
}

fn select_unchecked(asked: &HashSet<&str>) -> Vec<&'static Family> {
    FAMILIES.iter().filter(|f| asked.contains(f.name)).collect()
}

/// Comme `generate`, sur les seules familles nommées (jeu ciblé d'une séance continue, 5.54) :
/// même tirage pondéré, restreint à leurs poids. Une famille de `FOCUS_EXCLUDED` est refusée.
pub fn generate_families(
    seed: u64,
    count: usize,
    names: &[&str],
) -> Result<(Vec<Example>, Summary), String> {
    Ok(generate_from(
        seed,
        count,
        &check_focus(select_families(names)?)?,
    ))
}

/// Comme `generate_families`, sans le refus de `FOCUS_EXCLUDED` : pour le relevé de
/// `training/tests/recouvrement.rs` et les tests, jamais pour un jeu d'entraînement.
pub fn generate_families_unchecked(
    seed: u64,
    count: usize,
    names: &[&str],
) -> Result<(Vec<Example>, Summary), String> {
    Ok(generate_from(seed, count, &select_families(names)?))
}

/// Le tirage pondéré sur `families`, dans leur ordre ; `generate` le fait sur toutes, à
/// l'octet près comme avant 5.54.
fn generate_from(seed: u64, count: usize, families: &[&'static Family]) -> (Vec<Example>, Summary) {
    let registry = Registry::standard();
    let mut rng = Rng::new(seed);
    let mut summary = Summary::default();
    for family in families {
        summary.families.insert(family.name, Tally::default());
    }
    let total: u64 = families.iter().map(|f| f.weight).sum();
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(count);
    let mut attempts = 0usize;
    while out.len() < count && attempts < count.saturating_mul(50).max(1000) {
        attempts += 1;
        let mut roll = rng.below(total as usize) as u64;
        let family = families
            .iter()
            .find(|f| {
                if roll < f.weight {
                    true
                } else {
                    roll -= f.weight;
                    false
                }
            })
            .expect("tirage dans la somme des poids");
        let Some(example) = (family.make)(&mut rng) else {
            continue;
        };
        let tally = summary
            .families
            .get_mut(family.name)
            .expect("famille connue");
        if let Err(reason) = validate(&registry, &example) {
            tally.rejected += 1;
            if summary.reasons.len() < 20 {
                summary
                    .reasons
                    .push(format!("{} : {reason} — {}", family.name, example.request));
            }
            continue;
        }
        let line = to_line(&example);
        if !seen.insert(line) {
            tally.duplicates += 1;
            continue;
        }
        tally.kept += 1;
        out.push(example);
    }
    (out, summary)
}

/// Un enregistrement JSON sur une ligne : `{inventory, request, expected}`. La famille reste
/// dans `Example` et le résumé, pas dans le fichier.
pub fn to_line(example: &Example) -> String {
    let mut line = json!({
        "inventory": example.inventory,
        "request": example.request,
        "expected": example.expected,
    });
    // Un exemple de redemande porte aussi sa sortie rejetée et ses écarts.
    if let Some(retry) = &example.retry {
        line["retry"] = json!({"rejected": retry.rejected, "errors": retry.errors});
    }
    line.to_string()
}

/// Le texte exact vu par le modèle pour cet exemple : le prompt du premier essai, ou celui de
/// la redemande.
pub fn prompt_text(example: &Example) -> String {
    match &example.retry {
        None => prompt::render_prompt(&example.inventory, &example.request),
        Some(retry) => prompt::render_retry_prompt(
            &example.inventory,
            &example.request,
            &retry.rejected,
            &retry.errors,
        ),
    }
}

/// Le fichier JSONL entier, une ligne par exemple, fin de ligne `\n`.
pub fn to_jsonl(examples: &[Example]) -> String {
    let mut text = String::new();
    for e in examples {
        text.push_str(&to_line(e));
        text.push('\n');
    }
    text
}

/// Un enregistrement de prompt sur une ligne : `{prompt, completion}`, mis en forme par
/// `prompt` (tâche 2). C'est ce que lit la chaîne d'entraînement, sans rien reformater.
pub fn to_chat_line(example: &Example) -> String {
    let plan: Plan = serde_json::from_value(example.expected.clone())
        .expect("un exemple écrit a passé `validate`, son plan se relit");
    json!({
        "prompt": prompt_text(example),
        "completion": prompt::completion(&plan),
    })
    .to_string()
}

/// Le fichier JSONL de prompts entier, une ligne par exemple, fin de ligne `\n`.
pub fn to_chat_jsonl(examples: &[Example]) -> String {
    let mut text = String::new();
    for e in examples {
        text.push_str(&to_chat_line(e));
        text.push('\n');
    }
    text
}

/// Forme normalisée d'une phrase pour comparer deux jeux : minuscules, sans accents ni
/// ponctuation, espaces réduits.
pub fn normalize(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        let mapped = match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => "a",
            'ç' => "c",
            'é' | 'è' | 'ê' | 'ë' => "e",
            'î' | 'ï' | 'í' | 'ì' => "i",
            'ô' | 'ö' | 'ó' | 'ò' => "o",
            'ù' | 'û' | 'ü' | 'ú' => "u",
            'ÿ' => "y",
            'œ' => "oe",
            'æ' => "ae",
            c if c.is_alphanumeric() => {
                out.push(c);
                continue;
            }
            _ => " ",
        };
        out.push_str(mapped);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn the_same_seed_gives_the_same_file_and_another_seed_another() {
        let (a, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let (b, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        assert_eq!(to_jsonl(&a), to_jsonl(&b));
        let (c, _) = generate(DEFAULT_SEED + 1, DEFAULT_COUNT);
        assert_ne!(to_jsonl(&a), to_jsonl(&c));
    }

    #[test]
    fn every_written_example_passes_check_plan_again() {
        let registry = Registry::standard();
        let (examples, summary) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        assert_eq!(examples.len(), DEFAULT_COUNT, "{:?}", summary.families);
        for line in to_jsonl(&examples).lines() {
            let record: Value = serde_json::from_str(line).unwrap();
            let plan: Plan = serde_json::from_value(record["expected"].clone()).unwrap();
            assert_eq!(registry.check_plan(&plan), Ok(()), "{line}");
            assert!(record["request"].as_str().is_some_and(|r| !r.is_empty()));
            assert!(record["inventory"]["layers"].is_array());
        }
    }

    #[test]
    fn an_invalid_plan_is_never_written() {
        let registry = Registry::standard();
        let inventory = json!({"layers": [{"layer_id": "gares_1"}], "unloaded_files": []});
        let make = |expected: Value| Example {
            family: "essai",
            inventory: inventory.clone(),
            request: "Tampon de 300 m autour des gares.".into(),
            expected,
            retry: None,
            listed: None,
            focus: Vec::new(),
        };
        // Référence vers une étape qui n'existe pas : refusée par `check_plan`.
        let bad = make(json!({"kind": "calls", "calls": [
            {"command": "analysis.buffer", "params": {"layer_id": "step:3.layer_id",
                "distance": {"value": 300, "unit": "m"}}}]}));
        assert!(validate(&registry, &bad)
            .unwrap_err()
            .contains("plan.bad_reference"));
        // Couche absente de l'inventaire, commande inconnue, refus en forme de question.
        let ghost = make(json!({"kind": "calls", "calls": [
            {"command": "analysis.buffer", "params": {"layer_id": "routes_9",
                "distance": {"value": 300, "unit": "m"}}}]}));
        assert!(validate(&registry, &ghost)
            .unwrap_err()
            .contains("routes_9"));
        let unknown =
            make(json!({"kind": "calls", "calls": [{"command": "no.such", "params": {}}]}));
        assert!(validate(&registry, &unknown).is_err());
        let asking = make(json!({"kind": "refusal", "reason": "Quelle couche ?"}));
        assert!(validate(&registry, &asking).is_err());
        let good = make(json!({"kind": "calls", "calls": [
            {"command": "analysis.buffer", "params": {"layer_id": "gares_1",
                "distance": {"value": 300, "unit": "m"}}}]}));
        assert_eq!(validate(&registry, &good), Ok(()));
    }

    #[test]
    fn a_dem_path_must_be_the_source_of_a_raster_of_the_inventory() {
        let registry = Registry::standard();
        let inventory = json!({"layers": [
            {"layer_id": "mnt_1", "kind": "raster", "source": "donnees/mnt_5m.tif"},
            {"layer_id": "gares_1", "kind": "vector", "source": "donnees/gares.gpkg"},
            {"layer_id": "mnt_tmp", "kind": "raster"}
        ], "unloaded_files": []});
        let slope = |dem: Value| Example {
            family: "essai",
            inventory: inventory.clone(),
            request: "Carte des pentes du MNT en degrés.".into(),
            expected: json!({"kind": "calls", "calls": [
                {"command": "terrain.slope", "params": {"dem": dem, "unit": "degrees"}}]}),
            retry: None,
            listed: None,
            focus: Vec::new(),
        };
        assert_eq!(
            validate(&registry, &slope(json!("donnees/mnt_5m.tif"))),
            Ok(())
        );
        // Un chemin inventé, l'id de la couche, la source d'un vecteur : écartés.
        for bad in ["donnees/mnt.tif", "mnt_1", "donnees/gares.gpkg"] {
            let err = validate(&registry, &slope(json!(bad))).unwrap_err();
            assert!(err.contains("raster absent"), "{bad} : {err}");
        }
    }

    #[test]
    fn a_raster_input_is_a_raster_of_the_inventory_or_a_previous_step() {
        // 11.12, lot 2a : `raster.reclassify` lit le résultat d'une étape (`step:1`) ; un
        // chemin inventé reste écarté.
        let registry = Registry::standard();
        let inventory = json!({"layers": [
            {"layer_id": "mnt_1", "kind": "raster", "source": "donnees/mnt_5m.tif"}
        ], "unloaded_files": []});
        let threshold = |raster: Value| Example {
            family: "essai",
            inventory: inventory.clone(),
            request: "Où la pente du MNT dépasse-t-elle 15 % ?".into(),
            expected: json!({"kind": "calls", "calls": [
                {"command": "terrain.slope",
                    "params": {"dem": "donnees/mnt_5m.tif", "unit": "percent"}},
                {"command": "raster.reclassify",
                    "params": {"raster": raster, "classes": [{"min": 15, "value": 1}]}}]}),
            retry: None,
            listed: None,
            focus: Vec::new(),
        };
        assert_eq!(validate(&registry, &threshold(json!("step:1"))), Ok(()));
        assert_eq!(
            validate(&registry, &threshold(json!("donnees/mnt_5m.tif"))),
            Ok(())
        );
        let err = validate(&registry, &threshold(json!("donnees/pente.tif"))).unwrap_err();
        assert!(err.contains("raster absent"), "{err}");
    }

    #[test]
    fn a_cost_raster_is_the_source_of_a_raster_of_the_inventory() {
        // 11.12, lot 2b-1 : `cost` se contrôle comme `dem` ; un chemin inventé, l'id de la
        // couche ou la source d'un vecteur sont écartés.
        let registry = Registry::standard();
        let inventory = json!({"layers": [
            {"layer_id": "friction_1", "kind": "raster", "source": "donnees/friction.tif"},
            {"layer_id": "routes_1", "kind": "vector", "source": "donnees/routes.gpkg"}
        ], "unloaded_files": []});
        let path = |cost: Value| Example {
            family: "essai",
            inventory: inventory.clone(),
            request: "Trace le chemin de moindre coût du point (652 300 ; 6 862 100) en \
                Lambert-93 au point (653 100 ; 6 861 500) en Lambert-93."
                .into(),
            expected: json!({"kind": "calls", "calls": [
                {"command": "terrain.least_cost_path", "params": {"cost": cost,
                    "start": {"x": 652300, "y": 6862100, "epsg": 2154},
                    "end": {"x": 653100, "y": 6861500, "epsg": 2154}}}]}),
            retry: None,
            listed: None,
            focus: Vec::new(),
        };
        assert_eq!(
            validate(&registry, &path(json!("donnees/friction.tif"))),
            Ok(())
        );
        for bad in ["donnees/cout.tif", "friction_1", "donnees/routes.gpkg"] {
            let err = validate(&registry, &path(json!(bad))).unwrap_err();
            assert!(err.contains("raster absent"), "{bad} : {err}");
        }
    }

    #[test]
    fn listed_rasters_of_an_overlay_are_sources_of_rasters_of_the_inventory() {
        // 11.12, lot 2b-2 : `criteria[i].raster` et `exclusions[i]` se contrôlent comme `dem` ;
        // un chemin inventé, l'id d'une couche ou une étape en liste sont écartés.
        let registry = Registry::standard();
        let inventory = json!({"layers": [
            {"layer_id": "calme_1", "kind": "raster", "source": "donnees/calme.tif"},
            {"layer_id": "accessibilite_1", "kind": "raster", "source": "rasters/accessibilite.tif"},
            {"layer_id": "zi_masque_1", "kind": "raster", "source": "zi_masque.tif"}
        ], "unloaded_files": []});
        let overlay = |second: Value, mask: Value| Example {
            family: "essai",
            inventory: inventory.clone(),
            request: "Combine le calme et l'accessibilité, en excluant les zones inondables."
                .into(),
            expected: json!({"kind": "calls", "calls": [
                {"command": "raster.reclassify", "params": {"raster": "donnees/calme.tif",
                    "classes": [{"min": 2, "value": 1}]}},
                {"command": "raster.weighted_overlay", "params": {
                    "criteria": [{"raster": "donnees/calme.tif", "weight": 1},
                        {"raster": second, "weight": 1}],
                    "exclusions": [mask]}}]}),
            retry: None,
            listed: None,
            focus: Vec::new(),
        };
        let good = (json!("rasters/accessibilite.tif"), json!("zi_masque.tif"));
        assert_eq!(
            validate(&registry, &overlay(good.0.clone(), good.1.clone())),
            Ok(())
        );
        for bad in ["donnees/ensoleillement.tif", "accessibilite_1", "step:1"] {
            // `step:k` dans une liste est refusé dès la vérification du plan (entrée 11.13).
            let expected = if bad.starts_with("step:") {
                "plan.bad_reference"
            } else {
                "raster absent"
            };
            let err = validate(&registry, &overlay(json!(bad), good.1.clone())).unwrap_err();
            assert!(err.contains(expected), "critère {bad} : {err}");
            let err = validate(&registry, &overlay(good.0.clone(), json!(bad))).unwrap_err();
            assert!(err.contains(expected), "masque {bad} : {err}");
        }
    }

    #[test]
    fn every_family_gives_at_least_one_valid_example() {
        let registry = Registry::standard();
        for family in FAMILIES {
            let mut rng = Rng::new(11);
            let valid = (0..50)
                .filter_map(|_| (family.make)(&mut rng))
                .filter(|e| validate(&registry, e).is_ok())
                .count();
            assert!(valid > 0, "{} ne rend aucun exemple valide", family.name);
        }
    }

    #[test]
    fn the_default_set_holds_a_quarter_of_big_projects_and_close_layers() {
        // 5.47 : 23 à 27 % de gros projets de 20 à 60 couches, dont au moins 70 % vus triés ;
        // aucun prompt au-delà de la borne ; au moins 100 couches proches, un tiers environ en plan.
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let big: Vec<&Example> = examples
            .iter()
            .filter(|e| e.family == "gros_projet")
            .collect();
        assert!(
            big.len() * 100 >= examples.len() * 23 && big.len() * 100 <= examples.len() * 27,
            "{} gros projets",
            big.len()
        );
        let mut sorted = 0;
        for e in &big {
            let layers = match e.inventory.get("index") {
                Some(index) => {
                    sorted += 1;
                    index.as_array().unwrap().len()
                }
                None => e.inventory["layers"].as_array().unwrap().len(),
            };
            assert!((20..=60).contains(&layers), "{layers} couches");
        }
        assert!(sorted * 10 >= big.len() * 7, "{sorted} / {}", big.len());
        assert!(examples
            .iter()
            .all(|e| prompt_text(e).chars().count() <= MAX_PROMPT_CHARS));
        let close: Vec<&Example> = examples
            .iter()
            .filter(|e| e.family == "couches_proches")
            .collect();
        let planned = close
            .iter()
            .filter(|e| e.expected["kind"] == "calls")
            .count();
        assert!(close.len() >= 100, "{}", close.len());
        assert!(
            planned * 5 >= close.len() && planned * 2 <= close.len(),
            "{planned} / {}",
            close.len()
        );
    }

    #[test]
    fn no_family_produces_rejected_examples() {
        // Un écart signale une famille mal écrite : le filtre est un garde-fou, pas un tamis.
        // Seul `gros_projet` écarte, par construction (5.47) : une couche utile que la vue triée
        // ne détaille pas, un prompt au-delà de la borne, ou (5.54, séance 3) un champ à valeurs
        // cité par une question de valeur absente que la vue triée ne montre plus en entier.
        let (_, summary) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let rejected: usize = summary
            .families
            .iter()
            .filter(|(name, _)| **name != "gros_projet")
            .map(|(_, t)| t.rejected)
            .sum();
        assert_eq!(rejected, 0, "{:#?}", summary.reasons);
        for reason in &summary.reasons {
            assert!(
                reason.starts_with("gros_projet : couche utile")
                    || reason.starts_with("gros_projet : prompt trop long")
                    || (reason.starts_with("gros_projet : couche « ")
                        && reason.contains("champ à valeurs"))
                    || (reason.starts_with("gros_projet : étape")
                        && reason.contains("raster absent")),
                "{reason}"
            );
        }
    }

    #[test]
    fn the_utm_isochrones_of_the_default_set_pass_the_guard() {
        // 5.37 : au jeu par défaut, la famille est là et le garde-fou laisse ses plans.
        let registry = Registry::standard();
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let utm: Vec<_> = examples
            .iter()
            .filter(|e| e.family == "isochrone_utm")
            .collect();
        assert!(!utm.is_empty());
        for e in utm {
            let plan: cassini_core::plan::Plan =
                serde_json::from_value(e.expected.clone()).unwrap();
            let guarded =
                cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
            assert_eq!(guarded, plan, "{}", e.request);
        }
    }

    #[test]
    fn the_geometry_families_of_the_default_set_cover_every_command_and_pass_the_guard() {
        // 11.12, lot 1 : au jeu par défaut, chacune des huit familles (dont `gisement`, 10.28, et
        // `azimut_magnetique`, 10.29) est là, chacune des cinq
        // commandes de géométrie figure dans un plan, chaque plan passe `check_plan` puis le
        // garde-fou inchangé, et aucun refus ne dit qu'aucune commande ne reprojette un point.
        const GEOMETRY: [&str; 8] = [
            "conversion_coordonnees",
            "reprojection_point",
            "distance_azimut",
            "point_projete",
            "gisement",
            "azimut_magnetique",
            "recoupement",
            "terrain_autre_systeme",
        ];
        let registry = Registry::standard();
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let mut planned = HashSet::new();
        for name in GEOMETRY {
            let family: Vec<_> = examples.iter().filter(|e| e.family == name).collect();
            assert!(!family.is_empty(), "famille absente : {name}");
            for e in family {
                let plan: Plan = serde_json::from_value(e.expected.clone()).unwrap();
                assert_eq!(registry.check_plan(&plan), Ok(()), "{}", e.request);
                let guarded =
                    cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
                assert_eq!(guarded, plan, "{}", e.request);
            }
        }
        for e in &examples {
            assert!(!families::reprojects_no_point(&e.expected), "{}", e.request);
            for c in e.expected["calls"].as_array().into_iter().flatten() {
                planned.insert(c["command"].as_str().unwrap_or_default().to_string());
            }
        }
        for command in [
            "coord.convert",
            "data.transform_point",
            "geo.distance_azimuth",
            "geo.destination_point",
            "geo.bearing_intersection",
        ] {
            assert!(planned.contains(command), "{command}");
        }
    }

    #[test]
    fn the_dem_families_of_the_default_set_cover_every_command_and_refuse_no_threshold() {
        // 11.12, lot 2a : au jeu par défaut, chacune des cinq familles est là, sans rejet, chacune
        // des cinq commandes figure dans un plan, chaque plan passe le garde-fou inchangé, et
        // aucun refus ne dit qu'aucune commande ne sélectionne les cellules d'un raster.
        const DEM: [&str; 5] = [
            "exposition",
            "ombrage",
            "champ_de_vision_cumule",
            "points_hauts",
            "pente_seuil",
        ];
        let registry = Registry::standard();
        let (examples, summary) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        for name in DEM {
            let tally = &summary.families[name];
            assert!(tally.kept > 0 && tally.rejected == 0, "{name} : {tally:?}");
            for e in examples.iter().filter(|e| e.family == name) {
                let plan: Plan = serde_json::from_value(e.expected.clone()).unwrap();
                let guarded =
                    cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
                assert_eq!(guarded, plan, "{}", e.request);
            }
        }
        let mut planned = HashSet::new();
        for e in &examples {
            assert!(!families::selects_no_cell(&e.expected), "{}", e.request);
            for c in e.expected["calls"].as_array().into_iter().flatten() {
                planned.insert(c["command"].as_str().unwrap_or_default().to_string());
            }
        }
        for command in [
            "terrain.aspect",
            "terrain.hillshade",
            "terrain.viewshed_cumulative",
            "terrain.peaks",
            "raster.reclassify",
        ] {
            assert!(planned.contains(command), "{command}");
        }
    }

    #[test]
    fn the_cost_families_of_the_default_set_cite_a_cost_raster_never_the_dem() {
        // 11.12, lot 2b-1 : au jeu par défaut, les deux familles sont là, sans rejet, chaque
        // commande de coût figure dans un plan, et tout `cost` est la `source` d'un raster de
        // coût de l'inventaire, jamais celle d'un MNT ; aucune source n'est une étape.
        let cost_slugs = inventories::theme("cout").slugs;
        let (examples, summary) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        for name in ["cout_cumule", "chemin_moindre_cout"] {
            let tally = &summary.families[name];
            assert!(tally.kept > 0 && tally.rejected == 0, "{name} : {tally:?}");
        }
        let mut planned = HashSet::new();
        for e in &examples {
            for c in e.expected["calls"].as_array().into_iter().flatten() {
                let Some(cost) = c["params"].get("cost") else {
                    continue;
                };
                planned.insert(c["command"].as_str().unwrap_or_default().to_string());
                let layer = e.inventory["layers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|l| l["kind"] == "raster" && l.get("source") == Some(cost))
                    .unwrap_or_else(|| panic!("coût absent : {}", e.request));
                let id = layer["layer_id"].as_str().unwrap();
                assert!(
                    cost_slugs.iter().any(|s| id.starts_with(&format!("{s}_"))),
                    "{id} : {}",
                    e.request
                );
                for s in c["params"]["sources"].as_array().into_iter().flatten() {
                    assert!(!s.to_string().contains("step:"), "{}", e.request);
                }
            }
        }
        for command in ["terrain.cost_distance", "terrain.least_cost_path"] {
            assert!(planned.contains(command), "{command}");
        }
    }

    #[test]
    fn the_overlay_families_of_the_default_set_cite_criteria_and_masks_never_the_dem() {
        // 11.12, lot 2b-2 : au jeu par défaut, les deux familles sont là, sans rejet ;
        // `raster.weighted_overlay` figure dans un plan, `raster.reclassify` dans un plan sans
        // `terrain.slope` ; aucun critère ni masque n'est le MNT, un raster de coût, ni absent.
        let (examples, summary) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        for name in ["superposition_ponderee", "reclassement_raster"] {
            let tally = &summary.families[name];
            assert!(tally.kept > 0 && tally.rejected == 0, "{name} : {tally:?}");
        }
        let forbidden: Vec<&str> = ["mnt", "cout"]
            .iter()
            .flat_map(|k| inventories::theme(k).slugs.iter().copied())
            .collect();
        let (mut overlay, mut direct) = (false, false);
        for e in &examples {
            let calls = e.expected["calls"].as_array().into_iter().flatten();
            let commands: Vec<&str> = calls
                .clone()
                .filter_map(|c| c["command"].as_str())
                .collect();
            direct |=
                commands.contains(&"raster.reclassify") && !commands.contains(&"terrain.slope");
            for c in calls.filter(|c| c["command"] == "raster.weighted_overlay") {
                overlay = true;
                let p = &c["params"];
                let criteria = p["criteria"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| &c["raster"]);
                for raster in criteria.chain(p["exclusions"].as_array().into_iter().flatten()) {
                    let layer = e.inventory["layers"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|l| l["kind"] == "raster" && l.get("source") == Some(raster))
                        .unwrap_or_else(|| panic!("raster absent : {}", e.request));
                    let id = layer["layer_id"].as_str().unwrap();
                    assert!(
                        !forbidden.iter().any(|s| id.starts_with(&format!("{s}_"))),
                        "{id} : {}",
                        e.request
                    );
                }
            }
        }
        assert!(overlay && direct);
    }

    #[test]
    fn the_exports_at_a_scale_of_the_default_set_pass_the_guard() {
        // 5.38 : au jeu par défaut, la famille est là, ses plans passent `check_plan` et le
        // garde-fou les laisse.
        let registry = Registry::standard();
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let scaled: Vec<_> = examples
            .iter()
            .filter(|e| e.family == "export_pdf_scale")
            .collect();
        assert!(!scaled.is_empty());
        for e in scaled {
            let plan: cassini_core::plan::Plan =
                serde_json::from_value(e.expected.clone()).unwrap();
            assert_eq!(registry.check_plan(&plan), Ok(()), "{}", e.request);
            let guarded =
                cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
            assert_eq!(guarded, plan, "{}", e.request);
        }
    }

    #[test]
    fn exports_at_a_scale_without_the_word_pdf_are_one_export_at_that_scale() {
        // 5.40 : au jeu par défaut, des exports à l'échelle ne disent pas « PDF » ; chacun
        // passe `check_plan` et porte un seul `layout.export_pdf`, à `scale`.
        let registry = Registry::standard();
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let bare: Vec<_> = examples
            .iter()
            .filter(|e| e.family == "export_pdf_scale" && !e.request.to_lowercase().contains("pdf"))
            .collect();
        assert!(!bare.is_empty());
        // La carte seule (un appel) et au bout d'une recherche (plusieurs).
        let lengths = |e: &&Example| e.expected["calls"].as_array().unwrap().len();
        assert!(bare.iter().any(|e| lengths(e) == 1));
        assert!(bare.iter().any(|e| lengths(e) > 1));
        for e in bare {
            let plan: cassini_core::plan::Plan =
                serde_json::from_value(e.expected.clone()).unwrap();
            assert_eq!(registry.check_plan(&plan), Ok(()), "{}", e.request);
            let exports: Vec<&Value> = e.expected["calls"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["command"] == "layout.export_pdf")
                .collect();
            assert_eq!(exports.len(), 1, "{}", e.request);
            assert!(
                exports[0]["params"]["scale"].as_u64().is_some(),
                "{}",
                e.request
            );
        }
    }

    #[test]
    fn the_default_set_teaches_styles_points_as_written_and_the_project_utm_zone() {
        // 11.23 : au jeu par défaut, les deux commandes de style sont planifiées et aucun refus
        // ne dit qu'aucune commande ne change le style ; aucune question ne demande de
        // reprojeter une liste de points, et un point de terrain d'un autre système est recopié
        // tel qu'écrit ; le carroyage UTM au fuseau du projet a au moins 60 exemples, recopies
        // et questions comprises.
        let registry = Registry::standard();
        let (examples, summary) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let mut planned = HashSet::new();
        let mut copied_points = 0;
        for e in &examples {
            assert!(!families::styles_no_layer(&e.expected), "{}", e.request);
            if let Some(q) = e.expected["question"].as_str() {
                assert!(
                    !q.contains("reprojet") && !q.contains("pas dans le système"),
                    "{q}"
                );
            }
            for c in e.expected["calls"].as_array().into_iter().flatten() {
                planned.insert(c["command"].as_str().unwrap_or_default().to_string());
            }
            if e.family == "terrain_autre_systeme"
                && e.expected["calls"].as_array().is_some_and(|c| c.len() == 1)
            {
                let plan: Plan = serde_json::from_value(e.expected.clone()).unwrap();
                let guarded =
                    cassini_core::point::guard_coordinates(&registry, plan.clone(), &e.request);
                assert_eq!(guarded, plan, "{}", e.request);
                copied_points += 1;
            }
        }
        assert!(planned.contains("map.topographic_style"));
        assert!(planned.contains("map.graduated_style"));
        assert!(copied_points > 0);
        let grid = &summary.families["carroyage_utm_projet"];
        assert!(grid.kept >= 60 && grid.rejected == 0, "{grid:?}");
        let grids: Vec<&Example> = examples
            .iter()
            .filter(|e| e.family == "carroyage_utm_projet")
            .collect();
        assert!(grids.iter().any(|e| e.expected["kind"] == "calls"));
        assert!(grids.iter().any(|e| e.expected["kind"] == "question"));
        assert!(grids.iter().all(|e| e.expected["kind"] != "refusal"));
    }

    #[test]
    fn the_default_set_never_reprojects_a_metric_layer_unasked_and_asks_for_a_missing_utm_zone() {
        // Second essai de 11.23 (constats S07 et F03) : au jeu par défaut, un `data.reproject`
        // ne vise une couche en mètres (ou de système inconnu) que si la phrase écrit le
        // système d'arrivée (`reprojection_couche`) ; les analyses en mètres sur une couche en
        // mètres, à côté d'une couche en degrés, sont nombreuses ; un carroyage « UTM » sans
        // fuseau écrit pose la question quand l'inventaire n'a pas de `utm_epsg`, jamais un
        // fuseau deviné, et ces questions font au moins la moitié des recopies.
        let (examples, summary) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let contrasts = &summary.families["analyse_couche_metrique"];
        assert!(contrasts.kept >= 60, "{contrasts:?}");
        let (mut unasked_degrees, mut copies, mut questions) = (0, 0, 0);
        for e in &examples {
            let layers = e.inventory["layers"].as_array().unwrap();
            let calls = e.expected["calls"].as_array().cloned().unwrap_or_default();
            for c in calls.iter().filter(|c| c["command"] == "data.reproject") {
                let id = c["params"]["layer_id"].as_str().unwrap();
                if id.starts_with("step:") {
                    continue;
                }
                let layer = layers.iter().find(|l| l["layer_id"] == id).unwrap();
                if layer["epsg"] == 4326 {
                    unasked_degrees += 1;
                    continue;
                }
                let target = c["params"]["target_epsg"].as_u64().unwrap();
                let (_, forms) = families::LAYER_TARGETS
                    .iter()
                    .find(|(t, _)| u64::from(*t) == target)
                    .unwrap_or_else(|| panic!("cible {target} : {}", e.request));
                assert!(
                    forms.iter().any(|f| e.request.contains(f)),
                    "couche en mètres reprojetée sans demande : {} ({})",
                    e.request,
                    e.family
                );
            }
            let r = e.request.to_lowercase();
            let zone_written = ["30", "31", "32"].iter().any(|z| {
                r.contains(&format!("utm {z}"))
                    || r.contains(&format!("utm{z}"))
                    || r.contains(&format!("zone {z}"))
            });
            if !r.contains("utm") || zone_written {
                continue;
            }
            let asks = e.expected["question"] == families::GRID_SYSTEM_QUESTION;
            let grid = calls
                .iter()
                .find(|c| c["command"] == "layout.export_pdf")
                .and_then(|c| c["params"].get("grid"));
            let utm = e.inventory["project"]["utm_epsg"].as_u64();
            match (utm, grid) {
                (None, Some(g)) => panic!("fuseau deviné {g} : {}", e.request),
                (None, None) if asks => questions += 1,
                (Some(code), Some(g)) if g["kind"] == "projected" => {
                    assert_eq!(g["epsg"].as_u64(), Some(code), "{}", e.request);
                    copies += 1;
                }
                _ => {}
            }
        }
        assert!(unasked_degrees > 0);
        assert!(copies > 0, "{copies}");
        assert!(
            2 * questions >= copies,
            "{questions} questions, {copies} recopies"
        );
    }

    #[test]
    fn paper_and_orientation_are_plans_never_refusals() {
        // 11.12, lot 3 (inverse le test de 5.40) : au jeu par défaut, aucun refus ne cite
        // « A4 », `export_pdf_format` a des plans pour A3, A0, portrait et chacune des deux
        // formes de grille, et aucun export à l'échelle ne nomme un papier ni une orientation.
        let registry = Registry::standard();
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        for e in &examples {
            if let Some(reason) = e.expected["reason"].as_str() {
                assert!(!reason.contains("A4"), "{} → {reason}", e.request);
            }
        }
        let exports: Vec<&Value> = examples
            .iter()
            .filter(|e| e.family == "export_pdf_format" && e.expected["kind"] == "calls")
            .inspect(|e| {
                let plan: cassini_core::plan::Plan =
                    serde_json::from_value(e.expected.clone()).unwrap();
                assert_eq!(registry.check_plan(&plan), Ok(()), "{}", e.request);
            })
            .filter_map(|e| e.expected["calls"].as_array()?.last())
            .map(|c| &c["params"])
            .collect();
        for size in ["a3", "a0"] {
            assert!(exports.iter().any(|p| p["page_size"] == size), "{size}");
        }
        assert!(exports.iter().any(|p| p["orientation"] == "portrait"));
        for kind in ["degrees", "projected"] {
            assert!(exports.iter().any(|p| p["grid"]["kind"] == kind), "{kind}");
        }
        assert!(exports.iter().all(|p| p.get("scale").is_none()));
        for e in examples.iter().filter(|e| e.family == "export_pdf_scale") {
            for paper in ["A3", "A0", "portrait"] {
                assert!(!e.request.contains(paper), "{}", e.request);
            }
        }
    }

    #[test]
    fn the_decided_families_and_vocabulary_are_present() {
        let (examples, summary) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        assert_eq!(examples.len(), DEFAULT_COUNT);
        for (name, tally) in &summary.families {
            assert!(tally.kept > 0, "famille absente : {name}");
        }
        let requests: Vec<String> = examples.iter().map(|e| e.request.to_lowercase()).collect();
        for needle in [
            "constructible",
            "garde seulement",
            "ne garde que",
            "conserve",
            "à moins de",
            "pdf",
        ] {
            assert!(requests.iter().any(|r| r.contains(needle)), "{needle}");
        }
        // Environ 5 % de redemandes (au moins 4 %, jamais plus de 7 %).
        let retries = examples.iter().filter(|e| e.retry.is_some()).count();
        assert!(
            retries * 25 >= examples.len() && retries * 100 <= examples.len() * 7,
            "{retries} redemandes"
        );
        let kinds = |k: &str| examples.iter().filter(|e| e.expected["kind"] == k).count();
        assert!(kinds("question") > 0 && kinds("refusal") > 0 && kinds("calls") > 0);
        // Chaînes à plusieurs conditions : au moins deux intersections successives.
        assert!(examples.iter().any(|e| {
            e.expected["calls"].as_array().is_some_and(|c| {
                c.iter()
                    .filter(|s| s["command"] == "analysis.intersect")
                    .count()
                    >= 2
            })
        }));
    }

    #[test]
    fn a_commune_named_only_as_context_adds_no_step() {
        // « … à Orly » : sans tournure d'appartenance, la commune ne crée aucune sélection.
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let mut seen = 0;
        for e in &examples {
            let r = e.request.to_lowercase();
            if r.contains("appartien") || r.contains("territoire") {
                continue;
            }
            let Some(calls) = e.expected["calls"].as_array() else {
                continue;
            };
            for commune in families::COMMUNES {
                if !e.request.contains(&format!(" à {commune}")) {
                    continue;
                }
                seen += 1;
                assert!(
                    calls.iter().all(|c| c["params"]["value"] != *commune),
                    "{}",
                    e.request
                );
            }
        }
        assert!(seen > 100, "{seen} demandes avec une commune de contexte");
    }

    #[test]
    fn the_chat_line_of_a_retry_example_carries_the_retry_prompt() {
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let mut retries = 0;
        for e in &examples {
            let line: Value = serde_json::from_str(&to_chat_line(e)).unwrap();
            let want = match &e.retry {
                Some(r) => {
                    retries += 1;
                    prompt::render_retry_prompt(&e.inventory, &e.request, &r.rejected, &r.errors)
                }
                None => prompt::render_prompt(&e.inventory, &e.request),
            };
            assert_eq!(line["prompt"], want, "{}", e.request);
        }
        assert!(retries > 0);
    }

    /// Une distance écrite dans la demande : un nombre suivi de « m » ou « km » (espace
    /// facultative, pas de lettre après), ou les mots entiers « mètre(s) », « kilomètre(s) ».
    /// Une durée (« 10 min ») ou « périmètre » n'en sont pas.
    fn has_distance(request: &str) -> bool {
        let lower = request.to_lowercase();
        let words = lower
            .split(|c: char| !c.is_alphabetic())
            .any(|w| matches!(w, "mètre" | "mètres" | "kilomètre" | "kilomètres"));
        let chars: Vec<char> = lower.chars().collect();
        let unit = (0..chars.len()).any(|i| {
            if !chars[i].is_ascii_digit() {
                return false;
            }
            let mut j = i + 1;
            if chars.get(j) == Some(&' ') {
                j += 1;
            }
            if chars.get(j) == Some(&'k') {
                j += 1;
            }
            chars.get(j) == Some(&'m') && !chars.get(j + 1).is_some_and(|c| c.is_alphabetic())
        });
        words || unit
    }

    #[test]
    fn a_travel_without_coordinates_never_becomes_a_plan() {
        // Une durée de trajet depuis un lieu sans coordonnées mène à une question (un refus
        // depuis plusieurs lieux, décision a) ; une question de coordonnées ne porte jamais sur
        // une distance en mètres, qui reste un plan.
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let modes = [
            "à pied",
            "en marchant",
            "à la marche",
            "en voiture",
            "en auto",
            "en véhicule",
        ];
        let mut travels = 0;
        for e in &examples {
            let r = e.request.to_lowercase();
            let travel = modes.iter().any(|m| r.contains(m));
            let coordinates = ["lambert", "epsg", "wgs", "utm"]
                .iter()
                .any(|c| r.contains(c));
            if travel && !coordinates {
                travels += 1;
                let refused = e.expected["reason"] == families::NO_MULTI_ORIGIN;
                assert!(
                    e.expected["kind"] == "question" || refused,
                    "{} → {}",
                    e.request,
                    e.expected
                );
            }
            let asks_point = e.expected["question"]
                .as_str()
                .is_some_and(|q| q.contains("coordonnées"));
            // Depuis 4.6, une vue depuis un lieu nommé demande aussi son point (`question_terrain`),
            // et depuis 11.12, lot 2b-1, un coût de déplacement depuis un lieu nommé.
            if asks_point {
                // Un gros projet (5.47) emballe aussi ces familles-là.
                let terrain = matches!(
                    e.family,
                    "question_terrain" | "cout_cumule" | "chemin_moindre_cout" | "gros_projet"
                );
                assert!(
                    (travel || terrain) && !has_distance(&e.request),
                    "{}",
                    e.request
                );
            }
        }
        assert!(travels > 50, "{travels}");
        assert!(has_distance("à moins de 500 m d'une gare"));
        assert!(has_distance("à 300m"));
        assert!(has_distance("à moins de 2 km"));
        assert!(has_distance("à deux kilomètres"));
        assert!(!has_distance("à 10 min à pied"));
        assert!(!has_distance("dans le périmètre"));
    }

    #[test]
    fn the_isochrone_still_starts_from_a_single_point() {
        // Le refus des origines multiples tient tant que l'origine est un seul point.
        let registry = Registry::standard();
        let spec = registry.spec("analysis.isochrone").expect("isochrone");
        let schema = &spec.params_schema;
        let mut origin = &schema["properties"]["origin"];
        // `{"$ref": "#/$defs/Point"}` : la forme est dans les définitions du schéma.
        if let Some(name) = origin["$ref"]
            .as_str()
            .and_then(|r| r.strip_prefix("#/$defs/"))
        {
            origin = &schema["$defs"][name];
        }
        assert!(origin.is_object(), "{schema}");
        assert_ne!(origin["type"], "array", "{origin}");
        for key in ["x", "y", "epsg"] {
            assert!(origin["properties"][key].is_object(), "{key} : {origin}");
        }
    }

    #[test]
    fn no_plan_of_the_generated_set_passes_six_steps() {
        let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        for e in &examples {
            let n = e.expected["calls"].as_array().map_or(0, Vec::len);
            assert!(n <= 6, "{n} étapes : {}", e.request);
        }
    }

    #[test]
    fn a_focused_set_draws_only_the_named_families_and_the_full_set_is_unchanged() {
        // 5.54 : le jeu ciblé vient du même générateur, restreint aux familles nommées.
        let names = ["tampon", "refus"];
        let (examples, summary) = generate_families(FOCUS_SEED, 200, &names).unwrap();
        assert_eq!(examples.len(), 200, "{:?}", summary.families);
        assert!(examples.iter().all(|e| names.contains(&e.family)));
        assert!(names
            .iter()
            .all(|n| examples.iter().any(|e| e.family == *n)));
        assert_eq!(
            summary.families.keys().copied().collect::<Vec<_>>(),
            vec!["refus", "tampon"]
        );
        // Même graine, même suite ; l'ordre de la liste donnée ne compte pas.
        let (again, _) = generate_families(FOCUS_SEED, 200, &["refus", "tampon"]).unwrap();
        assert_eq!(to_jsonl(&examples), to_jsonl(&again));
        // Toutes les familles nommées : le jeu complet, à l'octet près.
        let all: Vec<&str> = FAMILIES.iter().map(|f| f.name).collect();
        let (full, _) = generate_families_unchecked(DEFAULT_SEED, 500, &all).unwrap();
        assert_eq!(to_jsonl(&full), to_jsonl(&generate(DEFAULT_SEED, 500).0));
    }

    #[test]
    fn the_stratified_recall_is_a_subset_of_the_full_set_with_a_floor_per_family() {
        // 5.54, séance 2 : le rappel d'une séance continue est fait d'exemples du jeu graine 1
        // complet (que `recouvrement.rs` garde), au moins `RECALL_FLOOR` par famille (ou tous ceux
        // du jeu complet), le reste au poids ; le plancher vient d'abord, la marge coupée ne mord
        // que le surplus.
        let (full, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
        let lines: Vec<String> = full.iter().map(to_line).collect();
        let index: std::collections::HashMap<&str, usize> = lines
            .iter()
            .enumerate()
            .map(|(i, l)| (l.as_str(), i))
            .collect();
        let mut sizes: BTreeMap<&str, usize> = BTreeMap::new();
        for e in &full {
            *sizes.entry(e.family).or_default() += 1;
        }
        let recall = stratified_recall(1560, RECALL_FLOOR).unwrap();
        assert_eq!(recall.len(), 1560);
        // Chaque exemple est un exemple distinct du jeu complet.
        let positions: Vec<usize> = recall.iter().map(|e| index[to_line(e).as_str()]).collect();
        assert_eq!(positions.iter().collect::<HashSet<_>>().len(), 1560);
        let count = |examples: &[Example], family: &str| {
            examples.iter().filter(|e| e.family == family).count()
        };
        for (family, &n) in &sizes {
            let floor = n.min(RECALL_FLOOR);
            assert!(count(&recall, family) >= floor, "{family}");
            // Coupé aux 1 500 premiers (la marge de 60 de `pipeline.py`), le plancher reste.
            assert!(count(&recall[..1500], family) >= floor, "{family}");
        }
        // Les gros projets, sur les 1 500 réellement entraînés.
        let big = count(&recall[..1500], "gros_projet");
        assert!(big * 100 >= 1500 * 20, "{big} gros projets");
        // Le plancher puis le surplus, chacun dans l'ordre du jeu complet.
        let floor_len: usize = sizes.values().map(|&n| n.min(RECALL_FLOOR)).sum();
        assert!(positions[..floor_len].windows(2).all(|w| w[0] < w[1]));
        assert!(positions[floor_len..].windows(2).all(|w| w[0] < w[1]));
        // Déterministe, et le jeu complet entier à son compte.
        let again = stratified_recall(1560, RECALL_FLOOR).unwrap();
        assert_eq!(to_jsonl(&recall), to_jsonl(&again));
        let all = stratified_recall(DEFAULT_COUNT, RECALL_FLOOR).unwrap();
        let mut all_lines: Vec<String> = all.iter().map(to_line).collect();
        let mut sorted = lines.clone();
        all_lines.sort();
        sorted.sort();
        assert_eq!(all_lines, sorted);
        // Au-delà du jeu complet, ou un plancher qui n'y tient pas : refusé.
        assert!(stratified_recall(DEFAULT_COUNT + 1, RECALL_FLOOR).is_err());
        assert!(stratified_recall(100, RECALL_FLOOR).is_err());
        assert!(stratified_recall(1560, 0).is_err());
        // Le plancher rendu : `min(RECALL_FLOOR, n_f)` par famille.
        let (_, minimum) = stratified_recall_with_floor(1560, RECALL_FLOOR).unwrap();
        let expected: BTreeMap<&str, usize> = sizes
            .iter()
            .map(|(f, &n)| (*f, n.min(RECALL_FLOOR)))
            .collect();
        assert_eq!(minimum, expected);
    }

    #[test]
    fn the_pipeline_reads_the_family_names_and_constants_as_written_here() {
        // `pipeline.py` lit, avant le GPU, les noms (`Family {` puis `name: "…"` dans
        // `families.rs`), `FOCUS_EXCLUDED`, `FOCUS_SEED`, `FOCUS_COUNT` et `DEFAULT_COUNT` dans
        // le texte des sources : la forme qu'il cherche doit rendre exactement ces valeurs.
        let source = include_str!("families.rs");
        let read: Vec<&str> = source
            .split("Family {")
            .skip(1)
            .filter_map(|block| {
                // Les lignes de commentaire entre `Family {` et `name:` sont sautées.
                let line = block
                    .lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty() && !l.starts_with("//"))?;
                line.strip_prefix("name: \"")?.split('"').next()
            })
            .collect();
        let names: Vec<&str> = FAMILIES.iter().map(|f| f.name).collect();
        assert_eq!(read, names);
        let lib = include_str!("lib.rs");
        for line in [
            format!("pub const FOCUS_SEED: u64 = {FOCUS_SEED};"),
            format!("pub const FOCUS_COUNT: usize = {FOCUS_COUNT};"),
            format!("pub const DEFAULT_COUNT: usize = {DEFAULT_COUNT};"),
            format!("pub const RECALL_FLOOR: usize = {RECALL_FLOOR};"),
            format!("pub const RECALL_VERSION: usize = {RECALL_VERSION};"),
            format!("pub const FOCUS_EXCLUDED: &[&str] = &{}", "["),
        ] {
            assert_eq!(lib.matches(line.as_str()).count(), 1, "{line}");
        }
    }

    #[test]
    fn an_unknown_twice_named_or_empty_family_list_is_refused_with_the_known_names() {
        let err = generate_families(FOCUS_SEED, 10, &["tampon", "nexiste_pas"]).unwrap_err();
        assert!(err.contains("nexiste_pas"), "{err}");
        assert!(
            FAMILIES.iter().all(|f| err.contains(f.name)),
            "liste des noms connus : {err}"
        );
        let err = generate_families(FOCUS_SEED, 10, &["tampon", "tampon"]).unwrap_err();
        assert!(err.contains("deux fois"), "{err}");
        assert!(generate_families(FOCUS_SEED, 10, &[]).is_err());
        assert!(generate_families(FOCUS_SEED, 10, &[""]).is_err());
        // Une famille qui, tirée seule, engendre une phrase de barre est refusée au jeu ciblé
        // (`FOCUS_EXCLUDED`, vide depuis 5.54, lot 2 : le contrôle reste pour les familles futures).
        if let Some(excluded) = FOCUS_EXCLUDED.first() {
            let err = generate_families(FOCUS_SEED, 10, &["tampon", excluded]).unwrap_err();
            assert!(err.contains(excluded), "{err}");
            assert!(generate_families_unchecked(FOCUS_SEED, 10, &[excluded]).is_ok());
        }
        // La branche de refus, sur une liste d'exclusion donnée au test.
        let asked = select_unchecked(&HashSet::from(["tampon", "refus"]));
        let Err(err) = check_focus_against(asked.clone(), &["refus"]) else {
            panic!("refus attendu");
        };
        assert!(
            err.contains("refus") && err.contains("refusée au jeu ciblé"),
            "{err}"
        );
        let kept = check_focus_against(asked, &["pente"]).map(|f| f.len());
        assert_eq!(kept, Ok(2));
    }

    #[test]
    fn the_first_session_families_give_the_whole_focused_set_without_a_bar_sentence() {
        // 5.54, lot 2 : la liste de la première séance est acceptée par le générateur, rend
        // exactement `FOCUS_COUNT` exemples à `FOCUS_SEED`, tous de ses familles, chacune présente,
        // et ne reprend aucune phrase de barre.
        let (examples, summary) =
            generate_families(FOCUS_SEED, FOCUS_COUNT, FIRST_SESSION_FAMILIES).unwrap();
        assert_eq!(examples.len(), FOCUS_COUNT, "{:?}", summary.families);
        assert!(examples
            .iter()
            .all(|e| FIRST_SESSION_FAMILIES.contains(&e.family)));
        for name in FIRST_SESSION_FAMILIES {
            let tally = &summary.families[name];
            assert!(tally.kept > 0 && tally.rejected == 0, "{name} : {tally:?}");
        }
        assert_eq!(barres::copies(&examples), Vec::<String>::new());
        // Aucune n'est refusée au jeu ciblé.
        assert!(FIRST_SESSION_FAMILIES
            .iter()
            .all(|n| !FOCUS_EXCLUDED.contains(n)));
    }

    #[test]
    fn the_session_command_of_the_doc_names_the_first_session_families() {
        // La commande de la séance, recopiée dans la doc, nomme exactement `FIRST_SESSION_FAMILIES`.
        let doc = include_str!("../../docs/entrainement-du-modele.md");
        let line = format!("nuit-5-54.sh {}", FIRST_SESSION_FAMILIES.join(","));
        assert!(doc.contains(&format!("{line}\n")), "{line}");
    }

    #[test]
    fn the_second_session_families_give_the_whole_focused_set_without_a_bar_sentence() {
        // 5.54, séance 2 : la liste de la première séance plus la ligne de vue et le recoupement ;
        // `FOCUS_COUNT` exemples à `FOCUS_SEED`, chacune présente, aucune phrase de barre.
        let first: Vec<&str> = SECOND_SESSION_FAMILIES[..FIRST_SESSION_FAMILIES.len()].to_vec();
        assert_eq!(first, FIRST_SESSION_FAMILIES);
        assert_eq!(
            &SECOND_SESSION_FAMILIES[FIRST_SESSION_FAMILIES.len()..],
            &["ligne_de_vue", "recoupement"]
        );
        let (examples, summary) =
            generate_families(FOCUS_SEED, FOCUS_COUNT, SECOND_SESSION_FAMILIES).unwrap();
        assert_eq!(examples.len(), FOCUS_COUNT, "{:?}", summary.families);
        assert!(examples
            .iter()
            .all(|e| SECOND_SESSION_FAMILIES.contains(&e.family)));
        for name in SECOND_SESSION_FAMILIES {
            let tally = &summary.families[name];
            assert!(tally.kept > 0 && tally.rejected == 0, "{name} : {tally:?}");
        }
        assert_eq!(barres::copies(&examples), Vec::<String>::new());
        assert!(SECOND_SESSION_FAMILIES
            .iter()
            .all(|n| !FOCUS_EXCLUDED.contains(n)));
    }

    #[test]
    fn the_session_command_of_the_doc_names_the_second_session_families() {
        let doc = include_str!("../../docs/entrainement-du-modele.md");
        let line = format!("nuit-5-54.sh {}", SECOND_SESSION_FAMILIES.join(","));
        assert!(doc.contains(&format!("{line}\n")), "{line}");
    }

    #[test]
    fn the_third_session_families_give_the_whole_focused_set_without_a_bar_sentence() {
        // 5.54, séance 3 : la liste de la séance 2 plus les plans à plusieurs conditions et les
        // questions de champ ou de valeur absents ; `FOCUS_COUNT` exemples à `FOCUS_SEED`, chacune
        // présente, aucune phrase de barre ; plans à 4 étapes et questions plus denses qu'à la
        // séance 2 (6 plans à 4 étapes et aucune question sur 1 800).
        assert_eq!(
            &THIRD_SESSION_FAMILIES[..SECOND_SESSION_FAMILIES.len()],
            SECOND_SESSION_FAMILIES
        );
        assert_eq!(
            &THIRD_SESSION_FAMILIES[SECOND_SESSION_FAMILIES.len()..],
            &[
                "conditions_multiples",
                "constructible",
                "distance_reference_filtree",
                "question_champ_absent",
                "question_valeur_absente",
            ]
        );
        let (examples, summary) =
            generate_families(FOCUS_SEED, FOCUS_COUNT, THIRD_SESSION_FAMILIES).unwrap();
        assert_eq!(examples.len(), FOCUS_COUNT, "{:?}", summary.families);
        assert!(examples
            .iter()
            .all(|e| THIRD_SESSION_FAMILIES.contains(&e.family)));
        for name in THIRD_SESSION_FAMILIES {
            let tally = &summary.families[name];
            assert!(tally.kept > 0 && tally.rejected == 0, "{name} : {tally:?}");
        }
        assert_eq!(barres::copies(&examples), Vec::<String>::new());
        assert!(THIRD_SESSION_FAMILIES
            .iter()
            .all(|n| !FOCUS_EXCLUDED.contains(n)));
        let four = examples
            .iter()
            .filter(|e| e.expected["calls"].as_array().is_some_and(|c| c.len() == 4))
            .count();
        let questions = examples
            .iter()
            .filter(|e| e.expected["kind"] == "question")
            .count();
        assert!(four * 100 >= 8 * FOCUS_COUNT, "plans à 4 étapes : {four}");
        assert!(
            questions * 100 >= 10 * FOCUS_COUNT,
            "questions : {questions}"
        );
    }

    #[test]
    fn the_session_command_of_the_doc_names_the_third_session_families() {
        let doc = include_str!("../../docs/entrainement-du-modele.md");
        let line = format!("nuit-5-54.sh {}", THIRD_SESSION_FAMILIES.join(","));
        assert!(doc.contains(&format!("{line}\n")), "{line}");
    }

    #[test]
    fn normalization_drops_case_accents_and_punctuation() {
        assert_eq!(
            normalize("Trouve les parcelles « constructibles » à 500 m d'une gare !"),
            "trouve les parcelles constructibles a 500 m d une gare"
        );
        assert_eq!(normalize("Écoles   Œuvre"), "ecoles oeuvre");
    }
}
