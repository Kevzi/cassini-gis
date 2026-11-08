//! Complétion d'un plan par les conversions de points (amendement d'AD-19, 2026-11-08,
//! ticket 11.22).
//!
//! Le modèle local écrit un point tel que la demande le donne. Avant de montrer le plan, le
//! noyau ajoute une étape `data.transform_point` (exécutée par QGIS : le noyau ne reprojette
//! jamais) pour chaque point littéral d'une commande du noyau dont le système diffère de celui
//! de son raster :
//! - un point par étape, dans l'ordre alphabétique des noms de paramètres (`serde_json` est
//!   construit sans `preserve_order`), puis dans l'ordre des éléments d'un tableau ;
//! - placée juste avant la commande qui l'emploie ;
//! - la commande reçoit `step:k.point` à la place du point ;
//! - les références des étapes suivantes sont renumérotées.
//!
//! [`complete_conversions`] est une fonction pure : le système d'un raster nommé lui est donné
//! par une fermeture (dans le canal, la lecture de l'en-tête du GeoTIFF, AD-20). Un raster cité
//! par `step:k` est suivi jusqu'à l'étape `k` : une commande du noyau garde la grille de son
//! raster de premier niveau, de proche en proche ; toute autre étape rend un système inconnu.
//! Un système inconnu, ou un raster illisible, n'ajoute rien : l'exécution rend son erreur
//! habituelle. Un point déjà cité par une référence n'est jamais converti une seconde fois.
//!
//! Seul `plan.propose` complète un plan, et seulement si l'extension annonce
//! `data.transform_point` : un appel direct (`command.run`, MCP, CLI `run`) ou un rejeu ne
//! l'est jamais.

use serde_json::{json, Map, Value};

use crate::plan::{parse_reference, point_items_schema, Plan, PlannedCall};
use crate::point::{admits, POINT_MARK};
use crate::raster_path::is_raster_path;
use crate::registry::{Registry, RunsIn};

/// La commande ajoutée par la complétion.
pub const TRANSFORM_POINT: &str = "data.transform_point";

/// Où se trouve un point à convertir dans les paramètres d'une étape.
struct Site {
    param: String,
    /// Rang dans le tableau de `Point`, ou `None` pour un paramètre `Point` de premier niveau.
    index: Option<usize>,
    point: Value,
}

/// Complète `plan` par les conversions de points ; un plan sans point à convertir, une question
/// ou un refus passent inchangés. `raster_epsg` rend le code EPSG d'un raster nommé (chemin tel
/// qu'écrit dans le plan), ou `None` s'il est illisible ou introuvable.
pub fn complete_conversions(
    registry: &Registry,
    plan: Plan,
    raster_epsg: &dyn Fn(&str) -> Option<u32>,
) -> Plan {
    let Plan::Calls { calls } = plan else {
        return plan;
    };
    let mut grids: Vec<Option<Option<u32>>> = vec![None; calls.len()];
    // Nouvelle place de chaque étape d'origine, la première en tête.
    let mut moved: Vec<usize> = Vec::with_capacity(calls.len());
    let mut completed: Vec<PlannedCall> = Vec::with_capacity(calls.len());
    for (index, call) in calls.iter().enumerate() {
        let mut params = renumber(&call.params, index + 1, &moved);
        let grid = grid_of(registry, &calls, index, raster_epsg, &mut grids);
        let sites = match grid {
            Some(epsg) => sites_to_convert(registry, call, epsg)
                .into_iter()
                .map(|site| (site, epsg))
                .collect(),
            None => Vec::new(),
        };
        for (site, epsg) in sites {
            completed.push(PlannedCall {
                command: TRANSFORM_POINT.to_string(),
                params: json!({"point": site.point, "target_epsg": epsg}),
            });
            let reference = Value::String(format!("step:{}.point", completed.len()));
            let slot = &mut params[site.param.as_str()];
            match site.index {
                None => *slot = reference,
                Some(i) => slot[i] = reference,
            }
        }
        completed.push(PlannedCall {
            command: call.command.clone(),
            params,
        });
        moved.push(completed.len());
    }
    Plan::Calls { calls: completed }
}

