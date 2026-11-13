//! Grammaire GBNF des paramètres d'un plan, tirée des schémas du registre (ticket 5.56).
//!
//! Le traducteur part du schéma JSON que le modèle voit dans le catalogue (`catalog_entry`, marques
//! `x-cassini` comprises, qui ne servent qu'à reconnaître un `Point`) et rend, pour chaque commande
//! offerte, une règle qui n'admet que ses champs connus, dans l'ordre trié des clés (celui du
//! prompt et de l'entraînement, sous-objets compris), avec leurs types, leurs `enum`/`const` et
//! leurs champs requis. Chaque paramètre de premier niveau admet aussi une référence `step:`, et
//! chaque élément d'un tableau de `Point` aussi : ce sont les seuls endroits où `check_plan` les
//! admet.
//!
//! Seuls le signe des entiers (`minimum ≥ 0`) et `minItems`/`maxItems` sont traduits ; les autres
//! bornes, et tout contrôle propre à la désérialisation, restent à `check_plan`, qui garde le
//! dernier mot (AD-23). Une construction que le traducteur ne connaît pas fait retomber ce seul
//! champ sur le JSON libre (`value`), et le repli est noté.
//!
//! La traduction est déterministe : mêmes schémas, même texte, à l'octet.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

/// Longueur maximale d'une question ou d'un refus, en caractères (un échappement JSON compte pour
/// un) : décision du propriétaire, 2026-11-11.
pub const MAX_TEXT_CHARS: usize = 240;

/// Plafond de llama.cpp sur une répétition bornée (`MAX_REPETITION_THRESHOLD`) : au-delà, la borne
/// haute d'un tableau n'est pas exprimée.
const MAX_REPETITION: u64 = 2000;

/// Les règles communes : le JSON libre (le repli), les textes bornés, les nombres, la référence.
pub(crate) fn base_rules() -> String {
    let hex = "[0-9a-fA-F]";
    format!(
        "value ::= object | array | string | number | \"true\" | \"false\" | \"null\"\n\
         object ::= \"{{\" (pair (\",\" pair)*)? \"}}\"\n\
         pair ::= string \":\" value\n\
         array ::= \"[\" (value (\",\" value)*)? \"]\"\n\
         string ::= \"\\\"\" char* \"\\\"\"\n\
         char ::= [^\"\\\\\\x00-\\x1f] | \"\\\\\" ([\"\\\\/bfnrt] | \"u\" {hex} {hex} {hex} {hex})\n\
         text ::= \"\\\"\" char{{1,{MAX_TEXT_CHARS}}} \"\\\"\"\n\
         number ::= \"-\"? (\"0\" | [1-9] [0-9]*) (\".\" [0-9]+)? ([eE] [-+]? [0-9]+)?\n\
         integer ::= \"-\"? integer-nn\n\
         integer-nn ::= \"0\" | [1-9] [0-9]*\n\
         boolean ::= \"true\" | \"false\"\n\
         ref ::= \"\\\"step:\" [1-9] [0-9]* (\".\" [a-zA-Z0-9_]+)? \"\\\"\"\n"
    )
}

/// Les noms des règles communes et de l'enveloppe du plan, réservés.
const BASE_NAMES: [&str; 18] = [
    "root",
    "calls",
    "steps",
    "step",
    "refusal",
    "question",
    "value",
    "object",
    "pair",
    "array",
    "string",
    "char",
    "text",
    "number",
    "integer",
    "integer-nn",
    "boolean",
    "ref",
];

/// Les mots-clés de schéma qui ne contraignent rien que la grammaire doive porter : annotations,
/// et bornes laissées à `check_plan` (Décisions du ticket 5.56).
const IGNORED_KEYWORDS: [&str; 16] = [
    "$schema",
    "$comment",
    "title",
    "description",
    "default",
    "examples",
    "format",
    "deprecated",
    "readOnly",
    "writeOnly",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "minLength",
    "maxLength",
];

/// Les mots-clés que le traducteur sait lire.
const KNOWN_KEYWORDS: [&str; 13] = [
    "$ref",
    "$defs",
    "type",
    "enum",
    "const",
    "oneOf",
    "anyOf",
    "properties",
    "required",
    "additionalProperties",
    "items",
    "minItems",
    "maxItems",
];

/// Une grammaire de paramètres construite : ses règles, et les champs retombés sur le JSON libre.
#[derive(Debug, Default)]
pub struct ParamsGrammar {
    /// Les règles, une par ligne, dans l'ordre de construction.
    rules: Vec<String>,
    /// Noms déjà pris (règles communes comprises).
    names: BTreeSet<String>,
    /// Règle déjà construite pour une définition, par nom, contenu résolu et admission des
    /// références par élément ; avec les replis de sa construction, relatifs à la définition.
    defs: BTreeMap<(String, String, bool), (String, Vec<String>)>,
    /// Les champs retombés sur `value`, chacun par son chemin (`commande/champ/...`).
    pub fallbacks: Vec<String>,
}

