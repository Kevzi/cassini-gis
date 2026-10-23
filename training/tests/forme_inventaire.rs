//! Garde-fou de la forme de l'inventaire (ticket 5.10, tâche 4a ; 5.15) : chaque forme que rend
//! le produit, sorties de `project.inventory` traduites par `agent_inventory` du noyau, doit être
//! vue à l'entraînement (graine 1, jeu par défaut) : jeu de clés de l'inventaire, du projet et
//! de chaque sorte de couche, sortes de valeurs, géométries, types et valeurs de champ. Le cas
//! nominal comme les formes de QGIS : projet non enregistré, SCR qui n'est pas EPSG, couche
//! temporaire sans fichier, table sans géométrie, raster sans nombre d'objets. Si le
//! générateur change de forme, ce test échoue, et la traduction du noyau se met à jour avec le
//! ré-entraînement.

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
    /// Jeux de clés des couches, par sorte (`vector`, `raster`).
    layer: HashMap<String, BTreeSet<BTreeSet<String>>>,
    /// Sortes de valeurs des clés de couche, par sorte de couche.
    layer_sorts: HashMap<(String, String), BTreeSet<&'static str>>,
    geometries: BTreeSet<String>,
    field: BTreeSet<BTreeSet<String>>,
    types: BTreeSet<String>,
    valued_types: BTreeSet<String>,
    max_values: usize,
    unsorted_values: usize,
    empty_files: usize,
    total: usize,
}

