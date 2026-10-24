//! Inventaire vu par l'agent (ticket 5.10, tâche 4a).
//!
//! L'extension rend, par `project.inventory`, des faits bruts de QGIS : SCR en code explicite,
//! type de champ nommé par le fournisseur, nom WKB de la géométrie. Le modèle ajusté, lui, a
//! appris sur des inventaires d'une autre forme, celle du générateur de `training/` :
//!
//! ```json
//! {"project": {"name": "...", "epsg": 2154},
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
//! par ne plus tenir dans le contexte. `within_budget` borne l'inventaire vu par le modèle : toutes
//! les couches de l'utilisateur, et les couches de résultat les plus récentes qui tiennent, dans
//! l'ordre de l'inventaire. Rien n'est retiré de QGIS, et la forme d'une couche gardée ne change pas.

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

/// Un fichier du dossier de travail Cassini du projet (`<projet>.cassini/`, ou de son dossier
/// de rejeu `replay/`) et sa date de
/// modification : une couche dont la source porte ce nom est une couche de résultat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkFile {
    /// Nom du fichier, sans dossier (la forme de `source` dans l'inventaire).
    pub name: String,
    pub modified: SystemTime,
}

/// Nombre de caractères du texte JSON compact de `value`.
fn json_chars(value: &Value) -> usize {
    value.to_string().chars().count()
}

/// L'inventaire traduit, borné à `budget` caractères pour le modèle.
///
/// Sous le budget, l'inventaire est rendu tel quel. Au-delà, seules des couches de résultat
/// (source nommée dans `work_files`) sont retirées, l'ordre de l'inventaire restant celui
/// d'entrée : les plus récentes sont gardées tant qu'elles tiennent ; dès qu'une ne tient plus,
/// elle et toutes les plus anciennes sont retirées. Les plus récentes d'abord : date du fichier
/// décroissante, puis numéro d'étape en tête du nom décroissant (un nom sans numéro après ceux
/// qui en ont un), puis nom décroissant, puis rang dans l'inventaire. Si les couches de
/// l'utilisateur seules dépassent le budget, elles restent toutes : le contrôle du contexte de
/// l'IA tranche. Une couche gardée n'est jamais modifiée.
pub fn within_budget(inventory: Value, work_files: &[WorkFile], budget: usize) -> Value {
    if json_chars(&inventory) <= budget {
        return inventory;
    }
    let Some(layers) = inventory.get("layers").and_then(Value::as_array) else {
        return inventory;
    };
    // Les couches de résultat, les plus récentes d'abord : (rang dans l'inventaire, date, nom).
    let mut results: Vec<(usize, SystemTime, &str)> = layers
        .iter()
        .enumerate()
        .filter_map(|(index, layer)| {
            result_date(layer, work_files).map(|(date, name)| (index, date, name))
        })
        .collect();
    results.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| step_number(b.2).cmp(&step_number(a.2)))
            .then_with(|| b.2.cmp(a.2))
            .then(a.0.cmp(&b.0))
    });
    // Toutes les couches d'abord, puis les résultats retirés du plus ancien au plus récent
    // jusqu'à tenir. Une couche retirée raccourcit le tableau de sa longueur et d'une virgule
    // (le tableau ne contient jamais moins d'une couche à ce moment-là).
    let mut keep = vec![true; layers.len()];
    let mut size = json_chars(&inventory);
    let mut count = layers.len();
    for (index, _, _) in results.iter().rev() {
        if size <= budget {
            break;
        }
        keep[*index] = false;
        size -= json_chars(&layers[*index]) + usize::from(count > 1);
        count -= 1;
    }
    let mut out = inventory.clone();
    out["layers"] = layers
        .iter()
        .zip(&keep)
        .filter(|(_, &k)| k)
        .map(|(l, _)| l.clone())
        .collect();
    out
}

/// Numéro d'étape en tête d'un nom de fichier de résultat (`21-analysis.intersect.gpkg` donne
/// 21) ; `None` sans numéro, qui se range avant tout numéro (donc après en ordre décroissant).
fn step_number(name: &str) -> Option<u64> {
    let digits: &str = &name[..name.bytes().take_while(u8::is_ascii_digit).count()];
    digits.parse().ok()
}

