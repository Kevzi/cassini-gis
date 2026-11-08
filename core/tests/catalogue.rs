//! Recette du catalogue des commandes (AD-6, amendement du 2026-11-05) : chaque commande du
//! registre standard figure au catalogue figé avec les mêmes identifiant, paramètres et
//! résultat ; une fiche absente du registre y est marquée planifiée ou réservée.
//!
//! Le catalogue est `spec-cassini-mvp/catalogue-des-commandes.md` : un bloc ` ```json fiche `
//! par commande, sous un titre de niveau 3, et un bloc ` ```json type ` par type nommé (§3).
//! La recette lit ces blocs, normalise chaque `CommandSpec` sous la même forme, en
//! profondeur (types nommés comparés à leur `$defs`, éléments de tableau, champs des objets
//! en ligne, unions étiquetées), et compare. Les fiches planifiées ou réservées ne sont
//! confrontées à aucun code : seule leur forme est vérifiée.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use cassini_core::point::Point;
use cassini_core::registry::{CommandSpec, Registry, RunsIn};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// Types nommés : la forme normalisée garde leur nom au lieu de leur type JSON, et leur
/// définition est comparée une fois, à leur bloc ` ```json type `. Les quatre premiers sont
/// les types partagés du catalogue ; les suivants, les types de mesure déjà au code et le
/// contrat de réseau d'AD-28, le gabarit de résumé d'AD-27 et la description de style du
/// changement de cap du 2026-11-08.
const NAMED_TYPES: [&str; 10] = [
    "Point",
    "MgrsRef",
    "Angle",
    "RasterPath",
    "Distance",
    "Height",
    "Duration",
    "NetworkOptions",
    "SummaryRow",
    "StyleSpec",
];

const JSON_TYPES: [&str; 6] = ["string", "number", "integer", "boolean", "object", "array"];

/// Type d'une union étiquetée (`oneOf` d'objets distingués par un champ constant).
const UNION: &str = "union";

const FICHE_KEYS: [&str; 10] = [
    "id",
    "status",
    "runs_in",
    "dictated_by_core",
    "params",
    "result",
    "planned_by",
    "planned_params",
    "planned_result",
    "replaced_by",
];

/// Clés d'un type (paramètre, champ, élément de tableau ou bloc de type).
const TYPE_KEYS: [&str; 6] = ["type", "enum", "items", "fields", "tag", "variants"];

/// Clés d'un paramètre ou d'un champ : celles d'un type, plus `required`.
const PARAM_KEYS: [&str; 7] = [
    "type", "required", "enum", "items", "fields", "tag", "variants",
];

/// Préfixe des commandes compilées pour les tests seulement, absentes du catalogue.
const TEST_PREFIX: &str = "test.";

// ---- Forme normalisée, commune au catalogue et au registre ----

/// Un type normalisé : type JSON, nom d'un type nommé ou `union`.
#[derive(Debug, Clone, PartialEq, Default)]
struct Ty {
    ty: String,
    /// Valeurs permises, comparées comme un ensemble (chacune en JSON).
    enum_values: Option<BTreeSet<String>>,
    /// Type des éléments d'un tableau.
    items: Option<Box<Ty>>,
    /// Champs d'un objet en ligne.
    fields: Option<BTreeMap<String, Param>>,
    /// Champ qui distingue les variantes d'une union.
    tag: Option<String>,
    /// Variantes d'une union : valeur de l'étiquette vers ses champs (étiquette exclue).
    variants: Option<BTreeMap<String, BTreeMap<String, Param>>>,
}

impl Ty {
    fn named(ty: &str) -> Self {
        Ty {
            ty: ty.to_string(),
            ..Ty::default()
        }
    }
}

/// Un paramètre ou un champ : son type, et s'il est obligatoire.
#[derive(Debug, Clone, PartialEq)]
struct Param {
    ty: Ty,
    required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Done,
    Planned,
    Reserved,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Done => "faite",
            Status::Planned => "planifiee",
            Status::Reserved => "reservee",
        }
    }
}

/// Une fiche du catalogue, lue et validée dans sa forme.
#[derive(Debug, Clone)]
struct Fiche {
    heading: String,
    id: String,
    status: Status,
    runs_in: String,
    dictated_by_core: bool,
    params: BTreeMap<String, Param>,
    result: BTreeSet<String>,
    /// Fiche planifiée qui remplace cette fiche faite (renommage décidé).
    replaced_by: Option<String>,
}

/// Le catalogue lu : les fiches et les définitions des types nommés.
#[derive(Debug, Clone)]
struct Catalogue {
    fiches: Vec<Fiche>,
    types: BTreeMap<String, Ty>,
}

/// Une commande du registre, sous la forme d'une fiche.
#[derive(Debug, Clone)]
struct Normalized {
    id: String,
    runs_in: String,
    dictated_by_core: bool,
    params: BTreeMap<String, Param>,
    result: BTreeSet<String>,
    /// Définitions des types nommés trouvées dans les `$defs` de ses schémas.
    defs: Vec<(String, Ty)>,
}

// ---- Lecture du catalogue ----

fn catalogue_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("_bmad-output")
        .join("initiative-cassini-mvp")
        .join("spec-cassini-mvp")
        .join("catalogue-des-commandes.md")
}

/// Lit les blocs ` ```json fiche ` et ` ```json type ` d'un catalogue. Rend le catalogue, ou
/// toutes les erreurs de forme, chacune nommant la fiche (son titre et sa ligne) et le champ
/// fautif.
fn parse_catalogue(text: &str) -> Result<Catalogue, Vec<String>> {
    let mut fiches = Vec::new();
    let mut types = BTreeMap::new();
    let mut errors = Vec::new();
    let mut heading = String::from("(sans titre)");
    let mut lines = text.lines().enumerate();
    while let Some((n, line)) = lines.next() {
        if let Some(title) = line.strip_prefix("### ") {
            heading = title.trim().to_string();
            continue;
        }
        let kind = match line.trim() {
            "```json fiche" => "fiche",
            "```json type" => "type",
            _ => continue,
        };
        let name = format!("« {heading} » (ligne {})", n + 1);
        let mut body = String::new();
        let mut closed = false;
        for (_, inner) in lines.by_ref() {
            if inner.trim() == "```" {
                closed = true;
                break;
            }
            body.push_str(inner);
            body.push('\n');
        }
        if !closed {
            errors.push(format!("{kind} {name} : bloc JSON non refermé"));
            break;
        }
        let value = match serde_json::from_str::<Value>(&body) {
            Err(e) => {
                errors.push(format!("{kind} {name} : JSON illisible ({e})"));
                continue;
            }
            Ok(value) => value,
        };
        if kind == "type" {
            if let Some((tname, ty)) = read_type_block(&name, &value, &mut errors) {
                if types.insert(tname.clone(), ty).is_some() {
                    errors.push(format!("type {name} : « {tname} » défini deux fois"));
                }
            }
            continue;
        }
        match read_fiche(&name, &heading, &value) {
            Ok(fiche) => fiches.push(fiche),
            Err(mut e) => errors.append(&mut e),
        }
    }
    let mut seen = BTreeSet::new();
    for fiche in &fiches {
        if !seen.insert(fiche.id.clone()) {
            errors.push(format!(
                "fiche « {} » : identifiant « {} » décrit par deux fiches",
                fiche.heading, fiche.id
            ));
        }
    }
    for fiche in &fiches {
        if let Some(target) = &fiche.replaced_by {
            if !seen.contains(target) {
                errors.push(format!(
                    "fiche « {} » : champ « replaced_by » : « {target} » n'a pas de fiche au catalogue",
                    fiche.heading
                ));
            }
        }
    }
    if fiches.is_empty() && errors.is_empty() {
        errors.push("aucune fiche ```json fiche``` dans le catalogue".to_string());
    }
    if errors.is_empty() {
        Ok(Catalogue { fiches, types })
    } else {
        Err(errors)
    }
}