fn learned() -> Learned {
    let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
    let mut l = Learned {
        total: examples.len(),
        ..Learned::default()
    };
    for e in &examples {
        let inv = e.inventory.as_object().expect("inventaire objet");
        l.top.insert(keys(inv));
        if inv["unloaded_files"] == json!([]) {
            l.empty_files += 1;
        }
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
            let kind = layer["kind"].as_str().unwrap().to_string();
            l.layer.entry(kind.clone()).or_default().insert(keys(layer));
            for (k, v) in layer {
                l.layer_sorts
                    .entry((kind.clone(), k.clone()))
                    .or_default()
                    .insert(sort_of(v));
            }
            if let Some(g) = layer["geometry"].as_str() {
                l.geometries.insert(g.to_string());
            }
            for f in layer["fields"].as_array().expect("champs") {
                let f = f.as_object().expect("champ objet");
                l.field.insert(keys(f));
                let ty = f["type"].as_str().unwrap().to_string();
                if let Some(values) = f.get("values") {
                    let values: Vec<&str> = values
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().expect("valeur texte"))
                        .collect();
                    l.max_values = l.max_values.max(values.len());
                    l.unsorted_values += usize::from(!values.is_sorted());
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

/// Les formes de QGIS hors du cas nominal : projet non enregistré dans un SCR qui n'est pas
/// EPSG, couche temporaire sans fichier (sortie d'une étape précédente), couche dans un SCR
/// IGNF, table sans géométrie, raster sans fichier.
fn qgis_edges() -> Value {
    json!({
        "project_name": null,
        "project_crs": "IGNF:LAMB93",
        "layers": [
            {"layer_id": "tampon_5e1", "name": "Zone tampon", "crs": "EPSG:2154",
             "kind": "vector", "geometry": "MultiPolygon", "feature_count": 3,
             "fields": [{"name": "nom", "type": "String", "values": ["Gare B", "Gare A"]}]},
            {"layer_id": "routes_ign", "name": "Routes", "source": "routes.shp",
             "crs": "IGNF:LAMB93", "kind": "vector", "geometry": "MultiLineString",
             "feature_count": 120, "fields": [{"name": "classe", "type": "String"}]},
            {"layer_id": "table_7", "name": "Propriétaires", "source": "proprietaires.csv",
             "crs": "EPSG:2154", "kind": "vector", "geometry": "NoGeometry", "feature_count": 40,
             "fields": [{"name": "nom", "type": "String"}]},
            {"layer_id": "mnt_tmp", "name": "MNT", "crs": "EPSG:2154", "kind": "raster",
             "geometry": null, "fields": []}
        ]
    })
}

/// Un projet enregistré dans un SCR qui n'est pas EPSG, et un projet nommé sans SCR.
fn project_edges() -> Vec<Value> {
    vec![
        json!({"project_name": "etude", "project_crs": "IGNF:LAMB93", "layers": []}),
        json!({"project_name": null, "project_crs": "EPSG:2154", "layers": []}),
    ]
}

fn assert_sorts(
    what: &str,
    map: &Map<String, Value>,
    learned: impl Fn(&str) -> Option<BTreeSet<&'static str>>,
) {
    for (k, v) in map {
        let seen = learned(k)
            .unwrap_or_else(|| panic!("{what} : clé « {k} » jamais vue à l'entraînement"));
        assert!(
            seen.contains(sort_of(v)),
            "{what} : « {k} » est {} ; l'entraînement montre {seen:?}",
            sort_of(v)
        );
    }
}

/// Vérifie qu'une sortie traduite n'a que des formes vues à l'entraînement.
fn assert_seen(l: &Learned, out: &Value) {
    let out = out.as_object().unwrap();
    assert!(l.top.contains(&keys(out)), "inventaire : {:?}", keys(out));
    assert_eq!(out["unloaded_files"], json!([]));
    let project = out["project"].as_object().unwrap();
    assert!(
        l.project.contains(&keys(project)),
        "projet : {:?} ; vus : {:?}",
        keys(project),
        l.project
    );
    assert_sorts("projet", project, |k| l.project_sorts.get(k).cloned());
    for layer in out["layers"].as_array().unwrap() {
        let layer = layer.as_object().unwrap();
        let name = layer["name"].as_str().unwrap();
        let kind = layer["kind"].as_str().unwrap();
        let seen = l
            .layer
            .get(kind)
            .unwrap_or_else(|| panic!("{name} : sorte {kind} jamais vue"));
        assert!(
            seen.contains(&keys(layer)),
            "{name} : {:?} ; vus pour {kind} : {seen:?}",
            keys(layer)
        );
        assert_sorts(name, layer, |k| {
            l.layer_sorts
                .get(&(kind.to_string(), k.to_string()))
                .cloned()
        });
        if let Some(g) = layer["geometry"].as_str() {
            assert!(l.geometries.contains(g), "{name} : {g}");
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
}

#[test]
fn every_shape_the_product_renders_is_seen_in_the_generated_set() {
    let l = learned();
    assert!(l.max_values <= MAX_VALUES);
    // Valeurs triées comme le produit les trie.
    assert_eq!(l.unsorted_values, 0);
    // `unloaded_files` vide, comme dans QGIS, sauf quand la famille ouvre un fichier.
    assert!(
        l.empty_files * 10 > l.total * 9,
        "{} / {}",
        l.empty_files,
        l.total
    );

    let nominal = agent_inventory(&representative());
    assert_seen(&l, &nominal);
    let edges = agent_inventory(&qgis_edges());
    assert_seen(&l, &edges);
    for raw in project_edges() {
        assert_seen(&l, &agent_inventory(&raw));
    }

    // La matrice du plan, vue d'ici : valeurs triées, champ trop varié sans valeurs.
    let layers = nominal["layers"].as_array().unwrap();
    assert_eq!(layers.len(), 4);
    assert_eq!(
        layers[0]["fields"][0]["values"],
        json!(["AUc", "N", "Nh", "U"])
    );
    assert!(layers[1]["fields"][0].get("values").is_none());
    assert_eq!(layers[2]["geometry"], "Point");
    // Un raster n'a pas de nombre d'objets dans QGIS : la clé manque.
    assert!(layers[3].get("feature_count").is_none());
    // Les formes de QGIS sont bien celles que le test croit couvrir.
    assert_eq!(edges["project"], json!({}));
    let edges = edges["layers"].as_array().unwrap();
    assert!(edges[0].get("source").is_none());
    assert_eq!(edges[0]["fields"][0]["values"], json!(["Gare A", "Gare B"]));
    assert!(edges[1].get("epsg").is_none());
    assert_eq!(edges[2]["geometry"], Value::Null);
    assert_eq!(edges[2]["kind"], "vector");
    assert!(edges[3].get("source").is_none() && edges[3].get("feature_count").is_none());
}
