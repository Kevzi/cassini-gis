//! Point en coordonnées, et garde-fou sur sa provenance (AD-19, AD-23 étendu, 2026-10-23).
//!
//! Toute coordonnée du registre est un [`Point`] : x, y et code EPSG, marqué dans le schéma
//! engendré par l'extension `x-cassini: point`. Le contrôle de contrat refuse un champ `x` ou
//! `y` numérique hors de ce type (`contract.bare_coordinate`).
//!
//! [`guard_coordinates`] est une fonction pure, appliquée au plan que la composition par le
//! modèle local a validé : tout `Point` du plan doit venir de la demande (x, y et système y
//! figurent, comme nombres) ou d'une étape précédente (référence `step:` vers un champ déclaré
//! `Point`). Sinon le plan n'est pas montré : le garde-fou rend une question à provenance
//! `guard`. Le système compte s'il est écrit en code EPSG ou nommé : Lambert-93, WGS84, ou
//! une zone UTM du WGS84 avec son hémisphère (« UTM 31N ») ; écrit une fois, il vaut pour tout
//! point du plan qui porte ce même epsg, alors que x et y sont lus pour chaque point. En cas de
//! doute, c'est la question. [`guard_rejected`] applique la même règle à une sortie que la
//! validation a rejetée : un point inventé y rend la question tout de suite, sans redemande.
//!
//! Une référence MGRS est le type [`MgrsRef`], marqué `x-cassini: mgrs` et soumis à la même
//! garde (CAP-15) : elle figure dans la demande (espaces et casse ignorés, zone sur un ou deux
//! chiffres) ou vient d'une référence `step:` vers un champ déclaré `MgrsRef`. Elle ne prend
//! aucun nombre à la demande, et les chiffres de la demande qui lui appartiennent ne servent
//! pas de x ou de y à un `Point`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::plan::{parse_reference, Plan, PlannedCall, QuestionSource};
use crate::registry::Registry;

/// Clé d'extension qui marque un type de Cassini dans un schéma engendré.
pub const MARK_KEY: &str = "x-cassini";
/// Valeur de la marque d'un [`Point`].
pub const POINT_MARK: &str = "point";

/// Question rendue par le garde-fou quand un point du plan ne vient ni de la demande, ni
/// d'une étape.
pub const GUARD_QUESTION: &str = "Il manque à la demande un point en coordonnées : quels sont x, y et le système (code EPSG, par exemple 2154 pour le Lambert-93) ?";

/// Un point en coordonnées, avec son système (jamais de coordonnée nue, AD-19).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(extend("x-cassini" = "point"))]
pub struct Point {
    /// Abscisse (X, ou longitude).
    pub x: f64,
    /// Ordonnée (Y, ou latitude).
    pub y: f64,
    /// Code EPSG du système des coordonnées (ex. 2154 pour le Lambert-93).
    pub epsg: u32,
}

/// Valeur de la marque d'une [`MgrsRef`].
pub const MGRS_MARK: &str = "mgrs";

// Le commentaire de documentation ci-dessous devient la description du type dans le schéma,
// lue par le modèle : il reste court. La marque (clé d'extension, valeur `mgrs`) soumet la
// référence à la même garde de provenance qu'un `Point` (AD-23, CAP-15). La chaîne passe telle
// quelle à la désérialisation et n'est validée par `MgrsRef::parse` que dans la commande qui la
// lit, pour que le refus porte le code `coord.invalid_mgrs` et nomme la partie fautive (AD-21).
/// Référence MGRS, recopiée de la demande.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(extend("x-cassini" = "mgrs"))]
pub struct MgrsRef(pub String);

impl MgrsRef {
    /// Lit la référence selon la grammaire de la NGA.
    pub fn parse(&self) -> Result<crate::geodesy::Mgrs, crate::geodesy::MgrsError> {
        crate::geodesy::Mgrs::parse(&self.0)
    }

    /// La référence d'une case MGRS, sous sa forme canonique (`31UDQ4825111750`).
    pub fn canonical(mgrs: &crate::geodesy::Mgrs) -> Self {
        MgrsRef(mgrs.canonical())
    }
}

/// Profondeur maximale de parcours d'un schéma (références circulaires comprises).
const MAX_DEPTH: usize = 16;
/// Alternatives d'un schéma.
const ALTERNATIVES: [&str; 3] = ["anyOf", "oneOf", "allOf"];

/// Suit les `$ref` internes d'un sous-schéma, jusqu'à la cible.
fn resolve<'a>(schema: &'a Value, root: &'a Value) -> &'a Value {
    let mut node = schema;
    for _ in 0..MAX_DEPTH {
        let Some(target) = node.get("$ref").and_then(Value::as_str) else {
            break;
        };
        match target.strip_prefix('#').and_then(|p| root.pointer(p)) {
            Some(next) => node = next,
            None => break,
        }
    }
    node
}

/// Le sous-schéma porte-t-il, après ses `$ref`, la marque `mark` ?
fn is_marked(schema: &Value, root: &Value, mark: &str) -> bool {
    resolve(schema, root).get(MARK_KEY).and_then(Value::as_str) == Some(mark)
}

/// Le sous-schéma est-il, après ses `$ref`, celui d'un [`Point`] ?
pub fn is_point(schema: &Value, root: &Value) -> bool {
    is_marked(schema, root, POINT_MARK)
}

/// Le sous-schéma est-il, après ses `$ref`, celui d'une [`MgrsRef`] ?
pub fn is_mgrs(schema: &Value, root: &Value) -> bool {
    is_marked(schema, root, MGRS_MARK)
}

/// Le sous-schéma admet-il un type marqué `mark` : lui-même, ou l'une de ses alternatives
/// (`Option<Point>` s'engendre en `anyOf`).
fn admits(schema: &Value, root: &Value, mark: &str, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    if is_marked(schema, root, mark) {
        return true;
    }
    let node = resolve(schema, root);
    ALTERNATIVES.iter().any(|key| {
        node.get(*key)
            .and_then(Value::as_array)
            .is_some_and(|alts| alts.iter().any(|a| admits(a, root, mark, depth + 1)))
    })
}

/// Le sous-schéma admet-il un [`Point`] ?
fn admits_point(schema: &Value, root: &Value, depth: usize) -> bool {
    admits(schema, root, POINT_MARK, depth)
}

/// Un type marqué `mark` apparaît-il quelque part sous ce sous-schéma (objets imbriqués,
/// tableaux, alternatives) ?
fn contains(schema: &Value, root: &Value, mark: &str, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    if is_marked(schema, root, mark) {
        return true;
    }
    let node = resolve(schema, root);
    let Value::Object(map) = node else {
        return false;
    };
    map.iter().any(|(key, value)| {
        if key == "properties" || key == "patternProperties" {
            value
                .as_object()
                .is_some_and(|m| m.values().any(|s| contains(s, root, mark, depth + 1)))
        } else if ALTERNATIVES.contains(&key.as_str()) || key == "prefixItems" {
            value
                .as_array()
                .is_some_and(|a| a.iter().any(|s| contains(s, root, mark, depth + 1)))
        } else if key == "items" || key == "additionalProperties" {
            contains(value, root, mark, depth + 1)
        } else {
            false
        }
    })
}

/// Sous-schéma de la propriété `name` d'un objet, cherché aussi dans ses alternatives.
fn property_schema<'a>(
    schema: &'a Value,
    root: &'a Value,
    name: &str,
    depth: usize,
) -> Option<&'a Value> {
    if depth > MAX_DEPTH {
        return None;
    }
    let node = resolve(schema, root);
    if let Some(found) = node.get("properties").and_then(|p| p.get(name)) {
        return Some(found);
    }
    ALTERNATIVES.iter().find_map(|key| {
        node.get(*key)
            .and_then(Value::as_array)?
            .iter()
            .find_map(|a| property_schema(a, root, name, depth + 1))
    })
}

/// Sous-schéma des éléments d'un tableau, cherché aussi dans ses alternatives.
fn items_schema<'a>(schema: &'a Value, root: &'a Value, depth: usize) -> Option<&'a Value> {
    if depth > MAX_DEPTH {
        return None;
    }
    let node = resolve(schema, root);
    if let Some(items) = node.get("items").filter(|i| i.is_object()) {
        return Some(items);
    }
    ALTERNATIVES.iter().find_map(|key| {
        node.get(*key)
            .and_then(Value::as_array)?
            .iter()
            .find_map(|a| items_schema(a, root, depth + 1))
    })
}

