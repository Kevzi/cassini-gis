//! Table de sémiologie : les thèmes reconnus, leurs couleurs sourcées et le style composé de
//! chacun (AD-5 : la règle et la table vivent dans le noyau).
//!
//! Décision du propriétaire (2026-11-08, option a) : pour les zones du PLU, les codes exacts des
//! préconisations de symbologie du standard CNIG ; pour le reste, des couleurs d'après la légende
//! publique de l'IGN (eau bleue, bois verts, bâti gris, courbes orangées), chacune avec sa source.
//! Les valeurs sont **relevées** dans la documentation publique et réécrites ici une à une : aucun
//! fichier de style tiers n'est recopié (le dépôt `cnigfr/DDU-SG5-SYMBOLISATION` n'a pas de
//! licence ; le style du Plan IGN est seulement lu). Une couleur que la documentation ne donne
//! pas est **choisie** d'après la légende, et sa source le dit. Rien d'Esri.
//!
//! Chaque couleur est un [`Swatch`] : sa valeur `#rrggbb` et sa source écrite. Un test vérifie
//! que tout style composé n'emploie que des couleurs de [`SWATCHES`].

use super::recognize::LayerFacts;
use super::spec::{ColorRamp, GraduatedMethod, RampStop, StyleClass, StyleOther, StyleSpec};

/// Une couleur de la table et sa source publique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swatch {
    pub hex: &'static str,
    pub source: &'static str,
}

/// Préconisations de symbologie du standard CNIG PLU, zones, petite échelle (< 1:2500).
const CNIG: &str = "CNIG, standard PLU, « Préconisations de symbologie », section Zones, \
     petite échelle (< 1:2500) : https://github.com/cnigfr/DDU-SG5-SYMBOLISATION/blob/HEAD/Archives/PLU/preconisations.md \
     (documentation lue le 2026-11-08, aucun fichier recopié)";

/// Style standard du Plan IGN, tuiles vectorielles de la Géoplateforme.
const PLAN_IGN: &str = "d'après la légende IGN : style « Plan IGN » standard de la Géoplateforme, \
     https://data.geopf.fr/annexes/ressources/vectorTiles/styles/PLAN.IGN/standard.json \
     (lu le 2026-11-08)";

/// Couleur choisie par nous, faute de valeur publiée pour ce rendu.
const CHOSEN: &str = "d'après la légende IGN : couleur choisie, non relevée — la carte IGN au \
     1:25 000 (TOP 25, https://www.ign.fr) ne publie pas de code couleur";

macro_rules! swatch {
    ($name:ident, $hex:literal, $source:expr, $what:literal) => {
        #[doc = $what]
        pub const $name: Swatch = Swatch {
            hex: $hex,
            source: $source,
        };
    };
}

// --- Zones du PLU : codes exacts du CNIG (remplissage à petite échelle, trait commun). ---
swatch!(PLU_U, "#e60000", CNIG, "Zone U, urbaine : RVB 230,0,0.");
swatch!(
    PLU_AUC,
    "#ff6565",
    CNIG,
    "Zone AUc, à urbaniser : RVB 255,101,101."
);
swatch!(
    PLU_AUS,
    "#feccbe",
    CNIG,
    "Zone AUs, à urbaniser bloquée : RVB 254,204,190."
);
swatch!(PLU_A, "#ffff00", CNIG, "Zone A, agricole : RVB 255,255,0.");
swatch!(
    PLU_N,
    "#56aa02",
    CNIG,
    "Zone N, naturelle et forestière : RVB 86,170,2."
);
swatch!(
    PLU_STROKE,
    "#343434",
    CNIG,
    "Contour des zones : RVB 52,52,52."
);

// --- Hydrographie. ---
swatch!(
    WATER,
    "#aad5e9",
    PLAN_IGN,
    "Eau : couches « hydro surfacique » et « réseau hydro - cours d'eau »."
);
swatch!(
    WATER_EDGE,
    "#82a3b2",
    PLAN_IGN,
    "Bord des surfaces en eau : couche « limite cote »."
);

