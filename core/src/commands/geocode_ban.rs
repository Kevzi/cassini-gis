//! Commande `geocode.ban` : géocodage d'un CSV d'adresses sur un CSV de la Base Adresse
//! Nationale, dans le noyau (AD-26), sans réseau ni GDAL (AD-1).
//!
//! La BAN est lue comme un `adresses-XX.csv` décompressé : séparateur `;`, UTF-8, colonnes
//! `numero`, `rep`, `nom_voie`, `code_postal`, `nom_commune`, `lon`, `lat`. L'appariement est
//! déterministe et exact après normalisation (accents, casse, ponctuation, abréviations
//! usuelles) : aucun flou typographique. Une adresse sans correspondance unique reste dans la
//! sortie, `lon` et `lat` vides. Aucune reprojection (AD-19) : la BAN est en WGS84, la sortie
//! le déclare. Les deux fichiers d'entrée ne sont jamais modifiés ; le noyau nomme la sortie
//! dans le dossier de travail (AD-10).

use std::collections::{HashMap, HashSet};
use std::fs::File;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

/// Colonnes que la BAN doit porter.
const BAN_COLUMNS: [&str; 7] = [
    "numero",
    "rep",
    "nom_voie",
    "code_postal",
    "nom_commune",
    "lon",
    "lat",
];

/// Paramètres de `geocode.ban`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeocodeBanParams {
    /// CSV d'adresses à placer, chemin relatif au projet (UTF-8, séparateur `;`, `,` ou tabulation).
    /// Address CSV to place, path relative to the project.
    pub addresses: String,
    /// CSV de la Base Adresse Nationale (`adresses-XX.csv` décompressé), chemin relatif au projet.
    /// National Address Base CSV (unzipped `adresses-XX.csv`), path relative to the project.
    pub ban: String,
    /// Colonne de `addresses` qui porte l'adresse (numéro et voie, ex. « 12 av. de la Gare »).
    pub address_field: String,
    /// Colonne du code postal : s'il est donné pour une adresse, il départage les homonymes.
    #[serde(default)]
    pub postcode_field: Option<String>,
    /// Colonne de la commune : si elle est donnée pour une adresse, elle départage les homonymes.
    #[serde(default)]
    pub city_field: Option<String>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

/// Résultat de `geocode.ban`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GeocodeBanOutput {
    /// CSV écrit : les colonnes d'origine, puis `lon` et `lat`.
    pub path: String,
    /// Système des colonnes `lon` et `lat` : toujours `EPSG:4326`.
    pub crs: String,
    /// Colonne des abscisses : `lon`.
    pub x_field: String,
    /// Colonne des ordonnées : `lat`.
    pub y_field: String,
    /// Nombre d'adresses placées.
    pub matched_count: u64,
    /// Nombre d'adresses sans correspondance unique (`lon` et `lat` vides).
    pub unmatched_count: u64,
}

fn invalid(message: String) -> CommandFailure {
    CommandFailure::new("registry.invalid_params", message)
}

fn unreadable(path: &str, why: impl std::fmt::Display) -> CommandFailure {
    CommandFailure::new(
        "geocode.unreadable_csv",
        format!("Le fichier « {path} » n'a pas pu être lu comme un CSV UTF-8 : {why}."),
    )
}

/// Lettre sans accent d'un caractère latin courant, en minuscule.
fn fold(c: char) -> &'static str {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "a",
        'ç' | 'Ç' => "c",
        'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => "e",
        'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => "i",
        'ñ' | 'Ñ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => "o",
        'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => "u",
        'ý' | 'ÿ' | 'Ý' | 'Ÿ' => "y",
        'œ' | 'Œ' => "oe",
        'æ' | 'Æ' => "ae",
        _ => "",
    }
}

/// Forme complète d'une abréviation usuelle de voie ou de saint.
fn expand(token: &str) -> &str {
    match token {
        "av" | "ave" => "avenue",
        "bd" | "bld" | "bvd" => "boulevard",
        "imp" => "impasse",
        "all" => "allee",
        "pl" => "place",
        "st" => "saint",
        "ste" => "sainte",
        "chem" => "chemin",
        "rte" => "route",
        other => other,
    }
}