/// Un identifiant `domaine.verbe`, anglais et snake_case.
fn is_command_id(id: &str) -> bool {
    let snake = |s: &str| {
        !s.is_empty()
            && s.starts_with(|c: char| c.is_ascii_lowercase())
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    matches!(id.split_once('.'), Some((d, v)) if snake(d) && snake(v))
}

fn read_type_block(name: &str, value: &Value, errors: &mut Vec<String>) -> Option<(String, Ty)> {
    let at = format!("type {name}");
    let Some(map) = value.as_object() else {
        errors.push(format!("{at} : le bloc n'est pas un objet JSON"));
        return None;
    };
    let tname = match map.get("name") {
        Some(Value::String(s)) if NAMED_TYPES.contains(&s.as_str()) => s.clone(),
        Some(Value::String(s)) => {
            errors.push(format!(
                "{at} : champ « name » : « {s} » n'est pas un type nommé"
            ));
            return None;
        }
        _ => {
            errors.push(format!("{at} : champ « name » manquant ou non textuel"));
            return None;
        }
    };
    let mut rest = map.clone();
    rest.remove("name");
    let ty = read_ty(
        &format!("{at} : type « {tname} »"),
        &rest,
        &TYPE_KEYS,
        errors,
    )?;
    Some((tname, ty))
}

fn read_fiche(name: &str, heading: &str, value: &Value) -> Result<Fiche, Vec<String>> {
    let mut errors = Vec::new();
    let Some(map) = value.as_object() else {
        return Err(vec![format!(
            "fiche {name} : le bloc n'est pas un objet JSON"
        )]);
    };
    for key in map.keys() {
        if !FICHE_KEYS.contains(&key.as_str()) {
            errors.push(format!("fiche {name} : champ inconnu « {key} »"));
        }
    }
    let text = |field: &str, errors: &mut Vec<String>| match map.get(field) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => {
            errors.push(format!(
                "fiche {name} : champ « {field} » : une chaîne est attendue"
            ));
            None
        }
        None => {
            errors.push(format!("fiche {name} : champ « {field} » manquant"));
            None
        }
    };
    let id = text("id", &mut errors);
    if let Some(id) = &id {
        if !is_command_id(id) {
            errors.push(format!(
                "fiche {name} : champ « id » : « {id} » n'est pas en domaine.verbe snake_case"
            ));
        }
        if !heading.contains(&format!("`{id}`")) {
            errors.push(format!(
                "fiche {name} : champ « id » : le titre ne nomme pas `{id}`"
            ));
        }
    }
    let status = match text("status", &mut errors).as_deref() {
        Some("faite") => Some(Status::Done),
        Some("planifiee") => Some(Status::Planned),
        Some("reservee") => Some(Status::Reserved),
        Some(other) => {
            errors.push(format!(
                "fiche {name} : champ « status » : « {other} » n'est ni faite, ni planifiee, ni reservee"
            ));
            None
        }
        None => None,
    };
    let runs_in = text("runs_in", &mut errors);
    if let Some(r) = &runs_in {
        if r != "core" && r != "qgis" {
            errors.push(format!(
                "fiche {name} : champ « runs_in » : « {r} » n'est ni core ni qgis"
            ));
        }
    }
    // Une réservée peut n'avoir que id, status et runs_in ; les autres disent tout.
    let complete = status != Some(Status::Reserved);
    let dictated_by_core = match map.get("dictated_by_core") {
        Some(Value::Bool(b)) => *b,
        Some(_) => {
            errors.push(format!(
                "fiche {name} : champ « dictated_by_core » : un booléen est attendu"
            ));
            false
        }
        None => {
            if complete {
                errors.push(format!(
                    "fiche {name} : champ « dictated_by_core » manquant"
                ));
            }
            false
        }
    };
    let params = match map.get("params") {
        Some(p) => read_params(name, "params", p, &mut errors),
        None => {
            if complete {
                errors.push(format!("fiche {name} : champ « params » manquant"));
            }
            BTreeMap::new()
        }
    };
    let result = match map.get("result") {
        Some(r) => read_names(name, "result", r, &mut errors),
        None => {
            if complete {
                errors.push(format!("fiche {name} : champ « result » manquant"));
            }
            BTreeSet::new()
        }
    };
    // Évolution figée d'une commande faite : forme vérifiée, jamais confrontée au code.
    let planned_params = map.get("planned_params");
    let planned_result = map.get("planned_result");
    if let Some(p) = planned_params {
        read_params(name, "planned_params", p, &mut errors);
    }
    if let Some(r) = planned_result {
        read_names(name, "planned_result", r, &mut errors);
    }
    let planned = planned_params.is_some() || planned_result.is_some();
    match map.get("planned_by") {
        Some(Value::String(_)) if planned => {}
        None if !planned => {}
        _ => errors.push(format!(
            "fiche {name} : champ « planned_by » (chaîne) va avec « planned_params » ou « planned_result »"
        )),
    }
    if planned && status != Some(Status::Done) {
        errors.push(format!(
            "fiche {name} : champs « planned_params » et « planned_result » : réservés aux fiches faites"
        ));
    }
    let replaced_by = match map.get("replaced_by") {
        None => None,
        Some(Value::String(r)) if status == Some(Status::Done) => Some(r.clone()),
        Some(_) => {
            errors.push(format!(
                "fiche {name} : champ « replaced_by » : un identifiant, sur une fiche faite seulement"
            ));
            None
        }
    };
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Fiche {
        heading: heading.to_string(),
        id: id.unwrap_or_default(),
        status: status.unwrap_or(Status::Reserved),
        runs_in: runs_in.unwrap_or_default(),
        dictated_by_core,
        params,
        result,
        replaced_by,
    })
}