// --- Végétation, d'après la `nature` de la BD TOPO. ---
swatch!(
    FOREST,
    "#dfe8d5",
    PLAN_IGN,
    "Bois : couche « ocs - vegetation - zone boisée, forêt fermée, peupleraie »."
);
swatch!(
    OPEN_FOREST,
    "#edf2d9",
    PLAN_IGN,
    "Forêt ouverte : couche « ocs - vegetation - forêt ouverte »."
);
swatch!(
    MOOR,
    "#f2eecd",
    PLAN_IGN,
    "Lande ligneuse : couche « ocs - vegetation - lande ligneuse »."
);
swatch!(
    VINEYARD,
    "#ffedd9",
    PLAN_IGN,
    "Vigne : couche « ocs - vegetation - vigne »."
);
swatch!(
    ORCHARD,
    "#fae2c5",
    PLAN_IGN,
    "Verger : couche « ocs - vegetation - verger »."
);

// --- Routes, par `importance` de la BD TOPO (filet intérieur, puis filet extérieur). ---
swatch!(
    ROAD_1,
    "#f18800",
    PLAN_IGN,
    "Importance 1 : « filet interieur - autoroute », petite échelle."
);
swatch!(
    ROAD_2,
    "#f3c66d",
    PLAN_IGN,
    "Importance 2 : « filet interieur - route principale »."
);
swatch!(
    ROAD_3,
    "#fdf28b",
    PLAN_IGN,
    "Importance 3 : « filet interieur - route regionale »."
);
swatch!(
    ROAD_4,
    "#fcf4a8",
    PLAN_IGN,
    "Importance 4 : « filet interieur - route locale »."
);
swatch!(
    ROAD_5,
    "#969696",
    PLAN_IGN,
    "Importance 5 : « filet extérieur - route non classee »."
);
swatch!(
    ROAD_6,
    "#8c8c8c",
    PLAN_IGN,
    "Importance 6 et autres : « filet extérieur - route non revetu carrosable »."
);

// --- Voies ferrées et gares. ---
swatch!(
    RAIL,
    "#787878",
    PLAN_IGN,
    "Voie ferrée : couche « Ferre a niveau - voie normale »."
);
swatch!(
    STATION,
    "#b3b5f5",
    PLAN_IGN,
    "Gare : couche « bati surfacique fonctionnel gare »."
);
swatch!(
    BLACK,
    "#000000",
    PLAN_IGN,
    "Contour noir : couche « bati infra surfacique - poste electrique bord »."
);

// --- Bâti, par `usage_1` de la BD TOPO. ---
swatch!(
    BUILDING,
    "#b4b4b4",
    CHOSEN,
    "Bâti, gris (le bâti est gris foncé sur la carte au 1:25 000)."
);
swatch!(
    BUILDING_WORK,
    "#c8c8c8",
    PLAN_IGN,
    "Bâti industriel ou commercial : « bati surfacique fonctionnel industriel ou commercial »."
);
swatch!(
    BUILDING_RELIGIOUS,
    "#f7cbcb",
    PLAN_IGN,
    "Bâti religieux : « bati religieux surfacique »."
);
swatch!(
    BUILDING_SPORT,
    "#bcd9ab",
    PLAN_IGN,
    "Bâti sportif : « bati sportif surfacique »."
);
swatch!(
    DARK_GREY,
    "#646464",
    PLAN_IGN,
    "Trait gris foncé : « construction hydrographique linéaire - Barrage »."
);

// --- Limites administratives. ---
swatch!(
    ADMIN,
    "#7e77b8",
    PLAN_IGN,
    "Limite de commune : « limite admin - limite de commune », RVB 126,119,184."
);

// --- Courbes de niveau : orangées (décision du propriétaire). ---
swatch!(
    CONTOUR,
    "#d08a3c",
    CHOSEN,
    "Courbe de niveau, bistre orangé (le Plan IGN les donne en #d9c8a9, trop pâle à l'impression)."
);
swatch!(
    CONTOUR_LOW,
    "#e3b97a",
    CHOSEN,
    "Dégradé des courbes, bas : brun clair."
);
swatch!(
    CONTOUR_HIGH,
    "#8c4a12",
    CHOSEN,
    "Dégradé des courbes, haut : brun foncé."
);