/// Texte comparable : minuscules, sans accent ni ponctuation, mots séparés par une espace,
/// abréviations usuelles développées.
fn normalize(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    for c in text.chars() {
        let folded = fold(c);
        if ('\u{300}'..='\u{36f}').contains(&c) {
            // Accent combinant (texte décomposé, export macOS) : il se perd comme l'accent précomposé.
            continue;
        }
        if !folded.is_empty() {
            plain.push_str(folded);
        } else if c.is_alphanumeric() {
            plain.extend(c.to_lowercase());
        } else {
            plain.push(' ');
        }
    }
    plain
        .split_whitespace()
        .map(expand)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Code postal comparable : chiffres seuls, complétés à 5 par des zéros de tête perdus.
fn normalize_postcode(text: &str) -> String {
    let t = text.trim();
    if !t.is_empty() && t.len() < 5 && t.chars().all(|c| c.is_ascii_digit()) {
        format!("{t:0>5}")
    } else {
        t.to_string()
    }
}

/// Adresse découpée : numéro, répétition (bis, ter, b…), voie normalisée.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct AddressKey {
    number: String,
    rep: String,
    street: String,
}

/// Découpe « 12 bis rue du Lac » ou « 12b rue du Lac, 74000 Annecy » ; `None` sans numéro ni voie.
fn parse_address(text: &str) -> Option<AddressKey> {
    // Ce qui suit la première virgule (code postal, commune) n'est pas la voie.
    // Une virgule collée au numéro (« 12, rue de la Gare ») ne coupe pas l'adresse.
    let trimmed = text.trim_start();
    let digits_end = trimmed
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(trimmed.len());
    let text = match trimmed[digits_end..].trim_start().strip_prefix(',') {
        Some(rest) if digits_end > 0 => format!("{} {}", &trimmed[..digits_end], rest),
        _ => text.to_string(),
    };
    let head = text.split(',').next().unwrap_or("");
    let normalized = normalize(head);
    let mut tokens = normalized.split(' ').filter(|t| !t.is_empty());
    let first = tokens.next()?;
    let digits: String = first.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let number = digits.trim_start_matches('0').to_string();
    let mut rep = first[digits.len()..].to_string();
    let mut rest: Vec<&str> = tokens.collect();
    if rep.is_empty() {
        if let Some(&next) = rest.first() {
            if matches!(next, "bis" | "ter" | "quater") {
                rep = next.to_string();
                rest.remove(0);
            }
        }
    }
    if number.is_empty() || rest.is_empty() {
        return None;
    }
    Some(AddressKey {
        number,
        rep,
        street: rest.join(" "),
    })
}

/// Une adresse de la BAN retenue : filtres et point (texte d'origine, recopié tel quel).
struct BanPoint {
    postcode: String,
    city: String,
    lon: String,
    lat: String,
}

/// Séparateur d'un CSV d'après sa ligne d'en-tête : `;`, puis `,`, puis tabulation à égalité.
fn detect_delimiter(text: &str) -> u8 {
    let header = text.lines().next().unwrap_or("");
    let count = |c: char| header.matches(c).count();
    let (semi, comma, tab) = (count(';'), count(','), count('\t'));
    if semi >= comma && semi >= tab && semi > 0 {
        b';'
    } else if comma >= tab && comma > 0 {
        b','
    } else if tab > 0 {
        b'\t'
    } else {
        b','
    }
}

fn column_index(headers: &[String], name: &str, role: &str) -> Result<usize, CommandFailure> {
    headers.iter().position(|h| h == name).ok_or_else(|| {
        invalid(format!(
            "La colonne « {name} » ({role}) n'existe pas dans le fichier d'adresses."
        ))
    })
}

fn is_coordinate(text: &str, limit: f64) -> bool {
    text.trim()
        .parse::<f64>()
        .is_ok_and(|v| v.is_finite() && v.abs() <= limit)
}