/// Une liste de noms de champs, sans doublon.
fn read_names(
    name: &str,
    field: &str,
    value: &Value,
    errors: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    let Some(list) = value.as_array() else {
        errors.push(format!(
            "fiche {name} : champ « {field} » : une liste de noms est attendue"
        ));
        return set;
    };
    for f in list {
        match f.as_str() {
            Some(s) if set.insert(s.to_string()) => {}
            Some(s) => errors.push(format!(
                "fiche {name} : champ « {field} » : « {s} » cité deux fois"
            )),
            None => errors.push(format!(
                "fiche {name} : champ « {field} » : chaque champ est une chaîne"
            )),
        }
    }
    set
}

fn is_known_type(ty: &str) -> bool {
    JSON_TYPES.contains(&ty) || NAMED_TYPES.contains(&ty) || ty == UNION
}

fn read_params(
    name: &str,
    field: &str,
    value: &Value,
    errors: &mut Vec<String>,
) -> BTreeMap<String, Param> {
    read_fields(&format!("fiche {name} : champ « {field}"), value, errors)
}

/// Un objet de champs (paramètres, champs d'un objet, variante d'une union). `at` est le
/// début du message, auquel s'ajoute `.<nom> »`.
fn read_fields(at: &str, value: &Value, errors: &mut Vec<String>) -> BTreeMap<String, Param> {
    let mut fields = BTreeMap::new();
    let Some(map) = value.as_object() else {
        errors.push(format!("{at} » : un objet est attendu"));
        return fields;
    };
    for (fname, spec) in map {
        let here = format!("{at}.{fname} »");
        let Some(spec) = spec.as_object() else {
            errors.push(format!("{here} : un objet est attendu"));
            continue;
        };
        let required = match spec.get("required") {
            Some(Value::Bool(b)) => *b,
            _ => {
                errors.push(format!("{here} : « required » manquant ou non booléen"));
                // Les autres défauts du champ sont rendus quand même.
                let _ = read_ty(&here, spec, &PARAM_KEYS, errors);
                continue;
            }
        };
        let Some(ty) = read_ty(&here, spec, &PARAM_KEYS, errors) else {
            continue;
        };
        fields.insert(fname.clone(), Param { ty, required });
    }
    fields
}

/// Lit un type : `type`, et selon lui `enum`, `items`, `fields`, `tag` et `variants`.
fn read_ty(
    at: &str,
    spec: &Map<String, Value>,
    allowed: &[&str],
    errors: &mut Vec<String>,
) -> Option<Ty> {
    let before = errors.len();
    for key in spec.keys() {
        if !allowed.contains(&key.as_str()) {
            errors.push(format!("{at} : clé inconnue « {key} »"));
        }
    }
    let ty = match spec.get("type") {
        Some(Value::String(t)) if is_known_type(t) => t.clone(),
        Some(Value::String(t)) => {
            errors.push(format!("{at} : type « {t} » inconnu"));
            return None;
        }
        _ => {
            errors.push(format!("{at} : « type » manquant ou non textuel"));
            return None;
        }
    };
    let enum_values = match spec.get("enum") {
        None => None,
        Some(Value::Array(v)) if !v.is_empty() => Some(v.iter().map(Value::to_string).collect()),
        Some(_) => {
            errors.push(format!("{at} : « enum » doit être une liste non vide"));
            None
        }
    };
    let items = match spec.get("items") {
        None => None,
        Some(_) if ty != "array" => {
            errors.push(format!("{at} : « items » sur un type autre que array"));
            None
        }
        Some(Value::String(t)) if t == "object" || t == "array" || t == UNION => {
            errors.push(format!(
                "{at} : « items » « {t} » s'écrit en objet de type, avec ses champs ou ses éléments"
            ));
            None
        }
        Some(Value::String(t)) if is_known_type(t) => Some(Box::new(Ty::named(t))),
        Some(Value::Object(inner)) => {
            let here = match at.strip_suffix(" »") {
                Some(prefix) => format!("{prefix}[] »"),
                None => format!("{at}[]"),
            };
            read_ty(&here, inner, &TYPE_KEYS, errors).map(Box::new)
        }
        Some(_) => {
            errors.push(format!(
                "{at} : « items » est un type connu ou un objet de type"
            ));
            None
        }
    };
    if ty == "array" && !spec.contains_key("items") {
        errors.push(format!(
            "{at} : un tableau déclare ses éléments (« items »)"
        ));
    }
    let fields = match spec.get("fields") {
        Some(_) if ty != "object" => {
            errors.push(format!("{at} : « fields » sur un type autre que object"));
            None
        }
        Some(f) => {
            let prefix = at.strip_suffix(" »").unwrap_or(at);
            Some(read_fields(prefix, f, errors))
        }
        None if ty == "object" => {
            errors.push(format!(
                "{at} : un objet en ligne déclare ses champs (« fields »)"
            ));
            None
        }
        None => None,
    };
    let (tag, variants) = match (spec.get("tag"), spec.get("variants")) {
        (None, None) if ty != UNION => (None, None),
        (Some(Value::String(tag)), Some(Value::Object(vars)))
            if ty == UNION && !vars.is_empty() =>
        {
            let prefix = at.strip_suffix(" »").unwrap_or(at);
            let variants = vars
                .iter()
                .map(|(k, v)| (k.clone(), read_fields(&format!("{prefix}.{k}"), v, errors)))
                .collect();
            (Some(tag.clone()), Some(variants))
        }
        _ => {
            errors.push(format!(
                "{at} : une union porte « tag » (chaîne) et « variants » (objet non vide), et seulement elle"
            ));
            (None, None)
        }
    };
    if errors.len() > before {
        return None;
    }
    Some(Ty {
        ty,
        enum_values,
        items,
        fields,
        tag,
        variants,
    })
}

// ---- Normalisation d'un schéma engendré ----

fn def_name(reference: &str) -> &str {
    reference.rsplit('/').next().unwrap_or(reference)
}

fn resolve_ref<'a>(reference: &str, root: &'a Value) -> &'a Value {
    let name = def_name(reference);
    root.get("$defs")
        .and_then(|d| d.get(name))
        .unwrap_or(&Value::Null)
}

/// Suit les `$ref` jusqu'à la cible, sans s'arrêter aux types nommés.
fn resolve<'a>(schema: &'a Value, root: &'a Value) -> &'a Value {
    let mut node = schema;
    for _ in 0..16 {
        match node.get("$ref").and_then(Value::as_str) {
            Some(reference) => node = resolve_ref(reference, root),
            None => break,
        }
    }
    node
}

const MAX_DEPTH: usize = 16;

