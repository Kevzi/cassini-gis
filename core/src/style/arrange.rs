//! Rangement des couches du projet : groupes par thème et ordre d'affichage (changement de cap
//! du 2026-11-08, décision 5).
//!
//! Chaque couche que le noyau connaît figure exactement une fois. Les groupes vont de haut en
//! bas :
//! - « Autres couches », les vecteurs non reconnus, souvent les résultats de l'utilisateur
//!   (tampons, parcelles sélectionnées), en haut de tout ;
//! - les groupes des thèmes, ceux des points, puis des lignes, puis des surfaces (un groupe prend
//!   le rang de sa couche la plus haute ; à rang égal, l'ordre de [`Group`]) ;
//! - « Autres rasters », les rasters non reconnus (pente, champ de vision…) ;
//! - « Relief » en dessous de tout, l'ombrage au-dessus des teintes d'altitude.
//!
//! Dans un groupe : points, lignes, surfaces, autres, rasters, puis l'ordre de l'inventaire. Un
//! MNT ne recouvre donc jamais la carte. Une couche du projet absente de la liste (au rejeu, une
//! couche posée par une exécution précédente) n'est pas une erreur de `layer.arrange` : elle
//! reste au-dessus des groupes, intacte.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::recognize::{LayerFacts, Shape};
use super::table::{Group, Theme};

/// Un groupe posé : son nom, et ses couches de haut en bas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayerGroup {
    pub name: String,
    pub layers: Vec<String>,
}

/// Rang d'une couche dans son groupe : sa sorte, l'ombrage avant le MNT, puis sa place dans
/// l'inventaire.
fn layer_key(index: usize, layer: &LayerFacts, theme: Option<Theme>) -> (u8, u8, usize) {
    let relief = match theme {
        Some(Theme::Dem) => 1,
        _ => 0,
    };
    (layer.shape.rank(), relief, index)
}

/// Une couche d'un groupe : sa place dans l'inventaire, ses faits, son thème.
type Member<'a> = (usize, &'a LayerFacts, Option<Theme>);

/// Le groupe d'une couche : celui de son thème, sinon « Autres rasters » pour un raster et
/// « Autres couches » pour tout le reste.
fn group_of(layer: &LayerFacts, theme: Option<Theme>) -> Group {
    match theme {
        Some(theme) => theme.group(),
        None if layer.shape == Shape::Raster => Group::OtherRasters,
        None => Group::OtherVectors,
    }
}