/// Lit la BAN et ne garde que les adresses de `wanted`.
fn read_ban(
    path: &str,
    wanted: &HashSet<AddressKey>,
) -> Result<HashMap<AddressKey, Vec<BanPoint>>, CommandFailure> {
    let file = File::open(path).map_err(|e| unreadable(path, e))?;
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .flexible(true)
        .from_reader(file);
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| unreadable(path, e))?
        .iter()
        .map(|h| h.trim_start_matches('\u{feff}').to_string())
        .collect();
    let mut at = [0usize; 7];
    for (slot, name) in at.iter_mut().zip(BAN_COLUMNS) {
        *slot = headers.iter().position(|h| h == name).ok_or_else(|| {
            CommandFailure::new(
                "geocode.ban_columns",
                format!("Le fichier de la BAN « {path} » n'a pas la colonne « {name} »."),
            )
        })?;
    }
    let [i_num, i_rep, i_voie, i_cp, i_city, i_lon, i_lat] = at;
    let mut index: HashMap<AddressKey, Vec<BanPoint>> = HashMap::new();
    for record in reader.records() {
        let record = record.map_err(|e| unreadable(path, e))?;
        let get = |i: usize| record.get(i).unwrap_or("");
        let key = AddressKey {
            number: get(i_num).trim().trim_start_matches('0').to_string(),
            rep: normalize(get(i_rep)),
            street: normalize(get(i_voie)),
        };
        if !wanted.contains(&key) || !is_coordinate(get(i_lon), 180.0) || !is_coordinate(get(i_lat), 90.0)
        {
            continue;
        }
        index.entry(key).or_default().push(BanPoint {
            postcode: normalize_postcode(get(i_cp)),
            city: normalize(get(i_city)),
            lon: get(i_lon).trim().to_string(),
            lat: get(i_lat).trim().to_string(),
        });
    }
    Ok(index)
}

/// Point unique de l'adresse, après les filtres donnés ; `None` s'il n'y en a aucun ou plusieurs.
fn resolve<'a>(
    candidates: &'a [BanPoint],
    postcode: &str,
    city: &str,
) -> Option<&'a BanPoint> {
    let postcode = normalize_postcode(postcode);
    let city = normalize(city);
    let mut kept = candidates
        .iter()
        .filter(|c| postcode.is_empty() || c.postcode == postcode)
        .filter(|c| city.is_empty() || c.city == city);
    let first = kept.next()?;
    kept.all(|c| c.lon == first.lon && c.lat == first.lat)
        .then_some(first)
}

/// `geocode.ban` : commande du noyau.
pub struct GeocodeBan;

impl Command for GeocodeBan {
    type Params = GeocodeBanParams;
    type Output = GeocodeBanOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<GeocodeBanParams, GeocodeBanOutput>(
            "geocode.ban",
            "Géocode un CSV d'adresses sur la Base Adresse Nationale : apparie chaque adresse (numéro et voie, dans la colonne `address_field` du CSV `addresses`) à une adresse du CSV `ban` (adresses-XX.csv décompressé, que l'utilisateur apporte), sans tenir compte des accents, de la casse, de la ponctuation ni des abréviations usuelles (av, bd, imp, all, pl, st). Le code postal (`postcode_field`) ou la commune (`city_field`) départagent les homonymes. Écrit un nouveau CSV avec les colonnes d'origine plus `lon` et `lat` en EPSG:4326, vides pour une adresse sans correspondance unique (comptée dans `unmatched_count`). Pour la placer sur la carte, enchaîner `layer.add` avec `path`, `crs`, `x_field` et `y_field` en `step:k`. Aucun réseau ; ne modifie pas les fichiers d'entrée.",
            "Geocodes an address CSV against the French National Address Base: matches each address (number and street, in the `address_field` column of the `addresses` CSV) to an address of the `ban` CSV (unzipped adresses-XX.csv, supplied by the user), ignoring accents, case, punctuation and usual abbreviations (av, bd, imp, all, pl, st). The postcode (`postcode_field`) or the city (`city_field`) break ties between homonyms. Writes a new CSV with the original columns plus `lon` and `lat` in EPSG:4326, empty for an address with no unique match (counted in `unmatched_count`). To put it on the map, chain `layer.add` with `path`, `crs`, `x_field` and `y_field` as `step:k`. No network; does not modify the input files.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .path_params(&["addresses", "ban"])
        .output_params(&["out"])
        .output_ext(".csv")
        .example(serde_json::json!({"addresses": "adresses-test.csv", "ban": "ban-test.csv", "address_field": "adresse"}))
        .example(serde_json::json!({"addresses": "adresses-test.csv", "ban": "ban-test.csv", "address_field": "adresse", "postcode_field": "cp", "city_field": "commune"}))
        .tests(&[
            "an_exact_address_gets_the_ban_point",
            "a_differently_written_address_gets_the_same_point",
            "an_unknown_street_is_kept_empty_and_counted",
            "an_ambiguous_address_is_unmatched_until_a_filter_splits_it",
            "an_unknown_column_is_refused_naming_it",
            "a_ban_without_its_columns_is_refused",
            "two_runs_give_identical_bytes",
            "the_input_files_stay_untouched",
            "geocode_ban_runs_in_the_core_and_the_contract_accepts_it",
            "a_geocode_then_layer_add_plan_passes_the_check",
            "addresses_are_normalized_and_split",
        ])
    }

