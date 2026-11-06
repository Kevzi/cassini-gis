//! Désignation d'un raster, du noyau ou délégué (amendement du 2026-11-05 d'AD-24).
//!
//! Tout paramètre raster est un [`RasterPath`], marqué dans le schéma engendré
//! `x-cassini: raster` : un chemin relatif au projet (AD-9), ou, dans un plan, une référence
//! `step:k` vers le raster écrit par l'étape `k`. Avant l'appel, le noyau traduit cette
//! référence en repère `out:<ligne du journal>.1` (AD-11), que le journal garde tel quel et
//! que le rejeu résout vers son propre dossier : jamais un chemin résolu. Un raster ne se
//! désigne jamais par son identifiant de couche.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::point::admits;
use crate::registry::CommandSpec;

/// Valeur de la marque d'un [`RasterPath`].
pub const RASTER_MARK: &str = "raster";

/// Extensions de fichier d'une sortie raster.
pub const RASTER_EXTENSIONS: [&str; 2] = [".tif", ".tiff"];

// Le commentaire de documentation ci-dessous devient la description du type dans le schéma :
// il reste court. Le long prompt du modèle ne le voit pas (`RasterPath` y est rendu en chaîne).
/// Raster : chemin relatif au projet, ou `step:k` dans un plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
#[schemars(extend("x-cassini" = "raster"))]
pub struct RasterPath(pub String);

impl RasterPath {
    /// La chaîne telle que reçue (chemin déjà résolu par le journal à l'exécution).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Le sous-schéma admet-il un [`RasterPath`] : lui-même après ses `$ref`, ou l'une de ses
/// alternatives (`Option<RasterPath>` s'engendre en `anyOf`) ?
pub fn is_raster_path(schema: &Value, root: &Value) -> bool {
    admits(schema, root, RASTER_MARK, 0)
}

/// La commande écrit-elle exactement un raster, que `step:k` peut désigner ?
pub fn writes_one_raster(spec: &CommandSpec) -> bool {
    spec.output_params.len() == 1 && RASTER_EXTENSIONS.contains(&spec.output_ext.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::schema_for;
    use serde_json::json;

    #[derive(Deserialize, JsonSchema)]
    #[allow(dead_code)]
    struct Params {
        dem: RasterPath,
        name: String,
        mask: Option<RasterPath>,
    }

    #[test]
    fn raster_path_is_marked_in_the_schema() {
        let root = serde_json::to_value(schema_for!(Params)).unwrap();
        let dem = &root["properties"]["dem"];
        assert!(is_raster_path(dem, &root), "{root}");
        assert!(!is_raster_path(&root["properties"]["name"], &root));
        // Un raster facultatif (`anyOf` avec `null`) est reconnu aussi.
        assert!(is_raster_path(&root["properties"]["mask"], &root), "{root}");
        let def = &root["$defs"]["RasterPath"];
        assert_eq!(def["type"], json!("string"));
        assert_eq!(def["x-cassini"], json!("raster"));
    }

    #[test]
    fn raster_path_reads_a_plain_string() {
        let p: Params =
            serde_json::from_value(json!({"dem": "mnt.tif", "name": "a", "mask": null})).unwrap();
        assert_eq!(p.dem.as_str(), "mnt.tif");
        assert!(
            serde_json::from_value::<Params>(json!({"dem": 3, "name": "a", "mask": null})).is_err()
        );
        assert_eq!(
            serde_json::to_value(RasterPath("out:1.1".into())).unwrap(),
            json!("out:1.1")
        );
    }
}