// --- Teintes d'altitude d'un MNT : teintes hypsométriques des cartes de relief de l'IGN. ---
swatch!(
    ALT_0,
    "#9cc58a",
    CHOSEN,
    "Teinte hypsométrique, 0 m : vert."
);
swatch!(
    ALT_100,
    "#c3d99a",
    CHOSEN,
    "Teinte hypsométrique, 100 m : vert clair."
);
swatch!(
    ALT_200,
    "#e5e6a6",
    CHOSEN,
    "Teinte hypsométrique, 200 m : jaune vert."
);
swatch!(
    ALT_500,
    "#eed9a0",
    CHOSEN,
    "Teinte hypsométrique, 500 m : jaune."
);
swatch!(
    ALT_1000,
    "#ddb57f",
    CHOSEN,
    "Teinte hypsométrique, 1000 m : ocre."
);
swatch!(
    ALT_1500,
    "#c9925f",
    CHOSEN,
    "Teinte hypsométrique, 1500 m : brun clair."
);
swatch!(
    ALT_2000,
    "#b07a55",
    CHOSEN,
    "Teinte hypsométrique, 2000 m : brun."
);
swatch!(
    ALT_3000,
    "#d9cfc7",
    CHOSEN,
    "Teinte hypsométrique, 3000 m : gris rosé des hauts sommets."
);
swatch!(
    ALT_4000,
    "#ffffff",
    CHOSEN,
    "Teinte hypsométrique, 4000 m : blanc des neiges."
);

/// Toutes les couleurs de la table.
pub const SWATCHES: &[Swatch] = &[
    PLU_U,
    PLU_AUC,
    PLU_AUS,
    PLU_A,
    PLU_N,
    PLU_STROKE,
    WATER,
    WATER_EDGE,
    FOREST,
    OPEN_FOREST,
    MOOR,
    VINEYARD,
    ORCHARD,
    ROAD_1,
    ROAD_2,
    ROAD_3,
    ROAD_4,
    ROAD_5,
    ROAD_6,
    RAIL,
    STATION,
    BLACK,
    BUILDING,
    BUILDING_WORK,
    BUILDING_RELIGIOUS,
    BUILDING_SPORT,
    DARK_GREY,
    ADMIN,
    CONTOUR,
    CONTOUR_LOW,
    CONTOUR_HIGH,
    ALT_0,
    ALT_100,
    ALT_200,
    ALT_500,
    ALT_1000,
    ALT_1500,
    ALT_2000,
    ALT_3000,
    ALT_4000,
];

/// Un dégradé nommé de `map.graduated_style` : ses deux couleurs extrêmes et sa source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedRamp {
    pub name: &'static str,
    pub from: &'static str,
    pub to: &'static str,
    pub source: &'static str,
}

/// Dégradés nommés : couleurs extrêmes des schémas ColorBrewer à 9 classes (Cynthia A. Brewer,
/// Pennsylvania State University, licence Apache-2.0), relevées et réécrites ici. `red_blue` est
/// divergent, du rouge au bleu (RdBu). Attribution demandée par la licence, reprise dans le
/// fichier `NOTICE` à la racine du dépôt :
///
/// « This product includes color specifications and designs developed by Cynthia Brewer
/// (http://colorbrewer.org/). » — Licensed under the Apache License, Version 2.0.
pub const RAMPS: &[NamedRamp] = &[
    NamedRamp {
        name: "yellow_red",
        from: "#ffffcc",
        to: "#800026",
        source: "ColorBrewer 2.0, schéma séquentiel YlOrRd à 9 classes, https://colorbrewer2.org, licence Apache-2.0 (lu le 2026-11-08)",
    },
    NamedRamp {
        name: "blues",
        from: "#f7fbff",
        to: "#08306b",
        source: "ColorBrewer 2.0, schéma séquentiel Blues à 9 classes, https://colorbrewer2.org, licence Apache-2.0 (lu le 2026-11-08)",
    },
    NamedRamp {
        name: "greens",
        from: "#f7fcf5",
        to: "#00441b",
        source: "ColorBrewer 2.0, schéma séquentiel Greens à 9 classes, https://colorbrewer2.org, licence Apache-2.0 (lu le 2026-11-08)",
    },
    NamedRamp {
        name: "reds",
        from: "#fff5f0",
        to: "#67000d",
        source: "ColorBrewer 2.0, schéma séquentiel Reds à 9 classes, https://colorbrewer2.org, licence Apache-2.0 (lu le 2026-11-08)",
    },
    NamedRamp {
        name: "purples",
        from: "#fcfbfd",
        to: "#3f007d",
        source: "ColorBrewer 2.0, schéma séquentiel Purples à 9 classes, https://colorbrewer2.org, licence Apache-2.0 (lu le 2026-11-08)",
    },
    NamedRamp {
        name: "greys",
        from: "#ffffff",
        to: "#000000",
        source: "ColorBrewer 2.0, schéma séquentiel Greys à 9 classes, https://colorbrewer2.org, licence Apache-2.0 (lu le 2026-11-08)",
    },
    NamedRamp {
        name: "red_blue",
        from: "#b2182b",
        to: "#2166ac",
        source: "ColorBrewer 2.0, schéma divergent RdBu à 9 classes, https://colorbrewer2.org, licence Apache-2.0 (lu le 2026-11-08)",
    },
];