// -- lecture des nombres de la demande ---------------------------------------

/// Séparateurs de milliers admis : espace, espace insécable, espace fine, espace fine
/// insécable.
fn is_group_separator(c: char) -> bool {
    matches!(c, ' ' | '\u{a0}' | '\u{2009}' | '\u{202f}')
}

/// Séparateurs admis entre le nom d'un système et son numéro (« Lambert-93 », « WGS 84 »).
fn is_name_separator(c: char) -> bool {
    matches!(c, '-' | '\u{2011}' | ' ' | '\u{a0}')
}

/// Le nombre qui commence en `start` suit-il directement le nom d'un système (« Lambert 93 ») ?
/// Il n'ouvre alors pas de groupe de milliers : « Lambert 93 652 000 » n'est pas 93 652 000.
fn follows_system_name(chars: &[char], start: usize) -> bool {
    let mut end = start;
    if end > 0 && is_name_separator(chars[end - 1]) {
        end -= 1;
    }
    NAMED_SYSTEMS.iter().any(|(name, _, _)| {
        let n = name.chars().count();
        end >= n
            && chars[end - n..end]
                .iter()
                .flat_map(|c| c.to_lowercase())
                .eq(name.chars())
            && (end == n || !chars[end - n - 1].is_alphanumeric())
    })
}

/// Noms de systèmes que le noyau associe à un code fermé : (préfixe, numéro, code EPSG).
/// Écrits avec ou sans séparateur entre le nom et le numéro (« Lambert-93 », « WGS 84 »).
const NAMED_SYSTEMS: [(&str, &str, f64); 2] = [("lambert", "93", 2154.0), ("wgs", "84", 4326.0)];

/// Nombres écrits dans la demande, dans l'ordre (décision du propriétaire, 2026-10-23) :
/// `2.458` et `2,458` (virgule suivie d'un chiffre = décimale) ; groupes de trois chiffres
/// séparés par une espace, une espace insécable ou une espace fine (`6 850 000`) lus comme un
/// seul nombre ; signe moins accepté devant un nombre détaché d'un mot. S'y ajoutent les codes
/// des systèmes nommés (Lambert-93 : 2154, WGS84 : 4326) et des zones UTM du WGS84 écrites
/// avec leur hémisphère (« UTM 31N » : 32631, « UTM 31 Sud » : 32731).
/// Toute autre forme n'est pas lue.
pub fn request_numbers(request: &str) -> Vec<f64> {
    let chars: Vec<char> = request.chars().collect();
    let digit = |i: usize| chars.get(i).is_some_and(char::is_ascii_digit);
    let mut numbers = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !digit(i) {
            i += 1;
            continue;
        }
        let start = i;
        while digit(i) {
            i += 1;
        }
        let mut text: String = chars[start..i].iter().collect();
        // Milliers : une tête de un à trois chiffres, puis des groupes d'exactement trois.
        if i - start <= 3 && !follows_system_name(&chars, start) {
            while chars.get(i).is_some_and(|c| is_group_separator(*c))
                && digit(i + 1)
                && digit(i + 2)
                && digit(i + 3)
                && !digit(i + 4)
            {
                text.extend(&chars[i + 1..i + 4]);
                i += 4;
            }
        }
        // Décimale : un point ou une virgule suivi d'un chiffre.
        if chars.get(i).is_some_and(|c| *c == '.' || *c == ',') && digit(i + 1) {
            text.push('.');
            i += 1;
            while digit(i) {
                text.push(chars[i]);
                i += 1;
            }
        }
        // Signe : un moins collé au nombre, qui ne suit pas un mot (« Lambert-93 » n'est pas -93).
        let negative = start >= 1
            && matches!(chars[start - 1], '-' | '\u{2212}')
            && (start == 1 || !chars[start - 2].is_alphanumeric());
        if let Ok(value) = text.parse::<f64>() {
            numbers.push(if negative { -value } else { value });
        }
    }
    let lower: Vec<char> = request.to_lowercase().chars().collect();
    numbers.extend(named_codes(&lower));
    numbers.extend(utm_codes(&lower));
    numbers
}

/// Codes des systèmes nommés écrits dans la demande (en minuscules) : Lambert-93, WGS84.
fn named_codes(lower: &[char]) -> Vec<f64> {
    let mut codes = Vec::new();
    for (name, number, code) in NAMED_SYSTEMS {
        let name: Vec<char> = name.chars().collect();
        let number: Vec<char> = number.chars().collect();
        for start in 0..lower.len() {
            if !lower[start..].starts_with(&name)
                || (start > 0 && lower[start - 1].is_alphanumeric())
            {
                continue;
            }
            let mut j = start + name.len();
            if lower.get(j).is_some_and(|c| is_name_separator(*c)) {
                j += 1;
            }
            if lower[j.min(lower.len())..].starts_with(&number)
                && !lower
                    .get(j + number.len())
                    .is_some_and(char::is_ascii_digit)
            {
                codes.push(code);
            }
        }
    }
    codes
}

/// Systèmes écrits dans la demande : codes nommés, zones UTM, et nombres écrits juste après
/// « EPSG » (« EPSG:2154 », « EPSG 4326 »). Sert à savoir si un système écrit est resté sans
/// point (2026-11-05).
fn request_systems(request: &str) -> Vec<f64> {
    let lower: Vec<char> = request.to_lowercase().chars().collect();
    let mut systems = named_codes(&lower);
    systems.extend(utm_codes(&lower));
    for start in 0..lower.len() {
        if !lower[start..].starts_with(&['e', 'p', 's', 'g'])
            || (start > 0 && lower[start - 1].is_alphanumeric())
        {
            continue;
        }
        let mut j = start + 4;
        while lower
            .get(j)
            .is_some_and(|c| matches!(c, ':' | ' ' | '\u{a0}'))
        {
            j += 1;
        }
        let digits: String = lower[j..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(code) = digits.parse::<f64>() {
            systems.push(code);
        }
    }
    systems
}

/// Datums dont le nom, écrit juste avant « UTM » ou juste après la zone, en fait une autre
/// projection que le WGS84 UTM : la zone n'est alors pas lue (décision du propriétaire,
/// 2026-11-03). Nom et numéro, écrits collés ou séparés (« ED50 », « ED 50 »).
const OTHER_UTM_DATUMS: [(&str, &str); 3] = [("ed", "50"), ("etrs", "89"), ("ntf", "")];

/// Codes EPSG des zones UTM du WGS84 écrites dans la demande (en minuscules), décision du
/// propriétaire du 2026-11-03 : « utm » en début de mot, séparateur facultatif, « zone » et
/// séparateur facultatifs, un ou deux chiffres de 1 à 60, séparateur facultatif, puis
/// l'hémisphère « n », « nord », « s » ou « sud » qui finit le mot (une apostrophe ne le finit
/// pas : « 31 n'importe »). Nord : 32600 + zone ; sud : 32700 + zone. Sans hémisphère, ou avec
/// un autre datum juste avant ou juste après (« ED50 UTM 31N », « UTM 31N (ED50) »), aucun
/// code : en cas de doute, c'est la question.
fn utm_codes(lower: &[char]) -> Vec<f64> {
    let at = |j: usize, word: &str| {
        let word: Vec<char> = word.chars().collect();
        lower.get(j..).is_some_and(|rest| rest.starts_with(&word))
    };
    let skip_separator = |j: usize| {
        if lower.get(j).is_some_and(|c| is_name_separator(*c)) {
            j + 1
        } else {
            j
        }
    };
    let ends_word = |k: usize| {
        !lower
            .get(k)
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, '\'' | '\u{2019}'))
    };
    let mut codes = Vec::new();
    for start in 0..lower.len() {
        if !at(start, "utm")
            || (start > 0 && lower[start - 1].is_alphanumeric())
            || follows_other_datum(lower, start)
        {
            continue;
        }
        let mut j = skip_separator(start + 3);
        if at(j, "zone") {
            j = skip_separator(j + 4);
        }
        let digits = lower[j..].iter().take_while(|c| c.is_ascii_digit()).count();
        if !(1..=2).contains(&digits) {
            continue;
        }
        let zone: u32 = lower[j..j + digits]
            .iter()
            .fold(0, |n, c| n * 10 + c.to_digit(10).unwrap_or(0));
        if !(1..=60).contains(&zone) {
            continue;
        }
        j = skip_separator(j + digits);
        let hemisphere = [("nord", 32600), ("sud", 32700), ("n", 32600), ("s", 32700)]
            .into_iter()
            .map(|(word, base)| (j + word.chars().count(), word, base))
            .find(|(end, word, _)| at(j, word) && ends_word(*end));
        if let Some((end, _, base)) = hemisphere {
            if !precedes_other_datum(lower, end) {
                codes.push(f64::from(base + zone));
            }
        }
    }
    codes
}