/// Type d'un sous-schéma, et s'il admet `null`.
fn normalize_type(schema: &Value, root: &Value, depth: usize) -> (Ty, bool) {
    if depth > MAX_DEPTH {
        return (Ty::named("any"), false);
    }
    if let Some(Value::String(reference)) = schema.get("$ref") {
        let name = def_name(reference);
        if NAMED_TYPES.contains(&name) {
            return (Ty::named(name), false);
        }
        return normalize_type(resolve_ref(reference, root), root, depth + 1);
    }
    for key in ["anyOf", "oneOf", "allOf"] {
        let Some(alts) = schema.get(key).and_then(Value::as_array) else {
            continue;
        };
        if !alts.is_empty() && alts.iter().all(|a| a.get("const").is_some()) {
            let ty = alts[0]
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("string");
            return (
                Ty {
                    enum_values: Some(alts.iter().map(|a| a["const"].to_string()).collect()),
                    ..Ty::named(ty)
                },
                false,
            );
        }
        let is_null = |a: &Value| a.get("type") == Some(&json!("null"));
        let rest: Vec<&Value> = alts.iter().filter(|a| !is_null(a)).collect();
        let nullable = rest.len() < alts.len();
        if rest.len() == 1 {
            let (ty, inner_null) = normalize_type(rest[0], root, depth + 1);
            return (ty, nullable || inner_null);
        }
        let union = tagged_union(&rest, root, depth + 1).unwrap_or_else(|| Ty::named(UNION));
        return (union, nullable);
    }
    let (ty, nullable) = match schema.get("type") {
        Some(Value::String(t)) => (t.clone(), false),
        Some(Value::Array(types)) => {
            let rest: Vec<&str> = types
                .iter()
                .filter_map(Value::as_str)
                .filter(|t| *t != "null")
                .collect();
            let ty = if rest.len() == 1 { rest[0] } else { UNION };
            (ty.to_string(), rest.len() < types.len())
        }
        _ => ("any".to_string(), false),
    };
    let enum_values = schema.get("enum").and_then(Value::as_array).map(|v| {
        v.iter()
            .filter(|x| !x.is_null())
            .map(Value::to_string)
            .collect()
    });
    let items = if ty == "array" {
        schema
            .get("items")
            .map(|i| Box::new(normalize_type(i, root, depth + 1).0))
    } else {
        None
    };
    // Un objet sans `properties` (table de noms libres) a une liste de champs vide.
    let fields = if ty == "object" {
        Some(normalize_fields(schema, root, depth + 1))
    } else {
        None
    };
    (
        Ty {
            ty,
            enum_values,
            items,
            fields,
            tag: None,
            variants: None,
        },
        nullable,
    )
}

/// Valeur constante d'un champ (`const`, ou `enum` d'une seule valeur), en texte.
fn const_of(schema: &Value) -> Option<String> {
    let value = schema.get("const").or_else(|| {
        schema
            .get("enum")
            .and_then(Value::as_array)
            .filter(|e| e.len() == 1)
            .map(|e| &e[0])
    })?;
    Some(
        value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_string),
    )
}

/// Une union étiquetée : chaque alternative est un objet qui porte le même champ constant,
/// de valeur distincte.
fn tagged_union(alts: &[&Value], root: &Value, depth: usize) -> Option<Ty> {
    let objects: Vec<&Value> = alts.iter().map(|a| resolve(a, root)).collect();
    let first = objects.first()?.get("properties")?.as_object()?;
    let tag = first.keys().find_map(|name| {
        let all_const = objects.iter().all(|o| {
            o.get("properties")
                .and_then(|p| p.get(name))
                .and_then(|s| const_of(resolve(s, root)))
                .is_some()
        });
        all_const.then(|| name.clone())
    })?;
    let mut variants = BTreeMap::new();
    for object in objects {
        let value = const_of(resolve(&object["properties"][&tag], root))?;
        let mut fields = normalize_fields(object, root, depth + 1);
        fields.remove(&tag);
        if variants.insert(value, fields).is_some() {
            return None;
        }
    }
    Some(Ty {
        tag: Some(tag),
        variants: Some(variants),
        ..Ty::named(UNION)
    })
}

/// Champs d'un objet : `properties` et `required`, un champ qui admet `null` étant facultatif.
fn normalize_fields(object: &Value, root: &Value, depth: usize) -> BTreeMap<String, Param> {
    let required: BTreeSet<&str> = object
        .get("required")
        .and_then(Value::as_array)
        .map(|r| r.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let mut fields = BTreeMap::new();
    if let Some(props) = object.get("properties").and_then(Value::as_object) {
        for (name, schema) in props {
            let (ty, nullable) = normalize_type(schema, root, depth);
            fields.insert(
                name.clone(),
                Param {
                    ty,
                    required: required.contains(name.as_str()) && !nullable,
                },
            );
        }
    }
    fields
}

fn normalize(spec: &CommandSpec) -> Normalized {
    let params = normalize_fields(&spec.params_schema, &spec.params_schema, 0);
    let result = spec
        .output_schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|p| p.keys().cloned().collect())
        .unwrap_or_default();
    let mut defs = Vec::new();
    for root in [&spec.params_schema, &spec.output_schema] {
        if let Some(map) = root.get("$defs").and_then(Value::as_object) {
            for (name, def) in map {
                if NAMED_TYPES.contains(&name.as_str()) {
                    defs.push((name.clone(), normalize_type(def, root, 0).0));
                }
            }
        }
    }
    Normalized {
        id: spec.id.clone(),
        runs_in: match spec.runs_in {
            Some(RunsIn::Core) => "core",
            Some(RunsIn::Qgis) => "qgis",
            None => "(non déclaré)",
        }
        .to_string(),
        dictated_by_core: spec.dictated_by_core,
        params,
        result,
        defs,
    }
}

fn registry_commands(registry: &Registry) -> Vec<Normalized> {
    registry
        .specs()
        .filter(|s| !s.id.starts_with(TEST_PREFIX))
        .map(normalize)
        .collect()
}

// ---- Confrontation ----

fn show_set(v: &Option<BTreeSet<String>>) -> String {
    match v {
        Some(set) => format!("[{}]", set.iter().cloned().collect::<Vec<_>>().join(", ")),
        None => "aucun".to_string(),
    }
}

fn show_keys<V>(v: &Option<BTreeMap<String, V>>) -> String {
    match v {
        Some(map) => format!("[{}]", map.keys().cloned().collect::<Vec<_>>().join(", ")),
        None => "aucun".to_string(),
    }
}

/// Où se trouve un écart : `ctx` seul à la racine, sinon `ctx : label « chemin »`.
fn at(ctx: &str, label: &str, path: &str) -> String {
    if path.is_empty() {
        ctx.to_string()
    } else {
        format!("{ctx} : {label} « {path} »")
    }
}

fn child(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}.{name}")
    }
}