/// Le dégradé nommé `name`, s'il est dans la table.
pub fn ramp(name: &str) -> Option<&'static NamedRamp> {
    RAMPS.iter().find(|r| r.name == name)
}

/// Paliers de la rampe d'altitudes, en mètres, fixes. Resserrés en bas, mais un relief de plaine
/// (Villeneuve-le-Roi, 30 à 100 m) n'en couvre qu'un ou deux et paraît presque plat, alors qu'un
/// relief de montagne (Annecy, 400 à 2700 m) les parcourt presque tous. Étirer la rampe entre le
/// minimum et le maximum réels du MNT est reporté : le noyau ne lit pas le raster ici.
pub const ELEVATION_STOPS: &[(f64, Swatch)] = &[
    (0.0, ALT_0),
    (100.0, ALT_100),
    (200.0, ALT_200),
    (500.0, ALT_500),
    (1000.0, ALT_1000),
    (1500.0, ALT_1500),
    (2000.0, ALT_2000),
    (3000.0, ALT_3000),
    (4000.0, ALT_4000),
];

/// Opacité de l'ombrage posé en fondu sur les teintes d'altitude.
pub const HILLSHADE_OPACITY: f64 = 0.5;

/// Nombre de classes et méthode du dégradé des courbes de niveau à champ d'altitude.
pub const CONTOUR_CLASSES: u32 = 5;

/// Épaisseurs, en millimètres de carte. Le trait des zones du PLU vaut 0,8 pt (CNIG), soit 0,28 mm.
mod width {
    pub const PLU: f64 = 0.28;
    pub const WATERCOURSE: f64 = 0.6;
    pub const WATER_EDGE: f64 = 0.2;
    pub const RAIL: f64 = 0.5;
    pub const STATION: f64 = 0.3;
    pub const BUILDING: f64 = 0.1;
    pub const ADMIN: f64 = 0.5;
    pub const PARCEL: f64 = 0.13;
    pub const CONTOUR: f64 = 0.2;
    pub const VEGETATION: f64 = 0.0;
    /// Routes, de l'importance 1 à 6 ; la dernière vaut aussi pour les valeurs non listées.
    pub const ROADS: [f64; 6] = [1.0, 0.8, 0.6, 0.5, 0.35, 0.25];
}

/// Un thème reconnu d'une couche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Theme {
    Stations,
    Roads,
    Railways,
    Watercourses,
    WaterBodies,
    Contours,
    AdminLimits,
    Buildings,
    Parcels,
    PluZones,
    Vegetation,
    Hillshade,
    Dem,
}

/// Un groupe du rangement. L'ordre de déclaration est l'ordre d'affichage à rang égal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    /// Couches vecteur non reconnues, souvent les résultats de l'utilisateur (tampons,
    /// sélections) : en haut de tout.
    OtherVectors,
    Transport,
    Hydrography,
    Contours,
    AdminLimits,
    Buildings,
    Cadastre,
    Planning,
    Vegetation,
    /// Rasters non reconnus (pente, champ de vision…) : juste au-dessus du relief.
    OtherRasters,
    Relief,
}