/// Le mot (suite de caractères alphanumériques) qui finit avant `end`, en sautant tout ce qui
/// n'est pas alphanumérique : son début et son texte.
fn word_before(lower: &[char], mut end: usize) -> (usize, String) {
    while end > 0 && !lower[end - 1].is_alphanumeric() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && lower[start - 1].is_alphanumeric() {
        start -= 1;
    }
    (start, lower[start..end].iter().collect())
}

/// Le mot qui commence après `start`, en sautant tout ce qui n'est pas alphanumérique : sa fin
/// et son texte.
fn word_after(lower: &[char], mut start: usize) -> (usize, String) {
    while start < lower.len() && !lower[start].is_alphanumeric() {
        start += 1;
    }
    let mut end = start;
    while end < lower.len() && lower[end].is_alphanumeric() {
        end += 1;
    }
    (end, lower[start..end].iter().collect())
}

/// Le « utm » qui commence en `start` suit-il le nom d'un autre datum, quoi qu'il y ait entre
/// eux hors lettres et chiffres (« ED50 UTM », « ETRS89/UTM », « ED 50 (UTM ») ?
fn follows_other_datum(lower: &[char], start: usize) -> bool {
    let (before, word) = word_before(lower, start);
    OTHER_UTM_DATUMS.iter().any(|(name, number)| {
        word == format!("{name}{number}")
            || (!number.is_empty() && word == *number && word_before(lower, before).1 == *name)
    })
}

/// L'hémisphère qui finit en `end` est-il suivi du nom d'un autre datum (« UTM 31N (ED50) »,
/// « UTM 31 Nord, ETRS 89 ») ?
fn precedes_other_datum(lower: &[char], end: usize) -> bool {
    let (after, word) = word_after(lower, end);
    OTHER_UTM_DATUMS.iter().any(|(name, number)| {
        word == format!("{name}{number}")
            || (!number.is_empty() && word == *name && word_after(lower, after).1 == *number)
    })
}

/// Deux nombres égaux, à l'arrondi du flottant près.
fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

/// Retire de `pool` une occurrence égale à `value` ; vrai si elle y était.
fn take(pool: &mut Vec<f64>, value: f64) -> bool {
    match pool.iter().position(|n| same(*n, value)) {
        Some(index) => {
            pool.swap_remove(index);
            true
        }
        None => false,
    }
}

// -- garde-fou ---------------------------------------------------------------

// -- références MGRS de la demande -------------------------------------------

/// Forme comparable d'une référence MGRS écrite (CAP-15) : majuscules, sans espace, zone sur
/// deux chiffres. Rien n'est validé ici : une référence mal formée mais recopiée de la demande
/// passe la garde, et la commande la refuse en nommant la partie fautive.
fn mgrs_key(text: &str) -> String {
    let compact: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let mut chars = compact.chars();
    match (chars.next(), chars.next()) {
        (Some(a), Some(b)) if a.is_ascii_digit() && !b.is_ascii_digit() => format!("0{compact}"),
        _ => compact,
    }
}

/// Une référence MGRS lue dans la demande : sa forme comparable et l'étendue de ses
/// caractères.
struct WrittenMgrs {
    key: String,
    span: std::ops::Range<usize>,
}

/// Les références de forme MGRS écrites dans la demande : un ou deux chiffres en début de mot,
/// une lettre (la bande), deux lettres (le carré), des espaces facultatifs entre chacun, puis des
/// groupes de chiffres séparés par des espaces, le dernier finissant le mot. Chaque suite de
/// groupes en tête (« 48251 », « 48251 11750 », …) est une lecture possible : la demande
/// « 31U DQ 48251 11750 10 min » se lit `31UDQ48251`, `31UDQ4825111750` ou `31UDQ482511175010`.
fn written_mgrs(request: &str) -> Vec<WrittenMgrs> {
    let chars: Vec<char> = request.chars().collect();
    let digit = |i: usize| chars.get(i).is_some_and(char::is_ascii_digit);
    let letter = |i: usize| chars.get(i).is_some_and(char::is_ascii_alphabetic);
    let space = |i: usize| chars.get(i).is_some_and(|c| c.is_whitespace());
    let mut found = Vec::new();
    for start in 0..chars.len() {
        if !digit(start) || (start > 0 && chars[start - 1].is_alphanumeric()) {
            continue;
        }
        let mut i = start;
        while digit(i) {
            i += 1;
        }
        if i - start > 2 {
            continue;
        }
        let zone: String = chars[start..i].iter().collect();
        while space(i) {
            i += 1;
        }
        if !letter(i) {
            continue;
        }
        let band = chars[i];
        i += 1;
        while space(i) {
            i += 1;
        }
        if !letter(i) {
            continue;
        }
        let column = chars[i];
        i += 1;
        while space(i) {
            i += 1;
        }
        if !letter(i) {
            continue;
        }
        let head = format!("{zone}{band}");
        let square = format!("{column}{}", chars[i]);
        i += 1;
        let mut digits = String::new();
        loop {
            let mut j = i;
            while space(j) {
                j += 1;
            }
            if !digit(j) {
                break;
            }
            while digit(j) {
                digits.push(chars[j]);
                j += 1;
            }
            if chars.get(j).is_some_and(|c| c.is_alphanumeric()) {
                break;
            }
            i = j;
            found.push(WrittenMgrs {
                key: mgrs_key(&format!("{head}{square}{digits}")),
                span: start..j,
            });
        }
    }
    found
}

/// La demande, où les chiffres des références MGRS que le plan reprend sont effacés : ils ne
/// servent ni de x, ni de y, ni de système à un `Point`.
fn without_used_mgrs(request: &str, written: &[WrittenMgrs], used: &[usize]) -> String {
    let mut chars: Vec<char> = request.chars().collect();
    for index in used {
        for c in &mut chars[written[*index].span.clone()] {
            *c = ' ';
        }
    }
    chars.into_iter().collect()
}

/// Une lecture bien formée (grammaire de la NGA) que le plan ne reprend pas n'efface pas le
/// texte, qui peut porter d'autres nombres (« 25m de 652000 » se lit `25MDE652000`) : elle ôte
/// seulement du multiensemble une occurrence de chacune de ses deux moitiés de chiffres, lues
/// comme nombres, pour que « 31U DQ 48251 11750 » ne donne pas x = 48251, y = 11750 à un point.
fn take_unused_mgrs(pool: &mut Vec<f64>, written: &[WrittenMgrs], used: &[usize]) {
    for (index, w) in written.iter().enumerate() {
        if used.contains(&index) {
            continue;
        }
        if let Ok(mgrs) = crate::geodesy::Mgrs::parse(&w.key) {
            take(pool, f64::from(mgrs.easting));
            take(pool, f64::from(mgrs.northing));
        }
    }
}

/// Ce que porte un plan : ses nombres littéraux hors `Point`, ses points littéraux et ses
/// références MGRS littérales.
#[derive(Default)]
struct Found<'a> {
    numbers: Vec<f64>,
    points: Vec<&'a Value>,
    mgrs: Vec<&'a str>,
    /// Une coordonnée dont la provenance ne peut pas être établie.
    doubtful: bool,
}