    fn run(&self, params: GeocodeBanParams) -> Result<GeocodeBanOutput, CommandFailure> {
        if params.out.is_empty() {
            return Err(invalid("Le fichier de sortie n'est pas fourni.".to_string()));
        }
        let raw = std::fs::read(&params.addresses).map_err(|e| unreadable(&params.addresses, e))?;
        let text = String::from_utf8(raw).map_err(|e| unreadable(&params.addresses, e))?;
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let delimiter = detect_delimiter(text);
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .from_reader(text.as_bytes());
        let headers: Vec<String> = reader
            .headers()
            .map_err(|e| unreadable(&params.addresses, e))?
            .iter()
            .map(str::to_string)
            .collect();
        let i_address = column_index(&headers, &params.address_field, "adresse")?;
        let i_postcode = params
            .postcode_field
            .as_deref()
            .map(|n| column_index(&headers, n, "code postal"))
            .transpose()?;
        let i_city = params
            .city_field
            .as_deref()
            .map(|n| column_index(&headers, n, "commune"))
            .transpose()?;
        for added in ["lon", "lat"] {
            if headers.iter().any(|h| h == added) {
                return Err(invalid(format!(
                    "Le fichier d'adresses a déjà une colonne « {added} » : la sortie ne peut pas en ajouter une seconde."
                )));
            }
        }
        let mut rows = Vec::new();
        for record in reader.records() {
            rows.push(record.map_err(|e| unreadable(&params.addresses, e))?);
        }
        let keys: Vec<Option<AddressKey>> = rows
            .iter()
            .map(|r| parse_address(r.get(i_address).unwrap_or("")))
            .collect();
        let wanted: HashSet<AddressKey> = keys.iter().flatten().cloned().collect();
        let index = read_ban(&params.ban, &wanted)?;

        let unwritable = |e: &dyn std::fmt::Display| {
            CommandFailure::new(
                "geocode.unwritable",
                format!("Le CSV géocodé n'a pas pu être écrit : {e}."),
            )
        };
        let file = File::create(&params.out).map_err(|e| unwritable(&e))?;
        let mut writer = csv::WriterBuilder::new()
            // `layer.add` lit `,` et `;` (pas la tabulation) : la sortie passe alors en `;`.
            .delimiter(if delimiter == b'\t' { b';' } else { delimiter })
            .terminator(csv::Terminator::Any(b'\n'))
            .from_writer(file);
        let mut header = csv::StringRecord::from(headers.clone());
        header.push_field("lon");
        header.push_field("lat");
        writer.write_record(&header).map_err(|e| unwritable(&e))?;
        let (mut matched, mut unmatched) = (0u64, 0u64);
        for (row, key) in rows.iter().zip(&keys) {
            let field = |i: Option<usize>| i.and_then(|i| row.get(i)).unwrap_or("");
            let found = key
                .as_ref()
                .and_then(|k| index.get(k))
                .and_then(|c| resolve(c, field(i_postcode), field(i_city)));
            let mut out = row.clone();
            match found {
                Some(p) => {
                    matched += 1;
                    out.push_field(&p.lon);
                    out.push_field(&p.lat);
                }
                None => {
                    unmatched += 1;
                    out.push_field("");
                    out.push_field("");
                }
            }
            writer.write_record(&out).map_err(|e| unwritable(&e))?;
        }
        writer.flush().map_err(|e| unwritable(&e))?;
        Ok(GeocodeBanOutput {
            path: params.out,
            crs: "EPSG:4326".to_string(),
            x_field: "lon".to_string(),
            y_field: "lat".to_string(),
            matched_count: matched,
            unmatched_count: unmatched,
        })
    }
}

