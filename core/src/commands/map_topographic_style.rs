//! Commande `map.topographic_style` : met les couches du projet aux couleurs de la sémiologie
//! publique de l'IGN et du standard CNIG du PLU, puis les range par thème (changement de cap du
//! 2026-11-08, entrée 11.20 ; le geste QGIS vient en 21).
//!
//! Le noyau lit l'inventaire du projet auprès de QGIS, hors du journal (`inventory_param`), juste
//! avant l'exécution. La commande reconnaît le thème de chaque couche (`crate::style::recognize`),
//! compose son `StyleSpec` (`crate::style::table`) et le rangement (`crate::style::arrange`). Elle
//! rend `styled`, `unchanged` et `groups`, et, sous le champ caché `orders`, les ordres que la
//! session dicte juste après sa ligne : un `layer.set_style` par couche mise en forme, puis un seul
//! `layer.arrange` (amendement d'AD-30). Ni l'inventaire ni les ordres n'entrent dans la ligne ;
//! seul l'ordre de sa légende (ids) y est gardé, sous `replay.legend`, pour le rejeu.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::commands::layer_arrange::ARRANGE;
use crate::commands::layer_set_style::SET_STYLE;
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};
use crate::style::arrange::{arrange, LayerGroup};
use crate::style::recognize::{recognize, LayerFacts};
use crate::style::table::style_for;

/// Identifiant de la commande.
pub const TOPOGRAPHIC_STYLE: &str = "map.topographic_style";

/// Paramètres de `map.topographic_style`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MapTopographicStyleParams {
    /// Couches à mettre en forme ; absent : toutes les couches du projet.
    #[serde(default)]
    pub layers: Option<Vec<String>>,
    /// Inventaire brut du projet, rempli par le noyau seul, jamais par l'appelant ni le journal.
    #[serde(default)]
    #[schemars(skip)]
    pub inventory: Option<Value>,
}

/// Une couche mise en forme, avec le thème reconnu.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct StyledLayer {
    pub layer_id: String,
    pub theme: String,
}

/// Une couche laissée telle quelle, avec la raison.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct UnchangedLayer {
    pub layer_id: String,
    pub reason: String,
}

/// Un ordre que la session dicte après la ligne de la commande.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Order {
    pub command: String,
    pub params: Value,
}

/// Résultat de `map.topographic_style`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct MapTopographicStyleOutput {
    /// Couches mises en forme, dans l'ordre de l'inventaire.
    pub styled: Vec<StyledLayer>,
    /// Couches laissées telles quelles, dans l'ordre de l'inventaire.
    pub unchanged: Vec<UnchangedLayer>,
    /// Les groupes posés, de haut en bas.
    pub groups: Vec<LayerGroup>,
    /// Ordres à dicter (champ caché, ôté avant journalisation) : absent du schéma public.
    #[schemars(skip)]
    pub orders: Vec<Order>,
}

/// Raison d'une couche non reconnue.
pub const NOT_RECOGNIZED: &str =
    "thème non reconnu d'après le nom, le fichier et les champs : style gardé";
/// Raison d'une couche hors de `layers`.
pub const NOT_ASKED: &str = "couche hors de la liste demandée : style gardé";

/// `map.topographic_style` : commande du noyau.
pub struct MapTopographicStyle;