/// Parcourt une valeur littérale avec son schéma (s'il est connu).
fn walk<'a>(value: &'a Value, schema: Option<&Value>, root: &Value, found: &mut Found<'a>) {
    if schema.is_some_and(|s| admits(s, root, MGRS_MARK, 0)) {
        match value {
            Value::String(text) => found.mgrs.push(text),
            Value::Null => {}
            _ => found.doubtful = true,
        }
        return;
    }
    let point_here = schema.is_some_and(|s| admits_point(s, root, 0));
    match value {
        Value::Object(map) => {
            // Sans schéma connu, un objet qui porte `x` ou `y` est lu comme un point : le
            // contrat interdit ces champs numériques ailleurs, et le doute mène à la question.
            let bare = schema.is_none() && (map.contains_key("x") || map.contains_key("y"));
            if point_here || bare {
                found.points.push(value);
                return;
            }
            for (name, item) in map {
                let sub = schema.and_then(|s| property_schema(s, root, name, 0));
                walk(item, sub, root, found);
            }
        }
        Value::Array(items) => {
            let sub = schema.and_then(|s| items_schema(s, root, 0));
            for item in items {
                walk(item, sub, root, found);
            }
        }
        Value::Number(n) => {
            if let Some(v) = n.as_f64() {
                found.numbers.push(v);
            }
        }
        Value::String(text) => {
            if point_here {
                found.doubtful = true;
            } else if let Ok(v) = text.trim().parse::<f64>() {
                if v.is_finite() {
                    found.numbers.push(v);
                }
            }
        }
        Value::Bool(_) | Value::Null => {}
    }
}

/// La référence `value`, paramètre d'une étape `step` de `calls`, désigne-t-elle un champ que
/// le schéma de sortie de l'étape citée déclare du type marqué `mark` (`Point` ou `MgrsRef`),
/// à la place d'un paramètre lui-même de ce type ?
fn reference_is_a(
    registry: &Registry,
    calls: &[PlannedCall],
    step: usize,
    param: &Value,
    params_root: &Value,
    value: &Value,
    mark: &str,
) -> bool {
    let Some(Ok((k, field))) = parse_reference(value) else {
        return false;
    };
    if k >= step || !admits(param, params_root, mark, 0) {
        return false;
    }
    let Some(source) = registry.spec(&calls[k - 1].command) else {
        return false;
    };
    let root = &source.output_schema;
    match field {
        None => admits(root, root, mark, 0),
        Some(f) => {
            property_schema(root, root, &f, 0).is_some_and(|schema| admits(schema, root, mark, 0))
        }
    }
}

/// Garde-fou de AD-23 sur les coordonnées : rend `plan` inchangé si chaque `Point` de ses
/// étapes vient de `request` ou d'une étape précédente, sinon une question à provenance
/// `guard`. Une question ou un refus passe inchangé.
///
/// Les nombres de la demande forment un multiensemble : chaque nombre littéral du plan hors
/// `Point` (durée, distance) y retire d'abord une occurrence égale s'il en trouve une ; puis,
/// pour chaque `Point`, x et y retirent chacun une occurrence distincte, et l'epsg en retire
/// une aussi, sauf s'il est égal à un système qu'un point précédent du plan a déjà retiré : un
/// système écrit une fois vaut pour tous les points (2026-11-05). S'il en manque une, c'est la
/// question. Le système n'est jamais pris au projet.
pub fn guard_coordinates(registry: &Registry, plan: Plan, request: &str) -> Plan {
    match &plan {
        Plan::Calls { calls } if invents_a_point(registry, calls, request, true) => {
            guard_question()
        }
        _ => plan,
    }
}

/// Le même garde-fou sur une sortie du modèle que la validation a rejetée (décision du
/// propriétaire, 2026-10-23) : si elle porte un point qui ne vient ni de `request` ni d'une
/// étape (point incomplet compris, par exemple sans `epsg`), la question du garde-fou est
/// rendue tout de suite, sans redemande. Sinon `None` : le rejet suit son cours. Une commande
/// inconnue n'est pas une affaire de coordonnées : elle est laissée à la redemande.
pub fn guard_rejected(registry: &Registry, plan: &Plan, request: &str) -> Option<Plan> {
    match plan {
        Plan::Calls { calls } if invents_a_point(registry, calls, request, false) => {
            Some(guard_question())
        }
        _ => None,
    }
}

/// Vrai si un point de `calls` ne vient ni de `request` ni d'une étape. `strict` : une
/// commande inconnue compte comme un doute (plan validé) ; sinon elle est ignorée.
fn invents_a_point(
    registry: &Registry,
    calls: &[PlannedCall],
    request: &str,
    strict: bool,
) -> bool {
    let mut found = Found::default();
    for (index, call) in calls.iter().enumerate() {
        let step = index + 1;
        let Some(spec) = registry.spec(&call.command) else {
            if strict {
                return true;
            }
            continue;
        };
        let root = &spec.params_schema;
        let Value::Object(params) = &call.params else {
            walk(&call.params, Some(root), root, &mut found);
            continue;
        };
        for (name, value) in params {
            let schema = property_schema(root, root, name, 0);
            if parse_reference(value).is_some() {
                // Une référence ne tient lieu de point que si elle désigne un `Point`, ni de
                // référence MGRS que si elle désigne une `MgrsRef`.
                for mark in [POINT_MARK, MGRS_MARK] {
                    let carries = schema.is_some_and(|s| contains(s, root, mark, 0));
                    if carries
                        && !schema.is_some_and(|s| {
                            reference_is_a(registry, calls, step, s, root, value, mark)
                        })
                    {
                        return true;
                    }
                }
                continue;
            }
            walk(value, schema, root, &mut found);
        }
    }
    if found.doubtful {
        return true;
    }
    // Une référence MGRS du plan figure telle quelle dans la demande, espaces et casse ignorés
    // (CAP-15). Elle ne prend aucun nombre à la demande, et ses chiffres n'y restent pas pour
    // un `Point`.
    let written = written_mgrs(request);
    let mut used = Vec::new();
    for literal in &found.mgrs {
        let key = mgrs_key(literal);
        let Some(index) = written.iter().position(|w| w.key == key) else {
            return true;
        };
        used.push(index);
    }
    let request = without_used_mgrs(request, &written, &used);
    let request = request.as_str();
    let mut pool = request_numbers(request);
    take_unused_mgrs(&mut pool, &written, &used);
    for number in &found.numbers {
        take(&mut pool, *number);
    }
    // Un même point repris tel quel par plusieurs étapes (isochrones à pied puis en voiture
    // depuis le même lieu) ne prend ses nombres à la demande qu'une fois.
    let mut points: Vec<&Value> = Vec::new();
    for point in found.points {
        if !points.contains(&point) {
            points.push(point);
        }
    }
    // Un système écrit une fois dans la demande vaut pour tous les points du plan (décision du
    // propriétaire, 2026-11-05) : un epsg déjà retiré de la demande par un point précédent est
    // accepté sans en retirer une autre occurrence. x et y, eux, en retirent toujours une.
    let mut systems: Vec<f64> = Vec::new();
    let mut borrowed = false;
    let invented = points.iter().any(|point| {
        let invented = |key: &str, pool: &mut Vec<f64>| {
            let given = point.get(key).and_then(Value::as_f64);
            !given.is_some_and(|v| take(pool, v))
        };
        if invented("x", &mut pool) || invented("y", &mut pool) {
            return true;
        }
        let Some(epsg) = point.get("epsg").and_then(Value::as_f64) else {
            return true;
        };
        if take(&mut pool, epsg) {
            systems.push(epsg);
            return false;
        }
        borrowed = true;
        !systems.iter().any(|s| same(*s, epsg))
    });
    // L'emprunt n'est permis que si chaque système écrit dans la demande a servi à un point :
    // un système écrit resté sans point (« … en Lambert-93, … en WGS84 », deux points en
    // Lambert-93) est un doute, donc la question (décision du propriétaire, 2026-11-05).
    invented
        || (borrowed
            && request_systems(request)
                .iter()
                .any(|written| !systems.iter().any(|s| same(*s, *written))))
}

