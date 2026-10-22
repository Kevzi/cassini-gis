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

use serde_json::{json, Map, Value};

/// Plafond de valeurs d'un champ texte : le jeu tenu à l'écart en montre 10 au plus,
/// l'entraînement 9. Au-delà, le champ n'a pas de clé `values`.
pub const MAX_VALUES: usize = 10;

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
}