/// Les groupes de `layers` (dans l'ordre de l'inventaire), chacune avec son thème reconnu.
pub fn arrange(layers: &[(LayerFacts, Option<Theme>)]) -> Vec<LayerGroup> {
    let mut members: BTreeMap<Group, Vec<Member>> = BTreeMap::new();
    for (index, (layer, theme)) in layers.iter().enumerate() {
        members
            .entry(group_of(layer, *theme))
            .or_default()
            .push((index, layer, *theme));
    }
    let mut groups: Vec<((u8, u8), Group, Vec<String>)> = members
        .into_iter()
        .map(|(group, mut list)| {
            list.sort_by_key(|(i, l, t)| layer_key(*i, l, *t));
            // (palier, rang) : les vecteurs inconnus en haut, puis les thèmes par leur couche la
            // plus haute, puis les rasters inconnus, puis le relief.
            let rank = match group {
                Group::OtherVectors => (0, 0),
                Group::OtherRasters => (2, 0),
                Group::Relief => (3, 0),
                _ => (
                    1,
                    list.iter()
                        .map(|(_, l, _)| l.shape.rank())
                        .min()
                        .unwrap_or(0),
                ),
            };
            let ids = list
                .into_iter()
                .map(|(_, l, _)| l.layer_id.clone())
                .collect();
            (rank, group, ids)
        })
        .collect();
    groups.sort_by_key(|(rank, group, _)| (*rank, *group));
    groups
        .into_iter()
        .map(|(_, group, layers)| LayerGroup {
            name: group.name().to_string(),
            layers,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::recognize::recognize;
    use serde_json::{json, Value};

    fn facts(raw: Value) -> (LayerFacts, Option<Theme>) {
        let l = LayerFacts::from_raw(&raw).unwrap();
        let t = recognize(&l);
        (l, t)
    }

    fn names(groups: &[LayerGroup]) -> Vec<&str> {
        groups.iter().map(|g| g.name.as_str()).collect()
    }

    #[test]
    fn relief_goes_under_everything_and_unknown_rasters_above_it() {
        let layers = vec![
            facts(json!({"layer_id": "mnt", "name": "mnt", "kind": "raster"})),
            facts(json!({"layer_id": "pente", "name": "1-terrain.slope", "kind": "raster"})),
            facts(json!({"layer_id": "ombre", "name": "2-terrain.hillshade", "kind": "raster"})),
            facts(
                json!({"layer_id": "gares", "name": "gares", "kind": "vector", "geometry": "Point"}),
            ),
            facts(
                json!({"layer_id": "x", "name": "export_2024", "kind": "vector",
                "geometry": "MultiPolygon"}),
            ),
            facts(
                json!({"layer_id": "communes", "name": "communes", "kind": "vector",
                "geometry": "MultiPolygon"}),
            ),
        ];
        let groups = arrange(&layers);
        assert_eq!(
            names(&groups),
            [
                "Autres couches",
                "Transport",
                "Limites administratives",
                "Autres rasters",
                "Relief"
            ]
        );
        assert_eq!(groups[0].layers, ["x"]);
        assert_eq!(groups[3].layers, ["pente"]);
        assert_eq!(groups[4].layers, ["ombre", "mnt"]);
        // Chaque couche une fois.
        let mut all: Vec<&String> = groups.iter().flat_map(|g| &g.layers).collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), layers.len());
    }

    #[test]
    fn unknown_points_and_lines_go_on_top_of_everything() {
        let layers = vec![
            facts(
                json!({"layer_id": "route", "name": "routes", "kind": "vector",
                "geometry": "LineString"}),
            ),
            facts(
                json!({"layer_id": "gares", "name": "gares", "kind": "vector",
                "geometry": "Point"}),
            ),
            facts(
                json!({"layer_id": "tampon", "name": "2-analysis.buffer", "kind": "vector",
                "geometry": "MultiPolygon"}),
            ),
            facts(
                json!({"layer_id": "trace", "name": "itineraire", "kind": "vector",
                "geometry": "LineString"}),
            ),
            facts(
                json!({"layer_id": "pts", "name": "releves", "kind": "vector",
                "geometry": "Point"}),
            ),
        ];
        let groups = arrange(&layers);
        assert_eq!(names(&groups), ["Autres couches", "Transport"]);
        // Dans le groupe : points, lignes, surfaces.
        assert_eq!(groups[0].layers, ["pts", "trace", "tampon"]);
        assert_eq!(groups[1].layers, ["gares", "route"]);
    }

    #[test]
    fn a_group_takes_the_rank_of_its_highest_layer() {
        let layers = vec![
            facts(
                json!({"layer_id": "lac", "name": "plan_d_eau", "kind": "vector",
                "geometry": "MultiPolygon"}),
            ),
            facts(json!({"layer_id": "bois", "name": "bois", "kind": "vector",
                "geometry": "MultiPolygon"})),
            facts(
                json!({"layer_id": "riv", "name": "cours_d_eau", "kind": "vector",
                "geometry": "LineString"}),
            ),
            facts(
                json!({"layer_id": "route", "name": "routes", "kind": "vector",
                "geometry": "LineString"}),
            ),
        ];
        let groups = arrange(&layers);
        assert_eq!(names(&groups), ["Transport", "Hydrographie", "Végétation"]);
        assert_eq!(groups[1].layers, ["riv", "lac"]);
    }

    #[test]
    fn the_same_inventory_gives_the_same_arrangement() {
        let layers = vec![
            facts(
                json!({"layer_id": "a", "name": "routes", "kind": "vector", "geometry": "LineString"}),
            ),
            facts(
                json!({"layer_id": "b", "name": "voie ferrée", "kind": "vector", "geometry": "LineString"}),
            ),
        ];
        let first = arrange(&layers);
        assert_eq!(first, arrange(&layers));
        assert_eq!(first[0].layers, ["a", "b"]);
    }
}
