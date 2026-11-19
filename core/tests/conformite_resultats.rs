//! Conformité des résultats de l'extension aux schémas de sortie du noyau (suite de 10.1).
//!
//! `fixtures/resultats-extension.json` porte un exemple de résultat par commande déléguée à
//! QGIS (plusieurs quand une option change la forme). Le noyau les fait valider par
//! `Registry::validate_output` ; l'extension lit le même fichier et prouve qu'elle produit cette
//! forme (`qgis-plugin/tests/test_result_conformance.py`). Le schéma du noyau fait foi.

use std::collections::BTreeSet;
use std::path::PathBuf;

use cassini_core::registry::{Registry, RunsIn};
use serde_json::{json, Value};

const BAD_RESULT: &str = "qgis.bad_result";
const TEST_PREFIX: &str = "test.";

fn examples() -> Vec<(String, String, Value)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fixtures")
        .join("resultats-extension.json");
    let text = std::fs::read_to_string(&path).expect("fixtures/resultats-extension.json");
    let list: Vec<Value> = serde_json::from_str(&text).expect("JSON valide");
    let found: Vec<_> = list
        .into_iter()
        .map(|e| {
            (
                e["id"].as_str().expect("id").to_string(),
                e["case"].as_str().expect("case").to_string(),
                e["result"].clone(),
            )
        })
        .collect();
    // Garde-fou : un fichier vide ou mal lu ne doit pas faire passer la recette.
    assert!(found.len() >= 20, "fichier d'exemples vide ou tronqué");
    found
}

fn delegated_ids(registry: &Registry) -> BTreeSet<String> {
    registry
        .specs()
        .filter(|s| s.runs_in == Some(RunsIn::Qgis) && !s.id.starts_with(TEST_PREFIX))
        .map(|s| s.id.clone())
        .collect()
}

/// Un message par écart entre les commandes déléguées et les ids du fichier d'exemples.
fn id_set_errors(expected: &BTreeSet<String>, found: &BTreeSet<String>) -> Vec<String> {
    let mut errors: Vec<String> = expected
        .difference(found)
        .map(|id| format!("{id} : commande déléguée sans exemple dans resultats-extension.json"))
        .collect();
    errors.extend(found.difference(expected).map(|id| {
        format!("{id} : exemple d'une commande qui n'est pas déléguée à QGIS (ou inconnue)")
    }));
    errors
}

fn example_ids() -> BTreeSet<String> {
    examples().into_iter().map(|(id, _, _)| id).collect()
}

#[test]
fn every_delegated_command_has_an_example_and_every_example_a_command() {
    let expected = delegated_ids(&Registry::standard());
    let errors = id_set_errors(&expected, &example_ids());
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_missing_or_unknown_id_is_named_by_the_comparison() {
    let expected = delegated_ids(&Registry::standard());
    let mut found = example_ids();
    found.remove("layer.arrange");
    let errors = id_set_errors(&expected, &found);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("layer.arrange"));
    let mut found = example_ids();
    found.insert("analysis.fake".to_string());
    let errors = id_set_errors(&expected, &found);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("analysis.fake"));
}

#[test]
fn every_example_is_accepted_by_validate_output() {
    let registry = Registry::standard();
    for (id, case, result) in examples() {
        registry
            .validate_output(&id, &result)
            .unwrap_or_else(|e| panic!("{id} / {case} : {e}"));
    }
}

#[test]
fn the_shape_variants_are_all_present() {
    let found: BTreeSet<(String, String)> =
        examples().into_iter().map(|(i, c, _)| (i, c)).collect();
    for (id, case) in [
        ("layout.export_pdf", "sans échelle"),
        ("layout.export_pdf", "avec échelle et centre"),
        ("layout.export_pdf", "avec avertissements"),
        ("layout.export_pdf", "tronqué"),
        ("layer.add", "reprojetée à l'affichage"),
        ("layer.add", "non reprojetée"),
        ("layer.describe", "sans emprise"),
        ("project.inventory", "projet vide"),
        ("analysis.zonal_statistics", "tronqué"),
        ("analysis.zonal_statistics", "sans zone"),
        ("data.convert_raster", "sans EPSG"),
    ] {
        assert!(
            found.contains(&(id.to_string(), case.to_string())),
            "variante absente : {id} / {case}"
        );
    }
}

/// Un cas négatif prouve que le test détecte : sans lui, un validateur qui accepterait tout
/// ferait passer la recette.
#[test]
fn a_required_field_missing_or_of_the_wrong_type_is_refused() {
    let registry = Registry::standard();
    let mut refused = 0;
    for (id, case, result) in examples() {
        let schema = registry
            .specs()
            .find(|s| s.id == id)
            .map(|s| s.output_schema.clone())
            .unwrap();
        let required: Vec<String> = schema["required"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        for key in required {
            // Champ requis absent.
            let mut without = result.clone();
            without.as_object_mut().unwrap().remove(&key);
            let err = registry.validate_output(&id, &without).unwrap_err();
            assert_eq!(err.code(), BAD_RESULT, "{id} / {case} sans {key}");
            // Type faux : un nombre à la place d'un texte, un texte à la place du reste.
            let mut wrong = result.clone();
            wrong[&key] = if result[&key].is_string() {
                json!(5)
            } else {
                json!("x")
            };
            let err = registry.validate_output(&id, &wrong).unwrap_err();
            assert_eq!(err.code(), BAD_RESULT, "{id} / {case} : {key} de type faux");
            refused += 2;
        }
    }
    assert!(refused >= 100, "trop peu de cas négatifs ({refused})");
}

#[test]
fn an_extra_field_is_refused_where_the_schema_is_closed() {
    let registry = Registry::standard();
    let err = registry
        .validate_output(
            "data.transform_point",
            &json!({"point": {"x": 1.0, "y": 2.0, "epsg": 2154, "z": 3.0}}),
        )
        .unwrap_err();
    assert_eq!(err.code(), BAD_RESULT);
    let err = registry
        .validate_output(
            "layout.export_pdf",
            &json!({
                "file": "a.pdf", "page_count": 1, "layer_count": 1, "sources": ["a.gpkg"],
                "truncated": false,
                "source_details": [{"source": "a.gpkg", "resolution": {"value": 10.0, "unit": "m", "extra": 1},
                                    "date": null, "date_origin": null}]
            }),
        )
        .unwrap_err();
    assert_eq!(err.code(), BAD_RESULT);
}

#[test]
fn a_nested_field_missing_or_wrong_is_refused() {
    let registry = Registry::standard();
    let base = json!({
        "file": "a.pdf", "page_count": 1, "layer_count": 1, "sources": ["a.gpkg"], "truncated": false,
        "source_details": [{"source": "a.gpkg", "resolution": null, "date": null, "date_origin": null}]
    });
    registry
        .validate_output("layout.export_pdf", &base)
        .unwrap();
    for bad in [
        json!({"resolution": null, "date": null, "date_origin": null}),
        json!({"source": "a.gpkg", "resolution": null, "date": null, "date_origin": "autre"}),
        json!({"source": "a.gpkg", "resolution": {"value": "10", "unit": "m"}, "date": null, "date_origin": null}),
    ] {
        let mut result = base.clone();
        result["source_details"] = json!([bad]);
        let err = registry
            .validate_output("layout.export_pdf", &result)
            .unwrap_err();
        assert_eq!(err.code(), BAD_RESULT);
    }
}
