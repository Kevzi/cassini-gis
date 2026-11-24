//! Commande `raster.polygonize` : transforme un raster en polygones, un par zone de cellules
//! contiguës de même valeur. Déclaration seule : le geste vit dans l'extension (AD-22 : aucun
//! nom d'algorithme, aucun nom de champ de QGIS ici). Le raster se désigne par un `RasterPath`
//! (AD-24) : chemin relatif au projet, ou `step:k` (un raster reclassé est le cas d'usage ; les
//! valeurs sont des classes, aucune unité en jeu). Le noyau nomme le fichier de sortie (AD-10) :
//! une couche de polygones, donc `.gpkg`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::raster_path::RasterPath;
use crate::registry::{CommandSpec, Delegated, Registration, RunsIn};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawParams {
    raster: RasterPath,
    #[serde(default)]
    #[schemars(range(min = 1))]
    band: Option<u32>,
    #[serde(default)]
    out: String,
}

/// Paramètres de `raster.polygonize`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(try_from = "RawParams")]
pub struct RasterPolygonizeParams {
    /// Raster à polygoniser (fichier GeoTIFF), chemin copié de l'inventaire, ou `step:k` (par
    /// exemple un raster reclassé). Pas un id de couche.
    pub raster: RasterPath,
    /// Numéro de la bande à polygoniser, entier à partir de 1 (1 par défaut). Une bande absente du
    /// raster est refusée avant tout lancement.
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub band: Option<u32>,
    /// Fichier de sortie : fourni par le noyau, dans le dossier de travail.
    #[serde(default)]
    pub out: String,
}

impl TryFrom<RawParams> for RasterPolygonizeParams {
    type Error = String;

    /// Refuse la bande 0 avant tout ordre à QGIS (un nombre négatif ou un texte échoue déjà au
    /// typage `u32`).
    fn try_from(raw: RawParams) -> Result<Self, String> {
        if raw.band == Some(0) {
            return Err("`band` doit valoir au moins 1".into());
        }
        Ok(Self {
            raster: raw.raster,
            band: raw.band,
            out: raw.out,
        })
    }
}

/// Résultat de `raster.polygonize`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RasterPolygonizeOutput {
    /// Id de la couche créée dans le projet : les polygones, avec le champ `value`.
    pub layer_id: String,
    /// Nom affiché de la couche créée.
    pub name: String,
    /// Nombre de polygones de la couche créée.
    pub feature_count: u64,
    /// SCR de la couche créée, en code explicite (ex. `EPSG:2154`).
    pub crs: String,
}

/// `raster.polygonize` : commande déléguée à QGIS.
pub struct RasterPolygonize;

impl Delegated for RasterPolygonize {
    type Params = RasterPolygonizeParams;
    type Output = RasterPolygonizeOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<RasterPolygonizeParams, RasterPolygonizeOutput>(
            "raster.polygonize",
            "Transforme un raster (fichier GeoTIFF désigné par `raster`, chemin recopié de l'inventaire ou `step:k`, par exemple un raster reclassé, pas un id de couche) en polygones : un polygone par zone de cellules contiguës de même valeur (voisinage de 4, pas de diagonale). La bande se choisit avec `band` (la première par défaut). Le résultat est une nouvelle couche de polygones avec le champ `value` (la valeur de la zone), ajoutée au projet. Ne modifie pas le raster.",
            "Turns a raster (GeoTIFF file given by `raster`, a path copied from the inventory or `step:k`, e.g. a reclassified raster, not a layer id) into polygons: one polygon per zone of contiguous cells with the same value (4-neighbour, no diagonal). The band is chosen with `band` (the first by default). The result is a new polygon layer with the `value` field (the value of the zone), added to the project. Does not modify the raster.",
        )
        .runs_in(RunsIn::Qgis)
        .cancellable(true)
        .path_params(&["raster"])
        .output_params(&["out"])
        .output_ext(".gpkg")
        .example(serde_json::json!({"raster": "donnees/classes.tif"}))
        .example(serde_json::json!({"raster": "step:1"}))
        .example(serde_json::json!({"raster": "donnees/multibande.tif", "band": 2}))
        .tests(&[
            "raster_polygonize_is_delegated",
            "raster_polygonize_band_is_optional_and_positive",
            "raster_polygonize_output_is_a_gpkg_named_by_the_core",
            "raster_polygonize_designates_the_raster_by_path",
            "raster_polygonize_accepts_a_step_raster",
        ])
    }
}

inventory::submit! { Registration(|r| r.register_delegated::<RasterPolygonize>()) }

#[cfg(test)]
mod tests {
    use crate::registry::{Registry, RunsIn};
    use serde_json::json;

    #[test]
    fn raster_polygonize_is_delegated() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.polygonize").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Qgis));
        assert_eq!(spec.cancellable, Some(true));
        let err = registry
            .call("raster.polygonize", json!({"raster": "classes.tif"}))
            .unwrap_err();
        assert_eq!(err.code(), "registry.unavailable");
        assert!(registry.check_contract().is_empty());
    }

    #[test]
    fn raster_polygonize_band_is_optional_and_positive() {
        let registry = Registry::standard();
        for ok in [
            json!({"raster": "classes.tif"}),
            json!({"raster": "classes.tif", "band": 1}),
            json!({"raster": "classes.tif", "band": 4}),
            json!({"raster": "step:1"}),
        ] {
            assert!(registry.validate_params("raster.polygonize", &ok).is_ok());
        }
        for bad in [
            json!({"raster": "classes.tif", "band": 0}),
            json!({"raster": "classes.tif", "band": -1}),
            json!({"raster": "classes.tif", "band": "deux"}),
            json!({"raster": "classes.tif", "band": 1.5}),
            json!({"band": 1}),
            json!({"raster": "classes.tif", "field": "x"}),
        ] {
            let err = registry
                .validate_params("raster.polygonize", &bad)
                .unwrap_err();
            assert_eq!(err.code(), "registry.invalid_params", "{bad}");
        }
    }

    #[test]
    fn raster_polygonize_output_is_a_gpkg_named_by_the_core() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.polygonize").unwrap();
        assert_eq!(spec.output_params, vec!["out".to_string()]);
        assert_eq!(spec.output_ext, ".gpkg");
    }

    #[test]
    fn raster_polygonize_designates_the_raster_by_path() {
        let registry = Registry::standard();
        let spec = registry.spec("raster.polygonize").unwrap();
        assert_eq!(spec.path_params, vec!["raster".to_string()]);
    }

    #[test]
    fn raster_polygonize_accepts_a_step_raster() {
        let registry = Registry::standard();
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "terrain.slope", "params": {"dem": "dem/mnt.tif", "unit": "degrees"}},
            {"command": "raster.polygonize", "params": {"raster": "step:1"}}
        ]}))
        .unwrap();
        assert_eq!(registry.check_plan(&plan), Ok(()));
    }
}