impl Command for MapTopographicStyle {
    type Params = MapTopographicStyleParams;
    type Output = MapTopographicStyleOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<MapTopographicStyleParams, MapTopographicStyleOutput>(
            TOPOGRAPHIC_STYLE,
            "Met les couches du projet aux couleurs topographiques (IGN, standard CNIG pour le PLU) et les range par thème, le relief en dessous ; sur demande seulement. `layers` (identifiants de couche) limite la mise en forme ; absent : toutes les couches. Ne modifie aucun fichier.",
            "Styles the project layers with topographic colours (IGN, CNIG standard for zoning plans) and groups them by theme, relief at the bottom; on request only. `layers` (layer ids) limits the styling; absent: all layers. Modifies no file.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .inventory_param("inventory")
        .dictates(&[SET_STYLE, ARRANGE])
        .example(json!({}))
        .example(json!({"layers": ["routes_1"]}))
        .tests(&[
            "villeneuve_is_styled_by_theme_and_relief_goes_last",
            "annecy_is_styled_with_water_buildings_and_hillshade_over_the_ramps",
            "an_unknown_layer_is_unchanged_and_goes_to_other_layers",
            "a_partial_list_styles_only_those_layers_and_arranges_all",
            "a_layer_outside_the_inventory_is_missing_and_nothing_is_ordered",
            "without_inventory_the_command_is_unavailable",
            "an_empty_layer_list_is_invalid",
            "map_topographic_style_declares_its_inventory_and_its_orders",
        ])
    }

    fn run(&self, params: MapTopographicStyleParams) -> Result<MapTopographicStyleOutput, CommandFailure> {
        let unavailable = || {
            let e = crate::style::unavailable();
            CommandFailure::new(e.code(), e.to_string())
        };
        if params.layers.as_ref().is_some_and(Vec::is_empty) {
            return Err(CommandFailure::new(
                crate::style::spec::STYLE_INVALID,
                "La liste des couches à mettre en forme est vide : nommez au moins une couche, \
                 ou omettez la liste pour toutes les couches.",
            ));
        }
        // Un inventaire sans liste de couches, ou une couche sans identifiant, ne vient pas d'un
        // QGIS en état de répondre : rien n'est mis en forme.
        let raw = params
            .inventory
            .as_ref()
            .and_then(|i| i.get("layers"))
            .and_then(Value::as_array)
            .ok_or_else(unavailable)?;
        let layers: Vec<LayerFacts> = raw
            .iter()
            .map(|l| LayerFacts::from_raw(l).ok_or_else(unavailable))
            .collect::<Result<_, _>>()?;
        let known: BTreeSet<&str> = layers.iter().map(|l| l.layer_id.as_str()).collect();
        let asked: Option<BTreeSet<String>> = params.layers.map(|l| l.into_iter().collect());
        if let Some(missing) = asked
            .iter()
            .flatten()
            .find(|id| !known.contains(id.as_str()))
        {
            return Err(CommandFailure::new(
                "qgis.layer_missing",
                format!("La couche « {missing} » n'est pas dans le projet : rien n'est mis en forme."),
            ));
        }
        let mut styled = Vec::new();
        let mut unchanged = Vec::new();
        let mut orders = Vec::new();
        let mut themed = Vec::with_capacity(layers.len());
        for layer in layers {
            let theme = recognize(&layer);
            let wanted = asked
                .as_ref()
                .is_none_or(|a| a.contains(&layer.layer_id));
            match (wanted, theme) {
                (false, _) => unchanged.push(UnchangedLayer {
                    layer_id: layer.layer_id.clone(),
                    reason: NOT_ASKED.to_string(),
                }),
                (true, None) => unchanged.push(UnchangedLayer {
                    layer_id: layer.layer_id.clone(),
                    reason: NOT_RECOGNIZED.to_string(),
                }),
                (true, Some(theme)) => {
                    let style = style_for(theme, &layer);
                    style.validate()?;
                    orders.push(Order {
                        command: SET_STYLE.to_string(),
                        params: json!({"layer_id": layer.layer_id, "style": style}),
                    });
                    styled.push(StyledLayer {
                        layer_id: layer.layer_id.clone(),
                        theme: theme.id().to_string(),
                    });
                }
            }
            themed.push((layer, theme));
        }
        let groups = arrange(&themed);
        if !groups.is_empty() {
            orders.push(Order {
                command: ARRANGE.to_string(),
                params: json!({ "groups": groups }),
            });
        }
        Ok(MapTopographicStyleOutput {
            styled,
            unchanged,
            groups,
            orders,
        })
    }
}

inventory::submit! { Registration(|r| r.register(MapTopographicStyle)) }