/// Le contexte d'un nœud : la racine du schéma de sa commande (pour les `$ref`), et si les éléments
/// `Point` d'un tableau admettent une référence (paramètre de premier niveau tableau de `Point`,
/// reconnu comme le fait `check_plan`).
#[derive(Clone, Copy)]
struct Ctx<'a> {
    root: &'a Value,
    point_refs: bool,
}

/// Échappe `text` en littéral GBNF entre guillemets.
pub(crate) fn literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Nom de règle tiré de `text` : lettres et chiffres gardés, tout le reste en `-` (llama.cpp
/// n'admet que `[a-zA-Z0-9-]` dans un nom).
fn sanitize(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Un terme est-il un nom de règle seul (sans espace, sans guillemet, sans parenthèse) ?
fn is_rule_name(term: &str) -> bool {
    !term.is_empty() && term.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

impl ParamsGrammar {
    /// Une grammaire vide, les noms des règles communes réservés.
    pub fn new() -> Self {
        ParamsGrammar {
            names: BASE_NAMES.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    /// Les règles construites, une par ligne, chacune finie par `\n`.
    pub fn text(&self) -> String {
        self.rules.iter().map(|r| format!("{r}\n")).collect()
    }

    /// Un nom de règle libre, tiré de `base` ; un suffixe `-2`, `-3`… en cas de collision.
    fn fresh(&mut self, base: &str) -> String {
        let base = sanitize(base);
        let mut name = base.clone();
        let mut n = 2;
        while self.names.contains(&name) {
            name = format!("{base}-{n}");
            n += 1;
        }
        self.names.insert(name.clone());
        name
    }

    fn define(&mut self, name: &str, body: &str) {
        self.rules.push(format!("{name} ::= {body}"));
    }

    /// Une règle nommée d'après `base`, de corps `body` ; son nom.
    fn rule(&mut self, base: &str, body: &str) -> String {
        let name = self.fresh(base);
        self.define(&name, body);
        name
    }

    /// Le terme `term`, rendu nom de règle (une règle est créée s'il ne l'est pas).
    fn named(&mut self, base: &str, term: String) -> String {
        if is_rule_name(&term) {
            term
        } else {
            self.rule(base, &term)
        }
    }

    fn fallback(&mut self, path: &str) -> String {
        self.fallbacks.push(path.to_string());
        "value".to_string()
    }

    /// Repli des paramètres entiers d'une commande : un objet JSON libre (jamais une chaîne, un
    /// tableau ou `null`), noté à part d'un repli de champ.
    fn params_fallback(&mut self, id: &str) -> String {
        self.fallbacks.push(format!("{id} (tous les paramètres)"));
        "object".to_string()
    }

    /// Ajoute la règle des paramètres de la commande `id`, de schéma `params` (celui du catalogue,
    /// marques comprises) ; rend son nom. Chaque paramètre de premier niveau admet aussi `ref`, et
    /// chaque élément d'un tableau de `Point` aussi.
    pub fn command_params(&mut self, id: &str, params: &Value) -> String {
        let base = format!("p-{id}");
        let Some(map) = params.as_object() else {
            return self.params_fallback(id);
        };
        if map.get("type").and_then(Value::as_str) != Some("object")
            || !closed(params)
            || !only_known(params)
        {
            return self.params_fallback(id);
        }
        let props = map
            .get("properties")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut fields = Vec::with_capacity(props.len());
        let mut keys: Vec<&String> = props.keys().collect();
        keys.sort();
        for key in keys {
            let schema = &props[key];
            let path = format!("{id}/{key}");
            let field_base = format!("{base}-{key}");
            // Même détection que `check_plan` : `$ref`, alternatives et `type` en tableau suivis.
            let ctx = Ctx {
                root: params,
                point_refs: cassini_core::plan::point_items_schema(schema, params).is_some(),
            };
            let term = self.node(&ctx, schema, &field_base, &path);
            let term = if term == "string" || term == "value" {
                // Une chaîne libre (ou le JSON libre) admet déjà la référence.
                term
            } else {
                self.rule(&field_base, &format!("{term} | ref"))
            };
            fields.push((key.clone(), term));
        }
        let required = required_set(params);
        self.object(&base, &fields, &required)
    }

    /// Ajoute la règle d'une étape de la commande `id` : `{"command":"<id>","params":<params>}`,
    /// les paramètres tirés de `params` (voir `command_params`) ; rend son nom.
    pub fn step(&mut self, id: &str, params: &Value) -> String {
        let params = self.command_params(id, params);
        let id_json = serde_json::to_string(id).expect("un identifiant se sérialise");
        let head = literal(&format!("{{\"command\":{id_json},\"params\":"));
        self.rule(&format!("c-{id}"), &format!("{head} {params} \"}}\""))
    }

    /// Le terme d'un nœud de schéma : un nom de règle, un littéral, ou un groupe parenthésé.
    fn node(&mut self, ctx: &Ctx, schema: &Value, base: &str, path: &str) -> String {
        let map = match schema {
            Value::Object(map) => map,
            // `true` (toute valeur) est aussi un champ libre : noté comme repli.
            _ => return self.fallback(path),
        };
        if !only_known(schema) {
            return self.fallback(path);
        }
        if let Some(target) = map.get("$ref") {
            // Les clés voisines d'un `$ref` (`default`, `description`) ne contraignent rien ici.
            let structural = [
                "type",
                "enum",
                "const",
                "oneOf",
                "anyOf",
                "properties",
                "items",
            ];
            if structural.iter().any(|k| map.contains_key(*k)) {
                return self.fallback(path);
            }
            return self.reference(ctx, target, path);
        }
        if let Some(value) = map.get("const") {
            return json_literal(value);
        }
        if let Some(values) = map.get("enum") {
            let Some(values) = values.as_array().filter(|v| !v.is_empty()) else {
                return self.fallback(path);
            };
            let alts: Vec<String> = values.iter().map(json_literal).collect();
            return group(&alts);
        }
        for key in ["oneOf", "anyOf"] {
            if let Some(variants) = map.get(key) {
                let Some(variants) = variants.as_array().filter(|v| !v.is_empty()) else {
                    return self.fallback(path);
                };
                if ["type", "properties", "items"]
                    .iter()
                    .any(|k| map.contains_key(*k))
                {
                    return self.fallback(path);
                }
                let mut alts = Vec::with_capacity(variants.len());
                for (i, variant) in variants.iter().enumerate() {
                    let term = self.node(ctx, variant, &format!("{base}-{i}"), path);
                    alts.push(term);
                }
                return group(&alts);
            }
        }
        let types: Vec<&str> = match map.get("type") {
            // Sans type ni mot-clé structurel, toute valeur JSON est admise : un champ libre, noté
            // comme repli.
            None => return self.fallback(path),
            Some(Value::String(t)) => vec![t.as_str()],
            Some(Value::Array(ts)) => {
                let named: Option<Vec<&str>> = ts.iter().map(Value::as_str).collect();
                match named {
                    Some(n) if !n.is_empty() => n,
                    _ => return self.fallback(path),
                }
            }
            Some(_) => return self.fallback(path),
        };
        let mut alts = Vec::with_capacity(types.len());
        for t in types {
            let term = match t {
                "string" => "string".to_string(),
                "number" => "number".to_string(),
                "integer" => {
                    let unsigned = map
                        .get("minimum")
                        .and_then(Value::as_f64)
                        .is_some_and(|m| m >= 0.0);
                    if unsigned { "integer-nn" } else { "integer" }.to_string()
                }
                "boolean" => "boolean".to_string(),
                "null" => "\"null\"".to_string(),
                "object" => {
                    if !closed(schema) {
                        return self.fallback(path);
                    }
                    let props = map
                        .get("properties")
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    let mut keys: Vec<&String> = props.keys().collect();
                    keys.sort();
                    let mut fields = Vec::with_capacity(keys.len());
                    // Un sous-objet n'est plus au premier niveau : pas de référence par élément.
                    let inner = Ctx {
                        point_refs: false,
                        ..*ctx
                    };
                    for key in keys {
                        let term = self.node(
                            &inner,
                            &props[key],
                            &format!("{base}-{key}"),
                            &format!("{path}/{key}"),
                        );
                        fields.push((key.clone(), term));
                    }
                    let required = required_set(schema);
                    self.object(base, &fields, &required)
                }
                "array" => {
                    let item = match map.get("items") {
                        None => "value".to_string(),
                        Some(items) => {
                            let inner = Ctx {
                                point_refs: false,
                                ..*ctx
                            };
                            let term = self.node(
                                &inner,
                                items,
                                &format!("{base}-item"),
                                &format!("{path}/[]"),
                            );
                            if ctx.point_refs && cassini_core::point::is_point(items, ctx.root) {
                                // Un élément d'un tableau de `Point` de premier niveau : un point
                                // ou `step:k.champ` (amendement d'AD-23, ticket 11.22).
                                self.rule(&format!("{base}-item"), &format!("{term} | ref"))
                            } else {
                                self.named(&format!("{base}-item"), term)
                            }
                        }
                    };
                    match self.array(schema, item, base) {
                        Some(array) => array,
                        None => return self.fallback(path),
                    }
                }
                _ => return self.fallback(path),
            };
            alts.push(term);
        }
        group(&alts)
    }

    /// La règle d'une définition `#/$defs/<Name>`, construite une fois par nom et contenu.
    fn reference(&mut self, ctx: &Ctx, target: &Value, path: &str) -> String {
        let Some(name) = target.as_str().and_then(def_name) else {
            return self.fallback(path);
        };
        let Some(def) = ctx.root.get("$defs").and_then(|d| d.get(&name)) else {
            return self.fallback(path);
        };
        let Some(resolved) = inline(def, ctx.root, 0) else {
            return self.fallback(path);
        };
        let prefix = format!("{path}/{name}");
        let key = (name.clone(), resolved.to_string(), ctx.point_refs);
        if let Some((rule, suffixes)) = self.defs.get(&key).cloned() {
            // Les replis de la définition valent aussi sous ce nouveau chemin.
            self.fallbacks
                .extend(suffixes.iter().map(|s| format!("{prefix}{s}")));
            return rule;
        }
        // Le nom est réservé avant la traduction : une définition s'écrit avant ce qui la cite.
        let rule = self.fresh(&format!("d-{name}"));
        self.defs.insert(key.clone(), (rule.clone(), Vec::new()));
        let start = self.fallbacks.len();
        let term = self.node(ctx, def, &rule, &prefix);
        let suffixes = self.fallbacks[start..]
            .iter()
            .map(|f| f.strip_prefix(&prefix).unwrap_or(f).to_string())
            .collect();
        self.defs.insert(key, (rule.clone(), suffixes));
        self.define(&rule, &term);
        rule
    }

    /// La règle d'un objet de champs `fields` (clés triées), dont `required` sont requis : chaque
    /// champ facultatif peut être omis, et une virgule sépare exactement deux champs écrits.
    fn object(
        &mut self,
        base: &str,
        fields: &[(String, String)],
        required: &BTreeSet<String>,
    ) -> String {
        let name = self.fresh(base);
        let mut memo: BTreeMap<(usize, bool), Option<String>> = BTreeMap::new();
        let tail = self.tail(&name, fields, required, 0, false, &mut memo);
        let body = match tail {
            Some(t) => format!("\"{{\" {t} \"}}\""),
            None => "\"{}\"".to_string(),
        };
        self.define(&name, &body);
        name
    }

    /// Le reste d'un objet à partir du champ `i` ; `started` dit si un champ est déjà écrit (une
    /// virgule précède alors le suivant). `None` quand il ne reste rien à écrire.
    #[allow(clippy::too_many_arguments)]
    fn tail(
        &mut self,
        object: &str,
        fields: &[(String, String)],
        required: &BTreeSet<String>,
        i: usize,
        started: bool,
        memo: &mut BTreeMap<(usize, bool), Option<String>>,
    ) -> Option<String> {
        if i == fields.len() {
            return None;
        }
        if let Some(done) = memo.get(&(i, started)) {
            return done.clone();
        }
        let (key, term) = &fields[i];
        let comma = if started { "," } else { "" };
        let key_json = serde_json::to_string(key).expect("une clé se sérialise");
        let mut written = format!("{} {term}", literal(&format!("{comma}{key_json}:")));
        if let Some(rest) = self.tail(object, fields, required, i + 1, true, memo) {
            written = format!("{written} {rest}");
        }
        let body = if required.contains(key) {
            written
        } else {
            match self.tail(object, fields, required, i + 1, started, memo) {
                Some(skip) => format!("{written} | {skip}"),
                None => format!("({written})?"),
            }
        };
        let name = self.rule(
            &format!("{object}-{i}{}", if started { "b" } else { "a" }),
            &body,
        );
        memo.insert((i, started), Some(name.clone()));
        Some(name)
    }

    /// Le terme d'un tableau d'éléments `item` (un nom de règle), borné par `minItems`/`maxItems` ;
    /// `None` quand les bornes ne se traduisent pas (au-delà du plafond de llama.cpp, illisibles
    /// ou contradictoires) : le champ se replie.
    fn array(&mut self, schema: &Value, item: String, base: &str) -> Option<String> {
        let bound = |key: &str| -> Result<Option<u64>, ()> {
            match schema.get(key) {
                None => Ok(None),
                Some(v) => v
                    .as_u64()
                    .filter(|n| *n <= MAX_REPETITION)
                    .map(Some)
                    .ok_or(()),
            }
        };
        let min = bound("minItems").ok()?.unwrap_or(0);
        let max = bound("maxItems").ok()?;
        if max.is_some_and(|m| m < min) {
            return None;
        }
        if max == Some(0) {
            return Some("\"[]\"".to_string());
        }
        let more = |from: u64| -> String {
            let sep = format!("(\",\" {item})");
            match max {
                Some(m) if m - 1 == from => {
                    if from == 0 {
                        String::new()
                    } else {
                        format!(" {sep}{{{from}}}")
                    }
                }
                Some(m) => format!(" {sep}{{{from},{}}}", m - 1),
                None if from == 0 => format!(" {sep}*"),
                None => format!(" {sep}{{{from},}}"),
            }
        };
        Some(if min >= 1 {
            let list = format!("{item}{}", more(min - 1));
            self.rule(&format!("{base}-list"), &format!("\"[\" {list} \"]\""))
        } else {
            let list = self.rule(&format!("{base}-list"), &format!("{item}{}", more(0)));
            self.rule(&format!("{base}-array"), &format!("\"[\" {list}? \"]\""))
        })
    }
}

/// Le littéral GBNF d'une valeur JSON, écrite compacte.
fn json_literal(value: &Value) -> String {
    literal(&value.to_string())
}

/// Des alternatives, groupées s'il y en a plusieurs.
fn group(alts: &[String]) -> String {
    let mut unique: Vec<&String> = Vec::with_capacity(alts.len());
    for a in alts {
        if !unique.contains(&a) {
            unique.push(a);
        }
    }
    if unique.len() == 1 {
        unique[0].clone()
    } else {
        format!(
            "({})",
            unique
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" | ")
        )
    }
}

/// Un objet n'admet que ses champs déclarés (`additionalProperties: false`).
fn closed(schema: &Value) -> bool {
    schema.get("additionalProperties") == Some(&Value::Bool(false))
}

/// Tous les mots-clés du nœud sont connus du traducteur (lus ou ignorés à dessein). La marque
/// `x-cassini` du noyau est une annotation.
fn only_known(schema: &Value) -> bool {
    schema.as_object().is_some_and(|map| {
        map.keys().all(|k| {
            KNOWN_KEYWORDS.contains(&k.as_str())
                || IGNORED_KEYWORDS.contains(&k.as_str())
                || k == cassini_core::point::MARK_KEY
        })
    })
}

/// Les champs requis d'un objet.
fn required_set(schema: &Value) -> BTreeSet<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|r| {
            r.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Le nom d'une définition visée par `target`, pointeur JSON `#/$defs/<nom>` décodé (`~1` vaut
/// `/`, `~0` vaut `~`) ; `None` pour toute autre cible.
fn def_name(target: &str) -> Option<String> {
    let raw = target.strip_prefix("#/$defs/")?;
    if raw.is_empty() || raw.contains('/') {
        return None;
    }
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '~' {
            match chars.next() {
                Some('0') => out.push('~'),
                Some('1') => out.push('/'),
                _ => return None,
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// Le nœud `schema`, ses `$ref` remplacés par leur définition : la clé de mémoire d'une règle de
/// définition. `None` au-delà d'une profondeur sûre (définition récursive).
fn inline(schema: &Value, root: &Value, depth: usize) -> Option<Value> {
    if depth > 32 {
        return None;
    }
    match schema {
        Value::Object(map) => {
            if let Some(target) = map.get("$ref").and_then(Value::as_str) {
                let def = root.get("$defs")?.get(def_name(target)?)?;
                let mut out = map.clone();
                out.remove("$ref");
                out.insert("$ref".into(), inline(def, root, depth + 1)?);
                return Some(Value::Object(out));
            }
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                out.insert(k.clone(), inline(v, root, depth + 1)?);
            }
            Some(Value::Object(out))
        }
        Value::Array(items) => items
            .iter()
            .map(|v| inline(v, root, depth + 1))
            .collect::<Option<Vec<_>>>()
            .map(Value::Array),
        other => Some(other.clone()),
    }
}

/// Un petit vérificateur d'appartenance, réservé aux tests : il lit le GBNF émis (pas l'arbre du
/// traducteur) et dit si un texte en est une phrase complète, depuis `root`. Il couvre la syntaxe
/// que ce module émet : littéraux, classes de caractères, références, groupes, alternatives et
/// répétitions `? * + {m} {m,} {m,n}`.
#[cfg(test)]
pub(crate) mod check {
    use std::collections::{BTreeSet, HashMap};

    #[derive(Debug, Clone)]
    enum Expr {
        Lit(Vec<char>),
        Class {
            negated: bool,
            ranges: Vec<(char, char)>,
        },
        Rule(usize),
        Seq(Vec<Expr>),
        Alt(Vec<Expr>),
        Repeat(Box<Expr>, usize, Option<usize>),
    }

    /// Une grammaire lue.
    pub struct Grammar {
        rules: Vec<Expr>,
        root: usize,
    }

    struct Parser<'a> {
        chars: Vec<char>,
        pos: usize,
        names: &'a HashMap<String, usize>,
    }

    impl Parser<'_> {
        fn peek(&self) -> Option<char> {
            self.chars.get(self.pos).copied()
        }
        fn skip_space(&mut self) {
            while self.peek() == Some(' ') {
                self.pos += 1;
            }
        }
        fn escaped(&mut self) -> char {
            let c = self.chars[self.pos];
            self.pos += 1;
            if c != '\\' {
                return c;
            }
            let e = self.chars[self.pos];
            self.pos += 1;
            match e {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'x' => {
                    let hex: String = self.chars[self.pos..self.pos + 2].iter().collect();
                    self.pos += 2;
                    char::from_u32(u32::from_str_radix(&hex, 16).unwrap()).unwrap()
                }
                other => other,
            }
        }
        fn alternates(&mut self) -> Expr {
            let mut alts = vec![self.sequence()];
            while self.peek() == Some('|') {
                self.pos += 1;
                self.skip_space();
                alts.push(self.sequence());
            }
            if alts.len() == 1 {
                alts.pop().unwrap()
            } else {
                Expr::Alt(alts)
            }
        }
        fn sequence(&mut self) -> Expr {
            let mut items = Vec::new();
            loop {
                self.skip_space();
                let Some(c) = self.peek() else { break };
                let item = match c {
                    '"' => {
                        self.pos += 1;
                        let mut lit = Vec::new();
                        while self.peek() != Some('"') {
                            lit.push(self.escaped());
                        }
                        self.pos += 1;
                        Expr::Lit(lit)
                    }
                    '[' => {
                        self.pos += 1;
                        let negated = self.peek() == Some('^');
                        if negated {
                            self.pos += 1;
                        }
                        let mut ranges = Vec::new();
                        while self.peek() != Some(']') {
                            let lo = self.escaped();
                            let hi = if self.peek() == Some('-') && self.chars[self.pos + 1] != ']'
                            {
                                self.pos += 1;
                                self.escaped()
                            } else {
                                lo
                            };
                            ranges.push((lo, hi));
                        }
                        self.pos += 1;
                        Expr::Class { negated, ranges }
                    }
                    '(' => {
                        self.pos += 1;
                        self.skip_space();
                        let inner = self.alternates();
                        assert_eq!(self.peek(), Some(')'), "parenthèse non fermée");
                        self.pos += 1;
                        inner
                    }
                    c if c.is_ascii_alphanumeric() || c == '-' => {
                        let start = self.pos;
                        while self
                            .peek()
                            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-')
                        {
                            self.pos += 1;
                        }
                        let name: String = self.chars[start..self.pos].iter().collect();
                        let id = *self
                            .names
                            .get(&name)
                            .unwrap_or_else(|| panic!("règle non définie : {name}"));
                        Expr::Rule(id)
                    }
                    _ => break,
                };
                let item = self.postfix(item);
                items.push(item);
            }
            if items.len() == 1 {
                items.pop().unwrap()
            } else {
                Expr::Seq(items)
            }
        }
        fn number(&mut self) -> usize {
            let start = self.pos;
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.pos += 1;
            }
            self.chars[start..self.pos]
                .iter()
                .collect::<String>()
                .parse()
                .unwrap()
        }
        fn postfix(&mut self, mut item: Expr) -> Expr {
            loop {
                let (min, max) = match self.peek() {
                    Some('*') => {
                        self.pos += 1;
                        (0, None)
                    }
                    Some('+') => {
                        self.pos += 1;
                        (1, None)
                    }
                    Some('?') => {
                        self.pos += 1;
                        (0, Some(1))
                    }
                    Some('{') => {
                        self.pos += 1;
                        let min = self.number();
                        let max = if self.peek() == Some(',') {
                            self.pos += 1;
                            if self.peek() == Some('}') {
                                None
                            } else {
                                Some(self.number())
                            }
                        } else {
                            Some(min)
                        };
                        assert_eq!(self.peek(), Some('}'));
                        self.pos += 1;
                        (min, max)
                    }
                    _ => return item,
                };
                item = Expr::Repeat(Box::new(item), min, max);
            }
        }
    }

    impl Grammar {
        /// Lit le texte GBNF `text`, une règle par ligne.
        pub fn parse(text: &str) -> Grammar {
            let lines: Vec<(&str, &str)> = text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.split_once(" ::= ").expect("une règle par ligne"))
                .collect();
            let mut names = HashMap::new();
            for (i, (name, _)) in lines.iter().enumerate() {
                assert!(
                    names.insert(name.to_string(), i).is_none(),
                    "règle définie deux fois : {name}"
                );
            }
            let rules = lines
                .iter()
                .map(|(name, body)| {
                    let mut p = Parser {
                        chars: body.chars().collect(),
                        pos: 0,
                        names: &names,
                    };
                    let e = p.alternates();
                    assert_eq!(p.pos, p.chars.len(), "règle {name} mal lue");
                    e
                })
                .collect();
            let root = names["root"];
            Grammar { rules, root }
        }

        /// Le texte `text` est-il une phrase complète de la grammaire ?
        pub fn accepts(&self, text: &str) -> bool {
            let chars: Vec<char> = text.chars().collect();
            let mut m = Matcher {
                g: self,
                chars: &chars,
                memo: HashMap::new(),
            };
            m.rule(self.root, 0).contains(&chars.len())
        }
    }

    struct Matcher<'a> {
        g: &'a Grammar,
        chars: &'a [char],
        memo: HashMap<(usize, usize), BTreeSet<usize>>,
    }

    impl Matcher<'_> {
        fn rule(&mut self, id: usize, pos: usize) -> BTreeSet<usize> {
            if let Some(done) = self.memo.get(&(id, pos)) {
                return done.clone();
            }
            let expr = self.g.rules[id].clone();
            let ends = self.expr(&expr, pos);
            self.memo.insert((id, pos), ends.clone());
            ends
        }

        fn expr(&mut self, e: &Expr, pos: usize) -> BTreeSet<usize> {
            match e {
                Expr::Lit(lit) => {
                    let end = pos + lit.len();
                    if end <= self.chars.len() && self.chars[pos..end] == lit[..] {
                        BTreeSet::from([end])
                    } else {
                        BTreeSet::new()
                    }
                }
                Expr::Class { negated, ranges } => match self.chars.get(pos) {
                    Some(c) if ranges.iter().any(|(lo, hi)| lo <= c && c <= hi) != *negated => {
                        BTreeSet::from([pos + 1])
                    }
                    _ => BTreeSet::new(),
                },
                Expr::Rule(id) => self.rule(*id, pos),
                Expr::Seq(items) => {
                    let mut cur = BTreeSet::from([pos]);
                    for item in items {
                        let mut next = BTreeSet::new();
                        for p in cur {
                            next.extend(self.expr(item, p));
                        }
                        if next.is_empty() {
                            return next;
                        }
                        cur = next;
                    }
                    cur
                }
                Expr::Alt(alts) => {
                    let mut out = BTreeSet::new();
                    for a in alts {
                        out.extend(self.expr(a, pos));
                    }
                    out
                }
                Expr::Repeat(inner, min, max) => {
                    let mut out = BTreeSet::new();
                    let mut cur = BTreeSet::from([pos]);
                    let mut seen = BTreeSet::new();
                    let mut count = 0;
                    loop {
                        if count >= *min {
                            out.extend(cur.iter().copied());
                        }
                        if max.is_some_and(|m| count >= m) || cur.is_empty() {
                            break;
                        }
                        let mut next = BTreeSet::new();
                        for p in &cur {
                            next.extend(self.expr(inner, *p));
                        }
                        count += 1;
                        if count > *min {
                            // Au-delà du minimum, une position déjà vue n'apporte rien.
                            next.retain(|p| !seen.contains(p));
                            seen.extend(next.iter().copied());
                        }
                        cur = next;
                    }
                    out
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::check::Grammar;
    use super::*;
    use serde_json::json;

    /// Une grammaire d'essai : la racine est la règle de paramètres de `params`.
    fn grammar_of(params: &Value) -> (Grammar, ParamsGrammar) {
        let mut g = ParamsGrammar::new();
        let root = g.command_params("test.cmd", params);
        let text = format!("root ::= {root}\n{}{}", g.text(), base_rules());
        (Grammar::parse(&text), g)
    }

    fn schema() -> Value {
        json!({
            "type": "object", "additionalProperties": false,
            "properties": {
                "b": {"type": "string"},
                "a": {"default": null, "type": ["integer", "null"], "minimum": 0},
                "c": {"$ref": "#/$defs/U", "default": "x"},
                "d": {"type": "array", "items": {"type": "number"}, "minItems": 1, "maxItems": 2},
            },
            "required": ["b", "d"],
            "$defs": {"U": {"oneOf": [{"const": "x", "type": "string"}, {"const": "y", "type": "string"}]}}
        })
    }

    #[test]
    fn sorted_keys_optional_fields_and_commas() {
        let (g, built) = grammar_of(&schema());
        assert!(built.fallbacks.is_empty());
        for ok in [
            r#"{"b":"t","d":[1]}"#,
            r#"{"a":3,"b":"t","d":[1,2.5]}"#,
            r#"{"a":null,"b":"t","c":"y","d":[1]}"#,
            r#"{"b":"step:1.layer_id","c":"step:2","d":"step:1.values"}"#,
        ] {
            assert!(g.accepts(ok), "{ok}");
        }
        for bad in [
            r#"{"d":[1],"b":"t"}"#,          // ordre non trié
            r#"{"b":"t"}"#,                  // requis absent
            r#"{"b":"t","d":[]}"#,           // minItems
            r#"{"b":"t","d":[1,2,3]}"#,      // maxItems
            r#"{"a":-1,"b":"t","d":[1]}"#,   // entier sans signe
            r#"{"b":"t","c":"z","d":[1]}"#,  // hors énumération
            r#"{"b":"t","d":[1],"e":1}"#,    // champ inconnu
            r#"{,"b":"t","d":[1]}"#,         // virgule de tête
            r#"{"b":"t",,"d":[1]}"#,         // virgule double
            r#"{"b":"t","d":["step:1.x"]}"#, // référence imbriquée
            r#"{"b":"t","d":"step:0"}"#,     // étape zéro
            r#"{"b":"t","d":"step:1.a.b"}"#, // deux champs
            r#"{"b":"t", "d":[1]}"#,         // espace
        ] {
            assert!(!g.accepts(bad), "{bad}");
        }
    }

    #[test]
    fn an_unknown_construction_falls_back_on_that_field_only() {
        let mut s = schema();
        s["properties"]["b"] = json!({"type": "string", "pattern": "^a"});
        let (g, built) = grammar_of(&s);
        assert_eq!(built.fallbacks, ["test.cmd/b"]);
        assert!(g.accepts(r#"{"b":{"any":[1]},"d":[1]}"#));
        assert!(!g.accepts(r#"{"b":"t","d":[1],"e":1}"#));
    }

    /// Un objet de paramètres d'essai, de champs `props` (tous facultatifs) et de définitions `defs`.
    fn params_of(props: Value, defs: Value) -> Value {
        json!({"type": "object", "additionalProperties": false, "properties": props, "$defs": defs})
    }

    fn point_def() -> Value {
        json!({"P": {"type": "object", "additionalProperties": false, "x-cassini": "point",
            "properties": {"x": {"type": "number"}, "y": {"type": "number"}},
            "required": ["x", "y"]}})
    }

    #[test]
    fn a_free_field_counts_as_a_fallback() {
        let (g, built) = grammar_of(&params_of(
            json!({"a": true, "b": {"description": "libre"}}),
            json!({}),
        ));
        assert_eq!(built.fallbacks, ["test.cmd/a", "test.cmd/b"]);
        assert!(g.accepts(r#"{"a":[1],"b":{"c":null}}"#));
    }

    #[test]
    fn untranslatable_params_fall_back_on_a_free_object_noted_apart() {
        let (g, built) = grammar_of(&json!({"type": "object", "properties": {}}));
        assert_eq!(built.fallbacks, ["test.cmd (tous les paramètres)"]);
        assert!(g.accepts(r#"{"any":1}"#));
        for bad in [r#""x""#, "[1]", "null"] {
            assert!(!g.accepts(bad), "{bad}");
        }
    }

    #[test]
    fn point_arrays_admit_references_as_check_plan_sees_them() {
        let props = json!({
            "a": {"anyOf": [{"type": "array", "items": {"$ref": "#/$defs/P"}}, {"type": "null"}]},
            "b": {"type": ["array", "null"], "items": {"$ref": "#/$defs/P"}},
            "c": {"type": "array", "items": {"type": "object", "additionalProperties": false,
                  "properties": {"p": {"type": "array", "items": {"$ref": "#/$defs/P"}}}}},
        });
        let (g, built) = grammar_of(&params_of(props, point_def()));
        assert!(built.fallbacks.is_empty(), "{:?}", built.fallbacks);
        assert!(g.accepts(r#"{"a":["step:1.point",{"x":1,"y":2}],"b":["step:2.point"]}"#));
        assert!(g.accepts(r#"{"a":null,"b":null}"#));
        // Imbriqué dans un objet : plus au premier niveau, pas de référence par élément.
        assert!(g.accepts(r#"{"c":[{"p":[{"x":1,"y":2}]}]}"#));
        assert!(!g.accepts(r#"{"c":[{"p":["step:1.point"]}]}"#));
        // Un mot-clé inconnu sur un tableau de points : repli noté.
        let props =
            json!({"a": {"type": "array", "uniqueItems": true, "items": {"$ref": "#/$defs/P"}}});
        let (_, built) = grammar_of(&params_of(props, point_def()));
        assert_eq!(built.fallbacks, ["test.cmd/a"]);
    }

    #[test]
    fn a_reused_definition_reports_its_fallbacks_under_each_path() {
        let defs = json!({"D": {"type": "object", "additionalProperties": false,
            "properties": {"f": {"type": "string", "pattern": "^a"}}}});
        let props = json!({"a": {"$ref": "#/$defs/D"}, "b": {"$ref": "#/$defs/D"}});
        let mut g = ParamsGrammar::new();
        g.command_params("one", &params_of(props.clone(), defs.clone()));
        g.command_params("two", &params_of(props, defs));
        assert_eq!(
            g.fallbacks,
            ["one/a/D/f", "one/b/D/f", "two/a/D/f", "two/b/D/f"]
        );
    }

    #[test]
    fn array_bounds_out_of_reach_fall_back() {
        for (key, bounds) in [
            ("a", json!({"minItems": 2001})),
            ("a", json!({"maxItems": 2001})),
            ("a", json!({"minItems": 3, "maxItems": 2})),
        ] {
            let mut field = json!({"type": "array", "items": {"type": "number"}});
            for (k, v) in bounds.as_object().unwrap() {
                field[k] = v.clone();
            }
            let (_, built) = grammar_of(&params_of(json!({key: field}), json!({})));
            assert_eq!(built.fallbacks, ["test.cmd/a"], "{bounds}");
        }
    }

    #[test]
    fn a_reference_name_is_decoded_or_falls_back() {
        let defs = json!({"a/b~c": {"enum": ["x"]}});
        let props = json!({"f": {"$ref": "#/$defs/a~1b~0c"}, "g": {"$ref": "#/definitions/X"},
            "h": {"$ref": "#/$defs/a~2"}});
        let (g, built) = grammar_of(&params_of(props, defs));
        assert_eq!(built.fallbacks, ["test.cmd/g", "test.cmd/h"]);
        assert!(g.accepts(r#"{"f":"x"}"#));
        assert!(!g.accepts(r#"{"f":"y"}"#));
    }

    #[test]
    fn literals_escape_quotes_and_backslashes() {
        assert_eq!(literal(r#"{"a":"\"}"#), r#""{\"a\":\"\\\"}""#);
        assert_eq!(
            sanitize("analysis.select_by_attribute"),
            "analysis-select-by-attribute"
        );
    }
}