/// Date et nom du fichier d'une couche de résultat ; `None` pour une couche de l'utilisateur.
fn result_date<'a>(layer: &'a Value, work_files: &[WorkFile]) -> Option<(SystemTime, &'a str)> {
    let source = layer.get("source").and_then(Value::as_str)?;
    let file = work_files.iter().find(|f| f.name == source)?;
    Some((file.modified, source))
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
    // L'extension ne parcourt pas le disque : aucun fichier non ouvert n'est connu.
    json!({"project": project, "layers": layers, "unloaded_files": []})
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
fn geometry(wkb: &str) -> Option<&'static str> {
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
fn field_type(qgis_type: &str) -> &'static str {
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
fn is_text(qgis_type: &str) -> bool {
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
    fn under_the_budget_the_inventory_is_unchanged() {
        let (inventory, files) = session();
        let size = json_chars(&inventory);
        assert_eq!(within_budget(inventory.clone(), &files, size), inventory);
        let alone = json!({"project": {}, "layers": [user_layer("plu")], "unloaded_files": []});
        assert_eq!(
            within_budget(alone.clone(), &[], INVENTORY_BUDGET_CHARS),
            alone
        );
    }

    #[test]
    fn over_the_budget_the_oldest_results_leave_first() {
        let (inventory, files) = session();
        let users = json!({"project": {"name": "p", "epsg": 2154},
            "layers": [user_layer("plu"), user_layer("gares"), user_layer("parcelles")],
            "unloaded_files": []});
        let per_result = json_chars(&result_layer(10)) + 1;
        // De quoi garder quatre résultats, pas cinq.
        let budget = json_chars(&users) + 4 * per_result + per_result / 2;
        let out = within_budget(inventory.clone(), &files, budget);
        // L'ordre de l'inventaire reste celui d'entrée : seuls les plus anciens résultats partent.
        assert_eq!(
            ids(&out),
            [
                "plu",
                "gares",
                "Intersection_10",
                "Intersection_9",
                "Intersection_8",
                "parcelles",
                "Intersection_7"
            ]
        );
        assert!(json_chars(&out) <= budget);
        // Le compte des caractères est exact : un résultat de plus ne tiendrait pas.
        let mut more = out.clone();
        more["layers"].as_array_mut().unwrap().push(result_layer(6));
        assert!(json_chars(&more) > budget);
        // Les couches gardées sont celles d'entrée, à l'octet près ; le reste aussi.
        for layer in out["layers"].as_array().unwrap() {
            assert!(inventory["layers"].as_array().unwrap().contains(layer));
        }
        assert_eq!(out["project"], inventory["project"]);
        assert_eq!(out["unloaded_files"], json!([]));
        // Déterministe.
        assert_eq!(within_budget(inventory, &files, budget), out);
    }

    #[test]
    fn the_newest_results_win_by_file_date_not_by_legend_order() {
        let (inventory, mut files) = session();
        // Le fichier de l'étape 2 est réécrit en dernier : il devient le plus récent.
        files[1].modified = UNIX_EPOCH + Duration::from_secs(5_000);
        let users_only = within_budget(inventory.clone(), &files, 0);
        let one = json_chars(&users_only) + json_chars(&result_layer(2)) + 1;
        let out = within_budget(inventory, &files, one);
        assert_eq!(ids(&out), ["plu", "gares", "parcelles", "Intersection_2"]);
    }

    #[test]
    fn equal_dates_are_broken_by_the_step_number_of_the_file_name() {
        let (inventory, files) = session();
        let same: Vec<WorkFile> = files
            .into_iter()
            .map(|f| WorkFile {
                modified: UNIX_EPOCH,
                ..f
            })
            .collect();
        let users_only = within_budget(inventory.clone(), &same, 0);
        let one = json_chars(&users_only) + json_chars(&result_layer(10)) + 1;
        // L'étape 10 est la plus récente, pas « 9-… » que l'ordre des textes mettrait devant.
        assert_eq!(
            ids(&within_budget(inventory, &same, one)),
            ["plu", "gares", "Intersection_10", "parcelles"]
        );
        // Un nom sans numéro d'étape vient après tous ceux qui en ont un.
        assert_eq!(step_number("21-analysis.intersect.gpkg"), Some(21));
        assert_eq!(step_number("resultat.gpkg"), None);
        assert!(step_number("resultat.gpkg") < step_number("1-a.gpkg"));
    }

    #[test]
    fn user_layers_are_never_removed_even_over_the_budget() {
        let (inventory, files) = session();
        let out = within_budget(inventory.clone(), &files, 10);
        assert_eq!(ids(&out), ["plu", "gares", "parcelles"]);
        // Sans couche de résultat, rien n'est retiré : le contrôle de l'IA tranche.
        let users = json!({"project": {}, "layers": [user_layer("plu"), user_layer("gares")],
            "unloaded_files": []});
        assert_eq!(within_budget(users.clone(), &files, 10), users);
    }

    #[test]
    fn without_a_work_folder_nothing_is_a_result() {
        let (inventory, _) = session();
        assert_eq!(within_budget(inventory.clone(), &[], 10), inventory);
        // Un fichier du dossier de travail que la couche ne nomme pas ne la rend pas résultat.
        let other = [WorkFile {
            name: "journal.jsonl".to_string(),
            modified: UNIX_EPOCH,
        }];
        assert_eq!(within_budget(inventory.clone(), &other, 10), inventory);
    }
}