/// Inventaires synthétiques des deux jeux de test, reproduits sans aucune donnée (entrée 11.20).
#[cfg(test)]
pub(crate) mod fixtures {
    use serde_json::{json, Value};

    /// Villeneuve-le-Roi : parcelles, zonage du PLU, gares, routes de la BD TOPO, MNT.
    pub fn villeneuve() -> Value {
        json!({
            "project_name": "villeneuve",
            "project_crs": "EPSG:2154",
            "extent": null,
            "layers": [
                {"layer_id": "mnt_1", "name": "RGEALTI_MNT_LAMB93", "source": "mnt.tif",
                 "crs": "EPSG:2154", "kind": "raster", "geometry": null, "fields": []},
                {"layer_id": "parcelles_2", "name": "parcelles", "source": "cadastre-94077-parcelles.json",
                 "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiPolygon", "feature_count": 9000,
                 "fields": [{"name": "id", "type": "String", "values": ["94077000AB0001"]},
                            {"name": "contenance", "type": "Integer"}]},
                {"layer_id": "plu_3", "name": "Zonage du PLU", "source": "zone_urba.gpkg",
                 "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiPolygon", "feature_count": 40,
                 "fields": [{"name": "typezone", "type": "String", "values": ["A", "AU", "N", "U"]},
                            {"name": "libelle", "type": "String", "values": ["UA", "UB"]}]},
                {"layer_id": "gares_4", "name": "gares", "source": "gares.gpkg",
                 "crs": "EPSG:2154", "kind": "vector", "geometry": "Point", "feature_count": 2,
                 "fields": [{"name": "nom", "type": "String", "values": ["Villeneuve-le-Roi"]}]},
                {"layer_id": "routes_5", "name": "troncon_de_route", "source": "bdtopo_routes.gpkg",
                 "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiLineStringZ", "feature_count": 1500,
                 "fields": [{"name": "nature", "type": "String", "values": ["Route à 1 chaussée"]},
                            {"name": "importance", "type": "String", "values": ["1", "2", "3", "4", "5", "6"]}]}
            ]
        })
    }