impl Group {
    /// Nom affiché du groupe, choisi par le noyau (AD-5).
    pub fn name(self) -> &'static str {
        match self {
            Group::Transport => "Transport",
            Group::Hydrography => "Hydrographie",
            Group::Contours => "Courbes de niveau",
            Group::AdminLimits => "Limites administratives",
            Group::Buildings => "Bâti",
            Group::Cadastre => "Cadastre",
            Group::Planning => "Urbanisme",
            Group::Vegetation => "Végétation",
            Group::OtherVectors => "Autres couches",
            Group::OtherRasters => "Autres rasters",
            Group::Relief => "Relief",
        }
    }
}

impl Theme {
    /// Identifiant du thème, rendu dans `styled`.
    pub fn id(self) -> &'static str {
        match self {
            Theme::Stations => "gares",
            Theme::Roads => "routes",
            Theme::Railways => "voies_ferrees",
            Theme::Watercourses => "cours_d_eau",
            Theme::WaterBodies => "eau_de_surface",
            Theme::Contours => "courbes_de_niveau",
            Theme::AdminLimits => "limites_administratives",
            Theme::Buildings => "bati",
            Theme::Parcels => "parcelles",
            Theme::PluZones => "zones_plu",
            Theme::Vegetation => "vegetation",
            Theme::Hillshade => "ombrage",
            Theme::Dem => "mnt",
        }
    }

    /// Groupe du rangement.
    pub fn group(self) -> Group {
        match self {
            Theme::Stations | Theme::Roads | Theme::Railways => Group::Transport,
            Theme::Watercourses | Theme::WaterBodies => Group::Hydrography,
            Theme::Contours => Group::Contours,
            Theme::AdminLimits => Group::AdminLimits,
            Theme::Buildings => Group::Buildings,
            Theme::Parcels => Group::Cadastre,
            Theme::PluZones => Group::Planning,
            Theme::Vegetation => Group::Vegetation,
            Theme::Hillshade | Theme::Dem => Group::Relief,
        }
    }
}

/// Les cinq zones du standard CNIG, qui portent chacune une classe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluZone {
    U,
    AUc,
    AUs,
    A,
    N,
}

/// La zone d'une valeur de `typezone`, par préfixe, chiffres de tête ôtés : `U`, `UA`… donnent
/// U ; `AU`, `AUc`, `1AU` donnent AUc ; `AUs`, `2AU` (zone bloquée) donnent AUs ; `A`, `Ah`,
/// `Ap` donnent A ; `N`, `Nh`, `Nd` donnent N. `None` pour une autre valeur.
pub fn plu_zone(value: &str) -> Option<PluZone> {
    let trimmed = value.trim();
    let blocked = trimmed.starts_with('2');
    let code = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
    let upper = code.to_ascii_uppercase();
    if upper.starts_with("AUS") || (blocked && upper.starts_with("AU")) {
        Some(PluZone::AUs)
    } else if upper.starts_with("AU") {
        Some(PluZone::AUc)
    } else if upper.starts_with('U') {
        Some(PluZone::U)
    } else if upper.starts_with('A') {
        Some(PluZone::A)
    } else if upper.starts_with('N') {
        Some(PluZone::N)
    } else {
        None
    }
}

fn hex(s: Swatch) -> String {
    s.hex.to_string()
}

fn class(values: &[&str], label: &str, stroke: Swatch, fill: Option<Swatch>, w: f64) -> StyleClass {
    StyleClass {
        values: values.iter().map(|v| v.to_string()).collect(),
        label: label.to_string(),
        stroke: hex(stroke),
        fill: fill.map(hex),
        width_mm: Some(w),
    }
}

fn single(stroke: Option<Swatch>, fill: Option<Swatch>, w: Option<f64>) -> StyleSpec {
    StyleSpec::Single {
        stroke: stroke.map(hex),
        fill: fill.map(hex),
        width_mm: w,
        opacity: None,
    }
}