fn compare_ty(ctx: &str, label: &str, path: &str, want: &Ty, got: &Ty, errors: &mut Vec<String>) {
    let here = at(ctx, label, path);
    if want.ty != got.ty {
        errors.push(format!(
            "{here}, type : attendu « {} », trouvé « {} »",
            want.ty, got.ty
        ));
        return;
    }
    if want.enum_values != got.enum_values {
        errors.push(format!(
            "{here}, valeurs permises : attendu {}, trouvé {}",
            show_set(&want.enum_values),
            show_set(&got.enum_values)
        ));
    }
    match (&want.items, &got.items) {
        (Some(w), Some(g)) => compare_ty(ctx, label, &format!("{path}[]"), w, g, errors),
        (None, None) => {}
        (w, g) => errors.push(format!(
            "{here}, éléments : attendu {}, trouvé {}",
            w.as_ref().map_or("aucun".to_string(), |t| t.ty.clone()),
            g.as_ref().map_or("aucun".to_string(), |t| t.ty.clone())
        )),
    }
    match (&want.fields, &got.fields) {
        (Some(w), Some(g)) => compare_fields(ctx, label, path, w, g, errors),
        (None, None) => {}
        (w, g) => errors.push(format!(
            "{here}, champs : attendu {}, trouvé {}",
            show_keys(w),
            show_keys(g)
        )),
    }
    if want.tag != got.tag {
        errors.push(format!(
            "{here}, étiquette : attendu {:?}, trouvé {:?}",
            want.tag, got.tag
        ));
    }
    match (&want.variants, &got.variants) {
        (Some(w), Some(g)) => {
            if w.keys().ne(g.keys()) {
                errors.push(format!(
                    "{here}, variantes : attendu {}, trouvé {}",
                    show_keys(&want.variants),
                    show_keys(&got.variants)
                ));
            }
            for (k, wf) in w {
                if let Some(gf) = g.get(k) {
                    compare_fields(ctx, label, &child(path, k), wf, gf, errors);
                }
            }
        }
        (None, None) => {}
        _ => errors.push(format!(
            "{here}, variantes : attendu {}, trouvé {}",
            show_keys(&want.variants),
            show_keys(&got.variants)
        )),
    }
}

fn compare_fields(
    ctx: &str,
    label: &str,
    path: &str,
    expected: &BTreeMap<String, Param>,
    found: &BTreeMap<String, Param>,
    errors: &mut Vec<String>,
) {
    for (name, want) in expected {
        let full = child(path, name);
        let Some(got) = found.get(name) else {
            errors.push(format!(
                "{} attendu par le catalogue, absent du registre",
                at(ctx, label, &full)
            ));
            continue;
        };
        if want.required != got.required {
            errors.push(format!(
                "{}, obligatoire : attendu {}, trouvé {}",
                at(ctx, label, &full),
                want.required,
                got.required
            ));
        }
        compare_ty(ctx, label, &full, &want.ty, &got.ty, errors);
    }
    for name in found.keys().filter(|n| !expected.contains_key(*n)) {
        errors.push(format!(
            "{} présent au registre, absent du catalogue",
            at(ctx, label, &child(path, name))
        ));
    }
}

fn compare_params(
    id: &str,
    expected: &BTreeMap<String, Param>,
    found: &BTreeMap<String, Param>,
    errors: &mut Vec<String>,
) {
    compare_fields(id, "paramètre", "", expected, found, errors);
}

/// Confronte le catalogue au registre ; rend chaque écart, nommé, une seule fois.
fn confront(catalogue: &Catalogue, commands: &[Normalized]) -> Vec<String> {
    let mut errors = Vec::new();
    let fiches = &catalogue.fiches;
    let by_id: BTreeMap<&str, &Fiche> = fiches.iter().map(|f| (f.id.as_str(), f)).collect();
    let registered: BTreeSet<&str> = commands.iter().map(|c| c.id.as_str()).collect();
    for cmd in commands {
        let id = cmd.id.as_str();
        // Les types nommés se comparent à leur bloc, quel que soit le statut de la fiche.
        for (name, ty) in &cmd.defs {
            match catalogue.types.get(name) {
                Some(want) => compare_ty(
                    &format!("type « {name} »"),
                    "champ",
                    "",
                    want,
                    ty,
                    &mut errors,
                ),
                None => errors.push(format!(
                    "type « {name} » : au registre, sans bloc json type au catalogue"
                )),
            }
        }
        let Some(fiche) = by_id.get(id) else {
            errors.push(format!(
                "{id} : commande enregistrée au registre, sans fiche au catalogue"
            ));
            continue;
        };
        if fiche.status != Status::Done {
            errors.push(format!(
                "{id} : la fiche est « {} » mais la commande est au registre : passer la fiche en « faite »",
                fiche.status.label()
            ));
            continue;
        }
        if fiche.runs_in != cmd.runs_in {
            errors.push(format!(
                "{id} : runs_in : attendu « {} », trouvé « {} »",
                fiche.runs_in, cmd.runs_in
            ));
        }
        if fiche.dictated_by_core != cmd.dictated_by_core {
            errors.push(format!(
                "{id} : dictated_by_core : attendu {}, trouvé {}",
                fiche.dictated_by_core, cmd.dictated_by_core
            ));
        }
        compare_params(id, &fiche.params, &cmd.params, &mut errors);
        for field in fiche.result.difference(&cmd.result) {
            errors.push(format!(
                "{id} : champ de résultat « {field} » attendu par le catalogue, absent du registre"
            ));
        }
        for field in cmd.result.difference(&fiche.result) {
            errors.push(format!(
                "{id} : champ de résultat « {field} » présent au registre, absent du catalogue"
            ));
        }
    }
    for fiche in fiches {
        if fiche.status == Status::Done && !registered.contains(fiche.id.as_str()) {
            errors.push(format!(
                "{} : fiche « faite » sans commande au registre",
                fiche.id
            ));
        }
    }
    let mut seen = BTreeSet::new();
    errors.retain(|e| seen.insert(e.clone()));
    errors
}

fn real_catalogue() -> Catalogue {
    let path = catalogue_path();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("catalogue illisible ({}) : {e}", path.display()));
    parse_catalogue(&text)
        .unwrap_or_else(|errors| panic!("catalogue mal formé :\n- {}", errors.join("\n- ")))
}

fn joined(errors: &[String]) -> String {
    errors.join("\n")
}

// ---- Retour de la forme normalisée au JSON du catalogue (tests seulement) ----