/// La question du garde-fou.
fn guard_question() -> Plan {
    Plan::Question {
        question: GUARD_QUESTION.to_string(),
        source: QuestionSource::Guard,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::registry::{Command, CommandFailure, CommandSpec, RunsIn};

    const ROUTES: &str = "routes_1";

    fn isochrone(origin: Value, duration: Value) -> Plan {
        calls(json!([{"command": "analysis.isochrone", "params": {
            "layer_id": ROUTES, "origin": origin, "mode": "walk",
            "duration": {"value": duration, "unit": "min"}}}]))
    }

    fn calls(calls: Value) -> Plan {
        serde_json::from_value(json!({"kind": "calls", "calls": calls})).unwrap()
    }

    fn is_guard(plan: &Plan) -> bool {
        matches!(plan, Plan::Question { question, source: QuestionSource::Guard }
            if question == GUARD_QUESTION)
    }

    /// Une commande dont la sortie est un `Point`, champ `point` : le cas d'une étape.
    #[derive(Debug, Serialize, JsonSchema)]
    struct Located {
        point: Point,
        layer_id: String,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct NoParams {}

    struct Locate;
    impl Command for Locate {
        type Params = NoParams;
        type Output = Located;
        fn spec() -> CommandSpec {
            CommandSpec::new::<NoParams, Located>("test.locate", "Situe.", "Locates.")
                .runs_in(RunsIn::Core)
                .cancellable(false)
                .tests(&["t"])
        }
        fn run(&self, _p: NoParams) -> Result<Located, CommandFailure> {
            Ok(Located {
                point: Point {
                    x: 1.0,
                    y: 2.0,
                    epsg: 2154,
                },
                layer_id: "l".into(),
            })
        }
    }

    fn registry_with_locate() -> Registry {
        let mut registry = Registry::standard();
        registry.register(Locate);
        registry.register(MaybeAt);
        registry
    }

    /// Une commande à point facultatif, et un objet à `x` texte que le contrat permet.
    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct MaybeAtParams {
        at: Option<Point>,
        label: Option<Label>,
    }

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct Label {
        x: String,
    }

    #[derive(Debug, Serialize, JsonSchema)]
    struct Done {}

    struct MaybeAt;
    impl Command for MaybeAt {
        type Params = MaybeAtParams;
        type Output = Done;
        fn spec() -> CommandSpec {
            CommandSpec::new::<MaybeAtParams, Done>("test.maybe_at", "Peut-être.", "Maybe.")
                .runs_in(RunsIn::Core)
                .cancellable(false)
                .tests(&["t"])
        }
        fn run(&self, _p: MaybeAtParams) -> Result<Done, CommandFailure> {
            Ok(Done {})
        }
    }

    #[test]
    fn an_optional_point_takes_a_step_reference_and_a_known_object_is_not_a_point() {
        let registry = registry_with_locate();
        let plan = calls(json!([
            {"command": "test.locate", "params": {}},
            {"command": "test.maybe_at", "params": {"at": "step:1.point"}}
        ]));
        assert_eq!(guard_coordinates(&registry, plan.clone(), ""), plan);
        let plan = calls(json!([
            {"command": "test.maybe_at", "params": {"label": {"x": "nord"}}}
        ]));
        assert_eq!(guard_coordinates(&registry, plan.clone(), ""), plan);
        // Un point facultatif littéral reste gardé.
        let plan = calls(json!([
            {"command": "test.maybe_at", "params": {"at": {"x": 1, "y": 2, "epsg": 2154}}}
        ]));
        assert!(is_guard(&guard_coordinates(&registry, plan, "")));
    }

    #[test]
    fn the_point_schema_is_marked() {
        let registry = Registry::standard();
        let spec = registry.spec("analysis.isochrone").unwrap();
        let root = &spec.params_schema;
        let origin = &root["properties"]["origin"];
        assert!(is_point(origin, root), "{origin}");
        assert!(!is_point(&root["properties"]["duration"], root));
        assert!(!is_point(&root["properties"]["layer_id"], root));
        // Même désérialisation qu'avant : pas de champ inconnu, pas de système manquant.
        assert!(serde_json::from_value::<Point>(json!({"x": 1, "y": 2})).is_err());
        assert!(
            serde_json::from_value::<Point>(json!({"x": 1, "y": 2, "epsg": 2154, "z": 0})).is_err()
        );
    }

    #[test]
    fn phrase_e_without_coordinates_becomes_a_guard_question() {
        let plan = isochrone(json!({"x": 2.458, "y": 48.658, "epsg": 4326}), json!(10));
        let request =
            "Quelle zone atteint-on à pied en 10 minutes depuis la gare de Villeneuve-le-Roi ?";
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            plan,
            request
        )));
    }

    #[test]
    fn the_same_given_point_reused_by_two_steps_leaves_the_plan_unchanged() {
        let registry = Registry::standard();
        let step = |mode: &str| {
            json!({"command": "analysis.isochrone", "params": {
                "layer_id": ROUTES, "mode": mode,
                "origin": {"x": 652000, "y": 6850000, "epsg": 2154},
                "duration": {"value": 10, "unit": "min"}}})
        };
        let plan = calls(json!([step("walk"), step("car")]));
        let request =
            "Zones à 10 min à pied et en voiture depuis x 652 000, y 6 850 000 en EPSG:2154.";
        assert_eq!(guard_coordinates(&registry, plan.clone(), request), plan);
        // Deux points différents demandent chacun leurs nombres.
        let other = calls(
            json!([step("walk"), {"command": "analysis.isochrone", "params": {
            "layer_id": ROUTES, "mode": "car",
            "origin": {"x": 655700, "y": 6849023, "epsg": 2154},
            "duration": {"value": 10, "unit": "min"}}}]),
        );
        assert!(is_guard(&guard_coordinates(&registry, other, request)));
    }

    #[test]
    fn a_point_given_in_the_request_leaves_the_plan_unchanged() {
        let registry = Registry::standard();
        let plan = isochrone(json!({"x": 652000, "y": 6850000, "epsg": 2154}), json!(10));
        for request in [
            "Zone à 10 min à pied depuis x 652 000, y 6 850 000 en EPSG:2154.",
            "Zone à 10 min à pied depuis x 652\u{a0}000, y 6\u{202f}850\u{202f}000 en EPSG:2154.",
            "Zone à 10 min à pied depuis le point 652000 / 6850000 (Lambert-93).",
            "Zone à 10 min à pied depuis le point (652 000 ; 6 850 000) en Lambert 93.",
            "Zone à 10 min à pied, en Lambert 93 652 000 / 6 850 000.",
            "Zone à 10 min à pied, en Lambert-93 : 652\u{2009}000 / 6\u{2009}850\u{2009}000.",
        ] {
            assert_eq!(
                guard_coordinates(&registry, plan.clone(), request),
                plan,
                "{request}"
            );
        }
        // Décimales au point ou à la virgule, système nommé ou en code.
        let wgs = isochrone(json!({"x": 2.458, "y": 48.658, "epsg": 4326}), json!(10));
        for request in [
            "10 min depuis 2,458 / 48,658 (WGS84)",
            "10 min depuis 2.458 / 48.658 en EPSG:4326",
            "10 min depuis la longitude 2.4580 et la latitude 48.6580 (WGS 84)",
        ] {
            assert_eq!(
                guard_coordinates(&registry, wgs.clone(), request),
                wgs,
                "{request}"
            );
        }
        // Signe moins.
        let west = isochrone(json!({"x": -1.5, "y": 47.2, "epsg": 4326}), json!(10));
        assert_eq!(
            guard_coordinates(
                &registry,
                west.clone(),
                "10 min depuis -1,5 / 47,2 (EPSG:4326)"
            ),
            west
        );
    }

    #[test]
    fn a_missing_system_is_a_guard_question() {
        let plan = isochrone(json!({"x": 652000, "y": 6850000, "epsg": 2154}), json!(10));
        let request = "Zone à 10 min à pied depuis x 652 000, y 6 850 000.";
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            plan,
            request
        )));
    }

    fn line_of_sight(observer: Value, target: Value) -> Plan {
        calls(json!([{"command": "terrain.line_of_sight", "params": {
            "dem": "dem/mnt.tif", "observer": observer, "target": target}}]))
    }

    #[test]
    fn a_system_written_once_holds_for_every_point() {
        let registry = Registry::standard();
        let plan = line_of_sight(
            json!({"x": 656500, "y": 6847800, "epsg": 2154}),
            json!({"x": 657300, "y": 6848400, "epsg": 2154}),
        );
        for request in [
            "Voit-on 657 300 / 6 848 400 depuis 656 500 / 6 847 800, en Lambert-93 ?",
            "Depuis 656500 / 6847800 vers 657300 / 6848400 (EPSG:2154), la cible est-elle visible ?",
        ] {
            assert_eq!(
                guard_coordinates(&registry, plan.clone(), request),
                plan,
                "{request}"
            );
        }
        // Deux points dans deux systèmes, chacun écrit : inchangé, comme avant.
        let mixed = line_of_sight(
            json!({"x": 656500, "y": 6847800, "epsg": 2154}),
            json!({"x": 2.4, "y": 48.7, "epsg": 4326}),
        );
        let request = "Depuis 656 500 / 6 847 800 en Lambert-93, voit-on 2,4 / 48,7 en WGS84 ?";
        assert_eq!(guard_coordinates(&registry, mixed.clone(), request), mixed);
        // La même règle vaut pour une sortie rejetée par la validation : pas de question.
        let request = "Voit-on 657 300 / 6 848 400 depuis 656 500 / 6 847 800, en Lambert-93 ?";
        assert_eq!(guard_rejected(&registry, &plan, request), None);
    }

    #[test]
    fn a_system_written_once_does_not_excuse_a_missing_coordinate_or_system() {
        let registry = Registry::standard();
        let plan = line_of_sight(
            json!({"x": 656500, "y": 6847800, "epsg": 2154}),
            json!({"x": 657300, "y": 6848400, "epsg": 2154}),
        );
        for request in [
            // y de la cible absent de la demande.
            "Voit-on 657 300 depuis 656 500 / 6 847 800, en Lambert-93 ?",
            // x de l'observateur absent.
            "Voit-on 657 300 / 6 848 400 depuis 6 847 800, en Lambert-93 ?",
            // Aucun système écrit : jamais pris au projet.
            "Voit-on 657 300 / 6 848 400 depuis 656 500 / 6 847 800 ?",
        ] {
            assert!(
                is_guard(&guard_coordinates(&registry, plan.clone(), request)),
                "{request}"
            );
        }
        // Un point dans un autre système que le seul écrit.
        let other = line_of_sight(
            json!({"x": 656500, "y": 6847800, "epsg": 2154}),
            json!({"x": 657300, "y": 6848400, "epsg": 4326}),
        );
        let request = "Voit-on 657 300 / 6 848 400 depuis 656 500 / 6 847 800, en Lambert-93 ?";
        assert!(is_guard(&guard_coordinates(&registry, other, request)));
        // Deux systèmes écrits, le modèle met le premier aux deux points : le WGS84 écrit reste
        // sans point, l'emprunt est refusé, quel que soit l'ordre des points.
        let request = "Depuis 656 500 / 6 847 800 en Lambert-93, voit-on 2,4 / 48,7 en WGS84 ?";
        for (observer, target) in [
            (
                json!({"x": 656500, "y": 6847800, "epsg": 2154}),
                json!({"x": 2.4, "y": 48.7, "epsg": 2154}),
            ),
            (
                json!({"x": 2.4, "y": 48.7, "epsg": 2154}),
                json!({"x": 656500, "y": 6847800, "epsg": 2154}),
            ),
        ] {
            let plan = line_of_sight(observer, target);
            assert!(is_guard(&guard_coordinates(&registry, plan, request)));
        }
        let request = "Depuis 656 500 / 6 847 800 (EPSG:2154) vers 2,4 / 48,7 (EPSG 4326) ?";
        let plan = line_of_sight(
            json!({"x": 656500, "y": 6847800, "epsg": 2154}),
            json!({"x": 2.4, "y": 48.7, "epsg": 2154}),
        );
        assert!(is_guard(&guard_coordinates(&registry, plan, request)));
        // Un point sans epsg reste un point incomplet.
        let bare = line_of_sight(
            json!({"x": 656500, "y": 6847800, "epsg": 2154}),
            json!({"x": 657300, "y": 6848400}),
        );
        assert!(guard_rejected(&registry, &bare, request).is_some());
    }

    #[test]
    fn a_number_is_not_taken_twice() {
        // La durée prend le premier 10, x le second : y n'a plus de nombre libre.
        let plan = isochrone(json!({"x": 10, "y": 10, "epsg": 2154}), json!(10));
        let request = "10 min depuis 10, 500 (EPSG:2154)";
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            plan,
            request
        )));
        // Le point du modèle tiré d'autres nombres de la demande (distance, durée).
        let plan = isochrone(json!({"x": 500, "y": 10, "epsg": 2154}), json!(10));
        let request = "à 500 m de la gare, en 10 minutes, en EPSG:2154";
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            plan,
            request
        )));
    }

    #[test]
    fn other_number_forms_are_not_read() {
        let plan = isochrone(json!({"x": 652000, "y": 6850000, "epsg": 2154}), json!(10));
        for request in [
            "10 min depuis 652.000 / 6.850.000 en EPSG:2154",
            "10 min depuis 652'000 / 6'850'000 en EPSG:2154",
        ] {
            assert!(
                is_guard(&guard_coordinates(
                    &Registry::standard(),
                    plan.clone(),
                    request
                )),
                "{request}"
            );
        }
        let dms = isochrone(json!({"x": 2.458, "y": 48.658, "epsg": 4326}), json!(10));
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            dms,
            "10 min depuis 2°27'29\" E, 48°39'29\" N (EPSG:4326)"
        )));
    }

    #[test]
    fn the_request_numbers_are_read_as_decided() {
        assert_eq!(
            request_numbers("652 000 et 6 850 000"),
            [652000.0, 6850000.0]
        );
        assert_eq!(request_numbers("2,458 / 48.658"), [2.458, 48.658]);
        assert_eq!(request_numbers("10, 500"), [10.0, 500.0]);
        assert_eq!(request_numbers("652000 6850000"), [652000.0, 6850000.0]);
        assert_eq!(request_numbers("de -3 à - 4"), [-3.0, 4.0]);
        assert_eq!(request_numbers("Lambert-93"), [93.0, 2154.0]);
        assert_eq!(request_numbers("EPSG:2154"), [2154.0]);
        assert_eq!(request_numbers("1 234,5 m"), [1234.5]);
        assert_eq!(
            request_numbers("WGS 84 ; wgs84"),
            [84.0, 84.0, 4326.0, 4326.0]
        );
        assert!(request_numbers("Lambert-930").iter().all(|n| *n != 2154.0));
        // Espace fine (U+2009) entre les milliers.
        assert_eq!(request_numbers("6\u{2009}850\u{2009}000"), [6850000.0]);
        // Le numéro d'un système nommé n'ouvre pas de groupe de milliers.
        assert_eq!(
            request_numbers("en Lambert 93 652 000 / 6 850 000"),
            [93.0, 652000.0, 6850000.0, 2154.0]
        );
        assert_eq!(request_numbers("WGS 84 652 000"), [84.0, 652000.0, 4326.0]);
        assert_eq!(request_numbers("le 93 652 000"), [93652000.0]);
    }

    #[test]
    fn a_utm_zone_with_its_hemisphere_is_read_as_its_code() {
        let utm = |request: &str| -> Vec<f64> {
            request_numbers(request)
                .into_iter()
                .filter(|n| (32600.0..32800.0).contains(n))
                .collect()
        };
        for request in [
            "en UTM 31N",
            "en UTM 31 N",
            "en UTM zone 31 Nord",
            "en UTM31N",
            "en utm 31n",
            "(UTM-31N)",
        ] {
            assert_eq!(utm(request), [32631.0], "{request}");
        }
        for request in ["en UTM 31S", "en UTM 31 Sud", "UTM zone 31 sud."] {
            assert_eq!(utm(request), [32731.0], "{request}");
        }
        assert_eq!(utm("UTM 1N, UTM 60S"), [32601.0, 32760.0]);
        for request in [
            "UTM 61N",
            "UTM 0N",
            "UTM 310N",
            "UTM 31",
            "UTM zone 31",
            "UTM 31 Nantes",
            "UTM 31 Sudouest",
            "ED50 UTM 31N",
            "ETRS89 / UTM 31N",
            "ETRS89/UTM 31N",
            "NTF UTM 31N",
            "ed50-utm 31n",
            "ed50utm 31n",
            "gutm 31n",
            "ED50 – UTM 31N",
            "ED50 (UTM 31N)",
            "ED50, UTM 31N",
            "ETRS89 : UTM 31N",
            "ED50\u{2009}UTM 31N",
            "ED 50 UTM 31N",
            "ETRS 89 UTM 31N",
            "UTM 31N (ED50)",
            "UTM 31N ED50",
            "UTM 31 Nord, ETRS89",
            "UTM 31N, ED 50",
            "UTM 31 S (ETRS 89)",
            "UTM 31N NTF",
            "UTM 31 n'importe",
            "UTM 31 s'il te plaît",
            "UTM 31 s\u{2019}il te plaît",
        ] {
            assert!(utm(request).is_empty(), "{request}");
        }
        // Le numéro de la zone reste un nombre ordinaire, comme le 93 de « Lambert-93 ».
        assert_eq!(
            request_numbers("X 412300 Y 1652800 en UTM 31N"),
            [412300.0, 1652800.0, 31.0, 32631.0]
        );
    }

    #[test]
    fn a_point_given_in_utm_leaves_the_plan_unchanged() {
        let registry = Registry::standard();
        let plan = isochrone(json!({"x": 412300, "y": 1652800, "epsg": 32631}), json!(10));
        for request in [
            "Zone à 10 min à pied depuis X 412300 Y 1652800 en UTM 31N.",
            "Zone à 10 min à pied depuis X 412 300, Y 1 652 800 (UTM zone 31 Nord).",
            "Zone à 10 min à pied depuis le point 412300 / 1652800, UTM31N.",
        ] {
            assert_eq!(
                guard_coordinates(&registry, plan.clone(), request),
                plan,
                "{request}"
            );
        }
        let south = isochrone(json!({"x": 412300, "y": 1652800, "epsg": 32731}), json!(10));
        let request = "Zone à 10 min à pied depuis X 412300 Y 1652800 en UTM 31 Sud.";
        assert_eq!(guard_coordinates(&registry, south.clone(), request), south);
    }

    #[test]
    fn a_utm_zone_not_read_is_a_guard_question() {
        let registry = Registry::standard();
        let point =
            |epsg: u32| isochrone(json!({"x": 412300, "y": 1652800, "epsg": epsg}), json!(10));
        for (epsg, request) in [
            (32661, "10 min à pied depuis X 412300 Y 1652800 en UTM 61N"),
            (32600, "10 min à pied depuis X 412300 Y 1652800 en UTM 0N"),
            (32631, "10 min à pied depuis X 412300 Y 1652800 en UTM 31"),
            (
                32631,
                "10 min à pied depuis X 412300 Y 1652800 en UTM zone 31",
            ),
            (
                32631,
                "10 min à pied depuis X 412300 Y 1652800 en UTM 31 Nantes",
            ),
            (
                32631,
                "10 min à pied depuis X 412300 Y 1652800 en UTM 31 Sudouest",
            ),
            (32632, "10 min à pied depuis X 412300 Y 1652800 en UTM 31N"),
            (
                32631,
                "10 min à pied depuis X 412300 Y 1652800 en ED50 UTM 31N",
            ),
            (
                32631,
                "10 min à pied depuis X 412300 Y 1652800 en ETRS89 / UTM 31N",
            ),
            (
                32631,
                "10 min à pied depuis X 412300 Y 1652800 en NTF UTM 31N",
            ),
            (
                32631,
                "10 min à pied depuis X 412300 Y 1652800 en ED 50 (UTM 31N)",
            ),
            (
                32631,
                "10 min à pied depuis X 412300 Y 1652800 en ETRS 89 UTM 31N",
            ),
            (
                32631,
                "10 min à pied depuis X 412300 Y 1652800 en UTM 31N (ED50)",
            ),
            (
                32631,
                "10 min à pied depuis X 412300 Y 1652800, UTM 31 Nord, ETRS89",
            ),
            (
                32631,
                "10 min à pied depuis X 412300 Y 1652800 UTM 31 n'importe",
            ),
            (
                32731,
                "10 min à pied depuis X 412300 Y 1652800 UTM 31 s'il te plaît",
            ),
        ] {
            assert!(
                is_guard(&guard_coordinates(&registry, point(epsg), request)),
                "{epsg} {request}"
            );
        }
    }

    #[test]
    fn a_point_from_a_step_leaves_the_plan_unchanged() {
        let registry = registry_with_locate();
        let plan = calls(json!([
            {"command": "test.locate", "params": {}},
            {"command": "analysis.isochrone", "params": {
                "layer_id": ROUTES, "origin": "step:1.point", "mode": "walk",
                "duration": {"value": 10, "unit": "min"}}}
        ]));
        assert_eq!(guard_coordinates(&registry, plan.clone(), "10 min"), plan);
    }

    #[test]
    fn a_reference_to_a_field_that_is_not_a_point_is_a_guard_question() {
        let registry = registry_with_locate();
        for reference in ["step:1.layer_id", "step:1", "step:2.point", "step:1.nope"] {
            let plan = calls(json!([
                {"command": "test.locate", "params": {}},
                {"command": "analysis.isochrone", "params": {
                    "layer_id": ROUTES, "origin": reference, "mode": "walk",
                    "duration": {"value": 10, "unit": "min"}}}
            ]));
            assert!(
                is_guard(&guard_coordinates(&registry, plan, "10 min")),
                "{reference}"
            );
        }
    }

    #[test]
    fn a_question_or_a_refusal_from_the_model_is_left_unchanged() {
        let registry = Registry::standard();
        let question = Plan::Question {
            question: "Laquelle ?".into(),
            source: QuestionSource::Model,
        };
        assert_eq!(guard_coordinates(&registry, question.clone(), ""), question);
        let refusal = Plan::Refusal {
            reason: "Hors catalogue.".into(),
        };
        assert_eq!(guard_coordinates(&registry, refusal.clone(), ""), refusal);
    }

    #[test]
    fn a_rejected_output_with_an_invented_point_is_guarded_at_once() {
        let registry = Registry::standard();
        let request = "Trouve les parcelles à 10 minutes à pied de la gare de Villeneuve-le-Roi.";
        // Le cas de la recette : une origine inventée sans système, rejetée par `check_plan`.
        let partial = isochrone(json!({"x": 2.458, "y": 48.658}), json!(10));
        assert!(registry.check_plan(&partial).is_err());
        assert!(guard_rejected(&registry, &partial, request).is_some_and(|q| is_guard(&q)));
        // Un rejet sans affaire de coordonnées suit son cours : commande inconnue, point donné.
        let unknown = calls(json!([{"command": "no.such", "params": {}}]));
        assert_eq!(guard_rejected(&registry, &unknown, request), None);
        let given = calls(json!([{"command": "analysis.isochrone", "params": {
            "layer_id": ROUTES, "origin": {"x": 652000, "y": 6850000, "epsg": 2154},
            "mode": "walk", "duration": {"value": 10, "unit": "min"}, "out": "x.gpkg"}}]));
        assert_eq!(
            guard_rejected(
                &registry,
                &given,
                "10 min depuis 652 000 / 6 850 000 (EPSG:2154)"
            ),
            None
        );
        // Une question ou un refus rejetés ne sont pas des points.
        let empty = Plan::Question {
            question: " ".into(),
            source: QuestionSource::Model,
        };
        assert_eq!(guard_rejected(&registry, &empty, request), None);
    }

    fn convert(params: Value) -> Value {
        json!({"command": "coord.convert", "params": params})
    }

    #[test]
    fn the_mgrs_schema_is_marked_like_a_point() {
        let registry = Registry::standard();
        let spec = registry.spec("coord.convert").unwrap();
        let root = &spec.params_schema;
        assert!(is_mgrs(&root["properties"]["mgrs"], root));
        assert!(!is_point(&root["properties"]["mgrs"], root));
        assert_eq!(root["$defs"]["MgrsRef"]["x-cassini"], MGRS_MARK);
        assert_eq!(root["$defs"]["MgrsRef"]["type"], "string");
        // La chaîne passe telle quelle : sa grammaire est lue par la commande.
        assert_eq!(
            serde_json::from_value::<MgrsRef>(json!("31u dq 48251 11750")).unwrap(),
            MgrsRef("31u dq 48251 11750".into())
        );
    }

    #[test]
    fn a_mgrs_absent_from_the_request_is_a_guard_question() {
        let registry = Registry::standard();
        let plan = calls(json!([convert(
            json!({"mgrs": "31UDQ4825111750", "to": ["wgs84"]})
        )]));
        for request in [
            "Convertis ce point en WGS84.",
            // D'autres chiffres que ceux du plan.
            "Convertis 31U DQ 48251 11751 en WGS84.",
            // Une autre zone, ou une autre précision.
            "Convertis 32U DQ 48251 11750 en WGS84.",
            "Convertis 31U DQ 4825 1175 en WGS84.",
        ] {
            assert!(
                is_guard(&guard_coordinates(&registry, plan.clone(), request)),
                "{request}"
            );
            assert!(
                guard_rejected(&registry, &plan, request).is_some(),
                "{request}"
            );
        }
        // Une référence qui n'est pas une chaîne n'est pas une référence recopiée.
        let odd = calls(json!([convert(json!({"mgrs": 31, "to": ["wgs84"]}))]));
        assert!(is_guard(&guard_coordinates(&registry, odd, "31 en WGS84")));
    }

    #[test]
    fn a_mgrs_written_in_the_request_leaves_the_plan_unchanged() {
        let registry = Registry::standard();
        let plan = calls(json!([convert(
            json!({"mgrs": "31UDQ4825111750", "to": ["wgs84", "utm"]})
        )]));
        for request in [
            "Donne-moi 31U DQ 48251 11750 en WGS84 et en UTM.",
            "Convertis 31u dq 48251 11750, puis montre-le en UTM.",
            "Convertis 31UDQ4825111750 en WGS84 et UTM",
            "Convertis 31UDQ 48251 11750 10 fois",
            "Où est « 31U DQ 4825111750 » ?",
        ] {
            assert_eq!(
                guard_coordinates(&registry, plan.clone(), request),
                plan,
                "{request}"
            );
        }
        // Le modèle peut écrire la référence avec espaces ou en minuscules, et une zone sur un
        // chiffre se compare à sa forme sur deux.
        let spaced = calls(json!([convert(
            json!({"mgrs": "31u dq 48251 11750", "to": ["wgs84"]})
        )]));
        assert_eq!(
            guard_coordinates(&registry, spaced.clone(), "Convertis 31UDQ4825111750."),
            spaced
        );
        let short = calls(json!([convert(
            json!({"mgrs": "4QFJ1234567890", "to": ["wgs84"]})
        )]));
        for request in ["Où tombe 4QFJ 12345 67890 ?", "Où tombe 04QFJ1234567890 ?"] {
            assert_eq!(
                guard_coordinates(&registry, short.clone(), request),
                short,
                "{request}"
            );
        }
        // Une référence mal formée mais recopiée passe la garde : la commande la refusera en
        // nommant la partie fautive.
        let bad = calls(json!([convert(
            json!({"mgrs": "31IDQ4825111750", "to": ["wgs84"]})
        )]));
        assert_eq!(
            guard_coordinates(&registry, bad.clone(), "Convertis 31I DQ 48251 11750."),
            bad
        );
    }

    #[test]
    fn the_digits_of_a_mgrs_do_not_make_a_point() {
        let registry = Registry::standard();
        let request = "Convertis 31U DQ 48251 11750 en UTM 31N, puis en WGS84.";
        for point in [
            json!({"x": 48251, "y": 11750, "epsg": 32631}),
            json!({"x": 48251, "y": 11750, "epsg": 31}),
        ] {
            let plan = calls(json!([convert(json!({"point": point, "to": ["wgs84"]}))]));
            assert!(is_guard(&guard_coordinates(&registry, plan, request)));
        }
        // Les autres nombres de la demande restent lus.
        let plan = calls(json!([
            convert(json!({"mgrs": "31UDQ4825111750", "to": ["wgs84"]})),
            convert(json!({"point": {"x": 2.4, "y": 48.7, "epsg": 4326}, "to": ["mgrs"]}))
        ]));
        let request = "Convertis 31U DQ 48251 11750 en WGS84, et 2,4 / 48,7 (WGS84) en MGRS.";
        assert_eq!(guard_coordinates(&registry, plan.clone(), request), plan);
    }

    #[test]
    fn a_number_after_a_mgrs_like_reading_stays_available_to_a_point() {
        // « 25m de 652000 » se lit `25MDE652000`, bien formé : 652000 reste pour x.
        let registry = Registry::standard();
        let plan = calls(json!([{"command": "terrain.line_of_sight", "params": {
            "dem": "mnt.tif",
            "observer": {"x": 652000, "y": 6862000, "epsg": 2154},
            "target": {"x": 653000, "y": 6863000, "epsg": 2154},
            "observer_height": {"value": 25, "unit": "m"}}}]));
        let request =
            "Ligne de vue à 25m de 652000 6862000 vers 653000 6863000 en Lambert-93 sur mnt.tif";
        assert_eq!(guard_coordinates(&registry, plan.clone(), request), plan);
    }

    #[test]
    fn a_spaced_zone_band_or_square_is_still_read() {
        let registry = Registry::standard();
        let plan = calls(json!([convert(
            json!({"mgrs": "31UDQ4825111750", "to": ["wgs84"]})
        )]));
        for request in [
            "Convertis 31 U DQ 48251 11750 en WGS84",
            "Convertis 31U D Q 48251 11750 en WGS84",
            "Convertis 31 U D Q 4825111750 en WGS84",
        ] {
            assert_eq!(
                guard_coordinates(&registry, plan.clone(), request),
                plan,
                "{request}"
            );
        }
    }

    #[test]
    fn the_digits_of_a_used_malformed_mgrs_do_not_make_a_point() {
        let registry = Registry::standard();
        let plan = calls(json!([
            convert(json!({"mgrs": "31IDQ4825111750", "to": ["wgs84"]})),
            convert(json!({"point": {"x": 48251, "y": 11750, "epsg": 32631}, "to": ["wgs84"]}))
        ]));
        let request = "Convertis 31I DQ 48251 11750, puis le point en UTM 31N.";
        assert!(is_guard(&guard_coordinates(&registry, plan, request)));
    }

    #[test]
    fn a_mgrs_from_a_step_leaves_the_plan_unchanged() {
        let registry = Registry::standard();
        let plan = calls(json!([
            convert(json!({"point": {"x": 2.4, "y": 48.7, "epsg": 4326}, "to": ["mgrs", "wgs84"]})),
            convert(json!({"mgrs": "step:1.mgrs", "to": ["utm"]})),
            convert(json!({"point": "step:1.wgs84", "to": ["utm"]}))
        ]));
        let request = "Mets 2,4 / 48,7 (WGS84) en MGRS, puis la case en UTM.";
        assert_eq!(guard_coordinates(&registry, plan.clone(), request), plan);
    }

    #[test]
    fn a_reference_to_a_field_that_is_not_a_mgrs_is_a_guard_question() {
        let registry = registry_with_locate();
        let first = convert(json!({"point": {"x": 2.4, "y": 48.7, "epsg": 4326},
            "to": ["mgrs", "utm"]}));
        let request = "Mets 2,4 / 48,7 (WGS84) en MGRS.";
        for (param, reference) in [
            ("mgrs", "step:1.utm"),
            ("mgrs", "step:1"),
            ("mgrs", "step:3.mgrs"),
            ("point", "step:1.mgrs"),
        ] {
            let plan = calls(json!([
                first,
                convert(json!({param: reference, "to": ["wgs84"]}))
            ]));
            assert!(
                is_guard(&guard_coordinates(&registry, plan, request)),
                "{param} {reference}"
            );
        }
        // Un champ `Point` d'une autre commande ne tient pas lieu de référence MGRS.
        let plan = calls(json!([
            {"command": "test.locate", "params": {}},
            convert(json!({"mgrs": "step:1.point", "to": ["wgs84"]}))
        ]));
        assert!(is_guard(&guard_coordinates(&registry, plan, "")));
    }

    #[test]
    fn a_plan_without_point_is_left_unchanged() {
        let plan = calls(json!([{"command": "analysis.buffer", "params": {
            "layer_id": "gares_1", "distance": {"value": 300, "unit": "m"}}}]));
        assert_eq!(
            guard_coordinates(&Registry::standard(), plan.clone(), "Tampon."),
            plan
        );
    }

    #[test]
    fn an_export_at_a_scale_is_left_unchanged() {
        // `scale` est un nombre du plan, pas un `Point` : le garde-fou n'a rien à demander.
        let plan = calls(json!([{"command": "layout.export_pdf", "params": {
            "title": "Carte", "scale": 25000}}]));
        for request in [
            "Exporte la carte au 1:25 000",
            "Exporte la carte au 1/25000",
            "Exporte la carte",
        ] {
            assert_eq!(
                guard_coordinates(&Registry::standard(), plan.clone(), request),
                plan,
                "{request}"
            );
        }
    }

    #[test]
    fn a_coordinate_outside_the_schema_is_still_guarded() {
        // Commande inconnue : rien n'est deviné.
        let unknown = calls(json!([{"command": "no.such", "params": {}}]));
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            unknown,
            ""
        )));
        // Un objet à `x`/`y` hors schéma est lu comme un point.
        let stray = calls(json!([{"command": "project.inventory", "params": {
            "where": {"x": 1, "y": 2, "epsg": 2154}}}]));
        assert!(is_guard(&guard_coordinates(
            &Registry::standard(),
            stray,
            "rien"
        )));
    }
}
