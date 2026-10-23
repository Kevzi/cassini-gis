//! Garde-fou de la forme de l'inventaire (ticket 5.10, tâche 4a) : une sortie représentative
//! de `project.inventory`, traduite par `agent_inventory` du noyau, doit avoir les clés et les
//! sortes de valeurs du jeu engendré (graine 1). Si le générateur change de forme, ce test
//! échoue, et la traduction du noyau se met à jour avec le ré-entraînement.

use std::collections::{BTreeSet, HashMap};

use cassini_core::agent_inventory::{agent_inventory, MAX_VALUES};
use cassini_training::{generate, DEFAULT_COUNT, DEFAULT_SEED};
use serde_json::{json, Map, Value};

fn keys(map: &Map<String, Value>) -> BTreeSet<String> {
    map.keys().cloned().collect()
}

/// Sorte JSON d'une valeur : `null`, `bool`, `integer`, `number`, `string`, `array`, `object`.
fn sort_of(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(n) if n.is_u64() || n.is_i64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Ce que le jeu engendré montre de la forme des inventaires.
#[derive(Default)]
struct Learned {
    top: BTreeSet<BTreeSet<String>>,
    project: BTreeSet<BTreeSet<String>>,
    project_sorts: HashMap<String, BTreeSet<&'static str>>,
    layer: BTreeSet<BTreeSet<String>>,
    layer_sorts: HashMap<String, BTreeSet<&'static str>>,
    kinds: BTreeSet<String>,
    geometries: BTreeSet<String>,
    field: BTreeSet<BTreeSet<String>>,
    types: BTreeSet<String>,
    valued_types: BTreeSet<String>,
    max_values: usize,
}

fn learned() -> Learned {
    let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
    let mut l = Learned::default();
    for e in &examples {
        let inv = e.inventory.as_object().expect("inventaire objet");
        l.top.insert(keys(inv));
        let project = inv["project"].as_object().expect("projet objet");
        l.project.insert(keys(project));
        for (k, v) in project {
            l.project_sorts
                .entry(k.clone())
                .or_default()
                .insert(sort_of(v));
        }
        for layer in inv["layers"].as_array().expect("couches") {
            let layer = layer.as_object().expect("couche objet");
            l.layer.insert(keys(layer));
            for (k, v) in layer {
                l.layer_sorts
                    .entry(k.clone())
                    .or_default()
                    .insert(sort_of(v));
            }
            l.kinds.insert(layer["kind"].as_str().unwrap().to_string());
            if let Some(g) = layer["geometry"].as_str() {
                l.geometries.insert(g.to_string());
            }
            for f in layer["fields"].as_array().expect("champs") {
                let f = f.as_object().expect("champ objet");
                l.field.insert(keys(f));
                let ty = f["type"].as_str().unwrap().to_string();
                if let Some(values) = f.get("values") {
                    let values = values.as_array().unwrap();
                    assert!(values.iter().all(Value::is_string));
                    l.max_values = l.max_values.max(values.len());
                    l.valued_types.insert(ty.clone());
                }
                l.types.insert(ty);
            }
        }
    }
    l
}

/// Une sortie de `project.inventory` comme la rend l'extension : PLU, gares, routes, MNT.
fn representative() -> Value {
    let many: Vec<String> = (0..11).map(|i| format!("94077000AB{i:04}")).collect();
    json!({
        "project_name": "projet-test-villeneuve",
        "project_crs": "EPSG:2154",
        "extent": {"xmin": 0.0, "ymin": 0.0, "xmax": 1.0, "ymax": 1.0, "crs": "EPSG:2154"},
        "layers": [
            {"layer_id": "plu_9f3a", "name": "Zonage du PLU", "source": "plu.gpkg",
             "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiPolygon", "feature_count": 87,
             "fields": [
                {"name": "typezone", "type": "String", "values": ["U", "N", "Nh", "AUc"]},
                {"name": "libelle", "type": "String", "values": ["UA", "UB"]},
                {"name": "surface", "type": "Real"},
                {"name": "datappro", "type": "Date"}]},
            {"layer_id": "parcelles_1c2d", "name": "Parcelles", "source": "parcelles.shp",
             "crs": "EPSG:2154", "kind": "vector", "geometry": "Polygon", "feature_count": 5000,
             "fields": [
                {"name": "id_parcelle", "type": "String", "values": many},
                {"name": "contenance", "type": "Integer64"}]},
            {"layer_id": "gares_77aa", "name": "Gares", "source": "gares.geojson",
             "crs": "EPSG:2154", "kind": "vector", "geometry": "PointZ", "feature_count": 2,
             "fields": [{"name": "nom", "type": "String", "values": ["Villeneuve-le-Roi"]},
                {"name": "nb_quais", "type": "Integer"}]},
            {"layer_id": "mnt_0b", "name": "MNT", "source": "mnt.tif",
             "crs": "EPSG:2154", "kind": "raster", "geometry": null, "fields": []}
        ]
    })
}

fn assert_sorts(
    what: &str,
    map: &Map<String, Value>,
    learned: &HashMap<String, BTreeSet<&'static str>>,
) {
    for (k, v) in map {
        let seen = learned
            .get(k)
            .unwrap_or_else(|| panic!("{what} : clé « {k} » jamais vue à l'entraînement"));
        assert!(
            seen.contains(sort_of(v)),
            "{what} : « {k} » est {} ; l'entraînement montre {seen:?}",
            sort_of(v)
        );
    }
}

#[test]
fn the_translated_inventory_has_the_shape_of_the_generated_set() {
    let l = learned();
    // Le jeu engendré n'a qu'une forme de couche, de projet et d'inventaire.
    assert_eq!(l.top.len(), 1, "{:?}", l.top);
    assert_eq!(l.project.len(), 1, "{:?}", l.project);
    assert_eq!(l.layer.len(), 1, "{:?}", l.layer);
    assert!(l.max_values <= MAX_VALUES);
    let layer_keys = l.layer.first().unwrap();

    let out = agent_inventory(&representative());
    let out = out.as_object().unwrap();
    assert_eq!(&keys(out), l.top.first().unwrap());
    assert_eq!(out["unloaded_files"], json!([]));
    let project = out["project"].as_object().unwrap();
    assert_eq!(&keys(project), l.project.first().unwrap());
    assert_sorts("projet", project, &l.project_sorts);

    let layers = out["layers"].as_array().unwrap();
    assert_eq!(layers.len(), 4);
    for layer in layers {
        let layer = layer.as_object().unwrap();
        let name = layer["name"].as_str().unwrap();
        assert!(l.kinds.contains(layer["kind"].as_str().unwrap()), "{name}");
        assert_sorts(name, layer, &l.layer_sorts);
        if layer["kind"] == "vector" {
            assert_eq!(&keys(layer), layer_keys, "{name}");
            assert!(
                l.geometries.contains(layer["geometry"].as_str().unwrap()),
                "{name}"
            );
        } else {
            // Un raster n'a pas de nombre d'objets dans QGIS : la clé manque, rien d'autre.
            let mut expected = layer_keys.clone();
            expected.remove("feature_count");
            assert_eq!(keys(layer), expected, "{name}");
            assert_eq!(layer["geometry"], Value::Null);
            assert_eq!(layer["fields"], json!([]));
        }
        for f in layer["fields"].as_array().unwrap() {
            let f = f.as_object().unwrap();
            assert!(l.field.contains(&keys(f)), "{name} : {f:?}");
            let ty = f["type"].as_str().unwrap();
            assert!(l.types.contains(ty), "{name} : {ty}");
            if let Some(values) = f.get("values") {
                assert!(l.valued_types.contains(ty), "{name} : {ty}");
                assert!(values.as_array().unwrap().iter().all(Value::is_string));
            }
        }
    }
    // La matrice du plan, vue d'ici : valeurs triées, champ trop varié sans valeurs.
    assert_eq!(
        layers[0]["fields"][0]["values"],
        json!(["AUc", "N", "Nh", "U"])
    );
    assert!(layers[1]["fields"][0].get("values").is_none());
    assert_eq!(layers[2]["geometry"], "Point");
}
