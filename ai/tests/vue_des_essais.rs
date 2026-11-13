//! Ticket 5.48 : les essais réels composent par la vue du noyau (`common::shown_view`). Sous le
//! seuil du tri, c'est, à l'octet près, l'inventaire aux identifiants courts qu'ils lisaient avant :
//! leurs scores ne changent pas. Au-dessus, la vue est triée par la demande. Sans modèle.

mod common;

use serde_json::{json, Value};

const INVENTORIES: [&str; 2] = [
    include_str!("../../tests/epreuve-modele-local/inventory.json"),
    include_str!("tenu_a_l_ecart/inventory.json"),
];

#[test]
fn under_the_threshold_the_view_of_the_tests_is_the_shortened_inventory() {
    for text in INVENTORIES {
        let inventory: Value = serde_json::from_str(text).unwrap();
        let (shortened, ids) = common::shown_inventory(&inventory);
        for request in ["Trouve les parcelles à moins de 500 m d'une gare.", ""] {
            let (view, users, view_ids) = common::shown_view(&inventory, request);
            assert_eq!(view.to_string(), shortened.to_string());
            assert_eq!(view_ids, ids);
            let layers = inventory["layers"].as_array().unwrap().len();
            assert_eq!(users, Some(layers));
        }
    }
}

#[test]
fn a_raw_inventory_of_qgis_is_translated_then_sorted_when_big() {
    let fields: Vec<Value> = (0..12)
        .map(|i| {
            json!({"name": format!("attribut_{i:02}"), "type": "String",
            "values": ["valeur_a", "valeur_b", "valeur_c"]})
        })
        .collect();
    let mut layers: Vec<Value> = (0..30)
        .map(|k| {
            json!({"layer_id": format!("objet_{k:02}_0b8f3a2c_1111_2222_3333_444455556666"),
                "name": format!("objet_{k:02}"), "source": "fond.gpkg", "crs": "EPSG:2154",
                "kind": "vector", "geometry": "MultiPolygon", "feature_count": 3,
                "fields": fields})
        })
        .collect();
    layers.push(
        json!({"layer_id": "gares_83d935aa_ca33_431d_966f_dbbf0fb7ac0f", "name": "gares",
        "source": "gares.gpkg", "crs": "EPSG:2154", "kind": "vector", "geometry": "Point",
        "feature_count": 3, "fields": []}),
    );
    let raw = json!({"project_name": "p", "project_crs": "EPSG:2154", "layers": layers});
    let (view, users, ids) = common::shown_view(&raw, "les gares");
    assert_eq!(view["index"].as_array().map(Vec::len), Some(31));
    assert_eq!(users, Some(31));
    let detailed: Vec<&str> = view["layers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["name"].as_str().unwrap())
        .collect();
    assert!(detailed.contains(&"gares"), "{detailed:?}");
    // Identifiants courts montrés, retraduits par la table.
    let mut short = view["layers"][0]["layer_id"].clone();
    assert!(short.as_str().unwrap().len() < 20);
    ids.restore(&mut short);
    assert!(short.as_str().unwrap().len() > 36);
}
