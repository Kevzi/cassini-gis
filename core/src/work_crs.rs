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

/// Code de refus d'un MNT en degrés pour lequel aucun système en mètres ne se trouve (AD-21 :
/// le code que [`crate::dem::Dem::open`] donne déjà à un MNT hors mètres).
pub const DEM_NOT_PROJECTED: &str = "dem.not_projected";

/// Code de refus d'une commande de terrain dont le MNT n'est pas dans le système de calcul que le
/// noyau a choisi (appel direct qui contourne la reprojection dictée).
pub const DEM_WORK_MISMATCH: &str = "dem.work_mismatch";

/// Code EPSG du système de calcul en mètres d'un MNT (ticket 33 ; règle de 9.16 appliquée à un
/// raster, désigné par son chemin et non par un id de couche).
///
/// 1. MNT en mètres : `Ok(None)`, rien n'est reprojeté ni rempli (amendement RA06) ;
/// 2. MNT en degrés : le système du projet s'il est en mètres, a un code `EPSG:n` et n'est pas un
///    Mercator mondial ([`GLOBAL_MERCATOR`]), sinon le fuseau UTM sur WGS84 de l'**étendue du
///    MNT** (la zone qu'on va reprojeter, toujours connue : elle est dans son en-tête), sinon
///    `dem.not_projected` qui dit pourquoi, avant tout ordre.
///
/// Une fonction pure de l'en-tête du MNT et de l'inventaire rendu par `project.inventory`.
pub fn work_epsg_for_dem(
    dem: &crate::dem::DemCrs,
    name: &str,
    inventory: &Value,
) -> Result<Option<u32>, RegistryError> {
    use crate::dem::DemCrs;
    let DemCrs::Degrees {
        epsg,
        west,
        south,
        east,
        north,
    } = *dem
    else {
        return Ok(None);
    };
    let project_metric = inventory.get("project_crs_unit").and_then(Value::as_str) == Some("m");
    let project = inventory
        .get("project_crs")
        .and_then(Value::as_str)
        .and_then(code_of)
        // Seule la liste fermée de `dem.rs` : `Dem::open` refuserait le reste (3035, 27700,
        // 2056…) après la reprojection (décision 3 du ticket 33).
        .filter(|code| {
            project_metric && u16::try_from(*code).is_ok_and(crate::dem::is_known_metric)
        });
    let utm = || {
        utm_epsg(&serde_json::json!({"lonlat": {
            "west": west, "south": south, "east": east, "north": north, "crs": "EPSG:4326"}}))
    };
    project.or_else(utm).map(Some).ok_or_else(|| {
        let why = if project_metric {
            "le système du projet, en mètres, ne convient pas au calcul (sans code EPSG, ou \
             Mercator mondial, ou hors de la liste des systèmes en mètres que Cassini lit)"
        } else {
            "le projet est lui aussi en degrés"
        };
        let shown = epsg.map_or_else(
            || "un système géographique".to_string(),
            |c| format!("EPSG:{c}"),
        );
        RegistryError::Delegated {
            code: DEM_NOT_PROJECTED.to_string(),
            message: format!(
                "Le MNT « {name} » est en {shown}, en degrés, et aucun système en mètres n'a pu \
                 être choisi pour le calcul : {why}, et son étendue est trop large pour un fuseau \
                 UTM (plus de deux fuseaux, ou latitude hors des limites). Passez le projet en \
                 Lambert-93 (EPSG:2154), ou reprojetez le MNT dans QGIS (Raster > Projections > \
                 Reprojeter)."
            ),
        }
    })
}

