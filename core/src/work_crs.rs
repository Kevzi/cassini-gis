//! Système de calcul en mètres d'une commande qui applique une distance en mètres à une couche
//! vecteur (changement de cap du 2026-11-12, `change-distance-en-metres-sur-couche-en-degres`,
//! §3.2 ; entrée 9.16).
//!
//! Le noyau choisit un code EPSG ; QGIS reprojette (AD-19). Le choix est une fonction pure de
//! l'inventaire rendu par `project.inventory`, testable sans QGIS. L'unité d'un système est un
//! fait lu par QGIS (`crs_unit`, `project_crs_unit`) : le noyau ne la déduit jamais d'un code.
//!
//! Règle (« on ne reprojette que les couches en degrés », amendement RA06 d'AD-24, décision du
//! propriétaire du 2026-11-12) :
//! 1. couche en degrés : le système du projet s'il est en mètres, a un code `EPSG:n` et n'est pas
//!    un Mercator mondial ([`GLOBAL_MERCATOR`], dont l'échelle fausse une distance de plus d'un
//!    tiers à nos latitudes), sinon le fuseau UTM sur WGS84 de l'étendue du projet
//!    (`project.utm_epsg`, même calcul que l'inventaire du modèle), sinon
//!    `analysis.crs_not_metric`, avant tout ordre ;
//! 2. toute autre unité (mètres, `other`, ou absente d'une extension plus ancienne) : rien n'est
//!    rempli, le tampon se fait seul, sans reprojection, et la garde « système en mètres » du
//!    geste décide, comme avant l'entrée 9.16.

use serde_json::Value;

use crate::agent_inventory::{epsg, utm_epsg};
use crate::registry::RegistryError;

/// Code de refus quand aucun système en mètres ne peut être choisi (AD-21 : code inchangé).
pub const CRS_NOT_METRIC: &str = "analysis.crs_not_metric";

/// Systèmes de Mercator mondiaux, en mètres mais dont l'échelle varie avec la latitude (un
/// tampon de 500 m y couvre environ 350 m à 46° N) : jamais retenus comme système de calcul ;
/// le fuseau UTM de l'étendue les remplace.
pub const GLOBAL_MERCATOR: [u32; 5] = [3857, 3395, 900913, 102100, 102113];