/// Le style d'une couche de thème `theme`, composé depuis la table. Une classe par attribut est
/// posée quand la couche porte le champ connu (`typezone`, `importance`, `usage_1`, `nature`, un
/// champ numérique d'altitude) ; le nom du champ est celui de la couche, casse comprise.
pub fn style_for(theme: Theme, layer: &LayerFacts) -> StyleSpec {
    match theme {
        Theme::PluZones => {
            let typezone = layer.field_named(&["typezone"]);
            let field = typezone.map_or_else(|| "typezone".to_string(), |f| f.name.clone());
            let w = width::PLU;
            // Les valeurs de la couche rejoignent leur classe par préfixe (`1AU`, `Ah`, `Nd`…).
            let seen: &[String] = typezone.map_or(&[], |f| f.values.as_slice());
            let values = |zone: PluZone, canonical: &[&str]| -> Vec<String> {
                let mut out: Vec<String> = canonical.iter().map(|v| v.to_string()).collect();
                for v in seen {
                    if plu_zone(v) == Some(zone) && !out.contains(v) {
                        out.push(v.clone());
                    }
                }
                out
            };
            let zone = |zone: PluZone, canonical: &[&str], label: &str, fill: Swatch| {
                let mut c = class(&[], label, PLU_STROKE, Some(fill), w);
                c.values = values(zone, canonical);
                c
            };
            // Le standard CNIG nomme AUc et AUs ; « AU » seul, d'un PLU plus ancien, prend AUc.
            StyleSpec::Categories {
                field,
                classes: vec![
                    zone(PluZone::U, &["U"], "Zone urbaine (U)", PLU_U),
                    zone(
                        PluZone::AUc,
                        &["AUc", "AU"],
                        "Zone à urbaniser (AUc)",
                        PLU_AUC,
                    ),
                    zone(
                        PluZone::AUs,
                        &["AUs"],
                        "Zone à urbaniser bloquée (AUs)",
                        PLU_AUS,
                    ),
                    zone(PluZone::A, &["A"], "Zone agricole (A)", PLU_A),
                    zone(
                        PluZone::N,
                        &["N"],
                        "Zone naturelle et forestière (N)",
                        PLU_N,
                    ),
                ],
                other: Some(StyleOther {
                    label: "Autre zone".to_string(),
                    stroke: hex(PLU_STROKE),
                    fill: None,
                    width_mm: Some(w),
                }),
            }
        }
        Theme::Roads => match layer.field_named(&["importance"]) {
            Some(field) => {
                let colors = [ROAD_1, ROAD_2, ROAD_3, ROAD_4, ROAD_5, ROAD_6];
                let classes = colors
                    .iter()
                    .zip(width::ROADS)
                    .enumerate()
                    .map(|(i, (color, w))| {
                        let value = (i + 1).to_string();
                        class(&[&value], &format!("Importance {value}"), *color, None, w)
                    })
                    .collect();
                StyleSpec::Categories {
                    field: field.name.clone(),
                    classes,
                    other: Some(StyleOther {
                        label: "Autre route".to_string(),
                        stroke: hex(ROAD_6),
                        fill: None,
                        width_mm: Some(width::ROADS[5]),
                    }),
                }
            }
            None => single(Some(ROAD_2), None, Some(width::ROADS[3])),
        },
        Theme::Railways => single(Some(RAIL), None, Some(width::RAIL)),
        Theme::Stations => single(Some(BLACK), Some(STATION), Some(width::STATION)),
        Theme::Watercourses => single(Some(WATER), None, Some(width::WATERCOURSE)),
        Theme::WaterBodies => single(Some(WATER_EDGE), Some(WATER), Some(width::WATER_EDGE)),
        Theme::Vegetation => match layer.text_field_named(&["nature"]) {
            Some(field) => {
                let w = width::VEGETATION;
                StyleSpec::Categories {
                    field: field.name.clone(),
                    classes: vec![
                        class(
                            &["Forêt ouverte"],
                            "Forêt ouverte",
                            OPEN_FOREST,
                            Some(OPEN_FOREST),
                            w,
                        ),
                        class(&["Lande ligneuse"], "Lande ligneuse", MOOR, Some(MOOR), w),
                        class(&["Vigne"], "Vigne", VINEYARD, Some(VINEYARD), w),
                        class(&["Verger"], "Verger", ORCHARD, Some(ORCHARD), w),
                    ],
                    other: Some(StyleOther {
                        label: "Bois".to_string(),
                        stroke: hex(FOREST),
                        fill: Some(hex(FOREST)),
                        width_mm: Some(w),
                    }),
                }
            }
            None => single(None, Some(FOREST), None),
        },
        Theme::Buildings => match layer.text_field_named(&["usage_1", "usage1"]) {
            Some(field) => {
                let w = width::BUILDING;
                StyleSpec::Categories {
                    field: field.name.clone(),
                    classes: vec![
                        class(
                            &["Industriel", "Commercial et services"],
                            "Industriel ou commercial",
                            DARK_GREY,
                            Some(BUILDING_WORK),
                            w,
                        ),
                        class(
                            &["Religieux"],
                            "Religieux",
                            DARK_GREY,
                            Some(BUILDING_RELIGIOUS),
                            w,
                        ),
                        class(&["Sportif"], "Sportif", DARK_GREY, Some(BUILDING_SPORT), w),
                    ],
                    other: Some(StyleOther {
                        label: "Autre bâtiment".to_string(),
                        stroke: hex(DARK_GREY),
                        fill: Some(hex(BUILDING)),
                        width_mm: Some(w),
                    }),
                }
            }
            None => single(Some(DARK_GREY), Some(BUILDING), Some(width::BUILDING)),
        },
        Theme::AdminLimits => single(Some(ADMIN), None, Some(width::ADMIN)),
        Theme::Parcels => single(Some(DARK_GREY), None, Some(width::PARCEL)),
        Theme::Contours => match layer.numeric_field_named(&["altitude", "alti", "elevation", "z"])
        {
            // Dégradé de bruns, seuils calculés par QGIS sur les vraies altitudes.
            Some(field) => StyleSpec::Graduated {
                field: field.name.clone(),
                method: GraduatedMethod::EqualInterval,
                class_count: CONTOUR_CLASSES,
                ramp: ColorRamp {
                    from: hex(CONTOUR_LOW),
                    to: hex(CONTOUR_HIGH),
                },
                stroke: None,
                width_mm: Some(width::CONTOUR),
            },
            None => single(Some(CONTOUR), None, Some(width::CONTOUR)),
        },
        Theme::Dem => StyleSpec::ElevationRamp {
            stops: ELEVATION_STOPS
                .iter()
                .map(|(value, s)| RampStop {
                    value: *value,
                    color: hex(*s),
                })
                .collect(),
            opacity: None,
        },
        Theme::Hillshade => StyleSpec::Hillshade {
            opacity: Some(HILLSHADE_OPACITY),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::recognize::LayerFacts;
    use serde_json::json;

    const ALL: [Theme; 13] = [
        Theme::Stations,
        Theme::Roads,
        Theme::Railways,
        Theme::Watercourses,
        Theme::WaterBodies,
        Theme::Contours,
        Theme::AdminLimits,
        Theme::Buildings,
        Theme::Parcels,
        Theme::PluZones,
        Theme::Vegetation,
        Theme::Hillshade,
        Theme::Dem,
    ];

    fn with_fields(fields: serde_json::Value) -> LayerFacts {
        LayerFacts::from_raw(&json!({"layer_id": "l", "name": "x", "kind": "vector",
            "geometry": "MultiPolygon", "fields": fields}))
        .unwrap()
    }

    #[test]
    fn every_color_of_the_table_has_a_written_source() {
        assert!(!SWATCHES.is_empty());
        for s in SWATCHES {
            assert!(crate::style::spec::is_color(s.hex), "{}", s.hex);
            assert_eq!(s.hex, s.hex.to_ascii_lowercase(), "{}", s.hex);
            assert!(s.source.contains("https://"), "{} : {}", s.hex, s.source);
            assert!(
                s.source.contains("2026-11-08") || s.source.contains("choisie"),
                "{}",
                s.hex
            );
            assert!(
                s.source.starts_with("CNIG") || s.source.starts_with("d'après la légende IGN"),
                "{} : {}",
                s.hex,
                s.source
            );
        }
        // Les codes du PLU sont ceux du CNIG, et seulement eux.
        for s in [PLU_U, PLU_AUC, PLU_AUS, PLU_A, PLU_N, PLU_STROKE] {
            assert!(s.source.starts_with("CNIG"));
        }
    }

    #[test]
    fn every_composed_style_is_valid_and_uses_only_sourced_colors() {
        let bare = with_fields(json!([]));
        let rich = with_fields(json!([
            {"name": "TYPEZONE", "type": "String", "values": ["U"]},
            {"name": "importance", "type": "String", "values": ["1"]},
            {"name": "nature", "type": "String", "values": ["Bois"]},
            {"name": "usage_1", "type": "String", "values": ["Résidentiel"]},
            {"name": "altitude", "type": "Real"}
        ]));
        for theme in ALL {
            for layer in [&bare, &rich] {
                let style = style_for(theme, layer);
                style.validate().unwrap();
                for color in style.colors() {
                    assert!(
                        SWATCHES.iter().any(|s| s.hex == color),
                        "{theme:?} : {color} hors de la table"
                    );
                }
            }
        }
    }

    #[test]
    fn the_plu_takes_the_cnig_codes_on_the_layer_field() {
        let layer = with_fields(json!([{"name": "TYPEZONE", "type": "String"}]));
        let StyleSpec::Categories { field, classes, .. } = style_for(Theme::PluZones, &layer)
        else {
            panic!("classes attendues");
        };
        assert_eq!(field, "TYPEZONE");
        let fills: Vec<(&str, &str)> = classes
            .iter()
            .map(|c| (c.values[0].as_str(), c.fill.as_deref().unwrap()))
            .collect();
        assert_eq!(
            fills,
            [
                ("U", "#e60000"),
                ("AUc", "#ff6565"),
                ("AUs", "#feccbe"),
                ("A", "#ffff00"),
                ("N", "#56aa02")
            ]
        );
    }

    #[test]
    fn plu_values_join_their_class_by_prefix() {
        for (value, zone) in [
            ("U", PluZone::U),
            ("UA", PluZone::U),
            ("AU", PluZone::AUc),
            ("AUc", PluZone::AUc),
            ("1AU", PluZone::AUc),
            ("1AUh", PluZone::AUc),
            ("AUs", PluZone::AUs),
            ("2AU", PluZone::AUs),
            ("A", PluZone::A),
            ("Ah", PluZone::A),
            ("Ap", PluZone::A),
            ("N", PluZone::N),
            ("Nh", PluZone::N),
            ("Nd", PluZone::N),
        ] {
            assert_eq!(plu_zone(value), Some(zone), "{value}");
        }
        assert_eq!(plu_zone("ZAC"), None);
        let layer = with_fields(json!([{"name": "typezone", "type": "String",
            "values": ["1AU", "2AU", "Ah", "Nd", "UB", "ZAC"]}]));
        let StyleSpec::Categories { classes, .. } = style_for(Theme::PluZones, &layer) else {
            panic!("classes attendues");
        };
        let values: Vec<Vec<&str>> = classes
            .iter()
            .map(|c| c.values.iter().map(String::as_str).collect())
            .collect();
        assert_eq!(
            values,
            [
                vec!["U", "UB"],
                vec!["AUc", "AU", "1AU"],
                vec!["AUs", "2AU"],
                vec!["A", "Ah"],
                vec!["N", "Nd"]
            ]
        );
    }

    #[test]
    fn contours_with_a_numeric_altitude_get_a_graduated_brown_ramp() {
        let numeric = with_fields(json!([{"name": "ALTITUDE", "type": "Integer"}]));
        assert!(matches!(
            style_for(Theme::Contours, &numeric),
            StyleSpec::Graduated { ref field, method: GraduatedMethod::EqualInterval, .. }
                if field == "ALTITUDE"
        ));
        // Un champ texte n'est pas une altitude : trait simple orangé.
        let text = with_fields(json!([{"name": "altitude", "type": "String"}]));
        assert!(matches!(
            style_for(Theme::Contours, &text),
            StyleSpec::Single { .. }
        ));
    }

    #[test]
    fn every_named_ramp_has_two_colors_and_a_written_source() {
        let mut names: Vec<&str> = RAMPS.iter().map(|r| r.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 7);
        for r in RAMPS {
            assert!(crate::style::spec::is_color(r.from) && crate::style::spec::is_color(r.to));
            assert!(r.source.contains("https://colorbrewer2.org"), "{}", r.name);
            assert!(r.source.contains("Apache-2.0"), "{}", r.name);
        }
        assert!(ramp("red_blue").unwrap().source.contains("divergent"));
        assert!(ramp("rainbow").is_none());
    }

    #[test]
    fn the_elevation_stops_increase() {
        assert!(ELEVATION_STOPS.windows(2).all(|w| w[0].0 < w[1].0));
    }
}