inventory::submit! { Registration(|r| r.register(GeocodeBan)) }

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::commands::test_support::{fixture, Scratch};
    use crate::registry::{Registry, RegistryError};
    use serde_json::{json, Value};

    fn geocode(extra: Value, out: &Path) -> Result<Value, RegistryError> {
        let mut params = json!({
            "addresses": fixture("adresses-test.csv").to_string_lossy(),
            "ban": fixture("ban-test.csv").to_string_lossy(),
            "address_field": "adresse",
            "out": out.to_string_lossy(),
        });
        for (k, v) in extra.as_object().unwrap() {
            params[k] = v.clone();
        }
        Registry::standard().call("geocode.ban", params)
    }

    /// Lignes du CSV de sortie, champs séparés par la virgule (les fixtures n'en citent aucune).
    fn rows(out: &Path) -> Vec<Vec<String>> {
        std::fs::read_to_string(out)
            .unwrap()
            .lines()
            .map(|l| l.split(',').map(str::to_string).collect())
            .collect()
    }

    #[test]
    fn addresses_are_normalized_and_split() {
        assert_eq!(normalize("Rue de l'Église"), "rue de l eglise");
        assert_eq!(normalize("12 AV. DE LA GARE"), "12 avenue de la gare");
        assert_eq!(normalize("Place Saint-Maurice"), normalize("pl st maurice"));
        let key = parse_address("12 bis rue du Lac, 74000 Annecy").unwrap();
        assert_eq!((key.number.as_str(), key.rep.as_str()), ("12", "bis"));
        assert_eq!(key.street, "rue du lac");
        let key = parse_address("007B Bd Foch").unwrap();
        assert_eq!(
            (key.number.as_str(), key.rep.as_str(), key.street.as_str()),
            ("7", "b", "boulevard foch")
        );
        assert_eq!(parse_address("12, rue du Lac, 74000 Annecy"), parse_address("12 rue du Lac"));
        assert_eq!(normalize("L'HAŸ"), normalize("l'Haÿ"));
        assert_eq!(normalize("E\u{301}glise"), normalize("Église"));
        assert_eq!(parse_address("Avenue de la Gare"), None);
        assert_eq!(parse_address("12"), None);
        assert_eq!(parse_address(""), None);
        assert_eq!(normalize_postcode("1000"), "01000");
    }

    #[test]
    fn an_exact_address_gets_the_ban_point() {
        let dir = Scratch::new("exacte");
        let out = dir.join("geo.csv");
        let result = geocode(json!({"postcode_field": "cp", "city_field": "commune"}), &out).unwrap();
        assert_eq!(result["path"], out.to_string_lossy().as_ref());
        assert_eq!(result["crs"], "EPSG:4326");
        assert_eq!((result["x_field"].as_str(), result["y_field"].as_str()), (Some("lon"), Some("lat")));
        assert_eq!(result["matched_count"], 7);
        assert_eq!(result["unmatched_count"], 2);
        let rows = rows(&out);
        assert_eq!(rows.len(), 10);
        assert_eq!(&rows[0][5..], ["lon", "lat"]);
        assert_eq!(&rows[1][5..], ["6.121000", "45.901000"]);
    }

    #[test]
    fn a_differently_written_address_gets_the_same_point() {
        let dir = Scratch::new("autrement");
        let out = dir.join("geo.csv");
        geocode(json!({"postcode_field": "cp", "city_field": "commune"}), &out).unwrap();
        let rows = rows(&out);
        // Casse et abréviation, sans filtre.
        assert_eq!(&rows[2][5..], &rows[1][5..]);
        // Sans accent, commune en minuscules.
        assert_eq!(&rows[3][5..], ["6.122000", "45.902000"]);
        // « pl st maurice » : abréviations de place et de saint.
        assert_eq!(&rows[4][5..], ["6.123000", "45.903000"]);
        // Répétition : « 7 bis » et « 7 » sont deux points.
        assert_eq!(&rows[8][5..], ["6.125500", "45.905500"]);
        assert_eq!(&rows[9][5..], ["6.125000", "45.905000"]);
    }

    #[test]
    fn an_unknown_street_is_kept_empty_and_counted() {
        let dir = Scratch::new("introuvable");
        let out = dir.join("geo.csv");
        geocode(json!({}), &out).unwrap();
        let rows = rows(&out);
        assert_eq!(rows.len(), 10);
        assert_eq!(rows[5][1], "99 Rue Inconnue");
        assert_eq!(&rows[5][5..], ["", ""]);
    }

    #[test]
    fn an_ambiguous_address_is_unmatched_until_a_filter_splits_it() {
        let dir = Scratch::new("ambigue");
        let out = dir.join("geo.csv");
        let result = geocode(json!({}), &out).unwrap();
        // Sans filtre : la rue de l'Église existe à Annecy et à Sévrier, les deux lignes
        // « rue de l'Église » restent non appariées.
        assert_eq!(&rows(&out)[6][5..], ["", ""]);
        assert_eq!(&rows(&out)[7][5..], ["", ""]);
        assert_eq!(result["matched_count"], 6);
        // Le code postal de la ligne 7 la départage.
        let result = geocode(json!({"postcode_field": "cp"}), &out).unwrap();
        assert_eq!(&rows(&out)[6][5..], ["", ""]);
        assert_eq!(&rows(&out)[7][5..], ["6.140000", "45.850000"]);
        assert_eq!(result["matched_count"], 7);
        // Un code postal qui contredit la BAN n'apparie pas.
        let dir2 = Scratch::new("contredit");
        let addresses = dir2.join("a.csv");
        std::fs::write(&addresses, "adresse,cp\n12 avenue de la gare,73000\n").unwrap();
        let result = geocode(
            json!({"addresses": addresses.to_string_lossy(), "postcode_field": "cp"}),
            &out,
        )
        .unwrap();
        assert_eq!(result["matched_count"], 0);
        assert_eq!(result["unmatched_count"], 1);
    }

    #[test]
    fn an_unknown_column_is_refused_naming_it() {
        let dir = Scratch::new("colonne");
        let out = dir.join("geo.csv");
        let err = geocode(json!({"address_field": "rue_et_numero"}), &out).unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
        assert!(err.to_string().contains("rue_et_numero"), "{err}");
        let err = geocode(json!({"postcode_field": "zip"}), &out).unwrap_err();
        assert_eq!(err.code(), "registry.invalid_params");
        assert!(err.to_string().contains("zip"), "{err}");
        assert!(!out.exists());
    }

    #[test]
    fn a_ban_without_its_columns_is_refused() {
        let dir = Scratch::new("ban-colonnes");
        let ban = dir.join("ban.csv");
        std::fs::write(&ban, "numero;rep;nom_voie\n1;;Rue X\n").unwrap();
        let out = dir.join("geo.csv");
        let err = geocode(json!({"ban": ban.to_string_lossy()}), &out).unwrap_err();
        assert_eq!(err.code(), "geocode.ban_columns");
        assert!(err.to_string().contains("code_postal"), "{err}");
        let err = geocode(json!({"ban": dir.join("absent.csv").to_string_lossy()}), &out).unwrap_err();
        assert_eq!(err.code(), "geocode.unreadable_csv");
    }

    #[test]
    fn two_runs_give_identical_bytes() {
        let dir = Scratch::new("deterministe");
        let (a, b) = (dir.join("a.csv"), dir.join("b.csv"));
        let extra = json!({"postcode_field": "cp", "city_field": "commune"});
        geocode(extra.clone(), &a).unwrap();
        geocode(extra, &b).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    }

    #[test]
    fn the_input_files_stay_untouched() {
        let dir = Scratch::new("intacts");
        let out = dir.join("geo.csv");
        let paths = [fixture("adresses-test.csv"), fixture("ban-test.csv")];
        let before: Vec<Vec<u8>> = paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
        geocode(json!({"postcode_field": "cp"}), &out).unwrap();
        let after: Vec<Vec<u8>> = paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn geocode_ban_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("geocode.ban").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert_eq!(spec.cancellable, Some(false));
        assert!(!spec.dictated_by_core);
        assert_eq!(spec.path_params, vec!["addresses".to_string(), "ban".to_string()]);
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".csv");
        assert!(registry.check_contract().is_empty(), "{:?}", registry.check_contract());
    }

    #[test]
    fn a_geocode_then_layer_add_plan_passes_the_check() {
        let registry = Registry::standard();
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "geocode.ban", "params": {"addresses": "adresses.csv",
                "ban": "adresses-74.csv", "address_field": "adresse", "postcode_field": "cp"}},
            {"command": "layer.add", "params": {"path": "step:1.path", "crs": "step:1.crs",
                "x_field": "step:1.x_field", "y_field": "step:1.y_field"}}
        ]}))
        .unwrap();
        let checked = registry.check_plan(&plan);
        assert!(checked.is_ok(), "{checked:?}");
    }
}