/// Code EPSG du système de calcul de la couche `layer_id` d'après l'inventaire `inventory`.
///
/// `Ok(None)` quand la couche n'est pas en degrés (le geste fait le tampon seul, sous sa garde)
/// ou n'est pas dans l'inventaire (le geste répond `layer.not_found`) : rien n'est rempli,
/// comme avant. `Err` (`analysis.crs_not_metric`) quand aucun système en mètres ne peut être
/// choisi pour une couche en degrés ; le message nomme la couche, son système et la raison.
pub fn work_epsg(inventory: &Value, layer_id: &str) -> Result<Option<u32>, RegistryError> {
    let Some(layer) = inventory
        .get("layers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|l| l.get("layer_id").and_then(Value::as_str) == Some(layer_id))
    else {
        return Ok(None);
    };
    let name = layer
        .get("name")
        .and_then(Value::as_str)
        .filter(|n| !n.is_empty())
        .unwrap_or(layer_id);
    if layer.get("crs_unit").and_then(Value::as_str) != Some("deg") {
        return Ok(None);
    }
    let crs = layer.get("crs").and_then(Value::as_str).unwrap_or("");
    let shown = if crs.is_empty() {
        "un système inconnu"
    } else {
        crs
    };
    let project_metric = inventory.get("project_crs_unit").and_then(Value::as_str) == Some("m");
    let project = inventory
        .get("project_crs")
        .and_then(Value::as_str)
        .and_then(code_of)
        .filter(|code| project_metric && !GLOBAL_MERCATOR.contains(code));
    let utm = || inventory.get("extent").and_then(utm_epsg);
    project.or_else(utm).map(Some).ok_or_else(|| {
        // La raison dite tient au système du projet tel que QGIS l'a lu.
        let why = if project_metric {
            "le système du projet, en mètres, ne convient pas au calcul (sans code EPSG, ou \
             Mercator mondial)"
        } else {
            "le projet est lui aussi en degrés"
        };
        refusal(format!(
            "La couche « {name} » est en {shown}, en degrés, et aucun système en mètres n'a pu \
             être choisi pour le calcul : {why}, et son étendue est inconnue ou trop large pour \
             un fuseau UTM. Passez le projet en Lambert-93 (EPSG:2154), ou demandez « reprojette \
             la couche {name} en Lambert-93 »."
        ))
    })
}

/// `EPSG:2154` donne `2154` ; tout autre système (`IGNF:LAMB93`, vide) ne donne rien.
fn code_of(authid: &str) -> Option<u32> {
    epsg(authid)
        .and_then(|c| u32::try_from(c).ok())
        .filter(|c| *c > 0)
}

fn refusal(message: String) -> RegistryError {
    RegistryError::Delegated {
        code: CRS_NOT_METRIC.to_string(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Étendue d'Annecy en degrés WGS84 (fuseau 32).
    fn annecy() -> Value {
        json!({"xmin": 0, "ymin": 0, "xmax": 1, "ymax": 1, "crs": "EPSG:2154",
               "lonlat": {"west": 5.8, "south": 45.7, "east": 6.4, "north": 46.1, "crs": "EPSG:4326"}})
    }

    fn inventory(
        layer: Value,
        project_crs: &str,
        project_unit: Option<&str>,
        extent: Value,
    ) -> Value {
        let mut inv = json!({"layers": [layer], "project_crs": project_crs, "extent": extent});
        if let Some(unit) = project_unit {
            inv["project_crs_unit"] = json!(unit);
        }
        inv
    }

    fn gares(crs: &str, unit: Option<&str>) -> Value {
        let mut l = json!({"layer_id": "gares_1", "name": "gares", "crs": crs});
        if let Some(unit) = unit {
            l["crs_unit"] = json!(unit);
        }
        l
    }

    #[test]
    fn a_metric_layer_fills_nothing() {
        // Le tampon se fait seul, sans reprojection (amendement RA06), même dans un projet en
        // degrés, et même pour un système en mètres sans code EPSG.
        for crs in ["EPSG:2154", "IGNF:LAMB93"] {
            let inv = inventory(gares(crs, Some("m")), "EPSG:4326", Some("deg"), annecy());
            assert_eq!(work_epsg(&inv, "gares_1").unwrap(), None, "{crs}");
        }
    }

    #[test]
    fn a_layer_in_degrees_takes_the_metric_project_system() {
        let inv = inventory(
            gares("EPSG:4326", Some("deg")),
            "EPSG:2154",
            Some("m"),
            annecy(),
        );
        assert_eq!(work_epsg(&inv, "gares_1").unwrap(), Some(2154));
    }

    #[test]
    fn a_project_in_degrees_gives_the_utm_zone_of_its_extent() {
        let inv = inventory(
            gares("EPSG:4326", Some("deg")),
            "EPSG:4326",
            Some("deg"),
            annecy(),
        );
        assert_eq!(work_epsg(&inv, "gares_1").unwrap(), Some(32632));
        // Système du projet en mètres mais sans code EPSG : le fuseau UTM, de même.
        let inv = inventory(
            gares("EPSG:4326", Some("deg")),
            "IGNF:LAMB93",
            Some("m"),
            annecy(),
        );
        assert_eq!(work_epsg(&inv, "gares_1").unwrap(), Some(32632));
    }

    #[test]
    fn no_metric_system_is_a_clear_refusal() {
        let wide = json!({"xmin": -10, "ymin": 40, "xmax": 10, "ymax": 50, "crs": "EPSG:4326",
                          "lonlat": {"west": -10.0, "south": 40.0, "east": 10.0, "north": 50.0, "crs": "EPSG:4326"}});
        for extent in [wide, Value::Null] {
            let inv = inventory(
                gares("EPSG:4326", Some("deg")),
                "EPSG:4326",
                Some("deg"),
                extent,
            );
            let err = work_epsg(&inv, "gares_1").unwrap_err();
            assert_eq!(err.code(), CRS_NOT_METRIC);
            let message = err.to_string();
            assert!(message.contains("« gares »"), "{message}");
            assert!(message.contains("EPSG:4326"), "{message}");
            assert!(message.contains("Lambert-93"), "{message}");
            assert!(message.contains("reprojette la couche gares"), "{message}");
        }
    }

    #[test]
    fn an_unknown_unit_fills_nothing() {
        // Extension plus ancienne (unité absente) ou unité illisible : le tampon seul, sous la
        // garde du geste, comme avant l'entrée 9.16.
        for unit in [Some("other"), None] {
            let inv = inventory(gares("EPSG:4326", unit), "EPSG:2154", Some("m"), annecy());
            assert_eq!(work_epsg(&inv, "gares_1").unwrap(), None, "{unit:?}");
        }
    }

    #[test]
    fn a_global_mercator_project_gives_the_utm_zone() {
        for code in GLOBAL_MERCATOR {
            let project = format!("EPSG:{code}");
            let inv = inventory(
                gares("EPSG:4326", Some("deg")),
                &project,
                Some("m"),
                annecy(),
            );
            assert_eq!(
                work_epsg(&inv, "gares_1").unwrap(),
                Some(32632),
                "{project}"
            );
        }
    }

    #[test]
    fn a_metric_project_without_code_and_utm_says_why() {
        let inv = inventory(
            gares("EPSG:4326", Some("deg")),
            "IGNF:LAMB93",
            Some("m"),
            Value::Null,
        );
        let message = work_epsg(&inv, "gares_1").unwrap_err().to_string();
        assert!(!message.contains("lui aussi en degrés"), "{message}");
        assert!(message.contains("en mètres, ne convient pas"), "{message}");
        let inv = inventory(
            gares("EPSG:4326", Some("deg")),
            "EPSG:4326",
            Some("deg"),
            Value::Null,
        );
        let message = work_epsg(&inv, "gares_1").unwrap_err().to_string();
        assert!(message.contains("lui aussi en degrés"), "{message}");
    }

    #[test]
    fn a_layer_made_by_an_earlier_step_is_read_like_any_other() {
        // La sortie d'une étape `step:k` du plan, déjà résolue en id par `plan.execute`, est
        // dans l'inventaire relu juste avant l'étape.
        let inv = json!({
            "layers": [
                {"layer_id": "zonage_1", "name": "zonage", "crs": "EPSG:2154", "crs_unit": "m"},
                {"layer_id": "extrait_9", "name": "1-analysis.select", "crs": "EPSG:4326", "crs_unit": "deg"}
            ],
            "project_crs": "EPSG:2154", "project_crs_unit": "m", "extent": annecy()
        });
        assert_eq!(work_epsg(&inv, "extrait_9").unwrap(), Some(2154));
        // La couche en mètres du même inventaire ne reçoit rien.
        assert_eq!(work_epsg(&inv, "zonage_1").unwrap(), None);
    }

    #[test]
    fn an_absent_layer_fills_nothing() {
        let inv = inventory(
            gares("EPSG:4326", Some("deg")),
            "EPSG:2154",
            Some("m"),
            annecy(),
        );
        assert_eq!(work_epsg(&inv, "autre").unwrap(), None);
        assert_eq!(work_epsg(&json!({}), "gares_1").unwrap(), None);
    }

    #[test]
    fn a_layer_without_an_epsg_code_fills_nothing() {
        // Une couche en mètres sans code EPSG (IGNF:LAMB93) se tamponne seule, comme avant 9.16.
        for unit in [Some("m"), Some("other"), None] {
            let inv = inventory(gares("IGNF:LAMB93", unit), "EPSG:2154", Some("m"), annecy());
            assert_eq!(work_epsg(&inv, "gares_1").unwrap(), None, "{unit:?}");
        }
    }
}