fn ty_json(ty: &Ty) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("type".into(), json!(ty.ty));
    if let Some(e) = &ty.enum_values {
        let values: Vec<Value> = e.iter().map(|s| serde_json::from_str(s).unwrap()).collect();
        m.insert("enum".into(), json!(values));
    }
    if let Some(i) = &ty.items {
        m.insert("items".into(), Value::Object(ty_json(i)));
    }
    if let Some(f) = &ty.fields {
        m.insert("fields".into(), fields_json(f));
    }
    if let Some(t) = &ty.tag {
        m.insert("tag".into(), json!(t));
    }
    if let Some(v) = &ty.variants {
        let vars: Map<String, Value> = v.iter().map(|(k, f)| (k.clone(), fields_json(f))).collect();
        m.insert("variants".into(), Value::Object(vars));
    }
    m
}

fn fields_json(fields: &BTreeMap<String, Param>) -> Value {
    let map: Map<String, Value> = fields
        .iter()
        .map(|(k, p)| {
            let mut m = ty_json(&p.ty);
            m.insert("required".into(), json!(p.required));
            (k.clone(), Value::Object(m))
        })
        .collect();
    Value::Object(map)
}

/// Un catalogue en mémoire, fiche faite et blocs de type tirés de la commande elle-même.
fn catalogue_text_for(n: &Normalized) -> String {
    let mut text = String::new();
    let mut done = BTreeSet::new();
    for (name, ty) in &n.defs {
        if done.insert(name.clone()) {
            let mut m = ty_json(ty);
            m.insert("name".into(), json!(name));
            text.push_str(&format!(
                "### `{name}`\n\n```json type\n{}\n```\n\n",
                Value::Object(m)
            ));
        }
    }
    let fiche = json!({"id": n.id, "status": "faite", "runs_in": n.runs_in,
        "dictated_by_core": n.dictated_by_core, "params": fields_json(&n.params),
        "result": n.result});
    text.push_str(&format!("### `{}`\n\n```json fiche\n{fiche}\n```\n", n.id));
    text
}

// ---- La recette ----

#[test]
fn the_registry_matches_the_catalogue() {
    let errors = confront(&real_catalogue(), &registry_commands(&Registry::standard()));
    assert!(
        errors.is_empty(),
        "le registre s'écarte du catalogue figé :\n- {}",
        errors.join("\n- ")
    );
}

#[test]
fn every_non_test_command_has_a_fiche_and_planned_ids_are_not_registered() {
    let catalogue = real_catalogue();
    let fiches = &catalogue.fiches;
    let registry = Registry::standard();
    for spec in registry.specs().filter(|s| !s.id.starts_with(TEST_PREFIX)) {
        let fiche = fiches.iter().find(|f| f.id == spec.id);
        assert_eq!(fiche.map(|f| f.status), Some(Status::Done), "{}", spec.id);
    }
    assert!(fiches.iter().any(|f| f.status == Status::Planned));
    assert!(fiches.iter().any(|f| f.status == Status::Reserved));
}

#[test]
fn every_named_type_has_its_block() {
    let catalogue = real_catalogue();
    let defined: BTreeSet<&str> = catalogue.types.keys().map(String::as_str).collect();
    let named: BTreeSet<&str> = NAMED_TYPES.iter().copied().collect();
    assert_eq!(defined, named);
}

// ---- Matrice des écarts, sur des registres et des catalogues fabriqués ----

fn mutate_spec(id: &str, mut edit: impl FnMut(&mut CommandSpec)) -> Vec<Normalized> {
    let registry = Registry::standard();
    registry
        .specs()
        .filter(|s| !s.id.starts_with(TEST_PREFIX))
        .map(|s| {
            let mut s = s.clone();
            if s.id == id {
                edit(&mut s);
            }
            normalize(&s)
        })
        .collect()
}

#[test]
fn a_renamed_parameter_fails_with_id_parameter_expected_and_found() {
    let commands = mutate_spec("terrain.slope", |s| {
        let props = s.params_schema["properties"].as_object_mut().unwrap();
        let dem = props.remove("dem").unwrap();
        props.insert("dem_path".to_string(), dem);
    });
    let errors = joined(&confront(&real_catalogue(), &commands));
    assert!(
        errors.contains(
            "terrain.slope : paramètre « dem » attendu par le catalogue, absent du registre"
        ),
        "{errors}"
    );
    assert!(
        errors.contains(
            "terrain.slope : paramètre « dem_path » présent au registre, absent du catalogue"
        ),
        "{errors}"
    );
}

#[test]
fn a_retyped_parameter_fails_with_expected_and_found() {
    let commands = mutate_spec("terrain.slope", |s| {
        s.params_schema["properties"]["dem"] = json!({"type": "integer"});
    });
    let errors = joined(&confront(&real_catalogue(), &commands));
    assert!(
        errors.contains(
            "terrain.slope : paramètre « dem », type : attendu « RasterPath », trouvé « integer »"
        ),
        "{errors}"
    );
}

#[test]
fn a_changed_enum_or_result_field_fails() {
    let commands = mutate_spec("analysis.isochrone", |s| {
        s.params_schema["properties"]["mode"] = json!({"type": "string", "enum": ["walk", "bike"]});
        let out = s.output_schema["properties"].as_object_mut().unwrap();
        out.remove("crs");
    });
    let errors = joined(&confront(&real_catalogue(), &commands));
    assert!(
        errors.contains("analysis.isochrone : paramètre « mode », valeurs permises"),
        "{errors}"
    );
    assert!(
        errors.contains("analysis.isochrone : champ de résultat « crs » attendu par le catalogue, absent du registre"),
        "{errors}"
    );
}

#[test]
fn enum_order_does_not_matter() {
    let commands = mutate_spec("analysis.isochrone", |s| {
        s.params_schema["properties"]["mode"] = json!({"type": "string", "enum": ["car", "walk"]});
    });
    let errors = confront(&real_catalogue(), &commands);
    assert!(errors.is_empty(), "{}", joined(&errors));
}