/// Le système de la grille de l'étape `index` (à partir de 0) : celui de son raster de premier
/// niveau, si c'est une commande du noyau. Mémorisé dans `grids`.
fn grid_of(
    registry: &Registry,
    calls: &[PlannedCall],
    index: usize,
    raster_epsg: &dyn Fn(&str) -> Option<u32>,
    grids: &mut Vec<Option<Option<u32>>>,
) -> Option<u32> {
    if let Some(known) = grids[index] {
        return known;
    }
    let found = (|| {
        let call = &calls[index];
        let spec = registry.spec(&call.command)?;
        if spec.runs_in == Some(RunsIn::Qgis) {
            return None;
        }
        let Value::Object(map) = &call.params else {
            return None;
        };
        let root = &spec.params_schema;
        // Le premier raster de premier niveau, dans l'ordre alphabétique des noms de paramètres
        // (`serde_json` sans `preserve_order`).
        let raster = map.iter().find_map(|(name, value)| {
            let schema = root.get("properties")?.get(name)?;
            (is_raster_path(schema, root) && value.is_string()).then_some(value)
        })?;
        match parse_reference(raster) {
            // Un raster écrit par une étape précédente : la grille de cette étape.
            Some(Ok((k, None))) if k <= index => {
                grid_of(registry, calls, k - 1, raster_epsg, grids)
            }
            Some(_) => None,
            None => raster_epsg(raster.as_str()?),
        }
    })();
    grids[index] = Some(found);
    found
}

/// Les points littéraux de l'étape, en paramètre `Point` ou dans un tableau de `Point`, dont le
/// système n'est pas `epsg` : dans l'ordre alphabétique des noms de paramètres, puis dans celui
/// des éléments. Seulement pour une
/// commande du noyau.
fn sites_to_convert(registry: &Registry, call: &PlannedCall, epsg: u32) -> Vec<Site> {
    let Some(spec) = registry.spec(&call.command) else {
        return Vec::new();
    };
    if spec.runs_in == Some(RunsIn::Qgis) {
        return Vec::new();
    }
    let Value::Object(map) = &call.params else {
        return Vec::new();
    };
    let root = &spec.params_schema;
    let other_system = |value: &Value| {
        value
            .get("epsg")
            .and_then(Value::as_u64)
            .is_some_and(|e| e != u64::from(epsg))
    };
    let mut sites = Vec::new();
    for (name, value) in map {
        let Some(schema) = root.get("properties").and_then(|p| p.get(name)) else {
            continue;
        };
        if let (Some(_), Value::Array(items)) = (point_items_schema(schema, root), value) {
            for (i, item) in items.iter().enumerate() {
                if item.is_object() && other_system(item) {
                    sites.push(Site {
                        param: name.clone(),
                        index: Some(i),
                        point: item.clone(),
                    });
                }
            }
        } else if admits(schema, root, POINT_MARK, 0) && value.is_object() && other_system(value) {
            sites.push(Site {
                param: name.clone(),
                index: None,
                point: value.clone(),
            });
        }
    }
    sites
}