    /// Annecy : cours d'eau, plans d'eau, bâti, communes, deux MNT, un ombrage.
    pub fn annecy() -> Value {
        json!({
            "project_name": "annecy",
            "project_crs": "EPSG:2154",
            "extent": null,
            "layers": [
                {"layer_id": "ombre_1", "name": "4-terrain.hillshade", "source": "4-terrain.hillshade.tif",
                 "crs": "EPSG:2154", "kind": "raster", "geometry": null, "fields": []},
                {"layer_id": "mnt_2", "name": "mnt_annecy_2154", "source": "mnt_annecy_2154.tif",
                 "crs": "EPSG:2154", "kind": "raster", "geometry": null, "fields": []},
                {"layer_id": "mnt_3", "name": "srtm_38_03", "source": "srtm_38_03.tif",
                 "crs": "EPSG:4326", "kind": "raster", "geometry": null, "fields": []},
                {"layer_id": "riv_4", "name": "cours_d_eau", "source": "cours_d_eau.gpkg",
                 "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiLineString", "fields": []},
                {"layer_id": "lac_5", "name": "plan_d_eau", "source": "plan_d_eau.gpkg",
                 "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiPolygon", "fields": []},
                {"layer_id": "bati_6", "name": "batiment", "source": "batiment.gpkg",
                 "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiPolygonZ",
                 "fields": [{"name": "usage_1", "type": "String", "values": ["Industriel", "Résidentiel"]}]},
                {"layer_id": "communes_7", "name": "communes", "source": "COMMUNE.shp",
                 "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiPolygon",
                 "fields": [{"name": "nom", "type": "String", "values": ["Annecy"]}]}
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{annecy, villeneuve};
    use super::*;
    use crate::registry::{Registry, INVENTORY, ORDERS_FIELD};

    fn style(inventory: Value, layers: Option<Value>) -> Result<Value, crate::registry::RegistryError> {
        let mut params = json!({ "inventory": inventory });
        if let Some(l) = layers {
            params["layers"] = l;
        }
        Registry::standard().call(TOPOGRAPHIC_STYLE, params)
    }

    fn themes(out: &Value) -> Vec<(String, String)> {
        out["styled"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                (
                    s["layer_id"].as_str().unwrap().to_string(),
                    s["theme"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    }

    fn style_of<'a>(out: &'a Value, layer: &str) -> &'a Value {
        &out[ORDERS_FIELD]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["command"] == SET_STYLE && o["params"]["layer_id"] == layer)
            .unwrap_or_else(|| panic!("aucun style pour {layer}"))["params"]["style"]
    }

    fn group_names(out: &Value) -> Vec<&str> {
        out["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g["name"].as_str().unwrap())
            .collect()
    }

    #[test]
    fn villeneuve_is_styled_by_theme_and_relief_goes_last() {
        let out = style(villeneuve(), None).unwrap();
        let t = themes(&out);
        assert_eq!(
            t,
            [
                ("mnt_1".into(), "mnt".into()),
                ("parcelles_2".into(), "parcelles".into()),
                ("plu_3".into(), "zones_plu".into()),
                ("gares_4".into(), "gares".into()),
                ("routes_5".into(), "routes".into()),
            ]
        );
        assert_eq!(out["unchanged"], json!([]));
        let plu = style_of(&out, "plu_3");
        assert_eq!(plu["kind"], "categories");
        assert_eq!(plu["field"], "typezone");
        assert_eq!(plu["classes"][0]["fill"], "#e60000");
        let routes = style_of(&out, "routes_5");
        assert_eq!(routes["kind"], "categories");
        assert_eq!(routes["field"], "importance");
        assert_eq!(routes["classes"].as_array().unwrap().len(), 6);
        assert_eq!(style_of(&out, "gares_4")["kind"], "single");
        assert_eq!(style_of(&out, "mnt_1")["kind"], "elevation_ramp");
        assert_eq!(
            group_names(&out),
            ["Transport", "Cadastre", "Urbanisme", "Relief"]
        );
        assert_eq!(out["groups"][0]["layers"], json!(["gares_4", "routes_5"]));
        // Les ordres : un style par couche reconnue, puis un seul rangement, le dernier.
        let orders = out[ORDERS_FIELD].as_array().unwrap();
        assert_eq!(orders.len(), 6);
        assert_eq!(orders[5]["command"], ARRANGE);
        assert_eq!(orders[5]["params"]["groups"], out["groups"]);
        let last = out["groups"].as_array().unwrap().last().unwrap();
        assert_eq!(last["layers"], json!(["mnt_1"]));
    }

    #[test]
    fn annecy_is_styled_with_water_buildings_and_hillshade_over_the_ramps() {
        let out = style(annecy(), None).unwrap();
        let t: Vec<String> = themes(&out).into_iter().map(|(_, t)| t).collect();
        assert_eq!(
            t,
            [
                "ombrage",
                "mnt",
                "mnt",
                "cours_d_eau",
                "eau_de_surface",
                "bati",
                "limites_administratives"
            ]
        );
        assert_eq!(style_of(&out, "riv_4")["stroke"], "#aad5e9");
        assert_eq!(style_of(&out, "lac_5")["fill"], "#aad5e9");
        assert_eq!(style_of(&out, "bati_6")["field"], "usage_1");
        let communes = style_of(&out, "communes_7");
        assert_eq!(communes["kind"], "single");
        assert!(communes.get("fill").is_none());
        assert_eq!(style_of(&out, "ombre_1")["kind"], "hillshade");
        assert_eq!(
            group_names(&out),
            ["Hydrographie", "Limites administratives", "Bâti", "Relief"]
        );
        assert_eq!(out["groups"][3]["layers"], json!(["ombre_1", "mnt_2", "mnt_3"]));
    }

    #[test]
    fn an_unknown_layer_is_unchanged_and_goes_to_other_layers() {
        let mut inv = villeneuve();
        inv["layers"].as_array_mut().unwrap().push(json!({
            "layer_id": "x_9", "name": "export_2024", "source": "export_2024.gpkg",
            "crs": "EPSG:2154", "kind": "vector", "geometry": "MultiPolygon",
            "fields": [{"name": "valeur", "type": "Real"}]}));
        let out = style(inv, None).unwrap();
        assert_eq!(
            out["unchanged"],
            json!([{"layer_id": "x_9", "reason": NOT_RECOGNIZED}])
        );
        let names = group_names(&out);
        // Un vecteur inconnu, souvent un résultat de l'utilisateur, va en haut de tout.
        assert_eq!(names[0], "Autres couches");
        assert_eq!(out["groups"][0]["layers"], json!(["x_9"]));
    }

    #[test]
    fn a_partial_list_styles_only_those_layers_and_arranges_all() {
        let out = style(villeneuve(), Some(json!(["routes_5"]))).unwrap();
        assert_eq!(themes(&out), [("routes_5".into(), "routes".into())]);
        assert_eq!(out["unchanged"].as_array().unwrap().len(), 4);
        assert!(out["unchanged"]
            .as_array()
            .unwrap()
            .iter()
            .all(|u| u["reason"] == NOT_ASKED));
        let arranged: usize = out["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g["layers"].as_array().unwrap().len())
            .sum();
        assert_eq!(arranged, 5);
        assert_eq!(out[ORDERS_FIELD].as_array().unwrap().len(), 2);
    }

    #[test]
    fn a_layer_outside_the_inventory_is_missing_and_nothing_is_ordered() {
        let err = style(villeneuve(), Some(json!(["routes_5", "x"]))).unwrap_err();
        assert_eq!(err.code(), "qgis.layer_missing");
        assert!(err.to_string().contains("« x »"));
    }

    #[test]
    fn without_inventory_the_command_is_unavailable() {
        let err = Registry::standard()
            .call(TOPOGRAPHIC_STYLE, json!({}))
            .unwrap_err();
        assert_eq!(err.code(), "style.unavailable");
        // Un inventaire sans liste de couches, ou une couche sans identifiant texte.
        for inventory in [
            json!({"project_crs": "EPSG:2154"}),
            json!({"layers": {"a": 1}}),
            json!({"layers": [{"name": "routes"}]}),
            json!({"layers": [{"layer_id": 3, "name": "routes"}]}),
        ] {
            let err = style(inventory.clone(), None).unwrap_err();
            assert_eq!(err.code(), "style.unavailable", "{inventory}");
        }
        // Un projet vide n'est pas une erreur : rien à mettre en forme ni à ranger.
        let out = style(json!({"layers": []}), None).unwrap();
        assert_eq!(out[ORDERS_FIELD], json!([]));
    }

    #[test]
    fn an_empty_layer_list_is_invalid() {
        let err = style(villeneuve(), Some(json!([]))).unwrap_err();
        assert_eq!(err.code(), "style.invalid");
    }

    #[test]
    fn map_topographic_style_declares_its_inventory_and_its_orders() {
        let registry = Registry::standard();
        let spec = registry.spec(TOPOGRAPHIC_STYLE).unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert!(!spec.dictated_by_core);
        assert_eq!(spec.inventory_param.as_deref(), Some("inventory"));
        assert_eq!(
            spec.required_gestures(),
            [INVENTORY, SET_STYLE, ARRANGE]
        );
        // Ni l'inventaire ni les ordres ne sont au schéma public.
        assert!(spec.params_schema["properties"].get("inventory").is_none());
        assert!(spec.output_schema["properties"].get(ORDERS_FIELD).is_none());
        let entry = serde_json::to_value(spec).unwrap();
        assert!(entry.get("inventory_param").is_none());
        assert!(entry.get("dictates").is_none());
        // Le même inventaire donne le même résultat, à l'octet.
        assert_eq!(style(annecy(), None).unwrap(), style(annecy(), None).unwrap());
    }
}