#[test]
fn a_named_type_whose_value_changes_fails() {
    let commands = mutate_spec("analysis.buffer", |s| {
        s.params_schema["$defs"]["DistanceUnit"]["oneOf"]
            .as_array_mut()
            .unwrap()
            .push(json!({"type": "string", "const": "mi"}));
    });
    let errors = joined(&confront(&real_catalogue(), &commands));
    assert!(
        errors.contains("type « Distance » : champ « unit », valeurs permises"),
        "{errors}"
    );
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct FakeParams {
    origin: Point,
    targets: Vec<Point>,
    label: Option<String>,
    mode: FakeMode,
    limit: Option<FakeMode>,
    modes: Vec<FakeMode>,
    rule: Option<FakeRule>,
    grid: Option<FakeGrid>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum FakeMode {
    /// Lent.
    Slow,
    /// Rapide.
    Fast,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct FakeRule {
    min: Option<f64>,
    value: f64,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(dead_code)]
enum FakeGrid {
    Degrees { step: f64 },
    Projected { epsg: u32, step: f64 },
}

#[derive(Serialize, JsonSchema)]
struct FakeOutput {
    file: String,
    count: u64,
}

fn fake_spec(id: &str) -> CommandSpec {
    CommandSpec::new::<FakeParams, FakeOutput>(id, "Essai.", "Test.").runs_in(RunsIn::Core)
}

fn set(values: &[&str]) -> Option<BTreeSet<String>> {
    Some(values.iter().map(|v| json!(v).to_string()).collect())
}

#[test]
fn the_normal_form_names_shared_types_and_reads_options_and_enums() {
    let n = normalize(&fake_spec("analysis.fake"));
    let p = |name: &str| n.params[name].clone();
    assert_eq!(p("origin").ty.ty, "Point");
    assert!(p("origin").required);
    assert_eq!(p("targets").ty.ty, "array");
    assert_eq!(p("targets").ty.items.as_deref(), Some(&Ty::named("Point")));
    assert_eq!(p("label").ty.ty, "string");
    assert!(!p("label").required);
    assert_eq!(p("mode").ty.enum_values, set(&["slow", "fast"]));
    assert!(p("mode").required);
    assert_eq!(p("limit").ty.enum_values, set(&["fast", "slow"]));
    assert!(!p("limit").required);
    assert_eq!(
        n.result,
        ["count", "file"].iter().map(|s| s.to_string()).collect()
    );
    assert_eq!(n.runs_in, "core");
    // Un Point du code est comparé à son bloc : ses champs sont lus.
    let point = &n.defs.iter().find(|(name, _)| name == "Point").unwrap().1;
    assert_eq!(point.fields.as_ref().unwrap()["epsg"].ty.ty, "integer");
}

#[test]
fn the_normal_form_reads_item_enums_inline_objects_and_tagged_unions() {
    let n = normalize(&fake_spec("analysis.fake"));
    let modes = &n.params["modes"].ty;
    assert_eq!(modes.ty, "array");
    assert_eq!(
        modes.items.as_ref().unwrap().enum_values,
        set(&["slow", "fast"])
    );
    let rule = &n.params["rule"];
    assert!(!rule.required);
    let fields = rule.ty.fields.as_ref().unwrap();
    assert!(!fields["min"].required);
    assert!(fields["value"].required);
    assert_eq!(fields["value"].ty.ty, "number");
    let grid = &n.params["grid"];
    assert!(!grid.required);
    assert_eq!(grid.ty.ty, UNION);
    assert_eq!(grid.ty.tag.as_deref(), Some("kind"));
    let variants = grid.ty.variants.as_ref().unwrap();
    assert_eq!(
        variants.keys().collect::<Vec<_>>(),
        ["degrees", "projected"]
    );
    assert_eq!(variants["projected"]["epsg"].ty.ty, "integer");
    assert!(!variants["projected"].contains_key("kind"));
}

/// Le catalogue tiré de la commande factice, et la commande dont le schéma est modifié.
fn fake_round_trip(edit: impl FnOnce(&mut CommandSpec)) -> String {
    let spec = fake_spec("analysis.fake");
    let catalogue = parse_catalogue(&catalogue_text_for(&normalize(&spec))).unwrap();
    let mut changed = spec.clone();
    edit(&mut changed);
    joined(&confront(&catalogue, &[normalize(&changed)]))
}

#[test]
fn a_fiche_written_from_the_normal_form_matches_its_command() {
    assert_eq!(fake_round_trip(|_| {}), "");
}

#[test]
fn a_changed_item_enum_fails() {
    let errors = fake_round_trip(|s| {
        s.params_schema["properties"]["modes"]["items"] =
            json!({"type": "string", "enum": ["slow", "medium"]});
    });
    assert!(
        errors.contains("analysis.fake : paramètre « modes[] », valeurs permises"),
        "{errors}"
    );
}

#[test]
fn a_changed_union_variant_or_inline_field_fails() {
    let errors = fake_round_trip(|s| {
        let defs = &mut s.params_schema["$defs"];
        let variants = defs["FakeGrid"]["oneOf"].as_array_mut().unwrap();
        let projected = variants
            .iter_mut()
            .find(|v| v["properties"]["kind"]["const"] == "projected")
            .unwrap();
        projected["properties"]["epsg"] = json!({"type": "string"});
        defs["FakeRule"]["properties"]
            .as_object_mut()
            .unwrap()
            .remove("min");
    });
    assert!(
        errors.contains(
            "analysis.fake : paramètre « grid.projected.epsg », type : attendu « integer », trouvé « string »"
        ),
        "{errors}"
    );
    assert!(
        errors.contains(
            "analysis.fake : paramètre « rule.min » attendu par le catalogue, absent du registre"
        ),
        "{errors}"
    );
}

#[test]
fn a_registered_command_without_fiche_fails_and_names_the_id() {
    let mut commands = registry_commands(&Registry::standard());
    commands.push(normalize(&fake_spec("analysis.fake")));
    let errors = joined(&confront(&real_catalogue(), &commands));
    assert!(
        errors
            .contains("analysis.fake : commande enregistrée au registre, sans fiche au catalogue"),
        "{errors}"
    );
}

#[test]
fn a_done_fiche_without_command_fails_and_names_the_id() {
    let commands: Vec<Normalized> = registry_commands(&Registry::standard())
        .into_iter()
        .filter(|c| c.id != "system.about")
        .collect();
    let errors = joined(&confront(&real_catalogue(), &commands));
    assert!(
        errors.contains("system.about : fiche « faite » sans commande au registre"),
        "{errors}"
    );
}

const SMALL_PLANNED: &str = r#"
### `analysis.fake`

```json fiche
{"id": "analysis.fake", "status": "planifiee", "runs_in": "core", "dictated_by_core": false,
 "params": {"origin": {"type": "Point", "required": true}}, "result": ["file", "count"]}
```
"#;

#[test]
fn a_planned_fiche_already_registered_asks_to_mark_it_done() {
    let catalogue = parse_catalogue(SMALL_PLANNED).unwrap();
    let commands = vec![normalize(&fake_spec("analysis.fake"))];
    let errors = joined(&confront(&catalogue, &commands));
    assert!(
        errors.contains("analysis.fake : la fiche est « planifiee » mais la commande est au registre : passer la fiche en « faite »"),
        "{errors}"
    );
}

#[test]
fn a_reserved_fiche_needs_only_id_status_and_runs_in() {
    let text = "### `terrain.watershed`\n\n```json fiche\n{\"id\": \"terrain.watershed\", \"status\": \"reservee\", \"runs_in\": \"core\"}\n```\n";
    let catalogue = parse_catalogue(text).unwrap();
    assert_eq!(catalogue.fiches[0].status, Status::Reserved);
    assert!(confront(&catalogue, &[]).is_empty());
}

#[test]
fn unreadable_json_names_the_fiche() {
    let text = "### `analysis.fake`\n\n```json fiche\n{\"id\": \"analysis.fake\", \n```\n";
    let errors = joined(&parse_catalogue(text).unwrap_err());
    assert!(
        errors.contains("fiche « `analysis.fake` » (ligne 3) : JSON illisible"),
        "{errors}"
    );
}

#[test]
fn a_missing_field_names_the_fiche_and_the_field() {
    let text = SMALL_PLANNED.replace(r#""runs_in": "core", "#, "");
    let errors = joined(&parse_catalogue(&text).unwrap_err());
    assert!(errors.contains("fiche « `analysis.fake` »"), "{errors}");
    assert!(errors.contains("champ « runs_in » manquant"), "{errors}");
}

#[test]
fn malformed_values_are_named() {
    for (from, to, expected) in [
        (
            r#""status": "planifiee""#,
            r#""status": "prevue""#,
            "champ « status »",
        ),
        (
            r#""runs_in": "core""#,
            r#""runs_in": "both""#,
            "champ « runs_in »",
        ),
        (
            r#""type": "Point""#,
            r#""type": "Coord""#,
            "champ « params.origin » : type « Coord » inconnu",
        ),
        (
            r#""required": true"#,
            r#""required": true, "optional": true"#,
            "champ « params.origin » : clé inconnue « optional »",
        ),
        (
            r#""result": ["file", "count"]"#,
            r#""outputs": []"#,
            "champ inconnu « outputs »",
        ),
        (
            r#""id": "analysis.fake""#,
            r#""id": "AnalysisFake""#,
            "n'est pas en domaine.verbe",
        ),
        (
            r#""type": "Point", "required": true"#,
            r#""type": "object", "required": true"#,
            "champ « params.origin » : un objet en ligne déclare ses champs",
        ),
        (
            r#""type": "Point", "required": true"#,
            r#""type": "union", "required": true, "tag": "kind""#,
            "champ « params.origin » : une union porte « tag »",
        ),
        (
            r#""result": ["file", "count"]"#,
            r#""result": ["file", "count"], "planned_by": "epic 10", "planned_result": ["x"]"#,
            "réservés aux fiches faites",
        ),
        (
            r#""result": ["file", "count"]"#,
            r#""result": ["file", "count"], "planned_result": ["x"]"#,
            "champ « planned_by » (chaîne) va avec",
        ),
    ] {
        let text = SMALL_PLANNED.replace(from, to);
        let errors = joined(&parse_catalogue(&text).unwrap_err());
        assert!(errors.contains(expected), "{expected} :\n{errors}");
    }
}

#[test]
fn a_malformed_type_block_is_named() {
    let text = format!(
        "{SMALL_PLANNED}\n### `Point`\n\n```json type\n{{\"name\": \"Coord\", \"type\": \"object\"}}\n```\n"
    );
    let errors = joined(&parse_catalogue(&text).unwrap_err());
    assert!(
        errors.contains("« Coord » n'est pas un type nommé"),
        "{errors}"
    );
}

#[test]
fn two_fiches_for_one_id_fail() {
    let text = format!("{SMALL_PLANNED}{SMALL_PLANNED}");
    let errors = joined(&parse_catalogue(&text).unwrap_err());
    assert!(
        errors.contains("« analysis.fake » décrit par deux fiches"),
        "{errors}"
    );
}

fn small_with_origin(origin: &str) -> Vec<String> {
    let text = SMALL_PLANNED.replace(r#"{"type": "Point", "required": true}"#, origin);
    parse_catalogue(&text).unwrap_err()
}

#[test]
fn items_written_as_object_are_refused() {
    let errors = joined(&small_with_origin(
        r#"{"type": "array", "required": true, "items": "object"}"#,
    ));
    assert!(
        errors.contains(
            "fiche « `analysis.fake` » (ligne 4) : champ « params.origin » : « items » « object »"
        ),
        "{errors}"
    );
}

#[test]
fn items_written_as_array_are_refused() {
    let errors = joined(&small_with_origin(
        r#"{"type": "array", "required": true, "items": "array"}"#,
    ));
    assert!(
        errors.contains(
            "fiche « `analysis.fake` » (ligne 4) : champ « params.origin » : « items » « array »"
        ),
        "{errors}"
    );
}

#[test]
fn an_array_without_items_is_refused() {
    let errors = joined(&small_with_origin(r#"{"type": "array", "required": true}"#));
    assert!(
        errors.contains("fiche « `analysis.fake` » (ligne 4) : champ « params.origin » : un tableau déclare ses éléments"),
        "{errors}"
    );
}

#[test]
fn replaced_by_names_an_existing_fiche() {
    let text = "### `system.about`\n\n```json fiche\n{\"id\": \"system.about\", \"status\": \"faite\", \"runs_in\": \"core\", \"dictated_by_core\": false, \"params\": {}, \"result\": [], \"replaced_by\": \"system.nothing\"}\n```\n";
    let errors = joined(&parse_catalogue(text).unwrap_err());
    assert!(
        errors.contains("champ « replaced_by » : « system.nothing » n'a pas de fiche au catalogue"),
        "{errors}"
    );
}

/// Le tableau §4 dit, pour chaque fiche, le même id, le même statut, la même exécution, et
/// « dictée » si et seulement si la fiche est dictée par le noyau.
#[test]
fn the_identifier_table_matches_the_fiches() {
    let text = std::fs::read_to_string(catalogue_path()).unwrap();
    let start = text.find("## 4. ").expect("section 4");
    let section = &text[start..];
    let end = section[3..].find("\n## ").map_or(section.len(), |i| i + 3);
    let mut rows = BTreeMap::new();
    for line in section[..end].lines().filter(|l| l.starts_with("| `")) {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let id = cells[1].trim_matches('`').to_string();
        assert!(
            rows.insert(id.clone(), (cells[2].to_string(), cells[3].to_string()))
                .is_none(),
            "{id} : deux lignes au tableau"
        );
    }
    let catalogue = real_catalogue();
    let ids: BTreeSet<&String> = catalogue.fiches.iter().map(|f| &f.id).collect();
    assert_eq!(
        rows.keys().collect::<BTreeSet<_>>(),
        ids,
        "ids du tableau et des fiches"
    );
    for fiche in &catalogue.fiches {
        let (status, runs_in) = &rows[&fiche.id];
        let expected = match fiche.status {
            Status::Done => "faite",
            Status::Planned => "planifiée",
            Status::Reserved => "réservée",
        };
        assert!(
            status.starts_with(expected),
            "{} : statut « {status} »",
            fiche.id
        );
        assert_eq!(
            status.contains("dictée"),
            fiche.dictated_by_core,
            "{} : mention « dictée »",
            fiche.id
        );
        assert_eq!(runs_in, &fiche.runs_in, "{} : exécution", fiche.id);
    }
}