/// Contrôle qu'un MNT lu est dans le système de calcul que le noyau a choisi pour lui
/// (`work_epsg`). Sans `work_epsg` (MNT en mètres, ou ligne d'avant le ticket 33), le système
/// du MNT est le système de calcul : rien à contrôler.
pub fn check_dem_system(
    dem: &crate::dem::Dem,
    work_epsg: Option<u32>,
) -> Result<(), crate::registry::CommandFailure> {
    match work_epsg {
        Some(code) if code != dem.epsg => Err(crate::registry::CommandFailure::new(
            DEM_WORK_MISMATCH,
            format!(
                "Le MNT est en EPSG:{}, le système de calcul choisi par le noyau est EPSG:{code} : \
                 la reprojection dictée avant la commande n'a pas eu lieu.",
                dem.epsg
            ),
        )),
        _ => Ok(()),
    }
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

    // --- MNT (ticket 33) ----------------------------------------------------------------

    use crate::dem::DemCrs;

    fn dem_deg(west: f64, south: f64, east: f64, north: f64) -> DemCrs {
        DemCrs::Degrees {
            epsg: Some(4326),
            west,
            south,
            east,
            north,
        }
    }

    fn project(crs: &str, unit: &str) -> Value {
        json!({"layers": [], "project_crs": crs, "project_crs_unit": unit})
    }

    #[test]
    fn a_metric_dem_fills_nothing_whatever_the_project() {
        for inv in [project("EPSG:4326", "deg"), project("EPSG:2154", "m")] {
            let metric = DemCrs::Metric(2154);
            assert_eq!(work_epsg_for_dem(&metric, "mnt", &inv).unwrap(), None);
        }
    }

    #[test]
    fn a_dem_in_degrees_takes_the_metric_project_system_then_the_utm_zone_of_the_dem() {
        let annecy = dem_deg(5.8, 45.7, 6.4, 46.1);
        let inv = project("EPSG:2154", "m");
        assert_eq!(work_epsg_for_dem(&annecy, "mnt", &inv).unwrap(), Some(2154));
        // Projet en degrés, ou en Mercator mondial, ou en mètres sans code : fuseau UTM du MNT.
        for inv in [
            project("EPSG:4326", "deg"),
            project("EPSG:3857", "m"),
            project("IGNF:LAMB93", "m"),
        ] {
            assert_eq!(
                work_epsg_for_dem(&annecy, "mnt", &inv).unwrap(),
                Some(32632),
                "{inv}"
            );
        }
        // Le fuseau suit le MNT, pas le projet : un MNT de l'ouest de la France est au fuseau 30.
        let brest = dem_deg(-4.6, 48.3, -4.3, 48.5);
        assert_eq!(
            work_epsg_for_dem(&brest, "mnt", &project("EPSG:4326", "deg")).unwrap(),
            Some(32630)
        );
        // Un projet métrique hors de la liste fermée (LAEA 3035, Grande-Bretagne 27700, Suisse
        // 2056) : le fuseau UTM du MNT, car `Dem::open` refuserait ces systèmes.
        for code in ["EPSG:3035", "EPSG:27700", "EPSG:2056"] {
            assert_eq!(
                work_epsg_for_dem(&annecy, "mnt", &project(code, "m")).unwrap(),
                Some(32632),
                "{code}"
            );
        }
        // Un projet absent de l'inventaire (extension plus ancienne) : le fuseau UTM aussi.
        assert_eq!(
            work_epsg_for_dem(&annecy, "mnt", &json!({})).unwrap(),
            Some(32632)
        );
    }

    #[test]
    fn a_dem_in_degrees_without_a_metric_system_is_a_clear_refusal() {
        // Étendue sur plus de deux fuseaux UTM et projet en degrés.
        let wide = dem_deg(-10.0, 40.0, 10.0, 50.0);
        let err = work_epsg_for_dem(&wide, "mnt_france", &project("EPSG:4326", "deg")).unwrap_err();
        assert_eq!(err.code(), DEM_NOT_PROJECTED);
        let message = err.to_string();
        assert!(message.contains("« mnt_france »"), "{message}");
        assert!(message.contains("EPSG:4326"), "{message}");
        assert!(message.contains("lui aussi en degrés"), "{message}");
        assert!(message.contains("Lambert-93"), "{message}");
        // Projet en mètres mais Mercator mondial : la raison change.
        let err = work_epsg_for_dem(&wide, "mnt", &project("EPSG:3857", "m")).unwrap_err();
        assert!(
            err.to_string().contains("en mètres, ne convient pas"),
            "{err}"
        );
        assert!(!err.to_string().contains("lui aussi en degrés"), "{err}");
        // Un projet métrique de code connu sauve un MNT trop large pour un fuseau.
        assert_eq!(
            work_epsg_for_dem(&wide, "mnt", &project("EPSG:2154", "m")).unwrap(),
            Some(2154)
        );
    }

    #[test]
    fn a_dem_system_check_compares_the_read_grid_to_the_chosen_system() {
        let dem = crate::dem::Dem {
            width: 1,
            height: 1,
            origin_x: 0.0,
            origin_y: 1.0,
            cell_size_x: 1.0,
            cell_size_y: 1.0,
            epsg: 32632,
            nodata: None,
            values: vec![1.0],
        };
        assert!(check_dem_system(&dem, None).is_ok());
        assert!(check_dem_system(&dem, Some(32632)).is_ok());
        let failure = check_dem_system(&dem, Some(2154)).unwrap_err();
        assert_eq!(failure.code, DEM_WORK_MISMATCH);
        assert!(
            failure.message.contains("EPSG:32632"),
            "{}",
            failure.message
        );
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