/// Les paramètres de l'étape d'origine `step`, où chaque référence de premier niveau, ou
/// élément d'un tableau, vers une étape précédente `k` désigne sa nouvelle place `moved[k - 1]`.
fn renumber(params: &Value, step: usize, moved: &[usize]) -> Value {
    let Value::Object(map) = params else {
        return params.clone();
    };
    let one = |value: &Value| -> Value {
        match parse_reference(value) {
            Some(Ok((k, field))) if k < step => {
                let k = moved[k - 1];
                Value::String(match field {
                    None => format!("step:{k}"),
                    Some(f) => format!("step:{k}.{f}"),
                })
            }
            _ => value.clone(),
        }
    };
    let mut out = Map::new();
    for (name, value) in map {
        let new_value = match value {
            Value::Array(items) => Value::Array(items.iter().map(one).collect()),
            other => one(other),
        };
        out.insert(name.clone(), new_value);
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(calls: Value) -> Plan {
        serde_json::from_value(json!({"kind": "calls", "calls": calls})).unwrap()
    }

    fn wgs(x: f64, y: f64) -> Value {
        json!({"x": x, "y": y, "epsg": 4326})
    }

    fn l93(x: f64, y: f64) -> Value {
        json!({"x": x, "y": y, "epsg": 2154})
    }

    fn convert(point: Value) -> Value {
        json!({"command": "data.transform_point", "params": {"point": point, "target_epsg": 2154}})
    }

    /// `mnt.tif` en Lambert-93, `utm.tif` en UTM 31N ; tout autre raster est illisible.
    fn rasters(path: &str) -> Option<u32> {
        match path {
            "mnt.tif" => Some(2154),
            "utm.tif" => Some(32631),
            _ => None,
        }
    }

    fn complete(calls: Value) -> Plan {
        let registry = Registry::standard();
        let completed = complete_conversions(&registry, plan(calls), &rasters);
        // Le plan complété reste un plan valide.
        assert_eq!(registry.check_plan(&completed), Ok(()), "{completed:?}");
        completed
    }

    #[test]
    fn one_wgs84_point_gets_one_conversion_before_its_step() {
        let completed = complete(json!([
            {"command": "terrain.viewshed", "params": {"dem": "mnt.tif", "observer": wgs(2.4, 48.7)}}
        ]));
        assert_eq!(
            completed,
            plan(json!([
                convert(wgs(2.4, 48.7)),
                {"command": "terrain.viewshed", "params": {"dem": "mnt.tif", "observer": "step:1.point"}}
            ]))
        );
    }

    #[test]
    fn two_points_get_two_conversions_in_parameter_order() {
        let completed = complete(json!([
            {"command": "terrain.line_of_sight", "params": {"dem": "mnt.tif",
                "observer": wgs(6.17, 45.9075), "target": wgs(6.1294, 45.8992)}}
        ]));
        assert_eq!(
            completed,
            plan(json!([
                convert(wgs(6.17, 45.9075)),
                convert(wgs(6.1294, 45.8992)),
                {"command": "terrain.line_of_sight", "params": {"dem": "mnt.tif",
                    "observer": "step:1.point", "target": "step:2.point"}}
            ]))
        );
    }

    #[test]
    fn a_list_of_points_converts_only_those_of_another_system() {
        let completed = complete(json!([
            {"command": "terrain.viewshed_cumulative", "params": {"dem": "mnt.tif",
                "observers": [wgs(2.4, 48.7), l93(652030.0, 6850250.0), wgs(2.5, 48.8)]}}
        ]));
        assert_eq!(
            completed,
            plan(json!([
                convert(wgs(2.4, 48.7)),
                convert(wgs(2.5, 48.8)),
                {"command": "terrain.viewshed_cumulative", "params": {"dem": "mnt.tif",
                    "observers": ["step:1.point", l93(652030.0, 6850250.0), "step:2.point"]}}
            ]))
        );
        // Les sources du coût cumulé, de même.
        let completed = complete(json!([
            {"command": "terrain.cost_distance", "params": {"cost": "mnt.tif",
                "sources": [l93(652030.0, 6850250.0), wgs(2.4, 48.7)]}}
        ]));
        assert_eq!(
            completed,
            plan(json!([
                convert(wgs(2.4, 48.7)),
                {"command": "terrain.cost_distance", "params": {"cost": "mnt.tif",
                    "sources": [l93(652030.0, 6850250.0), "step:1.point"]}}
            ]))
        );
    }

    #[test]
    fn points_are_converted_in_alphabetical_order_of_parameter_names() {
        // `end` vient avant `start` : l'ordre est celui des noms, non celui de l'écriture.
        let completed = complete(json!([
            {"command": "terrain.least_cost_path", "params": {"cost": "mnt.tif",
                "start": wgs(2.4, 48.7), "end": wgs(2.5, 48.8)}}
        ]));
        assert_eq!(
            completed,
            plan(json!([
                convert(wgs(2.5, 48.8)),
                convert(wgs(2.4, 48.7)),
                {"command": "terrain.least_cost_path", "params": {"cost": "mnt.tif",
                    "end": "step:1.point", "start": "step:2.point"}}
            ]))
        );
    }

    #[test]
    fn field_references_at_top_level_and_in_lists_are_renumbered() {
        let completed = complete(json!([
            {"command": "terrain.viewshed", "params": {"dem": "mnt.tif", "observer": wgs(2.4, 48.7)}},
            convert(wgs(2.5, 48.8)),
            {"command": "terrain.viewshed_cumulative", "params": {"dem": "mnt.tif",
                "observers": ["step:2.point"]}},
            {"command": "terrain.line_of_sight", "params": {"dem": "mnt.tif",
                "observer": "step:2.point", "target": l93(652030.0, 6850250.0)}}
        ]));
        assert_eq!(
            completed,
            plan(json!([
                convert(wgs(2.4, 48.7)),
                {"command": "terrain.viewshed", "params": {"dem": "mnt.tif", "observer": "step:1.point"}},
                convert(wgs(2.5, 48.8)),
                {"command": "terrain.viewshed_cumulative", "params": {"dem": "mnt.tif",
                    "observers": ["step:3.point"]}},
                {"command": "terrain.line_of_sight", "params": {"dem": "mnt.tif",
                    "observer": "step:3.point", "target": l93(652030.0, 6850250.0)}}
            ]))
        );
    }

    #[test]
    fn an_already_converted_point_or_one_in_the_right_system_is_left_unchanged() {
        for calls in [
            json!([
                convert(wgs(2.4, 48.7)),
                {"command": "terrain.viewshed", "params": {"dem": "mnt.tif", "observer": "step:1.point"}}
            ]),
            json!([
                {"command": "terrain.viewshed", "params": {"dem": "mnt.tif",
                    "observer": l93(652030.0, 6850250.0)}}
            ]),
            json!([
                {"command": "terrain.least_cost_path", "params": {"cost": "utm.tif",
                    "start": {"x": 412300.0, "y": 5652800.0, "epsg": 32631},
                    "end": {"x": 412900.0, "y": 5653800.0, "epsg": 32631}}}
            ]),
        ] {
            assert_eq!(complete(calls.clone()), plan(calls));
        }
    }

    #[test]
    fn the_mixed_case_converts_only_the_written_point() {
        let completed = complete(json!([
            convert(wgs(2.4, 48.7)),
            {"command": "terrain.viewshed_cumulative", "params": {"dem": "mnt.tif",
                "observers": ["step:1.point", wgs(2.5, 48.8)]}}
        ]));
        assert_eq!(
            completed,
            plan(json!([
                convert(wgs(2.4, 48.7)),
                convert(wgs(2.5, 48.8)),
                {"command": "terrain.viewshed_cumulative", "params": {"dem": "mnt.tif",
                    "observers": ["step:1.point", "step:2.point"]}}
            ]))
        );
    }

    #[test]
    fn steps_before_and_after_are_renumbered_and_the_raster_is_followed() {
        let completed = complete(json!([
            {"command": "terrain.slope", "params": {"dem": "mnt.tif", "unit": "degrees"}},
            {"command": "terrain.viewshed", "params": {"dem": "step:1", "observer": wgs(2.4, 48.7)}},
            {"command": "terrain.slope", "params": {"dem": "step:2", "unit": "percent"}}
        ]));
        assert_eq!(
            completed,
            plan(json!([
                {"command": "terrain.slope", "params": {"dem": "mnt.tif", "unit": "degrees"}},
                convert(wgs(2.4, 48.7)),
                {"command": "terrain.viewshed", "params": {"dem": "step:1", "observer": "step:2.point"}},
                {"command": "terrain.slope", "params": {"dem": "step:3", "unit": "percent"}}
            ]))
        );
        // De proche en proche : la grille d'une pente de pente reste celle du MNT, en UTM ici.
        let completed = complete(json!([
            {"command": "terrain.slope", "params": {"dem": "utm.tif", "unit": "degrees"}},
            {"command": "terrain.slope", "params": {"dem": "step:1", "unit": "percent"}},
            {"command": "terrain.viewshed", "params": {"dem": "step:2", "observer": wgs(3.0, 51.0)}}
        ]));
        let Plan::Calls { calls } = completed else {
            panic!("un plan");
        };
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[2].params["target_epsg"], json!(32631));
        assert_eq!(calls[3].params["dem"], json!("step:2"));
        assert_eq!(calls[3].params["observer"], json!("step:3.point"));
    }

    #[test]
    fn an_unknown_system_adds_nothing() {
        for calls in [
            // Raster illisible ou introuvable.
            json!([
                {"command": "terrain.viewshed", "params": {"dem": "absent.tif", "observer": wgs(2.4, 48.7)}}
            ]),
            // Raster cité d'une étape de QGIS, ou d'une commande sans raster de premier niveau.
            json!([
                {"command": "analysis.buffer", "params": {"layer_id": "a", "distance": {"value": 10, "unit": "m"}}},
                {"command": "terrain.viewshed", "params": {"dem": "step:1", "observer": wgs(2.4, 48.7)}}
            ]),
        ] {
            let registry = Registry::standard();
            let given = plan(calls);
            assert_eq!(
                complete_conversions(&registry, given.clone(), &rasters),
                given
            );
        }
        let registry = Registry::standard();
        let overlay = plan(json!([
            {"command": "raster.weighted_overlay", "params": {
                "criteria": [{"raster": "mnt.tif", "weight": 1}], "out": "s.tif"}},
            {"command": "terrain.viewshed", "params": {"dem": "step:1", "observer": wgs(2.4, 48.7)}}
        ]));
        assert_eq!(
            complete_conversions(&registry, overlay.clone(), &rasters),
            overlay
        );
    }

    #[test]
    fn a_command_run_by_qgis_or_without_raster_is_never_completed() {
        let registry = Registry::standard();
        // `data.transform_point` elle-même, et une commande de géodésie sans raster.
        let given = plan(json!([
            convert(wgs(2.4, 48.7)),
            {"command": "geo.distance_azimuth", "params": {"from": wgs(2.4, 48.7), "to": wgs(2.5, 48.8)}}
        ]));
        assert_eq!(
            complete_conversions(&registry, given.clone(), &rasters),
            given
        );
        // Une question ou un refus passe inchangé.
        let refusal = Plan::Refusal {
            reason: "Hors catalogue.".into(),
        };
        assert_eq!(
            complete_conversions(&registry, refusal.clone(), &rasters),
            refusal
        );
    }
}
