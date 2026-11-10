//! Inventaires synthétiques : des thèmes civils (urbanisme, environnement, réseaux, SIG
//! général), chacun avec ses noms de couche, ses fichiers et ses champs, tirés au hasard pour
//! composer l'inventaire d'un projet. Aucune valeur n'est tirée d'un fichier réel : les
//! valeurs imitent les standards publics (zonage du CNIG : `U`, `AUc`, `N`...).

use serde_json::{json, Value};

use crate::rng::Rng;

/// Type de géométrie d'un thème.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Geom {
    Point,
    Line,
    Polygon,
    Raster,
}

/// Un nom commun et ses formes : singulier, pluriel, genre.
#[derive(Debug)]
pub struct Noun {
    pub sg: &'static str,
    pub pl: &'static str,
    pub fem: bool,
}

impl Noun {
    pub fn les(&self) -> String {
        format!("les {}", self.pl)
    }
    pub fn des(&self) -> String {
        format!("des {}", self.pl)
    }
    pub fn un(&self) -> String {
        format!("{} {}", if self.fem { "une" } else { "un" }, self.sg)
    }
    pub fn dun(&self) -> String {
        format!("d'{}", self.un())
    }
    /// Participe accordé au pluriel : `situé` devient `situées` ou `situés`.
    pub fn agree(&self, stem: &str) -> String {
        format!("{stem}{}", if self.fem { "es" } else { "s" })
    }
    pub fn quels(&self) -> &'static str {
        if self.fem {
            "Quelles"
        } else {
            "Quels"
        }
    }
    pub fn ceux(&self) -> &'static str {
        if self.fem {
            "celles"
        } else {
            "ceux"
        }
    }
}

/// Une valeur d'un champ texte, avec son adjectif éventuel (singulier, pluriel) :
/// `nationale` se dit aussi « routes nationales ».
#[derive(Debug)]
pub struct Val {
    pub v: &'static str,
    pub adj: Option<(&'static str, &'static str)>,
}

const fn v(v: &'static str) -> Val {
    Val { v, adj: None }
}

const fn va(v: &'static str, sg: &'static str, pl: &'static str) -> Val {
    Val {
        v,
        adj: Some((sg, pl)),
    }
}

/// Nature d'un champ. Les tournures portent `{v}` (la valeur) ou `{n}` (le nombre).
#[derive(Debug)]
pub enum Kind {
    Text {
        values: &'static [Val],
        phrases: &'static [&'static str],
    },
    Number {
        int: bool,
        min: u64,
        max: u64,
        step: u64,
        gt: &'static [&'static str],
        lt: &'static [&'static str],
        gte: &'static [&'static str],
        lte: &'static [&'static str],
    },
    Name {
        fragments: &'static [&'static str],
    },
    Id,
}

/// Un champ possible d'un thème : son rôle (stable), ses noms possibles, et le libellé qui
/// sert à la question quand il manque (« aucun champ de hauteur »).
#[derive(Debug)]
pub struct FieldDef {
    pub role: &'static str,
    pub names: &'static [&'static str],
    pub label: &'static str,
    pub kind: Kind,
}

/// Un thème de couche.
#[derive(Debug)]
pub struct Theme {
    pub key: &'static str,
    /// Thèmes d'un même groupe ne coexistent pas dans un inventaire (deux couches de routes
    /// rendraient « une route » ambigu).
    pub group: &'static str,
    pub domain: &'static str,
    pub noun: Noun,
    /// « de parcelles » : « Aucune couche de parcelles ».
    pub of_none: &'static str,
    /// « la couche des parcelles ».
    pub the_layer: &'static str,
    pub names: &'static [&'static str],
    pub slugs: &'static [&'static str],
    pub files: &'static [&'static str],
    pub geom: Geom,
    pub fields: &'static [FieldDef],
}

impl Theme {
    pub fn field(&self, role: &str) -> Option<&'static FieldDef> {
        self.fields.iter().find(|f| f.role == role)
    }
}

macro_rules! number {
    ($int:expr, $min:expr, $max:expr, $step:expr, gt: $gt:expr, lt: $lt:expr, gte: $gte:expr, lte: $lte:expr) => {
        Kind::Number {
            int: $int,
            min: $min,
            max: $max,
            step: $step,
            gt: $gt,
            lt: $lt,
            gte: $gte,
            lte: $lte,
        }
    };
}

/// Noms du champ de rôle `nom`, communs à tous les thèmes (après les quatre causes, 2026-11-10) :
/// un nom tiré du thème (`nom_commune`, `nom_<thème>`) apprenait au modèle à le deviner au lieu
/// de le recopier de l'inventaire. Noms génériques de producteurs publics et d'usage courant.
pub const NAME_FIELDS: &[&str] = &[
    "nom",
    "nom_officiel",
    "NOM",
    "libelle",
    "toponyme",
    "nom_usuel",
    "appellation",
    "denomination",
];

/// Champ leurre posé à côté du champ de nom : sa version en majuscules ou un code. Rôle à part,
/// qu'aucune famille ne demande : un plan ne le cite jamais.
static DECOY: FieldDef = FieldDef {
    role: "leurre",
    names: &[],
    label: "",
    kind: Kind::Name { fragments: &[] },
};

const NOM: FieldDef = FieldDef {
    role: "nom",
    names: NAME_FIELDS,
    label: "de nom",
    kind: Kind::Name {
        fragments: &[
            "Saint", "Jean", "Moulin", "Gare", "Centre", "Pasteur", "Hugo", "Église", "Parc",
            "Bois", "Pont", "Haut",
        ],
    },
};

/// Tous les thèmes. L'ordre est fixe : il fait partie de la reproductibilité.
pub static THEMES: &[Theme] = &[
    Theme {
        key: "parcelles",
        group: "parcelles",
        domain: "urbanisme",
        noun: Noun {
            sg: "parcelle",
            pl: "parcelles",
            fem: true,
        },
        of_none: "de parcelles",
        the_layer: "la couche des parcelles",
        names: &[
            "Parcelles cadastrales",
            "Parcelles",
            "Cadastre - parcelles",
            "Parcelles du cadastre",
        ],
        slugs: &["parcelles", "cadastre_parcelles", "parcelle"],
        files: &["parcelles", "cadastre", "parcelles_cadastrales"],
        geom: Geom::Polygon,
        fields: &[
            FieldDef {
                role: "id",
                names: &["id_parcelle", "idu", "id"],
                label: "d'identifiant",
                kind: Kind::Id,
            },
            FieldDef {
                role: "section",
                names: &["section", "code_section"],
                label: "de section",
                kind: Kind::Text {
                    values: &[
                        v("AB"),
                        v("AC"),
                        v("AD"),
                        v("AE"),
                        v("AH"),
                        v("B"),
                        v("C"),
                        v("ZA"),
                        v("ZB"),
                    ],
                    // Pas « de la section {v} » : « Garde seulement les parcelles de la section AB »
                    // est une phrase des essais réels (`training/tests/recouvrement.rs`, 5.15).
                    phrases: &["situées en section {v}", "en section {v}", "de section {v}"],
                },
            },
            FieldDef {
                role: "surface",
                names: &["surface_m2", "contenance", "surf_m2"],
                label: "de surface",
                kind: number!(false, 200, 5000, 50,
                    gt: &["de plus de {n} m²", "d'une surface supérieure à {n} m²", "dépassant {n} m²"],
                    lt: &["de moins de {n} m²", "d'une surface inférieure à {n} m²"],
                    gte: &["d'au moins {n} m²", "de {n} m² ou plus"],
                    lte: &["d'au plus {n} m²", "de {n} m² au maximum"]),
            },
        ],
    },
    Theme {
        key: "zonage",
        group: "zonage",
        domain: "urbanisme",
        noun: Noun {
            sg: "zone du PLU",
            pl: "zones du PLU",
            fem: true,
        },
        of_none: "de zonage du PLU",
        the_layer: "la couche du zonage",
        names: &[
            "Zonage du PLU",
            "Zonage PLU",
            "PLU - zonage",
            "Zones du PLU",
            "Zonage d'urbanisme",
        ],
        slugs: &["plu_zonage", "zonage_plu", "zonage", "zone_urba"],
        files: &["plu_zonage", "zonage_plu", "zone_urba"],
        geom: Geom::Polygon,
        fields: &[
            FieldDef {
                role: "zone",
                names: &["typezone", "type_zone"],
                label: "de type de zone",
                kind: Kind::Text {
                    values: &[v("U"), v("AUc"), v("AUs"), v("A"), v("N"), v("Nh")],
                    phrases: &["de type {v}", "classées {v}"],
                },
            },
            FieldDef {
                role: "libelle",
                names: &["libelle", "libelong"],
                label: "de libellé",
                kind: Kind::Text {
                    values: &[
                        v("Zone urbaine"),
                        v("Zone à urbaniser"),
                        v("Zone agricole"),
                        v("Zone naturelle"),
                    ],
                    phrases: &["dont le libellé est « {v} »"],
                },
            },
            FieldDef {
                role: "destination",
                names: &["destdomi", "destination"],
                label: "de destination",
                kind: Kind::Text {
                    values: &[v("habitat"), v("activités"), v("équipements"), v("mixte")],
                    phrases: &["à dominante {v}", "de destination {v}"],
                },
            },
        ],
    },
    Theme {
        key: "batiments",
        group: "batiments",
        domain: "urbanisme",
        noun: Noun {
            sg: "bâtiment",
            pl: "bâtiments",
            fem: false,
        },
        of_none: "de bâtiments",
        the_layer: "la couche des bâtiments",
        names: &["Bâtiments", "Bâti", "Bâtiments BD", "Emprises bâties"],
        slugs: &["batiments", "bati", "batiment"],
        files: &["batiments", "bati", "batiments_indifferencies"],
        geom: Geom::Polygon,
        fields: &[
            FieldDef {
                role: "usage",
                names: &["usage", "usage_1", "nature"],
                label: "d'usage",
                kind: Kind::Text {
                    values: &[
                        va("résidentiel", "résidentiel", "résidentiels"),
                        va("commercial", "commercial", "commerciaux"),
                        va("industriel", "industriel", "industriels"),
                        va("public", "public", "publics"),
                        v("annexe"),
                        va("agricole", "agricole", "agricoles"),
                    ],
                    phrases: &["à usage {v}", "dont l'usage est {v}", "d'usage {v}"],
                },
            },
            FieldDef {
                role: "hauteur",
                names: &["hauteur_m", "hauteur", "haut_m"],
                label: "de hauteur",
                kind: number!(false, 3, 40, 1,
                    gt: &["de plus de {n} m de haut", "dont la hauteur dépasse {n} m", "hauts de plus de {n} m"],
                    lt: &["de moins de {n} m de haut", "dont la hauteur est inférieure à {n} m"],
                    gte: &["d'au moins {n} m de hauteur", "de {n} m de haut ou plus"],
                    lte: &["d'au plus {n} m de haut", "ne dépassant pas {n} m de hauteur"]),
            },
            FieldDef {
                role: "etages",
                names: &["nb_etages", "etages", "nb_niveaux"],
                label: "de nombre d'étages",
                kind: number!(true, 1, 15, 1,
                    gt: &["de plus de {n} étages", "comptant plus de {n} étages"],
                    lt: &["de moins de {n} étages"],
                    gte: &["d'au moins {n} étages", "de {n} étages ou plus"],
                    lte: &["d'au plus {n} étages", "de {n} étages au maximum"]),
            },
        ],
    },
    Theme {
        key: "communes",
        group: "communes",
        domain: "général",
        noun: Noun {
            sg: "commune",
            pl: "communes",
            fem: true,
        },
        of_none: "de communes",
        the_layer: "la couche des communes",
        names: &["Communes", "Limites communales", "Communes du département"],
        slugs: &["communes", "commune", "limites_communales"],
        files: &["communes", "commune", "limites_communales"],
        geom: Geom::Polygon,
        fields: &[
            FieldDef {
                role: "nom",
                names: NAME_FIELDS,
                label: "de nom",
                kind: Kind::Name {
                    fragments: &[
                        "Saint",
                        "sur-Seine",
                        "sur-Marne",
                        "Mont",
                        "Bois",
                        "le-Pont",
                        "Val",
                    ],
                },
            },
            FieldDef {
                role: "insee",
                names: &["code_insee", "insee_com"],
                label: "de code INSEE",
                kind: Kind::Id,
            },
            FieldDef {
                role: "population",
                names: &["population", "pop", "nb_hab"],
                label: "de population",
                kind: number!(true, 1000, 60000, 500,
                    gt: &["de plus de {n} habitants", "dépassant {n} habitants"],
                    lt: &["de moins de {n} habitants"],
                    gte: &["d'au moins {n} habitants"],
                    lte: &["d'au plus {n} habitants"]),
            },
        ],
    },
    Theme {
        key: "quartiers",
        group: "quartiers",
        domain: "général",
        noun: Noun {
            sg: "quartier",
            pl: "quartiers",
            fem: false,
        },
        of_none: "de quartiers",
        the_layer: "la couche des quartiers",
        names: &[
            "Quartiers",
            "Quartiers de la ville",
            "Découpage en quartiers",
        ],
        slugs: &["quartiers", "quartier"],
        files: &["quartiers", "decoupage_quartiers"],
        geom: Geom::Polygon,
        fields: &[NOM],
    },
    Theme {
        key: "gares",
        group: "gares",
        domain: "réseaux",
        noun: Noun {
            sg: "gare",
            pl: "gares",
            fem: true,
        },
        of_none: "de gares",
        the_layer: "la couche des gares",
        names: &["Gares", "Gares ferroviaires", "Gares SNCF et RER"],
        slugs: &["gares", "gare", "gares_ferro"],
        files: &["gares", "gares_ferroviaires", "gares_rer"],
        geom: Geom::Point,
        fields: &[
            NOM,
            FieldDef {
                role: "ligne",
                names: &["ligne", "code_ligne", "lignes"],
                label: "de ligne",
                kind: Kind::Text {
                    values: &[
                        v("A"),
                        v("B"),
                        v("C"),
                        v("D"),
                        v("E"),
                        v("H"),
                        v("J"),
                        v("P"),
                        v("U"),
                    ],
                    phrases: &[
                        "de la ligne {v}",
                        "desservies par la ligne {v}",
                        "sur la ligne {v}",
                    ],
                },
            },
        ],
    },
    Theme {
        key: "arrets",
        group: "arrets",
        domain: "réseaux",
        noun: Noun {
            sg: "arrêt de bus",
            pl: "arrêts de bus",
            fem: false,
        },
        of_none: "d'arrêts de bus",
        the_layer: "la couche des arrêts de bus",
        names: &["Arrêts de bus", "Arrêts bus", "Points d'arrêt des bus"],
        slugs: &["arrets_bus", "arrets", "arret_bus"],
        files: &["arrets_bus", "arrets", "points_arret"],
        geom: Geom::Point,
        fields: &[
            NOM,
            FieldDef {
                role: "ligne",
                names: &["ligne", "num_ligne"],
                label: "de ligne",
                kind: Kind::Text {
                    values: &[
                        v("12"),
                        v("34"),
                        v("103"),
                        v("183"),
                        v("TVM"),
                        v("N131"),
                        v("7"),
                    ],
                    phrases: &[
                        "de la ligne {v}",
                        "du bus {v}",
                        "desservis par la ligne {v}",
                    ],
                },
            },
        ],
    },
    Theme {
        key: "routes",
        group: "voirie",
        domain: "réseaux",
        noun: Noun {
            sg: "route",
            pl: "routes",
            fem: true,
        },
        of_none: "de routes",
        the_layer: "la couche des routes",
        names: &["Routes", "Tronçons de route", "Routes principales"],
        slugs: &["routes", "route", "troncons_route"],
        files: &["routes", "troncons_route", "routes_principales"],
        geom: Geom::Line,
        fields: &[
            FieldDef {
                role: "nom",
                names: NAME_FIELDS,
                label: "de nom",
                kind: Kind::Name {
                    fragments: &["Avenue", "Rue", "Boulevard", "Chemin", "Route de", "Quai"],
                },
            },
            FieldDef {
                role: "classe",
                names: &["classe", "importance", "type_route"],
                label: "de classe",
                kind: Kind::Text {
                    values: &[
                        v("autoroute"),
                        va("nationale", "nationale", "nationales"),
                        va("départementale", "départementale", "départementales"),
                        va("communale", "communale", "communales"),
                    ],
                    phrases: &["de classe {v}", "dont la classe est « {v} »"],
                },
            },
            FieldDef {
                role: "vitesse",
                names: &["vitesse_kmh", "vitesse_max", "vmax"],
                label: "de vitesse",
                kind: number!(true, 30, 110, 10,
                    gt: &["où la vitesse dépasse {n} km/h", "limitées à plus de {n} km/h"],
                    lt: &["limitées à moins de {n} km/h"],
                    gte: &["limitées à {n} km/h ou plus"],
                    lte: &["limitées à {n} km/h au plus", "où la vitesse ne dépasse pas {n} km/h"]),
            },
        ],
    },
    Theme {
        key: "reseau",
        group: "voirie",
        domain: "réseaux",
        noun: Noun {
            sg: "tronçon du réseau routier",
            pl: "tronçons du réseau routier",
            fem: false,
        },
        of_none: "de réseau routier",
        the_layer: "la couche du réseau routier",
        names: &["Réseau routier", "Réseau", "Voirie", "Réseau de voirie"],
        slugs: &["reseau_routier", "reseau", "voirie"],
        files: &["reseau_routier", "voirie", "reseau"],
        geom: Geom::Line,
        fields: &[],
    },
    Theme {
        key: "pistes",
        group: "pistes",
        domain: "réseaux",
        noun: Noun {
            sg: "piste cyclable",
            pl: "pistes cyclables",
            fem: true,
        },
        of_none: "de pistes cyclables",
        the_layer: "la couche des pistes cyclables",
        names: &[
            "Pistes cyclables",
            "Aménagements cyclables",
            "Réseau cyclable",
        ],
        slugs: &["pistes_cyclables", "amenagements_cyclables"],
        files: &["pistes_cyclables", "amenagements_cyclables"],
        geom: Geom::Line,
        fields: &[FieldDef {
            role: "type",
            names: &["type", "type_amenagement"],
            label: "de type",
            kind: Kind::Text {
                values: &[v("bande"), v("piste"), v("voie verte"), v("double sens")],
                phrases: &["de type {v}", "aménagées en {v}"],
            },
        }],
    },
    Theme {
        key: "lignes_elec",
        group: "lignes_elec",
        domain: "réseaux",
        noun: Noun {
            sg: "ligne électrique",
            pl: "lignes électriques",
            fem: true,
        },
        of_none: "de lignes électriques",
        the_layer: "la couche des lignes électriques",
        names: &[
            "Lignes électriques",
            "Réseau électrique",
            "Lignes haute tension",
        ],
        slugs: &["lignes_electriques", "reseau_elec", "lignes_ht"],
        files: &["lignes_electriques", "reseau_electrique"],
        geom: Geom::Line,
        fields: &[FieldDef {
            role: "tension",
            names: &["tension", "niveau_tension"],
            label: "de tension",
            kind: Kind::Text {
                values: &[v("HTA"), v("HTB"), v("BT")],
                phrases: &["de tension {v}", "en {v}"],
            },
        }],
    },
    Theme {
        key: "canalisations",
        group: "canalisations",
        domain: "réseaux",
        noun: Noun {
            sg: "canalisation",
            pl: "canalisations",
            fem: true,
        },
        of_none: "de canalisations",
        the_layer: "la couche des canalisations",
        names: &[
            "Canalisations d'eau potable",
            "Réseau d'eau potable",
            "Canalisations",
        ],
        slugs: &["canalisations_aep", "reseau_aep", "canalisations"],
        files: &["canalisations_aep", "reseau_eau_potable"],
        geom: Geom::Line,
        fields: &[
            FieldDef {
                role: "materiau",
                names: &["materiau", "matiere"],
                label: "de matériau",
                kind: Kind::Text {
                    values: &[
                        v("fonte"),
                        v("PVC"),
                        v("acier"),
                        v("PEHD"),
                        v("amiante-ciment"),
                    ],
                    phrases: &["en {v}", "dont le matériau est {v}"],
                },
            },
            FieldDef {
                role: "diametre",
                names: &["diametre_mm", "dn"],
                label: "de diamètre",
                kind: number!(true, 60, 600, 20,
                    gt: &["d'un diamètre supérieur à {n} mm", "de plus de {n} mm de diamètre"],
                    lt: &["d'un diamètre inférieur à {n} mm"],
                    gte: &["d'au moins {n} mm de diamètre"],
                    lte: &["d'au plus {n} mm de diamètre"]),
            },
        ],
    },
    Theme {
        key: "rivieres",
        group: "rivieres",
        domain: "environnement",
        noun: Noun {
            sg: "rivière",
            pl: "rivières",
            fem: true,
        },
        of_none: "de rivières",
        the_layer: "la couche des rivières",
        names: &[
            "Rivières",
            "Rivières et ruisseaux",
            "Hydrographie - rivières",
        ],
        slugs: &["rivieres", "riviere", "hydro_rivieres"],
        files: &["rivieres", "hydrographie_rivieres"],
        geom: Geom::Line,
        fields: &[
            FieldDef {
                role: "nom",
                names: NAME_FIELDS,
                label: "de nom",
                kind: Kind::Name {
                    fragments: &["Seine", "Yerres", "Orge", "ru de", "Marne", "Bièvre"],
                },
            },
            FieldDef {
                role: "regime",
                names: &["regime", "persistance"],
                label: "de régime",
                kind: Kind::Text {
                    values: &[
                        va("permanent", "permanente", "permanentes"),
                        va("intermittent", "intermittente", "intermittentes"),
                    ],
                    phrases: &["à écoulement {v}", "dont le régime est {v}"],
                },
            },
        ],
    },
    Theme {
        key: "zones_humides",
        group: "zones_humides",
        domain: "environnement",
        noun: Noun {
            sg: "zone humide",
            pl: "zones humides",
            fem: true,
        },
        of_none: "de zones humides",
        the_layer: "la couche des zones humides",
        names: &[
            "Zones humides",
            "Inventaire des zones humides",
            "Zones humides avérées",
        ],
        slugs: &["zones_humides", "zh_inventaire"],
        files: &["zones_humides", "inventaire_zh"],
        geom: Geom::Polygon,
        fields: &[FieldDef {
            role: "type",
            names: &["type", "typologie"],
            label: "de type",
            kind: Kind::Text {
                values: &[
                    v("mare"),
                    v("prairie humide"),
                    v("roselière"),
                    v("tourbière"),
                    v("boisement humide"),
                ],
                phrases: &["de type {v}", "classées en {v}"],
            },
        }],
    },
    Theme {
        key: "inondables",
        group: "inondables",
        domain: "environnement",
        noun: Noun {
            sg: "zone inondable",
            pl: "zones inondables",
            fem: true,
        },
        of_none: "de zones inondables",
        the_layer: "la couche des zones inondables",
        names: &[
            "Zones inondables",
            "PPRI - zones inondables",
            "Aléa inondation",
        ],
        slugs: &["zones_inondables", "ppri_zonage", "alea_inondation"],
        files: &["zones_inondables", "ppri", "alea_inondation"],
        geom: Geom::Polygon,
        fields: &[FieldDef {
            role: "alea",
            names: &["alea", "niveau_alea", "classe_alea"],
            label: "d'aléa",
            kind: Kind::Text {
                values: &[v("très fort"), v("fort"), v("moyen"), v("faible")],
                phrases: &["d'aléa {v}", "à aléa {v}", "où l'aléa est {v}"],
            },
        }],
    },
    Theme {
        key: "znieff",
        group: "znieff",
        domain: "environnement",
        noun: Noun {
            sg: "ZNIEFF",
            pl: "ZNIEFF",
            fem: true,
        },
        of_none: "de ZNIEFF",
        the_layer: "la couche des ZNIEFF",
        names: &[
            "ZNIEFF",
            "ZNIEFF de type 1 et 2",
            "Zones naturelles d'intérêt écologique (ZNIEFF)",
        ],
        slugs: &["znieff", "znieff_1_2"],
        files: &["znieff", "znieff_type1_type2"],
        geom: Geom::Polygon,
        fields: &[FieldDef {
            role: "type",
            names: &["type", "type_znieff"],
            label: "de type",
            kind: Kind::Text {
                values: &[v("1"), v("2")],
                phrases: &["de type {v}"],
            },
        }],
    },
    Theme {
        key: "captages",
        group: "captages",
        domain: "environnement",
        noun: Noun {
            sg: "captage d'eau potable",
            pl: "captages d'eau potable",
            fem: false,
        },
        of_none: "de captages d'eau potable",
        the_layer: "la couche des captages",
        names: &[
            "Captages d'eau potable",
            "Points de captage AEP",
            "Captages",
        ],
        slugs: &["captages_aep", "captages", "points_captage"],
        files: &["captages_aep", "captages"],
        geom: Geom::Point,
        fields: &[FieldDef {
            role: "etat",
            names: &["etat", "statut"],
            label: "d'état",
            kind: Kind::Text {
                values: &[
                    va("actif", "actif", "actifs"),
                    va("abandonné", "abandonné", "abandonnés"),
                    v("en projet"),
                ],
                phrases: &["dont l'état est « {v} »", "à l'état {v}"],
            },
        }],
    },
    Theme {
        key: "forets",
        group: "forets",
        domain: "environnement",
        noun: Noun {
            sg: "forêt",
            pl: "forêts",
            fem: true,
        },
        of_none: "de forêts",
        the_layer: "la couche des forêts",
        names: &["Forêts", "Massifs forestiers", "Forêts et boisements"],
        slugs: &["forets", "massifs_forestiers"],
        files: &["forets", "massifs_forestiers"],
        geom: Geom::Polygon,
        fields: &[FieldDef {
            role: "essence",
            names: &["essence", "essence_dominante"],
            label: "d'essence",
            kind: Kind::Text {
                values: &[
                    v("chêne"),
                    v("hêtre"),
                    v("pin"),
                    v("châtaignier"),
                    v("mixte"),
                ],
                phrases: &["à dominante de {v}", "dont l'essence dominante est « {v} »"],
            },
        }],
    },
    Theme {
        key: "espaces_verts",
        group: "espaces_verts",
        domain: "urbanisme",
        noun: Noun {
            sg: "espace vert",
            pl: "espaces verts",
            fem: false,
        },
        of_none: "d'espaces verts",
        the_layer: "la couche des espaces verts",
        names: &["Espaces verts", "Parcs et jardins", "Espaces verts publics"],
        slugs: &["espaces_verts", "parcs_jardins"],
        files: &["espaces_verts", "parcs_jardins"],
        geom: Geom::Polygon,
        fields: &[FieldDef {
            role: "type",
            names: &["type", "categorie"],
            label: "de type",
            kind: Kind::Text {
                values: &[v("parc"), v("square"), v("jardin partagé"), v("cimetière")],
                phrases: &["de type {v}", "classés {v}"],
            },
        }],
    },
    Theme {
        key: "ecoles",
        group: "ecoles",
        domain: "équipements",
        noun: Noun {
            sg: "école",
            pl: "écoles",
            fem: true,
        },
        of_none: "d'écoles",
        the_layer: "la couche des écoles",
        names: &["Écoles", "Établissements scolaires", "Écoles publiques"],
        slugs: &["ecoles", "ecole", "etab_scolaires"],
        files: &["ecoles", "etablissements_scolaires"],
        geom: Geom::Point,
        fields: &[
            FieldDef {
                role: "nom",
                names: NAME_FIELDS,
                label: "de nom",
                kind: Kind::Name {
                    fragments: &[
                        "Jules Ferry",
                        "Jean Jaurès",
                        "Victor Hugo",
                        "Pasteur",
                        "Curie",
                        "Prévert",
                    ],
                },
            },
            FieldDef {
                role: "type",
                names: &["type", "nature", "type_etab"],
                label: "de type d'établissement",
                kind: Kind::Text {
                    values: &[
                        va("maternelle", "maternelle", "maternelles"),
                        va("élémentaire", "élémentaire", "élémentaires"),
                        v("collège"),
                        v("lycée"),
                    ],
                    phrases: &["de type {v}", "dont le type est « {v} »"],
                },
            },
        ],
    },
    Theme {
        key: "hopitaux",
        group: "hopitaux",
        domain: "équipements",
        noun: Noun {
            sg: "hôpital",
            pl: "hôpitaux",
            fem: false,
        },
        of_none: "d'hôpitaux",
        the_layer: "la couche des hôpitaux",
        names: &[
            "Hôpitaux",
            "Établissements hospitaliers",
            "Hôpitaux et cliniques",
        ],
        slugs: &["hopitaux", "etab_hospitaliers"],
        files: &["hopitaux", "etablissements_hospitaliers"],
        geom: Geom::Point,
        fields: &[FieldDef {
            role: "statut",
            names: &["statut", "secteur"],
            label: "de statut",
            kind: Kind::Text {
                values: &[
                    va("public", "public", "publics"),
                    va("privé", "privé", "privés"),
                ],
                phrases: &["du secteur {v}", "de statut {v}"],
            },
        }],
    },
    Theme {
        key: "pharmacies",
        group: "pharmacies",
        domain: "équipements",
        noun: Noun {
            sg: "pharmacie",
            pl: "pharmacies",
            fem: true,
        },
        of_none: "de pharmacies",
        the_layer: "la couche des pharmacies",
        names: &["Pharmacies", "Officines de pharmacie"],
        slugs: &["pharmacies", "officines"],
        files: &["pharmacies", "officines"],
        geom: Geom::Point,
        fields: &[NOM],
    },
    Theme {
        key: "commerces",
        group: "commerces",
        domain: "équipements",
        noun: Noun {
            sg: "commerce",
            pl: "commerces",
            fem: false,
        },
        of_none: "de commerces",
        the_layer: "la couche des commerces",
        names: &["Commerces", "Commerces et services", "Locaux commerciaux"],
        slugs: &["commerces", "commerces_services"],
        files: &["commerces", "base_commerces"],
        geom: Geom::Point,
        fields: &[FieldDef {
            role: "categorie",
            names: &["categorie", "activite"],
            label: "de catégorie",
            kind: Kind::Text {
                values: &[
                    v("alimentation"),
                    v("restauration"),
                    v("santé"),
                    v("services"),
                    v("équipement de la maison"),
                ],
                phrases: &["de la catégorie {v}", "dont l'activité est « {v} »"],
            },
        }],
    },
    Theme {
        key: "parkings",
        group: "parkings",
        domain: "réseaux",
        noun: Noun {
            sg: "parking",
            pl: "parkings",
            fem: false,
        },
        of_none: "de parkings",
        the_layer: "la couche des parkings",
        names: &["Parkings", "Parcs de stationnement"],
        slugs: &["parkings", "stationnement"],
        files: &["parkings", "parcs_stationnement"],
        geom: Geom::Point,
        fields: &[
            FieldDef {
                role: "type",
                names: &["type", "type_parc"],
                label: "de type",
                kind: Kind::Text {
                    values: &[
                        va("souterrain", "souterrain", "souterrains"),
                        va("aérien", "aérien", "aériens"),
                        v("relais"),
                    ],
                    phrases: &["de type {v}"],
                },
            },
            FieldDef {
                role: "capacite",
                names: &["capacite", "nb_places"],
                label: "de capacité",
                kind: number!(true, 20, 800, 10,
                    gt: &["de plus de {n} places"],
                    lt: &["de moins de {n} places"],
                    gte: &["d'au moins {n} places"],
                    lte: &["d'au plus {n} places"]),
            },
        ],
    },
    Theme {
        key: "equipements_sportifs",
        group: "equipements_sportifs",
        domain: "équipements",
        noun: Noun {
            sg: "équipement sportif",
            pl: "équipements sportifs",
            fem: false,
        },
        of_none: "d'équipements sportifs",
        the_layer: "la couche des équipements sportifs",
        names: &["Équipements sportifs", "Installations sportives"],
        slugs: &["equipements_sportifs", "installations_sportives"],
        files: &["equipements_sportifs", "res_equipements"],
        geom: Geom::Point,
        fields: &[FieldDef {
            role: "type",
            names: &["type", "type_equip"],
            label: "de type",
            kind: Kind::Text {
                values: &[
                    v("gymnase"),
                    v("stade"),
                    v("piscine"),
                    v("terrain de tennis"),
                ],
                phrases: &["de type {v}"],
            },
        }],
    },
    Theme {
        key: "arbres",
        group: "arbres",
        domain: "environnement",
        noun: Noun {
            sg: "arbre",
            pl: "arbres",
            fem: false,
        },
        of_none: "d'arbres",
        the_layer: "la couche des arbres",
        names: &["Arbres d'alignement", "Patrimoine arboré", "Arbres"],
        slugs: &["arbres", "arbres_alignement"],
        files: &["arbres", "arbres_alignement"],
        geom: Geom::Point,
        fields: &[
            FieldDef {
                role: "essence",
                names: &["essence", "genre"],
                label: "d'essence",
                kind: Kind::Text {
                    values: &[
                        v("platane"),
                        v("tilleul"),
                        v("chêne"),
                        v("érable"),
                        v("marronnier"),
                    ],
                    phrases: &["dont l'essence est « {v} »", "de l'essence {v}"],
                },
            },
            FieldDef {
                role: "hauteur",
                names: &["hauteur_m", "hauteur"],
                label: "de hauteur",
                kind: number!(false, 2, 30, 1,
                    gt: &["de plus de {n} m de haut"],
                    lt: &["de moins de {n} m de haut"],
                    gte: &["d'au moins {n} m de hauteur"],
                    lte: &["d'au plus {n} m de haut"]),
            },
        ],
    },
    // Analyse après 11.23 : une intersection avec une couche de zones entière (« toutes zones
    // confondues ») n'avait aucun thème de zones d'exposition au bruit en vecteur ; le thème
    // `bruit` est un raster, d'un autre groupe. Groupe du zonage du PLU : leurs zones A se
    // confondraient (« en zone A »).
    Theme {
        key: "zones_bruit",
        group: "zonage",
        domain: "environnement",
        noun: Noun {
            sg: "zone d'exposition au bruit",
            pl: "zones d'exposition au bruit",
            fem: true,
        },
        of_none: "de zones d'exposition au bruit",
        the_layer: "la couche des zones d'exposition au bruit",
        names: &[
            "Zones d'exposition au bruit",
            "Plan d'exposition au bruit",
            "PEB - zones de bruit",
        ],
        slugs: &["zones_bruit", "peb_zonage", "exposition_bruit"],
        files: &["zones_bruit", "peb", "plan_exposition_bruit"],
        geom: Geom::Polygon,
        fields: &[FieldDef {
            role: "zone",
            names: &["zone", "zone_peb", "classe"],
            label: "de zone",
            kind: Kind::Text {
                values: &[v("A"), v("B"), v("C"), v("D")],
                phrases: &["de zone {v}", "classées en zone {v}"],
            },
        }],
    },
    // Constat durable après 11.23 : une cible dite par une catégorie d'une couche plus large
    // (des écoles parmi des équipements). Même groupe que `ecoles` : « les écoles » ne doit
    // désigner qu'une couche.
    Theme {
        key: "equipements_publics",
        group: "ecoles",
        domain: "équipements",
        noun: Noun {
            sg: "équipement public",
            pl: "équipements publics",
            fem: false,
        },
        of_none: "d'équipements publics",
        the_layer: "la couche des équipements publics",
        names: &[
            "Équipements publics",
            "Base des équipements",
            "Équipements de la commune",
        ],
        slugs: &["equipements_publics", "equip_publics", "bpe_equipements"],
        files: &["equipements_publics", "bpe", "equipements_commune"],
        geom: Geom::Point,
        fields: &[
            FieldDef {
                role: "categorie",
                names: &["categorie", "type_equip", "nature"],
                label: "de catégorie",
                kind: Kind::Text {
                    values: &[
                        v("crèche"),
                        v("médiathèque"),
                        v("mairie"),
                        v("poste"),
                        v("école"),
                        v("salle polyvalente"),
                    ],
                    phrases: &["de catégorie {v}", "dont la catégorie est « {v} »"],
                },
            },
            NOM,
        ],
    },
    Theme {
        key: "mnt",
        group: "mnt",
        domain: "général",
        noun: Noun {
            sg: "modèle numérique de terrain",
            pl: "modèles numériques de terrain",
            fem: false,
        },
        of_none: "de modèle numérique de terrain",
        the_layer: "la couche du MNT",
        names: &["Modèle numérique de terrain 5 m", "MNT", "MNT 1 m"],
        slugs: &["mnt_5m", "mnt", "mnt_1m"],
        files: &["mnt_5m", "mnt", "rge_alti_1m"],
        geom: Geom::Raster,
        fields: &[],
    },
    // Raster de coût de déplacement (11.12, lot 2b-1) : la `cost` de `terrain.cost_distance`
    // et de `terrain.least_cost_path`. Un raster apporté par l'utilisateur, jamais un dérivé du
    // MNT : ni son nom ni son fichier ne parlent de pente, d'altitude ou de relief.
    Theme {
        key: "cout",
        group: "cout",
        domain: "environnement",
        noun: Noun {
            sg: "raster de coût",
            pl: "rasters de coût",
            fem: false,
        },
        of_none: "de raster de coût",
        the_layer: "la couche du raster de coût",
        names: &["Coût de déplacement", "Friction", "Coût de traversée"],
        slugs: &["cout_deplacement", "friction", "cout_traversee"],
        files: &["cout_deplacement", "friction", "cout_traversee"],
        geom: Geom::Raster,
        fields: &[],
    },
    // Critères notés, masques et rasters bruts (11.12, lot 2b-2) : les `criteria` et les
    // `exclusions` de `raster.weighted_overlay`, le `raster` de `raster.reclassify`. Des rasters
    // apportés par l'utilisateur, jamais des dérivés du MNT : ni leur nom ni leur fichier ne
    // parlent de pente, d'altitude, de relief ou de coût. Un masque partage le groupe de la
    // couche vecteur de même sujet : les deux ne se rencontrent jamais dans un inventaire.
    Theme {
        key: "accessibilite",
        group: "accessibilite",
        domain: "urbanisme",
        noun: Noun {
            sg: "note d'accessibilité",
            pl: "notes d'accessibilité",
            fem: true,
        },
        of_none: "d'accessibilité",
        the_layer: "la couche d'accessibilité",
        names: &[
            "Accessibilité",
            "Note d'accessibilité",
            "Indice d'accessibilité",
        ],
        slugs: &[
            "accessibilite",
            "note_accessibilite",
            "indice_accessibilite",
        ],
        files: &[
            "accessibilite",
            "note_accessibilite",
            "indice_accessibilite",
        ],
        geom: Geom::Raster,
        fields: &[],
    },
    Theme {
        key: "ensoleillement",
        group: "ensoleillement",
        domain: "environnement",
        noun: Noun {
            sg: "note d'ensoleillement",
            pl: "notes d'ensoleillement",
            fem: true,
        },
        of_none: "d'ensoleillement",
        the_layer: "la couche d'ensoleillement",
        names: &[
            "Ensoleillement",
            "Note d'ensoleillement",
            "Potentiel solaire",
        ],
        slugs: &["ensoleillement", "note_ensoleillement", "potentiel_solaire"],
        files: &["ensoleillement", "note_ensoleillement", "potentiel_solaire"],
        geom: Geom::Raster,
        fields: &[],
    },
    Theme {
        key: "proximite_services",
        group: "proximite_services",
        domain: "urbanisme",
        noun: Noun {
            sg: "note de proximité des services",
            pl: "notes de proximité des services",
            fem: true,
        },
        of_none: "de proximité des services",
        the_layer: "la couche de proximité des services",
        names: &[
            "Proximité des services",
            "Accès aux services",
            "Note de proximité des services",
        ],
        slugs: &["proximite_services", "acces_services", "note_services"],
        files: &["proximite_services", "acces_services", "note_services"],
        geom: Geom::Raster,
        fields: &[],
    },
    Theme {
        key: "calme",
        group: "calme",
        domain: "environnement",
        noun: Noun {
            sg: "note de calme",
            pl: "notes de calme",
            fem: true,
        },
        of_none: "de calme",
        the_layer: "la couche de calme",
        names: &["Calme", "Note de calme", "Tranquillité"],
        slugs: &["calme", "note_calme", "tranquillite"],
        files: &["calme", "note_calme", "tranquillite"],
        geom: Geom::Raster,
        fields: &[],
    },
    Theme {
        key: "masque_inondable",
        group: "inondables",
        domain: "environnement",
        noun: Noun {
            sg: "masque des zones inondables",
            pl: "masques des zones inondables",
            fem: false,
        },
        of_none: "de masque des zones inondables",
        the_layer: "la couche du masque des zones inondables",
        names: &[
            "Masque des zones inondables",
            "Zones inondables (masque)",
            "Exclusion inondation",
        ],
        slugs: &["masque_inondable", "zi_masque", "exclusion_inondation"],
        files: &["masque_inondable", "zi_masque", "exclusion_inondation"],
        geom: Geom::Raster,
        fields: &[],
    },
    Theme {
        key: "masque_protege",
        group: "znieff",
        domain: "environnement",
        noun: Noun {
            sg: "masque des espaces protégés",
            pl: "masques des espaces protégés",
            fem: false,
        },
        of_none: "de masque des espaces protégés",
        the_layer: "la couche du masque des espaces protégés",
        names: &[
            "Masque des espaces protégés",
            "Espaces protégés (masque)",
            "Exclusion espaces naturels",
        ],
        slugs: &["masque_protege", "espaces_proteges", "exclusion_naturel"],
        files: &["masque_protege", "espaces_proteges", "exclusion_naturel"],
        geom: Geom::Raster,
        fields: &[],
    },
    Theme {
        key: "distance_routes",
        group: "voirie",
        domain: "réseaux",
        noun: Noun {
            sg: "raster de distance aux routes",
            pl: "rasters de distance aux routes",
            fem: false,
        },
        of_none: "de distance aux routes",
        the_layer: "la couche de distance aux routes",
        names: &[
            "Distance aux routes",
            "Éloignement des routes",
            "Distance à la voirie",
        ],
        slugs: &["distance_routes", "dist_routes", "distance_voirie"],
        files: &["distance_routes", "dist_routes", "distance_voirie"],
        geom: Geom::Raster,
        fields: &[],
    },
    Theme {
        key: "bruit",
        group: "bruit",
        domain: "environnement",
        noun: Noun {
            sg: "carte de bruit",
            pl: "cartes de bruit",
            fem: true,
        },
        of_none: "de bruit",
        the_layer: "la couche de bruit",
        names: &["Bruit routier", "Carte de bruit", "Niveau sonore Lden"],
        slugs: &["bruit_routier", "carte_bruit", "bruit_lden"],
        files: &["bruit_routier", "carte_bruit", "bruit_lden"],
        geom: Geom::Raster,
        fields: &[],
    },
];

/// Catégories du champ `categorie` d'`equipements_publics`, chacune dite comme un nom commun
/// (« les écoles », « les bureaux de poste ») : valeur du champ, nom.
pub static CATEGORIES: &[(&str, Noun)] = &[
    (
        "crèche",
        Noun {
            sg: "crèche",
            pl: "crèches",
            fem: true,
        },
    ),
    (
        "médiathèque",
        Noun {
            sg: "médiathèque",
            pl: "médiathèques",
            fem: true,
        },
    ),
    (
        "mairie",
        Noun {
            sg: "mairie",
            pl: "mairies",
            fem: true,
        },
    ),
    (
        "poste",
        Noun {
            sg: "bureau de poste",
            pl: "bureaux de poste",
            fem: false,
        },
    ),
    (
        "école",
        Noun {
            sg: "école",
            pl: "écoles",
            fem: true,
        },
    ),
    (
        "salle polyvalente",
        Noun {
            sg: "salle polyvalente",
            pl: "salles polyvalentes",
            fem: true,
        },
    ),
];

/// Objets désignés par leur nom propre (constat durable après 11.23 : une référence dite par
/// son nom perdait sa sélection) : thème, tournure « de ... » qui précède le nom, et noms
/// possibles, rendus comme valeurs du champ de nom de la couche.
pub static NAMED_PLACES: &[(&str, &str, &[Val])] = &[
    (
        "gares",
        "de la gare",
        &[
            v("Les Ardoines"),
            v("Pont de Rungis"),
            v("Vert de Maisons"),
            v("Les Saules"),
            v("Val de Fontenay"),
        ],
    ),
    (
        "arrets",
        "de l'arrêt",
        &[
            v("Les Tilleuls"),
            v("Marché"),
            v("Place de l'Église"),
            v("Collège"),
            v("Les Peupliers"),
        ],
    ),
    (
        "ecoles",
        "de l'école",
        &[
            v("Jules Ferry"),
            v("Paul Bert"),
            v("Louise Michel"),
            v("Jean Macé"),
            v("Anatole France"),
        ],
    ),
    (
        "quartiers",
        "du quartier",
        &[
            v("Les Navigateurs"),
            v("Centre-Ville"),
            v("Les Hautes Bornes"),
            v("La Pierre-au-Prêtre"),
        ],
    ),
];

/// Critères notés de `raster.weighted_overlay` (11.12, lot 2b-2).
pub const CRITERIA: [&str; 4] = [
    "accessibilite",
    "ensoleillement",
    "proximite_services",
    "calme",
];
/// Masques d'exclusion de `raster.weighted_overlay` : chacun dans le groupe de son vecteur.
pub const MASKS: [&str; 2] = ["masque_inondable", "masque_protege"];
/// Rasters bruts que `raster.reclassify` classe directement, sans étape de terrain.
pub const RAW_RASTERS: [&str; 2] = ["distance_routes", "bruit"];

/// Le thème de clé `key` (panique si inconnu : c'est une erreur de programmation).
pub fn theme(key: &str) -> &'static Theme {
    THEMES
        .iter()
        .find(|t| t.key == key)
        .unwrap_or_else(|| panic!("thème inconnu : {key}"))
}

/// Un champ tel qu'il apparaît dans l'inventaire.
#[derive(Debug, Clone)]
pub struct FieldInst {
    pub def: &'static FieldDef,
    pub name: String,
    pub values: Vec<&'static Val>,
}

/// Une couche tirée.
#[derive(Debug, Clone)]
pub struct Layer {
    pub theme: &'static Theme,
    pub id: String,
    pub name: String,
    pub source: String,
    pub epsg: u32,
    pub count: u64,
    pub geometry: Value,
    pub fields: Vec<FieldInst>,
}

impl Layer {
    pub fn field(&self, role: &str) -> Option<&FieldInst> {
        self.fields.iter().find(|f| f.def.role == role)
    }

    fn to_json(&self) -> Value {
        let fields: Vec<Value> = self
            .fields
            .iter()
            .map(|f| {
                let ty = match f.def.kind {
                    Kind::Number { int: true, .. } => "integer",
                    Kind::Number { int: false, .. } => "number",
                    _ => "string",
                };
                if f.values.is_empty() {
                    json!({"name": f.name, "type": ty})
                } else {
                    let values: Vec<&str> = f.values.iter().map(|v| v.v).collect();
                    json!({"name": f.name, "type": ty, "values": values})
                }
            })
            .collect();
        let kind = if self.theme.geom == Geom::Raster {
            "raster"
        } else {
            "vector"
        };
        json!({
            "layer_id": self.id,
            "name": self.name,
            "kind": kind,
            "geometry": self.geometry,
            "epsg": self.epsg,
            "feature_count": self.count,
            "source": self.source,
            "fields": fields,
        })
    }
}

/// Leurres d'un champ de nom (après les quatre causes, 2026-11-10 : à Annecy, `nom_officiel`
/// voisinait avec `nom_officiel_en_majuscules` et `code_insee`) : sa version en majuscules, un
/// code, ou les deux. Le plan cite toujours le champ de nom lui-même.
fn decoys(rng: &mut Rng, t: &Theme, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let both = rng.chance(1, 3);
    if both || rng.chance(1, 2) {
        out.push(match name {
            "nom_officiel" => "nom_officiel_en_majuscules".to_string(),
            "NOM" => "NOM_M".to_string(),
            "nom" => "nom_m".to_string(),
            other => format!("{other}_maj"),
        });
    }
    if both || out.is_empty() {
        // Jamais le nom d'un champ du thème (le code INSEE des communes en est un), casse
        // comprise : un GeoPackage ne distingue pas `INSEE_COM` d'`insee_com`.
        let codes: Vec<&str> = ["code_postal", "cleabs", "code", "id_source", "identifiant"]
            .into_iter()
            .filter(|c| {
                !t.fields
                    .iter()
                    .any(|f| f.names.iter().any(|n| n.eq_ignore_ascii_case(c)))
            })
            .collect();
        out.push((*rng.pick(&codes)).to_string());
    }
    out
}

/// Champs de remplissage des inventaires larges (2026-11-10), par type rendu. Rôle à part
/// (`remplissage`), distinct des leurres du nom : aucune famille ne les demande, un plan ne
/// les cite jamais.
static FILL_TEXT: FieldDef = FieldDef {
    role: "remplissage",
    names: &[],
    label: "",
    kind: Kind::Name { fragments: &[] },
};
static FILL_INT: FieldDef = FieldDef {
    role: "remplissage",
    names: &[],
    label: "",
    kind: Kind::Number {
        int: true,
        min: 0,
        max: 0,
        step: 1,
        gt: &[],
        lt: &[],
        gte: &[],
        lte: &[],
    },
};
static FILL_NUM: FieldDef = FieldDef {
    role: "remplissage",
    names: &[],
    label: "",
    kind: Kind::Number {
        int: false,
        min: 0,
        max: 0,
        step: 1,
        gt: &[],
        lt: &[],
        gte: &[],
        lte: &[],
    },
};

type Fill = (&'static str, &'static FieldDef);

/// Champs de remplissage par thème, dans l'ordre de leur producteur (BD TOPO, cadastre et
/// fichiers fonciers, standard du CNIG) : à Annecy, la couche des communes portait 17 champs
/// et le modèle, qui n'en avait vu que quelques-uns par couche, rendait `nom` au lieu de
/// `nom_officiel` (nuit du 2026-11-10). Aucun ne double le sens d'un champ de rôle du thème
/// (ni nom, ni hauteur, ni nature de route, ni surface de parcelle, ni type de zone) : une
/// question « aucun champ de … » reste juste, et le plan ne cite que le champ de rôle.
const WIDE_COMMUNES: &[Fill] = &[
    ("cleabs", &FILL_TEXT),
    ("code_insee_du_departement", &FILL_TEXT),
    ("code_insee_de_la_region", &FILL_TEXT),
    ("code_postal", &FILL_TEXT),
    ("surface_en_ha", &FILL_INT),
    ("date_creation", &FILL_TEXT),
    ("date_modification", &FILL_TEXT),
    ("date_d_apparition", &FILL_TEXT),
    ("date_de_confirmation", &FILL_TEXT),
    ("codes_siren_des_epci", &FILL_TEXT),
    ("code_siren", &FILL_TEXT),
    ("chef_lieu_d_arrondissement", &FILL_TEXT),
    ("chef_lieu_de_collectivite_terr", &FILL_TEXT),
    ("chef_lieu_de_departement", &FILL_TEXT),
    ("chef_lieu_de_region", &FILL_TEXT),
    ("capitale_d_etat", &FILL_TEXT),
    ("date_du_recensement", &FILL_TEXT),
    ("organisme_recenseur", &FILL_TEXT),
    ("liens_vers_autorite_administrative", &FILL_TEXT),
];

const WIDE_ROUTES: &[Fill] = &[
    ("cleabs", &FILL_TEXT),
    ("fictif", &FILL_TEXT),
    ("position_par_rapport_au_sol", &FILL_TEXT),
    ("etat_de_l_objet", &FILL_TEXT),
    ("date_creation", &FILL_TEXT),
    ("date_modification", &FILL_TEXT),
    ("date_d_apparition", &FILL_TEXT),
    ("date_de_confirmation", &FILL_TEXT),
    ("sources", &FILL_TEXT),
    ("identifiants_sources", &FILL_TEXT),
    ("methode_d_acquisition_planimetrique", &FILL_TEXT),
    ("precision_planimetrique", &FILL_NUM),
    ("methode_d_acquisition_altimetrique", &FILL_TEXT),
    ("precision_altimetrique", &FILL_NUM),
    ("nombre_de_voies", &FILL_INT),
    ("largeur_de_chaussee", &FILL_NUM),
    ("itineraire_vert", &FILL_TEXT),
    ("prive", &FILL_TEXT),
    ("sens_de_circulation", &FILL_TEXT),
    ("acces_pieton", &FILL_TEXT),
    ("restriction_de_poids_total", &FILL_NUM),
    ("insee_commune_gauche", &FILL_TEXT),
    ("insee_commune_droite", &FILL_TEXT),
];

const WIDE_BATIMENTS: &[Fill] = &[
    ("cleabs", &FILL_TEXT),
    ("construction_legere", &FILL_TEXT),
    ("etat_de_l_objet", &FILL_TEXT),
    ("date_creation", &FILL_TEXT),
    ("date_modification", &FILL_TEXT),
    ("date_d_apparition", &FILL_TEXT),
    ("date_de_confirmation", &FILL_TEXT),
    ("sources", &FILL_TEXT),
    ("identifiants_sources", &FILL_TEXT),
    ("methode_d_acquisition_planimetrique", &FILL_TEXT),
    ("methode_d_acquisition_altimetrique", &FILL_TEXT),
    ("precision_planimetrique", &FILL_NUM),
    ("precision_altimetrique", &FILL_NUM),
    ("nombre_de_logements", &FILL_INT),
    ("materiaux_des_murs", &FILL_TEXT),
    ("materiaux_de_la_toiture", &FILL_TEXT),
    ("origine_du_batiment", &FILL_TEXT),
    ("appariement_fichiers_fonciers", &FILL_TEXT),
];

const WIDE_GARES: &[Fill] = &[
    ("cleabs", &FILL_TEXT),
    ("nature", &FILL_TEXT),
    ("nature_detaillee", &FILL_TEXT),
    ("fictif", &FILL_TEXT),
    ("etat_de_l_objet", &FILL_TEXT),
    ("date_creation", &FILL_TEXT),
    ("date_modification", &FILL_TEXT),
    ("date_d_apparition", &FILL_TEXT),
    ("date_de_confirmation", &FILL_TEXT),
    ("sources", &FILL_TEXT),
    ("identifiants_sources", &FILL_TEXT),
    ("methode_d_acquisition_planimetrique", &FILL_TEXT),
    ("precision_planimetrique", &FILL_NUM),
    ("code_uic", &FILL_TEXT),
    ("exploitant", &FILL_TEXT),
    ("gestionnaire", &FILL_TEXT),
    ("nombre_de_quais", &FILL_INT),
    ("frequentation_annuelle", &FILL_INT),
    ("insee_commune", &FILL_TEXT),
];

const WIDE_PARCELLES: &[Fill] = &[
    ("gid", &FILL_INT),
    ("commune", &FILL_TEXT),
    ("prefixe", &FILL_TEXT),
    ("numero", &FILL_TEXT),
    ("arpente", &FILL_TEXT),
    ("created", &FILL_TEXT),
    ("updated", &FILL_TEXT),
    ("ccodep", &FILL_TEXT),
    ("ccocom", &FILL_TEXT),
    ("ccopre", &FILL_TEXT),
    ("dnupla", &FILL_TEXT),
    ("jdatat", &FILL_TEXT),
    ("code_insee", &FILL_TEXT),
    ("idcom", &FILL_TEXT),
    ("cgrnum", &FILL_TEXT),
    ("typprop", &FILL_TEXT),
    ("nlocal", &FILL_INT),
    ("nlogh", &FILL_INT),
];

const WIDE_ZONAGE: &[Fill] = &[
    ("gid", &FILL_INT),
    ("partition", &FILL_TEXT),
    ("idurba", &FILL_TEXT),
    ("idzone", &FILL_TEXT),
    ("nomfic", &FILL_TEXT),
    ("urlfic", &FILL_TEXT),
    ("insee", &FILL_TEXT),
    ("datappro", &FILL_TEXT),
    ("datvalid", &FILL_TEXT),
    ("ident", &FILL_TEXT),
    ("date_de_mise_a_jour", &FILL_TEXT),
    ("gpu_doc_id", &FILL_TEXT),
    ("gpu_status", &FILL_TEXT),
    ("gpu_timestamp", &FILL_TEXT),
];

/// Les champs de remplissage d'un thème qui en a dans la réalité, `None` sinon.
pub fn wide_fields(key: &str) -> Option<&'static [Fill]> {
    Some(match key {
        "communes" => WIDE_COMMUNES,
        "routes" => WIDE_ROUTES,
        "batiments" => WIDE_BATIMENTS,
        "gares" => WIDE_GARES,
        "parcelles" => WIDE_PARCELLES,
        "zonage" => WIDE_ZONAGE,
        _ => return None,
    })
}

/// Part des couches élargies parmi celles des thèmes qui en ont (`WIDE_CHANCE` sur
/// `WIDE_OUT_OF`), et leur nombre au plus par inventaire : la longueur des exemples reste
/// loin des 2 048 jetons de l'entraînement.
const WIDE_CHANCE: u64 = 2;
const WIDE_OUT_OF: u64 = 5;
pub const MAX_WIDE_LAYERS: usize = 2;
/// Nombre de champs d'une couche élargie (bornes comprises), quand le thème en a assez.
pub const WIDE_MIN: usize = 15;
pub const WIDE_MAX: usize = 20;

/// Élargit les champs d'une couche à 15 à 20 champs : le remplissage garde l'ordre de son
/// producteur, chaque champ de rôle (suivi de ses leurres) s'y insère à une place tirée, et le
/// champ de nom dans la moitié du milieu, noyé parmi les autres. Aucun nom en double, casse
/// comprise (un GeoPackage ne distingue pas `INSEE` d'`insee`).
fn widen(rng: &mut Rng, fields: Vec<FieldInst>, fill: &'static [Fill]) -> Vec<FieldInst> {
    let mut groups: Vec<Vec<FieldInst>> = Vec::new();
    for f in fields {
        match groups.last_mut() {
            Some(g) if f.def.role == "leurre" => g.push(f),
            _ => groups.push(vec![f]),
        }
    }
    let have: usize = groups.iter().map(Vec::len).sum();
    let target = rng.range(WIDE_MIN as u64, WIDE_MAX as u64) as usize;
    let taken: Vec<String> = groups
        .iter()
        .flatten()
        .map(|f| f.name.to_lowercase())
        .collect();
    let mut pool: Vec<usize> = (0..fill.len())
        .filter(|&i| !taken.contains(&fill[i].0.to_lowercase()))
        .collect();
    rng.shuffle(&mut pool);
    pool.truncate(target.saturating_sub(have));
    pool.sort_unstable();
    let mut out: Vec<FieldInst> = pool
        .into_iter()
        .map(|i| FieldInst {
            def: fill[i].1,
            name: fill[i].0.to_string(),
            values: Vec::new(),
        })
        .collect();
    for g in groups {
        let n = out.len() as u64;
        let at = if g[0].def.role == "nom" {
            rng.range(n / 4, n - n / 4)
        } else {
            rng.range(0, n)
        } as usize;
        out.splice(at..at, g);
    }
    out
}

/// Un fichier non encore ouvert, présent dans `unloaded_files`.
#[derive(Debug, Clone)]
pub struct File {
    pub theme: &'static Theme,
    pub path: String,
}

const PROJECTS: &[&str] = &[
    "orly-urbanisme",
    "etude-zones-humides",
    "plu-ablon-sur-seine",
    "diagnostic-foncier-thiais",
    "reseaux-choisy",
    "atlas-communal",
    "projet-amenagement-gare",
    "trame-verte-et-bleue",
    "schema-cyclable",
    "plan-de-prevention-inondation",
    "equipements-publics-rungis",
    "etude-mobilite",
];

/// Projets de l'ouest de la France, en UTM 30N (à l'ouest du méridien de Greenwich).
const PROJECTS_WEST: &[&str] = &[
    "plu-rennes",
    "etude-littoral-brest",
    "atlas-communal-nantes",
    "zones-humides-quimper",
];

/// Projets de l'est de la France, en UTM 32N (à l'est du 6e méridien).
const PROJECTS_EAST: &[&str] = &[
    "plu-strasbourg",
    "atlas-communal-colmar",
    "etude-risques-nice",
    "trame-verte-mulhouse",
];

/// Les noms de projet tirés pour le fuseau UTM `utm` du projet (`project.utm_epsg`) : un projet
/// de l'ouest en 30N, de l'est en 32N, du Val-de-Marne sinon (31N).
pub fn project_names(utm: u32) -> &'static [&'static str] {
    match utm {
        32630 => PROJECTS_WEST,
        32632 => PROJECTS_EAST,
        _ => PROJECTS,
    }
}

/// Identifiant de couche à la manière de QGIS : préfixe et suffixe aléatoire.
fn layer_id(rng: &mut Rng, slug: &str) -> String {
    match rng.below(3) {
        0 => format!("{slug}_{}", rng.hex(4)),
        1 => format!("{slug}_{}", rng.hex(8)),
        _ => format!(
            "{slug}_{}_{}_{}_{}_{}",
            rng.hex(8),
            rng.hex(4),
            rng.hex(4),
            rng.hex(4),
            rng.hex(12)
        ),
    }
}

fn extension(rng: &mut Rng, geom: Geom) -> &'static str {
    if geom == Geom::Raster {
        return "tif";
    }
    // Une couche ponctuelle vient parfois d'un CSV à coordonnées (recette d'Annecy du
    // 2026-11-09 : gares en `.csv`), en degrés comme en mètres : l'extension ne dit pas le
    // système.
    if geom == Geom::Point && rng.chance(1, 8) {
        return "csv";
    }
    const EXTENSIONS: [&str; 6] = ["gpkg", "gpkg", "shp", "geojson", "gpkg", "parquet"];
    EXTENSIONS[rng.below(EXTENSIONS.len())]
}

fn geometry(rng: &mut Rng, geom: Geom) -> Value {
    match geom {
        Geom::Point => json!(rng.pick(&["Point", "Point", "MultiPoint"])),
        Geom::Line => json!(rng.pick(&["LineString", "MultiLineString"])),
        Geom::Polygon => json!(rng.pick(&["MultiPolygon", "Polygon"])),
        Geom::Raster => Value::Null,
    }
}

/// Constructeur d'inventaire : les couches que la famille exige, puis des couches de
/// distraction, d'un groupe différent de tout thème présent ou exclu.
#[derive(Debug, Default)]
pub struct Builder {
    layers: Vec<Layer>,
    files: Vec<File>,
    excluded: Vec<&'static str>,
    /// Rang des couches de distraction dans `layers` : aucun plan ne les cite.
    distractors: Vec<usize>,
    /// Rang des couches dont le plan recopie la `source` (le MNT d'une commande de terrain) :
    /// elle ne disparaît jamais, une couche temporaire n'a pas de fichier à recopier.
    sourced: Vec<usize>,
    /// Système des couches vecteur (et du projet) ; 2154 par défaut. Posé par une famille de
    /// reprojection de couche (10.7), dont le plan dépend du système des couches.
    vector_epsg: Option<u32>,
    /// Vrai quand le plan dépend de l'`epsg` des couches (10.7) : aucune couche ni le projet ne
    /// le perdent dans les formes de QGIS (les tirages ont lieu quand même).
    keep_epsg: bool,
    /// Fuseau UTM du projet (`project.utm_epsg`) fixé par la famille dont le plan en dépend
    /// (le carroyage UTM sans fuseau écrit) : `Some(Some(code))` présent, `Some(None)` absent.
    /// `None` : tiré à la fin, et parfois retiré comme l'`epsg`.
    utm: Option<Option<u32>>,
    /// Système du projet quand il diffère de celui des couches vecteur : un projet en mètres
    /// (Lambert-93 ou UTM) dont toutes les couches sont en degrés. `None` : celui des couches.
    project_epsg: Option<u32>,
    /// Couches de distraction de plus à la fin (inventaires longs, comme celui d'Annecy).
    extra_distractors: usize,
    /// Couches élargies à 15 à 20 champs (`widen`), au plus `MAX_WIDE_LAYERS`.
    wide_layers: usize,
    /// La prochaine couche d'un thème qui en a est élargie (`wide_next`).
    force_wide: bool,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Exclut un thème (et son groupe) de l'inventaire : ni couche, ni fichier.
    pub fn exclude(&mut self, key: &str) {
        self.excluded.push(theme(key).group);
    }

    fn taken(&self, t: &Theme) -> bool {
        self.excluded.contains(&t.group)
            || self.layers.iter().any(|l| l.theme.group == t.group)
            || self.files.iter().any(|f| f.theme.group == t.group)
    }

    /// Pose le système des couches vecteur, déjà tirées ou à venir (distractions comprises), et
    /// celui du projet ; garde l'`epsg` de chaque couche dans l'inventaire rendu (10.7).
    pub fn vector_system(&mut self, epsg: u32) {
        self.vector_epsg = Some(epsg);
        self.keep_epsg = true;
        for layer in &mut self.layers {
            if layer.theme.geom != Geom::Raster {
                layer.epsg = epsg;
            }
        }
    }

    /// Fuseau UTM du projet tel que le noyau le déduirait du centre de l'emprise : celui des
    /// couches vecteur ou du MNT quand elles sont en UTM ; sinon 31N, celui des projets du jeu
    /// (Val-de-Marne, `PROJECTS`). Aucun tirage : la suite des autres familles ne bouge pas.
    fn layers_utm(&self) -> u32 {
        let utm = |e: u32| (32601..=32660).contains(&e);
        if let Some(e) = self.project_epsg.filter(|&e| utm(e)) {
            return e;
        }
        if let Some(e) = self.vector_epsg.filter(|&e| utm(e)) {
            return e;
        }
        if let Some(l) = self.layers.iter().find(|l| utm(l.epsg)) {
            return l.epsg;
        }
        32631
    }

    /// Fixe le fuseau UTM du projet (`project.utm_epsg`) dont dépend le plan : présent si
    /// `present`, absent sinon (projet dont l'emprise n'a pas pu être lue). Rend le code posé.
    /// Sur des couches en Lambert-93, qui couvre toute la France métropolitaine, le fuseau est
    /// tiré (31N le plus souvent, 30N à l'ouest, 32N à l'est) : le plan le recopie de
    /// l'inventaire, il ne le devine pas.
    pub fn project_utm(&mut self, rng: &mut Rng, present: bool) -> Option<u32> {
        let code = present.then(|| {
            let utm = |e: u32| (32601..=32660).contains(&e);
            let from_layers =
                self.vector_epsg.is_some_and(utm) || self.layers.iter().any(|l| utm(l.epsg));
            if from_layers {
                self.layers_utm()
            } else {
                *rng.pick(&[32631, 32631, 32630, 32632])
            }
        });
        self.utm = Some(code);
        code
    }

    /// Pose le système du projet, distinct de celui des couches vecteur : un projet en mètres
    /// dont toutes les couches sont en degrés (aucun projet réel n'est en EPSG:4326). L'`epsg`
    /// du projet reste à l'inventaire rendu.
    pub fn project_system(&mut self, epsg: u32) {
        self.project_epsg = Some(epsg);
        self.keep_epsg = true;
    }

    /// Élargit la prochaine couche ajoutée par `add` (MNT et rasters compris, qui passent par
    /// lui), si son thème a des champs de remplissage (`wide_fields`), même au-delà de
    /// `MAX_WIDE_LAYERS` ; sans effet, et oublié, si ce thème n'en a pas.
    pub fn wide_next(&mut self) {
        self.force_wide = true;
    }

    /// Ajoute `n` couches de distraction à celles que `finish` tire : un inventaire long, d'une
    /// dizaine de couches, comme celui d'un vrai projet (Annecy).
    pub fn long_inventory(&mut self, n: usize) {
        self.extra_distractors = n;
    }

    /// Donne à une couche ponctuelle une source `.csv` (CSV à coordonnées). Rend faux, sans rien
    /// changer, pour une autre géométrie.
    pub fn as_csv(&mut self, id: &str) -> bool {
        for layer in &mut self.layers {
            if layer.id == id && layer.theme.geom == Geom::Point {
                if let Some(dot) = layer.source.rfind('.') {
                    layer.source.truncate(dot);
                }
                layer.source.push_str(".csv");
                return true;
            }
        }
        false
    }

    /// Pose les valeurs listées du champ `role` d'une couche déjà tirée (les noms propres d'une
    /// couche de gares), triées comme le produit les rend. Rend le nom du champ, s'il existe.
    pub fn set_values(&mut self, id: &str, role: &str, values: &[&'static Val]) -> Option<String> {
        let layer = self.layers.iter_mut().find(|l| l.id == id)?;
        let field = layer.fields.iter_mut().find(|f| f.def.role == role)?;
        let mut values = values.to_vec();
        values.sort_unstable_by(|a, b| a.v.cmp(b.v));
        field.values = values;
        Some(field.name.clone())
    }

    /// Change le système d'une couche déjà tirée (une couche en degrés, 10.7).
    pub fn set_epsg(&mut self, id: &str, epsg: u32) {
        for layer in &mut self.layers {
            if layer.id == id {
                layer.epsg = epsg;
            }
        }
    }

    /// Thèmes encore libres qui vérifient `pred`.
    pub fn free(&self, pred: impl Fn(&Theme) -> bool) -> Vec<&'static Theme> {
        THEMES
            .iter()
            .filter(|t| !self.taken(t) && pred(t))
            .collect()
    }

    /// Ajoute une couche du thème `key`. Les champs de `require` sont toujours présents, ceux
    /// de `forbid` jamais ; `force` impose une valeur parmi celles listées d'un champ texte.
    pub fn add(
        &mut self,
        rng: &mut Rng,
        key: &str,
        require: &[&str],
        forbid: &[&str],
        force: &[(&str, &str)],
    ) -> Layer {
        let t = theme(key);
        assert!(!self.taken(t), "thème déjà pris : {key}");
        let slug = *rng.pick(t.slugs);
        let file = *rng.pick(t.files);
        let ext = extension(rng, t.geom);
        let mut fields = Vec::new();
        for def in t.fields {
            let wanted = require.contains(&def.role) || force.iter().any(|(r, _)| *r == def.role);
            if forbid.contains(&def.role) || !(wanted || rng.chance(1, 2)) {
                continue;
            }
            let name = (*rng.pick(def.names)).to_string();
            let values = match &def.kind {
                Kind::Text { values, .. } => {
                    let forced: Vec<&str> = force
                        .iter()
                        .filter(|(r, _)| *r == def.role)
                        .map(|(_, v)| *v)
                        .collect();
                    let k = rng.range(2.min(values.len() as u64), values.len() as u64) as usize;
                    let mut idx: Vec<usize> = (0..values.len()).collect();
                    rng.shuffle(&mut idx);
                    let mut chosen: Vec<usize> = idx.into_iter().take(k).collect();
                    for f in forced {
                        let i = values
                            .iter()
                            .position(|v| v.v == f)
                            .unwrap_or_else(|| panic!("valeur inconnue {f} pour {key}"));
                        if !chosen.contains(&i) {
                            chosen.push(i);
                        }
                    }
                    // Valeurs triées comme le produit les rend (`core/src/agent_inventory.rs`,
                    // `sort_unstable` sur les chaînes) : QGIS n'a pas d'ordre canonique.
                    chosen.sort_unstable_by(|a, b| values[*a].v.cmp(values[*b].v));
                    chosen.into_iter().map(|i| &values[i]).collect()
                }
                _ => Vec::new(),
            };
            let decoys = if def.role == "nom" && rng.chance(1, 2) {
                decoys(rng, t, &name)
            } else {
                Vec::new()
            };
            fields.push(FieldInst { def, name, values });
            fields.extend(decoys.into_iter().map(|name| FieldInst {
                def: &DECOY,
                name,
                values: Vec::new(),
            }));
        }
        // Inventaires larges (2026-11-10) : une part des couches des thèmes qui en ont dans la
        // réalité porte 15 à 20 champs, le champ de nom noyé au milieu.
        let forced_wide = std::mem::take(&mut self.force_wide);
        if let Some(fill) = wide_fields(t.key) {
            if forced_wide
                || (self.wide_layers < MAX_WIDE_LAYERS && rng.chance(WIDE_CHANCE, WIDE_OUT_OF))
            {
                self.wide_layers += 1;
                fields = widen(rng, fields, fill);
            }
        }
        let layer = Layer {
            theme: t,
            id: layer_id(rng, slug),
            name: (*rng.pick(t.names)).to_string(),
            source: format!("donnees/{file}.{ext}"),
            epsg: match self.vector_epsg {
                Some(epsg) if t.geom != Geom::Raster => epsg,
                _ => 2154,
            },
            count: rng.range(3, 12_000),
            geometry: geometry(rng, t.geom),
            fields,
        };
        self.layers.push(layer.clone());
        layer
    }

    /// Ajoute le MNT qu'un plan de terrain cite par son `source`, dans le système `epsg`
    /// (2154, 32631 ou 32632 : celui des points de la demande). Sa `source` est protégée des
    /// formes de QGIS ; son `epsg` peut manquer, comme pour toute couche.
    ///
    /// La source prend les formes que rend l'extension depuis 4.3 (chemin relatif au projet) :
    /// `donnees/x.tif`, `dem/x.tif`, `../donnees/x.tif` ou le nom seul. Le RGE ALTI n'existant
    /// qu'en Lambert-93, un MNT en UTM porte un nom de fichier neutre.
    pub fn add_dem(&mut self, rng: &mut Rng, epsg: u32) -> Layer {
        let files: &[&str] = if epsg == 2154 {
            theme("mnt").files
        } else {
            &["mnt", "mnt_5m", "mnt_25m"]
        };
        let dem = self.add_sourced(rng, "mnt", epsg, files, &["dem/"]);
        // Parfois un autre MNT du dossier, non ouvert (C02 et K02 d'`epic11_reel`, 2026-11-08) :
        // la phrase n'en désigne aucun, le plan cite le MNT ouvert.
        if rng.chance(1, 5) {
            let other = *rng.pick(&["zone_sud", "secteur_est", "ancien", "2012", "brut", "ouest"]);
            self.files.push(File {
                theme: theme("mnt"),
                path: format!("donnees/mnt_{other}.tif"),
            });
        }
        dem
    }

    /// Deux MNT du même système, de résolutions distinctes (Annecy, 2026-11-08 : 25 m et 5 m) :
    /// nom et fichier de chacun portent sa résolution (« MNT 5 m », `mnt_5m.tif`), pour qu'une
    /// phrase les désigne par elle (« sur le MNT à 5 m »). Rend les deux couches et leurs
    /// résolutions en mètres ; leurs `source` sont protégées comme celle de `add_dem`.
    pub fn add_dem_pair(&mut self, rng: &mut Rng, epsg: u32) -> ((Layer, u32), (Layer, u32)) {
        let mut res = [1u32, 5, 25];
        rng.shuffle(&mut res);
        let (r1, r2) = (res[0], res[1]);
        let first = self.add_dem(rng, epsg);
        let dir = first
            .source
            .rsplit_once('/')
            .map_or(String::new(), |(d, _)| format!("{d}/"));
        // Noms variés, dont un nom à l'allure de fichier (Annecy : « mnt_5m_annecy »), et un
        // identifiant fait du nom comme dans QGIS ; le fichier vient d'un autre vocabulaire, pour
        // que le plan cite la `source` et jamais l'identifiant (essai terrain-11-18, 2026-11-08).
        let place = *rng.pick(&["vallee", "bassin", "massif", "secteur", "lac", "plateau"]);
        let named = |rng: &mut Rng, r: u32| match rng.below(3) {
            0 => format!("MNT {r} m"),
            1 => format!("Modèle numérique de terrain {r} m"),
            _ => format!("mnt_{r}m_{place}"),
        };
        // Le fichier `mnt_5m.tif` n'accompagne jamais un nom « mnt_5m_… », dont l'identifiant
        // commencerait comme lui.
        // RGE ALTI et BD ALTI n'existent qu'en Lambert-93 : en UTM, des noms neutres.
        let lambert = epsg == 2154;
        let file = |rng: &mut Rng, r: u32, name: &str| {
            let n = if name.starts_with("mnt_") { 4 } else { 5 };
            match (rng.below(n), lambert) {
                (0, true) => format!("rgealti_{r}m.tif"),
                (0, false) => format!("terrain_{r}m.tif"),
                (1, _) => format!("mnt-{place}-{r}m.tif"),
                (2, true) => format!("bdalti_{r}m_l93.tif"),
                (2, false) => format!("elevation_{place}_{r}m.tif"),
                (3, _) => format!("dem_{place}_{r}m.tif"),
                _ => format!("mnt_{r}m.tif"),
            }
        };
        let qgis_id = |rng: &mut Rng, name: &str| {
            let slug: String = name
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect();
            layer_id(rng, &slug)
        };
        let mut a = first.clone();
        a.name = named(rng, r1);
        a.id = qgis_id(rng, &a.name);
        a.source = format!("{dir}{}", file(rng, r1, &a.name));
        let mut b = first;
        b.name = named(rng, r2);
        b.id = qgis_id(rng, &b.name);
        b.source = format!("{dir}{}", file(rng, r2, &b.name));
        if let Some(last) = self.layers.last_mut() {
            last.id = a.id.clone();
            last.name = a.name.clone();
            last.source = a.source.clone();
        }
        self.sourced.push(self.layers.len());
        self.layers.push(b.clone());
        ((a, r1), (b, r2))
    }

    /// Ajoute le raster du thème `key` qu'un plan cite par son `source` (le raster de coût de
    /// 11.12, lot 2b-1), dans le système `epsg`, sa `source` protégée comme celle du MNT.
    pub fn add_raster(&mut self, rng: &mut Rng, key: &str, epsg: u32) -> Layer {
        assert_eq!(theme(key).geom, Geom::Raster, "thème non raster : {key}");
        self.add_sourced(rng, key, epsg, theme(key).files, &["rasters/"])
    }

    /// Couche raster citée par sa `source` : fichier tiré de `files`, dossier relatif au projet
    /// (`donnees/`, `../donnees/`, le dossier propre `own` ou le nom seul).
    fn add_sourced(
        &mut self,
        rng: &mut Rng,
        key: &str,
        epsg: u32,
        files: &[&str],
        own: &[&'static str],
    ) -> Layer {
        self.sourced.push(self.layers.len());
        let mut layer = self.add(rng, key, &[], &[], &[]);
        let file = (*rng.pick(files)).to_string();
        let dirs = [
            ["donnees/", "donnees/"].as_slice(),
            own,
            &["../donnees/", ""],
        ]
        .concat();
        let dir = *rng.pick(&dirs);
        layer.epsg = epsg;
        layer.source = format!("{dir}{file}.tif");
        if let Some(last) = self.layers.last_mut() {
            last.epsg = epsg;
            last.source = layer.source.clone();
        }
        layer
    }

    /// Ajoute un fichier non ouvert du thème `key` ; rend son chemin.
    pub fn add_file(&mut self, rng: &mut Rng, key: &str, ext: Option<&str>) -> String {
        let t = theme(key);
        assert!(!self.taken(t), "thème déjà pris : {key}");
        let ext = ext.map_or_else(|| extension(rng, t.geom).to_string(), str::to_string);
        let path = format!("donnees/{}.{ext}", rng.pick(t.files));
        self.files.push(File {
            theme: t,
            path: path.clone(),
        });
        path
    }

    /// Ajoute `n` couches de distraction (au plus) parmi les thèmes libres qui vérifient `pred`.
    pub fn distract(&mut self, rng: &mut Rng, n: usize, pred: impl Fn(&Theme) -> bool) {
        for _ in 0..n {
            let free = self.free(&pred);
            if free.is_empty() {
                return;
            }
            let t = *rng.pick(&free);
            self.distractors.push(self.layers.len());
            self.add(rng, t.key, &[], &[], &[]);
        }
    }

    /// Ajoute une couche vecteur de distraction en degrés (EPSG:4326), qu'aucun plan ne cite
    /// (second essai de 11.23) : une analyse en mètres sur une autre couche, déjà en mètres, ne
    /// reprojette rien. Rend faux si aucun thème vecteur n'est libre.
    pub fn distract_in_degrees(&mut self, rng: &mut Rng) -> bool {
        let before = self.layers.len();
        self.distract(rng, 1, |t| t.geom != Geom::Raster);
        let Some(layer) = self.layers.get(before) else {
            return false;
        };
        let id = layer.id.clone();
        self.set_epsg(&id, 4326);
        self.keep_epsg = true;
        // Une couche ponctuelle en degrés vient souvent d'un CSV à coordonnées (Annecy).
        if rng.chance(1, 3) {
            self.as_csv(&id);
        }
        true
    }

    /// Identifiants des couches que la famille a posées elle-même, distractions exclues (5.47) :
    /// les couches « utiles » qu'un gros projet doit montrer en détail.
    pub fn focus_ids(&self) -> Vec<String> {
        self.layers
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.distractors.contains(i))
            .map(|(_, l)| l.id.clone())
            .collect()
    }

    /// Deux couches jumelles du thème `key` (5.47) : mêmes champs, nommées « `label`
    /// `first` » et « `label` `second` » (« Gares 2019 », « Gares 2024 »), chacune avec un
    /// identifiant et un fichier qui portent son qualificatif. Un `first` vide laisse la
    /// première au seul `label`. `copy` : la seconde est une copie de la première, de même
    /// source (QGIS donne sa source à une couche dupliquée). Seule façon de poser deux couches
    /// d'un même groupe : `taken` est contourné pour la seconde.
    pub fn add_twins(
        &mut self,
        rng: &mut Rng,
        key: &str,
        label: &str,
        first: &str,
        second: &str,
        copy: bool,
    ) -> (Layer, Layer) {
        let base = self.add(rng, key, &[], &[], &[]);
        let index = self.layers.len() - 1;
        let name = |q: &str| {
            if q.is_empty() {
                label.to_string()
            } else {
                format!("{label} {q}")
            }
        };
        let mut a = base.clone();
        a.name = name(first);
        if !first.is_empty() {
            let slug = format!("{}_{}", rng.pick(a.theme.slugs), slug_of(first));
            a.id = layer_id(rng, &slug);
            a.source = with_stem_suffix(&base.source, &slug_of(first));
        }
        let mut b = base.clone();
        b.name = name(second);
        let slug = format!("{}_{}", rng.pick(b.theme.slugs), slug_of(second));
        b.id = layer_id(rng, &slug);
        if !copy {
            b.source = with_stem_suffix(&base.source, &slug_of(second));
        } else {
            b.source = a.source.clone();
        }
        b.count = if copy { a.count } else { rng.range(3, 12_000) };
        self.layers[index] = a.clone();
        self.layers.push(b.clone());
        (a, b)
    }

    /// Termine l'inventaire : distractions par défaut, ordre mélangé. Les fichiers non ouverts
    /// sont ceux que la famille a posés (`add_file`), plus le MNT non ouvert qu'`add_dem` pose
    /// parfois : dans QGIS, l'extension ne parcourt pas le disque et `unloaded_files` est vide,
    /// mais l'inventaire de l'épreuve en porte un (`mnt_zone_nord.tif`), qu'aucun plan ne cite.
    ///
    /// Une part des inventaires prend les formes que rend le produit depuis QGIS
    /// (`core/src/agent_inventory.rs`, clé absente plutôt que `null`) : raster sans nombre
    /// d'objets, couche temporaire sans `source`, SCR qui n'est pas EPSG (sans `epsg`), table
    /// sans géométrie (`geometry: null` sur un vecteur), projet non enregistré (sans `name`),
    /// projet sans fuseau UTM (`utm_epsg`, emprise illisible).
    pub fn finish(mut self, rng: &mut Rng) -> Value {
        let n = rng.range(1, 4) as usize + self.extra_distractors;
        // Inventaire long (une dizaine de couches) : ses distractions ne s'élargissent pas, pour
        // que l'exemple reste loin des 2 048 jetons de l'entraînement.
        if self.extra_distractors > 0 {
            self.wide_layers = self.wide_layers.max(MAX_WIDE_LAYERS);
        }
        self.distract(rng, n, |t| t.geom != Geom::Raster);
        if rng.chance(1, 4) {
            self.distract(rng, 1, |t| t.geom == Geom::Raster);
        }
        let mut layers: Vec<Value> = self.layers.iter().map(Layer::to_json).collect();
        let utm = match self.utm {
            Some(fixed) => fixed,
            None => Some(self.layers_utm()),
        };
        // Le nom du projet est cohérent avec son fuseau (relecture du fuseau UTM, 2026-11-08) :
        // un projet de l'ouest en 30N, de l'est en 32N, du Val-de-Marne sinon.
        let names = utm.map_or(PROJECTS, project_names);
        let mut project = json!({"name": rng.pick(names), "epsg": self.project_epsg.or(self.vector_epsg).unwrap_or(2154)});
        if let Some(code) = utm {
            project["utm_epsg"] = json!(code);
        }
        if rng.chance(1, 3) {
            qgis_shapes(
                rng,
                &mut layers,
                &self.distractors,
                &self.sourced,
                &mut project,
                self.keep_epsg,
                self.utm.is_some(),
            );
        }
        rng.shuffle(&mut layers);
        let mut files: Vec<Value> = self
            .files
            .iter()
            .map(|f| {
                let kind = if f.theme.geom == Geom::Raster {
                    "raster"
                } else {
                    "vector"
                };
                json!({"path": f.path, "kind": kind})
            })
            .collect();
        rng.shuffle(&mut files);
        json!({
            "project": project,
            "layers": layers,
            "unloaded_files": files,
        })
    }
}

/// Formes d'inventaire que rend le produit depuis QGIS, posées sur un inventaire du jeu.
/// Seule une couche de distraction perd sa géométrie : une couche citée par le plan doit en
/// avoir une pour être tamponnée ou croisée. Une couche de `sourced` garde sa `source`.
fn qgis_shapes(
    rng: &mut Rng,
    layers: &mut [Value],
    distractors: &[usize],
    sourced: &[usize],
    project: &mut Value,
    keep_epsg: bool,
    keep_utm: bool,
) {
    let project = project.as_object_mut().expect("projet objet");
    if rng.chance(1, 4) {
        project.remove("name");
    }
    if rng.chance(1, 8) && !keep_epsg {
        project.remove("epsg");
    }
    // Emprise que QGIS n'a pas su ramener en degrés : pas de fuseau.
    if !keep_utm && rng.chance(1, 4) {
        project.remove("utm_epsg");
    }
    for (index, layer) in layers.iter_mut().enumerate() {
        let layer = layer.as_object_mut().expect("couche objet");
        if layer["kind"] == "raster" {
            // QGIS ne compte pas les objets d'un raster : la clé manque.
            layer.remove("feature_count");
        }
        // Le tirage a lieu même pour une source protégée : les autres familles gardent leur suite.
        if rng.chance(1, 8) && !sourced.contains(&index) {
            // Sortie temporaire d'une étape précédente : aucun fichier derrière.
            layer.remove("source");
        }
        if rng.chance(1, 12) && !keep_epsg {
            layer.remove("epsg");
        }
    }
    let tables: Vec<usize> = distractors
        .iter()
        .copied()
        .filter(|&i| layers[i]["kind"] == "vector")
        .collect();
    if !tables.is_empty() && rng.chance(1, 4) {
        let i = *rng.pick(&tables);
        layers[i]["geometry"] = Value::Null;
    }
}

/// `BD TOPO` donne `bd_topo`, `2019` reste `2019` : un qualificatif dans un identifiant ou un
/// nom de fichier (minuscules ASCII, chiffres, soulignés).
fn slug_of(text: &str) -> String {
    let folded = crate::normalize(text);
    folded
        .split(' ')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// `donnees/gares.gpkg` et `2019` donnent `donnees/gares_2019.gpkg`.
fn with_stem_suffix(source: &str, suffix: &str) -> String {
    match source.rsplit_once('.') {
        Some((stem, ext)) if !stem.ends_with('/') => format!("{stem}_{suffix}.{ext}"),
        _ => format!("{source}_{suffix}"),
    }
}

// -- gros projets (5.47) ---------------------------------------------------------------------

/// Nombre de couches d'un gros projet, bornes comprises (décision du propriétaire du
/// 2026-11-10 ; repli : 40 au plus).
pub const BIG_MIN_LAYERS: usize = 20;
pub const BIG_MAX_LAYERS: usize = 60;

/// Type d'un champ de remplissage, tel que QGIS le nomme dans `project.inventory`.
#[derive(Debug, Clone, Copy)]
pub enum Ft {
    /// Texte, avec ses valeurs possibles (aucune : un champ trop varié, sans clé `values`).
    Text(&'static [&'static str]),
    Int,
    Real,
    Date,
    Bool,
}

/// Producteur d'une couche de remplissage : il fixe ses champs communs et la forme de sa source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Producer {
    /// BD TOPO 3 de l'IGN : un GeoPackage départemental ou des Shapefiles par table.
    BdTopo,
    /// Standard du CNIG des documents d'urbanisme (export du Géoportail de l'Urbanisme).
    Cnig,
    /// Autre producteur public : les champs de la couche sont tous dans `fields`.
    Other,
}

/// Une couche de remplissage d'un gros projet, façon BD TOPO 3, CNIG ou autre producteur
/// public. Aucune ne recouvre un thème de `THEMES` : ni commune, ni bâtiment, ni route, ni
/// cours d'eau, ni canalisation, ni ligne électrique, ni forêt, ni ERP, ni zone d'habitation,
/// ni parc, ni IRIS, ni équipement de transport. Aucun plan ne la cite.
#[derive(Debug)]
pub struct BigFiller {
    pub names: &'static [&'static str],
    /// Nom de la table du producteur, radical de l'identifiant QGIS.
    pub table: &'static str,
    /// Radicaux de fichier (producteur `Other` et rasters) ; sinon la table.
    pub files: &'static [&'static str],
    pub geom: Geom,
    pub producer: Producer,
    /// Champs propres à la couche, dans l'ordre du producteur.
    pub fields: &'static [(&'static str, Ft)],
}

const T: Ft = Ft::Text(&[]);
const ETAT: Ft = Ft::Text(&["En construction", "En projet", "En service", "Non exploité"]);

/// Champs communs des tables de la BD TOPO 3, dans leur ordre (`cleabs` en tête).
const BDTOPO_COMMON: &[(&str, Ft)] = &[
    ("cleabs", T),
    ("fictif", Ft::Bool),
    ("etat_de_l_objet", ETAT),
    ("date_creation", Ft::Date),
    ("date_modification", Ft::Date),
    ("date_d_apparition", Ft::Date),
    ("date_de_confirmation", Ft::Date),
    ("sources", T),
    ("identifiants_sources", T),
    (
        "methode_d_acquisition_planimetrique",
        Ft::Text(&[
            "Levé GPS",
            "Orthophotographie",
            "Restitution photogrammétrique",
            "Saisie sur plan",
        ]),
    ),
    ("precision_planimetrique", Ft::Real),
    (
        "methode_d_acquisition_altimetrique",
        Ft::Text(&[
            "Corrélation",
            "Interpolation",
            "Restitution photogrammétrique",
            "Sans objet",
        ]),
    ),
    ("precision_altimetrique", Ft::Real),
];

/// Champs communs des tables du standard CNIG (export du Géoportail de l'Urbanisme).
const CNIG_COMMON: &[(&str, Ft)] = &[
    ("gid", Ft::Int),
    ("idurba", T),
    ("txt", T),
    ("nomfic", T),
    ("urlfic", T),
    ("insee", T),
    ("datappro", Ft::Date),
    ("datvalid", Ft::Date),
    ("partition", T),
    ("gpu_doc_id", T),
    ("gpu_status", Ft::Text(&["document.production"])),
    ("gpu_timestamp", T),
];

macro_rules! filler {
    ($names:expr, $table:expr, $geom:expr, $producer:expr, [$($f:expr),* $(,)?]) => {
        BigFiller {
            names: $names,
            table: $table,
            files: &[],
            geom: $geom,
            producer: $producer,
            fields: &[$($f),*],
        }
    };
    ($names:expr, $table:expr, files: $files:expr, $geom:expr, [$($f:expr),* $(,)?]) => {
        BigFiller {
            names: $names,
            table: $table,
            files: $files,
            geom: $geom,
            producer: Producer::Other,
            fields: &[$($f),*],
        }
    };
}

use Producer::{BdTopo, Cnig};

/// Les couches de remplissage des gros projets (5.47), données publiques seulement. L'ordre est
/// fixe : il fait partie de la reproductibilité.
pub static BIG_FILLERS: &[BigFiller] = &[
    // BD TOPO 3, thème administratif (sans les communes, qui sont un thème).
    filler!(
        &["Arrondissements"],
        "arrondissement",
        Geom::Polygon,
        BdTopo,
        [
            ("code_insee_de_l_arrondissement", T),
            ("code_insee_du_departement", T),
            ("code_insee_de_la_region", T),
            ("liens_vers_autorite_administrative", T)
        ]
    ),
    filler!(
        &["Départements"],
        "departement",
        Geom::Polygon,
        BdTopo,
        [
            ("code_insee", T),
            ("code_insee_de_la_region", T),
            ("liens_vers_autorite_administrative", T)
        ]
    ),
    filler!(
        &["Régions"],
        "region",
        Geom::Polygon,
        BdTopo,
        [("code_insee", T), ("liens_vers_autorite_administrative", T)]
    ),
    filler!(
        &["EPCI", "Intercommunalités"],
        "epci",
        Geom::Polygon,
        BdTopo,
        [
            ("code_siren", T),
            ("nature", T),
            ("liens_vers_autorite_administrative", T)
        ]
    ),
    filler!(
        &["Collectivités territoriales"],
        "collectivite_territoriale",
        Geom::Polygon,
        BdTopo,
        [
            ("code_insee", T),
            ("code_siren", T),
            ("liens_vers_autorite_administrative", T)
        ]
    ),
    // Thème bâti, sans les bâtiments ni les terrains de sport.
    filler!(
        &["Cimetières"],
        "cimetiere",
        Geom::Polygon,
        BdTopo,
        [
            ("nature", Ft::Text(&["Animalier", "Civil"])),
            ("nature_detaillee", T)
        ]
    ),
    filler!(
        &["Constructions linéaires"],
        "construction_lineaire",
        Geom::Line,
        BdTopo,
        [
            (
                "nature",
                Ft::Text(&[
                    "Barrage",
                    "Clôture",
                    "Mur",
                    "Mur de soutènement",
                    "Quai",
                    "Ruines"
                ])
            ),
            ("nature_detaillee", T)
        ]
    ),
    filler!(
        &["Constructions ponctuelles"],
        "construction_ponctuelle",
        Geom::Point,
        BdTopo,
        [
            (
                "nature",
                Ft::Text(&[
                    "Antenne",
                    "Calvaire",
                    "Cheminée",
                    "Clocher",
                    "Croix",
                    "Éolienne",
                    "Phare",
                    "Torchère"
                ])
            ),
            ("nature_detaillee", T),
            ("hauteur", Ft::Real)
        ]
    ),
    filler!(
        &["Constructions surfaciques"],
        "construction_surfacique",
        Geom::Polygon,
        BdTopo,
        [
            (
                "nature",
                Ft::Text(&["Barrage", "Dalle", "Écluse", "Escalier", "Pont"])
            ),
            ("nature_detaillee", T)
        ]
    ),
    filler!(
        &["Orographie linéaire"],
        "ligne_orographique",
        Geom::Line,
        BdTopo,
        [("nature", Ft::Text(&["Carrière", "Levée", "Talus"]))]
    ),
    filler!(
        &["Pylônes"],
        "pylone",
        Geom::Point,
        BdTopo,
        [("hauteur", Ft::Real)]
    ),
    filler!(
        &["Réservoirs"],
        "reservoir",
        Geom::Polygon,
        BdTopo,
        [
            (
                "nature",
                Ft::Text(&["Château d'eau", "Réservoir au sol", "Réservoir industriel"])
            ),
            ("hauteur", Ft::Real)
        ]
    ),
    // Thème hydrographie, sans les cours d'eau ni leurs tronçons.
    filler!(
        &["Bassins versants topographiques"],
        "bassin_versant_topographique",
        Geom::Polygon,
        BdTopo,
        [
            ("code_hydrographique", T),
            ("bassin_fluvial", T),
            ("code_du_bassin_fluvial", T),
            ("origine", T)
        ]
    ),
    filler!(
        &["Détails hydrographiques"],
        "detail_hydrographique",
        Geom::Point,
        BdTopo,
        [
            (
                "nature",
                Ft::Text(&[
                    "Cascade",
                    "Fontaine",
                    "Lavoir",
                    "Perte",
                    "Puits",
                    "Résurgence"
                ])
            ),
            ("nature_detaillee", T)
        ]
    ),
    filler!(
        &["Limites terre-mer"],
        "limite_terre_mer",
        Geom::Line,
        BdTopo,
        [
            ("code_hydrographique", T),
            ("type_de_limite", T),
            ("origine", T),
            ("niveau", T)
        ]
    ),
    filler!(
        &["Plans d'eau"],
        "plan_d_eau",
        Geom::Polygon,
        BdTopo,
        [
            (
                "nature",
                Ft::Text(&["Bassin", "Écoulement canalisé", "Lac", "Lagune", "Mare"])
            ),
            ("code_hydrographique", T),
            ("persistance", Ft::Text(&["Intermittent", "Permanent"])),
            ("salinite", Ft::Bool),
            ("altitude_moyenne", Ft::Real)
        ]
    ),
    filler!(
        &["Entités de transition"],
        "entite_de_transition",
        Geom::Polygon,
        BdTopo,
        [("code_hydrographique", T), ("origine", T)]
    ),
    // Lieux nommés et occupation du sol, sans les zones d'habitation ni la végétation.
    filler!(
        &["Détails orographiques"],
        "detail_orographique",
        Geom::Point,
        BdTopo,
        [
            (
                "nature",
                Ft::Text(&[
                    "Cirque", "Col", "Crête", "Gorge", "Grotte", "Pic", "Rochers", "Sommet"
                ])
            ),
            ("nature_detaillee", T)
        ]
    ),
    filler!(
        &["Lieux-dits non habités"],
        "lieu_dit_non_habite",
        Geom::Point,
        BdTopo,
        [("nature", T), ("importance", T)]
    ),
    filler!(
        &["Toponymie des lieux nommés"],
        "toponymie_lieux_nommes",
        Geom::Point,
        BdTopo,
        [
            ("classe_de_l_objet", T),
            ("nature_de_l_objet", T),
            ("graphie_du_toponyme", T),
            ("source_du_toponyme", T),
            ("date_du_toponyme", Ft::Date),
            ("statut_du_toponyme", T)
        ]
    ),
    filler!(
        &["Estran"],
        "estran",
        Geom::Polygon,
        BdTopo,
        [("nature", T)]
    ),
    // Services et transport, sans les ERP, les lignes électriques, les canalisations, les
    // routes ni les équipements de transport.
    filler!(
        &["Postes de transformation"],
        "poste_de_transformation",
        Geom::Polygon,
        BdTopo,
        [("importance", T)]
    ),
    filler!(
        &["Aérodromes"],
        "aerodrome",
        Geom::Polygon,
        BdTopo,
        [
            (
                "nature",
                Ft::Text(&["Aérodrome", "Altiport", "Héliport", "Hydrobase"])
            ),
            ("usage", Ft::Text(&["Civil", "Privé"])),
            ("code_icao", T),
            ("code_iata", T),
            ("altitude", Ft::Real)
        ]
    ),
    filler!(
        &["Voies ferrées", "Voies ferrées principales"],
        "troncon_de_voie_ferree",
        Geom::Line,
        BdTopo,
        [
            (
                "nature",
                Ft::Text(&[
                    "Funiculaire ou crémaillère",
                    "Métro",
                    "Tramway",
                    "Voie de service",
                    "Voie ferrée principale"
                ])
            ),
            ("electrifie", Ft::Bool),
            ("largeur", Ft::Text(&["Normale", "Étroite"])),
            ("nombre_de_voies", Ft::Int),
            ("position_par_rapport_au_sol", T),
            ("usage", T),
            ("vitesse_maximale", Ft::Int)
        ]
    ),
    filler!(
        &["Transports par câble"],
        "transport_par_cable",
        Geom::Line,
        BdTopo,
        [(
            "nature",
            Ft::Text(&["Télécabine, téléphérique", "Télésiège", "Téléski"])
        )]
    ),
    filler!(
        &["Points de repère"],
        "point_de_repere",
        Geom::Point,
        BdTopo,
        [
            ("identifiant", T),
            ("gestionnaire", T),
            ("cote", Ft::Real),
            ("abscisse", Ft::Real),
            ("type_de_pr", T),
            ("numero", T)
        ]
    ),
    filler!(
        &["Non-communications"],
        "non_communication",
        Geom::Point,
        BdTopo,
        [
            ("lien_vers_troncon_entree", T),
            ("liens_vers_troncon_sortie", T)
        ]
    ),
    filler!(
        &["Itinéraires autres", "Itinéraires de randonnée"],
        "itineraire_autre",
        Geom::Line,
        BdTopo,
        [
            (
                "nature",
                Ft::Text(&[
                    "Itinéraire équestre",
                    "Itinéraire piéton",
                    "Sentier de grande randonnée"
                ])
            ),
            ("numero", T)
        ]
    ),
    // Standard CNIG des documents d'urbanisme (sans le zonage, qui est un thème).
    filler!(
        &["Prescriptions surfaciques"],
        "prescription_surf",
        Geom::Polygon,
        Cnig,
        [("typepsc", T), ("stypepsc", T), ("libelong", T)]
    ),
    filler!(
        &["Prescriptions linéaires"],
        "prescription_lin",
        Geom::Line,
        Cnig,
        [("typepsc", T), ("stypepsc", T), ("libelong", T)]
    ),
    filler!(
        &["Prescriptions ponctuelles"],
        "prescription_pct",
        Geom::Point,
        Cnig,
        [("typepsc", T), ("stypepsc", T), ("libelong", T)]
    ),
    filler!(
        &["Informations surfaciques"],
        "info_surf",
        Geom::Polygon,
        Cnig,
        [("typeinf", T), ("stypeinf", T), ("libelong", T)]
    ),
    filler!(
        &["Informations linéaires"],
        "info_lin",
        Geom::Line,
        Cnig,
        [("typeinf", T), ("stypeinf", T), ("libelong", T)]
    ),
    filler!(
        &["Informations ponctuelles"],
        "info_pct",
        Geom::Point,
        Cnig,
        [("typeinf", T), ("stypeinf", T), ("libelong", T)]
    ),
    filler!(
        &["Documents d'urbanisme"],
        "doc_urba",
        Geom::Polygon,
        Cnig,
        [
            ("typedoc", Ft::Text(&["PLU", "PLUi", "POS", "PSMV"])),
            ("etat", T),
            ("nomproc", T),
            ("datefin", Ft::Date),
            ("siren", T),
            ("nomreg", T),
            ("urlreg", T),
            ("nomplan", T),
            ("urlplan", T),
            ("siteweb", T),
            ("typeref", T),
            ("dateref", Ft::Date)
        ]
    ),
    filler!(
        &["Habillage surfacique"],
        "habillage_surf",
        Geom::Polygon,
        Cnig,
        [("nattrac", T), ("couleur", T)]
    ),
    // Servitudes d'utilité publique (Géoportail de l'Urbanisme).
    filler!(&["Assiettes des servitudes"], "assiette_sup_s", files: &["assiette_sup_s"],
        Geom::Polygon, [
        ("gid", Ft::Int), ("idass", T), ("idgen", T), ("typeass", T), ("modegeoass", T),
        ("paramcalc", T), ("srcgeoass", T), ("datesrc", Ft::Date),
        ("suptype", Ft::Text(&["ac1", "as1", "el7", "int1", "pm1"])), ("insee", T)]),
    filler!(&["Générateurs des servitudes"], "generateur_sup_p", files: &["generateur_sup_p"],
        Geom::Point, [
        ("gid", Ft::Int), ("idgen", T), ("idsup", T), ("typegen", T), ("modegeogen", T),
        ("srcgeogen", T), ("datesrc", Ft::Date),
        ("suptype", Ft::Text(&["ac1", "as1", "el7", "int1", "pm1"])), ("insee", T),
        ("refbdext", T)]),
    // Autres producteurs publics.
    filler!(&["Adresses", "Adresses BAN"], "adresses", files: &["adresses_ban", "adresses"],
        Geom::Point, [
        ("id", T), ("id_fantoir", T), ("numero", Ft::Int), ("rep", T), ("code_postal", T),
        ("code_insee", T), ("x", Ft::Real), ("y", Ft::Real), ("lon", Ft::Real),
        ("lat", Ft::Real),
        ("position", Ft::Text(&["Cage d'escalier", "Délivrance postale", "Entrée", "Logement",
            "Segment", "Service technique"])),
        ("source_position", T), ("source_nom_voie", T), ("certification", Ft::Bool)]),
    filler!(&["Monuments historiques"], "monuments_historiques",
        files: &["monuments_historiques", "merimee"], Geom::Point, [
        ("reference", T),
        ("protection", Ft::Text(&["Classé", "Inscrit", "Partiellement classé",
            "Partiellement inscrit"])),
        ("date_protection", Ft::Date), ("siecle", T), ("statut_juridique", T), ("insee", T),
        ("departement", T), ("region", T), ("lien_pop", T), ("auteur", T)]),
    filler!(&["Carroyage INSEE 200 m", "Données carroyées 200 m"], "carreaux_200m",
        files: &["carreaux_200m_met", "filosofi_200m"], Geom::Polygon, [
        ("idcar_200m", T), ("idcar_1km", T), ("idcar_nat", T), ("i_est_200", Ft::Int),
        ("lcog_geo", T), ("ind", Ft::Real), ("men", Ft::Real), ("men_pauv", Ft::Real),
        ("men_1ind", Ft::Real), ("men_5ind", Ft::Real), ("men_prop", Ft::Real),
        ("men_fmp", Ft::Real), ("ind_snv", Ft::Real), ("men_coll", Ft::Real),
        ("men_mais", Ft::Real), ("log_av45", Ft::Real), ("log_45_70", Ft::Real),
        ("log_70_90", Ft::Real), ("log_ap90", Ft::Real), ("log_inc", Ft::Real),
        ("log_soc", Ft::Real), ("ind_0_3", Ft::Real), ("ind_65_79", Ft::Real)]),
    filler!(&["Courbes de niveau"], "courbe_de_niveau", files: &["courbes_niveau", "bdalti_courbes"],
        Geom::Line, [
        ("id", T), ("altitude", Ft::Real),
        ("nature", Ft::Text(&["Courbe intercalaire", "Courbe maîtresse", "Courbe normale"])),
        ("equidistance", Ft::Int), ("source", T), ("date_saisie", Ft::Date),
        ("precision", Ft::Real), ("producteur", T), ("millesime", Ft::Int), ("departement", T)]),
    filler!(&["Points géodésiques"], "site_geodesique", files: &["points_geodesiques", "rbf"],
        Geom::Point, [
        ("numero", T),
        ("type", Ft::Text(&["Borne", "Pilier", "Repère de nivellement"])),
        ("altitude", Ft::Real), ("systeme_altimetrique", T), ("systeme_planimetrique", T),
        ("e_l93", Ft::Real), ("n_l93", Ft::Real), ("date_visite", Ft::Date),
        ("etat", Ft::Text(&["Bon", "Détruit", "Non retrouvé"])), ("url_fiche", T),
        ("departement", T)]),
    filler!(&["Îlots culturaux", "Îlots du RPG"], "ilots_anonymes",
        files: &["rpg_ilots", "ilots_anonymes"], Geom::Polygon, [
        ("id_ilot", T), ("num_ilot", T), ("code_cultu", T), ("code_group", T),
        ("culture_d1", T), ("culture_d2", T), ("surf_ilot", Ft::Real), ("bio", Ft::Bool),
        ("millesime", Ft::Int), ("pacage", T)]),
    filler!(&["Corine Land Cover", "Occupation du sol CLC"], "clc", files: &["clc_2018", "clc"],
        Geom::Polygon, [
        ("id", T), ("code_18", T), ("code_12", T), ("area_ha", Ft::Real), ("remark", T),
        ("changement", Ft::Bool), ("niveau1", T), ("niveau2", T), ("niveau3", T),
        ("millesime", Ft::Int), ("surface_m2", Ft::Real)]),
    filler!(&["Bornes de recharge", "Points de recharge IRVE"], "irve",
        files: &["irve", "bornes_recharge"], Geom::Point, [
        ("id_station_itinerance", T), ("amenageur", T), ("siren_amenageur", T),
        ("operateur", T), ("enseigne", T), ("id_pdc_itinerance", T),
        ("puissance_nominale", Ft::Real), ("prise_type_2", Ft::Bool),
        ("prise_type_combo_ccs", Ft::Bool), ("gratuit", Ft::Bool), ("paiement_cb", Ft::Bool),
        ("condition_acces", Ft::Text(&["Accès libre", "Accès réservé"])), ("horaires", T),
        ("date_mise_en_service", Ft::Date), ("date_maj", Ft::Date),
        ("consolidated_code_postal", T)]),
    filler!(&["Défibrillateurs"], "geodae", files: &["defibrillateurs", "geodae"], Geom::Point, [
        ("gid", Ft::Int), ("expt_siren", T), ("acc", T), ("acc_lib", Ft::Bool),
        ("acc_pcsec", Ft::Bool), ("acc_acc", T), ("disp_j", T), ("disp_h", T),
        ("dermnt", Ft::Date), ("dtpr_lcped", Ft::Date),
        ("etat_fonct", Ft::Text(&["En fonctionnement", "Hors service"])), ("fab_siren", T),
        ("num_serie", T)]),
    filler!(&["Points d'apport volontaire"], "pav", files: &["pav", "points_apport_volontaire"],
        Geom::Point, [
        ("id", T), ("type_flux", Ft::Text(&["Biodéchets", "Emballages", "Papier", "Textile",
            "Verre"])),
        ("volume", Ft::Real), ("gestionnaire", T), ("adresse", T),
        ("date_installation", Ft::Date), ("collecte_jour", T), ("statut", T),
        ("code_insee", T), ("enterre", Ft::Bool)]),
    filler!(&["Cavités souterraines"], "cavites", files: &["cavites", "cavites_souterraines"],
        Geom::Point, [
        ("id_cavite", T), ("type_cavite", Ft::Text(&["Carrière", "Cave", "Naturelle",
            "Ouvrage civil", "Puits"])),
        ("reperage", T), ("insee", T), ("precision_xy", T), ("date_validite", Ft::Date),
        ("organisme", T), ("reference", T), ("x", Ft::Real), ("y", Ft::Real)]),
    filler!(&["Mouvements de terrain"], "mvt", files: &["mouvements_terrain", "mvt"],
        Geom::Point, [
        ("id_mvt", T), ("type_mvt", Ft::Text(&["Chute de blocs", "Coulée", "Effondrement",
            "Érosion de berges", "Glissement"])),
        ("date_debut", Ft::Date), ("date_fin", Ft::Date), ("fiabilite", T), ("precision", T),
        ("insee", T), ("commentaire", T), ("x", Ft::Real), ("y", Ft::Real), ("source", T)]),
    filler!(&["Sites pollués", "Sites et sols pollués"], "basol", files: &["basol", "ssp"],
        Geom::Point, [
        ("id_basol", T), ("statut", T), ("activite", T), ("date_maj", Ft::Date),
        ("polluants", T), ("insee", T), ("adresse", T), ("x", Ft::Real), ("y", Ft::Real),
        ("surveillance", Ft::Bool), ("url", T)]),
    filler!(&["Installations classées", "ICPE"], "icpe", files: &["icpe", "installations_classees"],
        Geom::Point, [
        ("code_aiot", T), ("raison_sociale", T),
        ("regime", Ft::Text(&["Autorisation", "Déclaration", "Enregistrement"])),
        ("statut_seveso", Ft::Text(&["Non Seveso", "Seuil bas", "Seuil haut"])),
        ("priorite_nationale", Ft::Bool), ("ied", Ft::Bool), ("etat_activite", T),
        ("code_naf", T), ("insee", T), ("date_maj", Ft::Date), ("x", Ft::Real),
        ("y", Ft::Real)]),
    filler!(&["Périmètres de ZAC"], "zac", files: &["zac", "perimetres_zac"], Geom::Polygon, [
        ("id", T), ("date_creation", Ft::Date), ("amenageur", T), ("surface_ha", Ft::Real),
        ("programme", T), ("etat", Ft::Text(&["Achevée", "En cours", "En projet"])),
        ("procedure", T), ("insee", T), ("acte", T), ("date_acte", Ft::Date)]),
    // Deux rasters : orthophotographie et carte topographique.
    filler!(&["Orthophotographie 20 cm", "BD ORTHO"], "ortho",
        files: &["ortho_hr_0m20", "bdortho_20cm", "orthophoto"], Geom::Raster, []),
    filler!(&["SCAN 25", "Fond topographique SCAN 25"], "scan25", files: &["scan25", "scan25_tour"],
        Geom::Raster, []),
];

/// Mots courts sans sens propre, retirés avant de comparer une demande et un nom.
const LINK_WORDS: &[&str] = &[
    "les", "des", "une", "aux", "dans", "sur", "sous", "pour", "par", "avec", "sans", "que", "qui",
    "est", "sont",
];

/// Les mots de `text` d'au moins 3 signes dont une lettre : minuscules, accents ôtés, découpe sur
/// tout ce qui n'est ni lettre ni chiffre (dont `_`), mots de liaison retirés, `s` ou `x` final
/// ôté au-delà de 3 lettres. Sert à écarter une couche de remplissage qu'un mot de la demande
/// désignerait (décision 3 du 2026-11-10).
pub fn plain_words(text: &str) -> std::collections::BTreeSet<String> {
    crate::normalize(text)
        .split(' ')
        .filter(|w| w.chars().count() >= 3 && w.chars().any(char::is_alphabetic))
        .filter(|w| !LINK_WORDS.contains(w))
        .map(|w| {
            if w.chars().count() > 3 && (w.ends_with('s') || w.ends_with('x')) {
                w[..w.len() - 1].to_string()
            } else {
                w.to_string()
            }
        })
        .collect()
}

/// Un identifiant QGIS `nom_<uuid>` (uuid aux soulignés, groupes 8, 4, 4, 4 et 12).
fn is_qgis_uuid(id: &str) -> bool {
    let parts: Vec<&str> = id.rsplitn(6, '_').collect();
    parts.len() == 6
        && !parts[5].is_empty()
        && parts[..5].iter().map(|p| p.len()).eq([12, 4, 4, 4, 8])
        && parts[..5]
            .iter()
            .all(|p| p.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn uuid_from(rng: &mut Rng, head: &str) -> String {
    let mut d = head.to_string();
    d.push_str(&rng.hex(32 - head.len()));
    format!(
        "{}_{}_{}_{}_{}",
        &d[0..8],
        &d[8..12],
        &d[12..16],
        &d[16..20],
        &d[20..32]
    )
}

/// L'identifiant QGIS d'une couche du jeu : `slug_<4 ou 8 hex>` devient `slug_<uuid>` dont
/// l'uuid commence par ces chiffres (le noyau montre alors `slug_<4 hex>`) ; un identifiant déjà
/// de forme QGIS reste tel quel.
pub fn qgis_layer_id(rng: &mut Rng, id: &str) -> String {
    if is_qgis_uuid(id) {
        return id.to_string();
    }
    match id.rsplit_once('_') {
        Some((stem, hex))
            if !stem.is_empty()
                && matches!(hex.len(), 4 | 8)
                && hex.bytes().all(|b| b.is_ascii_hexdigit()) =>
        {
            format!("{stem}_{}", uuid_from(rng, &hex.to_ascii_lowercase()))
        }
        _ => id.to_string(),
    }
}

/// Une couche de la forme apprise retraduite en sortie brute de `project.inventory` (la forme
/// que lit `agent_inventory`), sous l'identifiant QGIS `id`. Une clé apprise absente donne une clé
/// brute absente ; un vecteur sans géométrie est une table `NoGeometry`.
pub fn raw_layer(layer: &Value, id: &str) -> Value {
    let mut out = serde_json::Map::new();
    out.insert("layer_id".into(), json!(id));
    for key in ["name", "source", "kind"] {
        if let Some(v) = layer.get(key) {
            out.insert(key.into(), v.clone());
        }
    }
    if let Some(e) = layer.get("epsg").and_then(Value::as_u64) {
        out.insert("crs".into(), json!(format!("EPSG:{e}")));
    }
    if layer.get("kind").and_then(Value::as_str) == Some("raster") {
        out.insert("geometry".into(), Value::Null);
        out.insert("fields".into(), json!([]));
        return Value::Object(out);
    }
    if let Some(g) = layer.get("geometry") {
        out.insert("geometry".into(), json!(g.as_str().unwrap_or("NoGeometry")));
    }
    if let Some(n) = layer.get("feature_count") {
        out.insert("feature_count".into(), n.clone());
    }
    if let Some(fields) = layer.get("fields").and_then(Value::as_array) {
        let fields: Vec<Value> = fields
            .iter()
            .map(|f| {
                let ty = match f["type"].as_str() {
                    Some("integer") => "Integer64",
                    Some("number") => "Real",
                    _ => "String",
                };
                let mut raw = json!({"name": f["name"], "type": ty});
                if let Some(values) = f.get("values") {
                    raw["values"] = values.clone();
                }
                raw
            })
            .collect();
        out.insert("fields".into(), json!(fields));
    }
    Value::Object(out)
}

/// Le projet de la forme apprise en clés brutes : nom, SCR, et une emprise en degrés dont le
/// centre tombe dans le fuseau UTM appris (31N vers 3° E, 32N vers 9° E, 30N vers 3° O).
fn raw_project(project: &Value, out: &mut serde_json::Map<String, Value>) {
    if let Some(name) = project.get("name") {
        out.insert("project_name".into(), name.clone());
    }
    if let Some(e) = project.get("epsg").and_then(Value::as_u64) {
        out.insert("project_crs".into(), json!(format!("EPSG:{e}")));
    }
    if let Some(code) = project.get("utm_epsg").and_then(Value::as_u64) {
        let (zone, lat) = match code {
            32601..=32660 => (code - 32600, 46.0),
            _ => (code - 32700, -20.0),
        };
        let lon = (zone * 6) as f64 - 183.0;
        out.insert(
            "extent".into(),
            json!({"lonlat": {"west": lon - 0.2, "south": lat - 0.1, "east": lon + 0.2,
                "north": lat + 0.1, "crs": "EPSG:4326"}}),
        );
    }
}

impl BigFiller {
    /// La couche en sortie brute de QGIS, dans le système `epsg`, avec son nom et le radical de
    /// son fichier : 10 à 25 champs (selon ce que la table en a), le premier toujours gardé.
    fn raw(&self, rng: &mut Rng, epsg: u64) -> (Value, String, String) {
        let name = (*rng.pick(self.names)).to_string();
        let id = format!("{}_{}", self.table, uuid_from(rng, ""));
        let (file, source) = match (self.producer, self.geom) {
            (_, Geom::Raster) => {
                let file = *rng.pick(self.files);
                let ext = *rng.pick(&["tif", "tif", "vrt", "jp2"]);
                (file.to_string(), format!("donnees/{file}.{ext}"))
            }
            (BdTopo, _) => {
                // Le GeoPackage de la BD TOPO est en Lambert-93 : jamais dans un projet en UTM.
                if epsg == 2154 && rng.chance(1, 2) {
                    let dep = *rng.pick(&["D074", "D094", "D091", "D035", "D067"]);
                    let gpkg = format!("BDT_3-5_GPKG_LAMB93_{dep}");
                    (
                        self.table.to_string(),
                        format!("donnees/{gpkg}.gpkg|layername={}", self.table),
                    )
                } else {
                    let file = self.table.to_ascii_uppercase();
                    (file.clone(), format!("donnees/bdtopo/{file}.shp"))
                }
            }
            (Cnig, _) => {
                let insee = *rng.pick(&["74010", "94077", "200066793", "35238"]);
                let file = format!("{insee}_{}", self.table);
                (file.clone(), format!("donnees/plu/{file}.shp"))
            }
            (Producer::Other, _) => {
                let file = *rng.pick(self.files);
                let ext = *rng.pick(&["gpkg", "shp", "geojson", "csv", "gpkg"]);
                (file.to_string(), format!("donnees/{file}.{ext}"))
            }
        };
        let mut layer = json!({
            "layer_id": id, "name": name, "source": source, "crs": format!("EPSG:{epsg}"),
        });
        if self.geom == Geom::Raster {
            layer["kind"] = json!("raster");
            layer["geometry"] = Value::Null;
            layer["fields"] = json!([]);
            return (layer, name, file);
        }
        layer["kind"] = json!("vector");
        layer["geometry"] = json!(match self.geom {
            Geom::Point => *rng.pick(&["Point", "PointZ", "MultiPoint"]),
            Geom::Line => *rng.pick(&["MultiLineString", "MultiLineStringZ", "LineString"]),
            _ => *rng.pick(&["MultiPolygon", "MultiPolygonZ", "Polygon"]),
        });
        layer["feature_count"] = json!(rng.range(1, 40_000));
        let all: Vec<(&str, Ft)> = match self.producer {
            BdTopo => [&BDTOPO_COMMON[..1], self.fields, &BDTOPO_COMMON[1..]].concat(),
            Cnig => [&CNIG_COMMON[..2], self.fields, &CNIG_COMMON[2..]].concat(),
            Producer::Other => self.fields.to_vec(),
        };
        let most = all.len().min(25);
        let n = rng.range(10.min(most as u64), most as u64) as usize;
        let mut drop: Vec<usize> = (1..all.len()).collect();
        rng.shuffle(&mut drop);
        drop.truncate(all.len() - n);
        let fields: Vec<Value> = all
            .iter()
            .enumerate()
            .filter(|(i, _)| !drop.contains(i))
            .map(|(_, (field, ty))| match ty {
                Ft::Text(values) => {
                    let mut f = json!({"name": field, "type": "String"});
                    if !values.is_empty() {
                        let k = rng.range(2.min(values.len() as u64), values.len() as u64);
                        let mut idx: Vec<usize> = (0..values.len()).collect();
                        rng.shuffle(&mut idx);
                        let chosen: Vec<&str> = idx
                            .into_iter()
                            .take(k as usize)
                            .map(|i| values[i])
                            .collect();
                        f["values"] = json!(chosen);
                    }
                    f
                }
                Ft::Int => json!({"name": field, "type": "Integer64"}),
                Ft::Real => json!({"name": field, "type": "Real"}),
                Ft::Date => json!({"name": field, "type": *rng.pick(&["Date", "DateTime"])}),
                Ft::Bool => json!({"name": field, "type": "Boolean"}),
            })
            .collect();
        layer["fields"] = json!(fields);
        (layer, name, file)
    }
}

/// Un gros projet de `total` couches (5.47), en sortie brute de `project.inventory` : les couches
/// de l'inventaire appris `learned`, retraduites, dans leur ordre, et des couches de remplissage
/// posées entre elles à des rangs tirés, jusqu'à `total` (pas une seule si une de leurs
/// désignations, nom, table ou fichier, partage un mot avec `request` : aucune ambiguïté non
/// voulue). Rend aussi l'identifiant QGIS de chaque couche apprise. `None` si le projet n'atteint
/// pas `BIG_MIN_LAYERS` couches.
pub fn big_project(
    rng: &mut Rng,
    learned: &Value,
    request: &str,
    total: usize,
) -> Option<(Value, Vec<(String, String)>)> {
    let layers = learned.get("layers")?.as_array()?;
    let mut ids = Vec::with_capacity(layers.len());
    let mut raw: Vec<Value> = Vec::with_capacity(total);
    for layer in layers {
        let id = layer.get("layer_id")?.as_str()?;
        let qgis = qgis_layer_id(rng, id);
        raw.push(raw_layer(layer, &qgis));
        ids.push((id.to_string(), qgis));
    }
    let said = plain_words(request);
    let epsg = learned["project"]["epsg"].as_u64().unwrap_or(2154);
    let mut pool: Vec<&BigFiller> = BIG_FILLERS.iter().collect();
    rng.shuffle(&mut pool);
    for filler in pool {
        if raw.len() >= total {
            break;
        }
        let (layer, name, file) = filler.raw(rng, epsg);
        let names = [name.as_str(), filler.table, file.as_str()];
        if names.iter().any(|n| !plain_words(n).is_disjoint(&said)) {
            continue;
        }
        let at = rng.below(raw.len() + 1);
        raw.insert(at, layer);
    }
    if raw.len() < BIG_MIN_LAYERS {
        return None;
    }
    let mut out = serde_json::Map::new();
    raw_project(&learned["project"], &mut out);
    out.insert("layers".into(), Value::Array(raw));
    Some((Value::Object(out), ids))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_keys_are_unique_and_known_fields_resolve() {
        let mut keys: Vec<&str> = THEMES.iter().map(|t| t.key).collect();
        keys.sort_unstable();
        let n = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), n);
        assert!(theme("zonage").field("zone").is_some());
        assert!(theme("gares").field("nope").is_none());
    }

    #[test]
    fn required_forced_and_forbidden_fields_are_honoured() {
        let mut rng = Rng::new(3);
        for _ in 0..50 {
            let mut b = Builder::new();
            let z = b.add(
                &mut rng,
                "zonage",
                &["zone"],
                &["libelle"],
                &[("zone", "U")],
            );
            let zone = z.field("zone").unwrap();
            assert!(zone.values.iter().any(|v| v.v == "U"));
            assert!(z.field("libelle").is_none());
            b.exclude("rivieres");
            let inv = b.finish(&mut rng);
            let layers = inv["layers"].as_array().unwrap();
            assert!(layers.iter().any(|l| l["layer_id"] == z.id.as_str()));
            // Thème exclu : ni couche, ni fichier.
            let text = inv.to_string();
            assert!(!text.contains("rivieres"), "{text}");
        }
    }

    #[test]
    fn values_are_sorted_and_no_file_is_listed_unless_the_family_adds_one() {
        let mut rng = Rng::new(9);
        for _ in 0..200 {
            let mut b = Builder::new();
            b.add(&mut rng, "zonage", &["zone"], &[], &[]);
            let inv = b.finish(&mut rng);
            assert_eq!(inv["unloaded_files"], json!([]));
            for layer in inv["layers"].as_array().unwrap() {
                for f in layer["fields"].as_array().unwrap() {
                    if let Some(values) = f["values"].as_array() {
                        let v: Vec<&str> = values.iter().map(|v| v.as_str().unwrap()).collect();
                        assert!(v.is_sorted(), "{v:?}");
                    }
                }
            }
        }
        let mut b = Builder::new();
        let path = b.add_file(&mut rng, "ecoles", None);
        let inv = b.finish(&mut rng);
        assert_eq!(inv["unloaded_files"][0]["path"], path.as_str());
    }

    #[test]
    fn some_inventories_take_the_shapes_qgis_gives() {
        let mut rng = Rng::new(4);
        let (mut no_name, mut no_source, mut no_epsg, mut table, mut raster_count) =
            (0, 0, 0, 0, 0);
        for _ in 0..600 {
            let mut b = Builder::new();
            let kept = b.add(&mut rng, "gares", &[], &[], &[]);
            b.add(&mut rng, "mnt", &[], &[], &[]);
            let inv = b.finish(&mut rng);
            no_name += usize::from(inv["project"].get("name").is_none());
            for l in inv["layers"].as_array().unwrap() {
                no_source += usize::from(l.get("source").is_none());
                no_epsg += usize::from(l.get("epsg").is_none());
                if l["kind"] == "vector" && l["geometry"].is_null() {
                    table += 1;
                    // Seule une couche de distraction perd sa géométrie.
                    assert_ne!(l["layer_id"], kept.id.as_str());
                }
                if l["kind"] == "raster" && l.get("feature_count").is_some() {
                    raster_count += 1;
                }
            }
        }
        assert!(no_name > 0 && no_source > 0 && no_epsg > 0 && table > 0);
        // La forme nominale reste majoritaire.
        assert!(raster_count > 300, "{raster_count}");
    }

    #[test]
    fn the_cited_dem_keeps_its_source_and_takes_the_chosen_system() {
        let mut rng = Rng::new(6);
        let mut no_epsg = 0;
        let mut sources = std::collections::BTreeSet::new();
        for i in 0..600 {
            let epsg = [2154, 32631, 32632][i % 3];
            let mut b = Builder::new();
            let dem = b.add_dem(&mut rng, epsg);
            assert_eq!(dem.epsg, epsg);
            assert!(dem.source.ends_with(".tif"), "{}", dem.source);
            if epsg != 2154 {
                assert!(!dem.source.contains("rge_alti"), "{}", dem.source);
            }
            sources.insert(
                dem.source
                    .rsplit_once('/')
                    .map_or("", |(d, _)| d)
                    .to_string(),
            );
            let inv = b.finish(&mut rng);
            let layer = inv["layers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|l| l["layer_id"] == dem.id.as_str())
                .unwrap();
            assert_eq!(layer["kind"], "raster");
            assert_eq!(layer["source"], dem.source.as_str());
            match layer.get("epsg") {
                Some(e) => assert_eq!(e, epsg),
                None => no_epsg += 1,
            }
        }
        assert!(no_epsg > 0);
        // Les formes de chemin relatif que rend l'extension depuis 4.3, nom seul compris.
        let dirs: Vec<&str> = sources.iter().map(String::as_str).collect();
        assert_eq!(dirs, ["", "../donnees", "dem", "donnees"]);
    }

    #[test]
    fn the_cited_cost_raster_keeps_its_source_and_takes_the_chosen_system() {
        // 11.12, lot 2b-1 : le raster de coût est cité comme le MNT, par sa `source`, que les
        // formes de QGIS ne retirent jamais ; son nom ne dit rien du relief.
        let mut rng = Rng::new(8);
        let mut no_epsg = 0;
        let mut dirs = std::collections::BTreeSet::new();
        for i in 0..600 {
            let epsg = [2154, 32631, 32632][i % 3];
            let mut b = Builder::new();
            let cost = b.add_raster(&mut rng, "cout", epsg);
            assert_eq!(cost.epsg, epsg);
            assert!(cost.source.ends_with(".tif"), "{}", cost.source);
            for word in ["pente", "mnt", "altitude", "relief"] {
                assert!(
                    !cost.source.to_lowercase().contains(word),
                    "{}",
                    cost.source
                );
                assert!(!cost.name.to_lowercase().contains(word), "{}", cost.name);
            }
            dirs.insert(
                cost.source
                    .rsplit_once('/')
                    .map_or("", |(d, _)| d)
                    .to_string(),
            );
            let inv = b.finish(&mut rng);
            let layer = inv["layers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|l| l["layer_id"] == cost.id.as_str())
                .unwrap();
            assert_eq!(layer["kind"], "raster");
            assert_eq!(layer["source"], cost.source.as_str());
            match layer.get("epsg") {
                Some(e) => assert_eq!(e, epsg),
                None => no_epsg += 1,
            }
        }
        assert!(no_epsg > 0);
        let dirs: Vec<&str> = dirs.iter().map(String::as_str).collect();
        assert_eq!(dirs, ["", "../donnees", "donnees", "rasters"]);
    }

    #[test]
    fn criteria_masks_and_raw_rasters_are_plain_rasters_cited_by_their_source() {
        // 11.12, lot 2b-2 : aucun nom ni fichier ne parle du relief ou du coût ; un masque est
        // dans le groupe de son vecteur ; la `source` citée reste après les formes de QGIS.
        let all: Vec<&str> = CRITERIA
            .iter()
            .chain(&MASKS)
            .chain(&RAW_RASTERS)
            .copied()
            .collect();
        for key in &all {
            let t = theme(key);
            assert_eq!(t.geom, Geom::Raster, "{key}");
            assert!(t.fields.is_empty(), "{key}");
            for text in t.names.iter().chain(t.files).chain(t.slugs) {
                let text = text.to_lowercase();
                for word in ["pente", "mnt", "altitude", "relief", "coût", "cout"] {
                    assert!(!text.contains(word), "{key} : {text}");
                }
            }
        }
        assert_eq!(theme("masque_inondable").group, theme("inondables").group);
        assert_eq!(theme("masque_protege").group, theme("znieff").group);
        assert_eq!(theme("distance_routes").group, theme("routes").group);
        let mut rng = Rng::new(10);
        for i in 0..400 {
            let key = all[i % all.len()];
            let mut b = Builder::new();
            let r = b.add_raster(&mut rng, key, 2154);
            let inv = b.finish(&mut rng);
            let layers = inv["layers"].as_array().unwrap();
            let layer = layers
                .iter()
                .find(|l| l["layer_id"] == r.id.as_str())
                .unwrap();
            assert_eq!(layer["source"], r.source.as_str());
            // Ni le vecteur de même sujet, ni un autre thème du groupe.
            let group = theme(key).group;
            for t in THEMES.iter().filter(|t| t.group == group && t.key != key) {
                assert!(
                    !layers.iter().any(|l| {
                        let id = l["layer_id"].as_str().unwrap();
                        // Le slug, puis un suffixe aléatoire (`reseau_elec_…` n'est pas `reseau`).
                        t.slugs.iter().any(|s| {
                            id.strip_prefix(&format!("{s}_")).is_some_and(|rest| {
                                rest.chars().all(|c| c == '_' || c.is_ascii_hexdigit())
                            })
                        })
                    }),
                    "{key} et {}",
                    t.key
                );
            }
        }
    }

    #[test]
    fn two_themes_of_the_same_group_never_meet() {
        let mut rng = Rng::new(5);
        for _ in 0..200 {
            let mut b = Builder::new();
            b.add(&mut rng, "routes", &[], &[], &[]);
            b.distract(&mut rng, 10, |_| true);
            assert!(b.layers.iter().all(|l| l.theme.key != "reseau"));
        }
    }

    #[test]
    fn no_filler_doubles_a_role_field_or_a_name() {
        // Un champ de remplissage ne prend ni le nom d'un champ de rôle du thème, ni un nom du
        // pool de noms, ni un nom de leurre du nom (casse comprise) : le plan ne cite que le
        // champ de rôle, et une question « aucun champ de … » reste juste.
        for t in THEMES {
            let Some(fill) = wide_fields(t.key) else {
                continue;
            };
            let mut seen = std::collections::HashSet::new();
            for (name, _) in fill {
                let low = name.to_lowercase();
                assert!(seen.insert(low.clone()), "{} : {name} en double", t.key);
                assert!(
                    !t.fields
                        .iter()
                        .any(|f| f.names.iter().any(|n| n.eq_ignore_ascii_case(name))),
                    "{} : {name}",
                    t.key
                );
                assert!(
                    !NAME_FIELDS.iter().any(|n| n.eq_ignore_ascii_case(name))
                        && !low.ends_with("_maj")
                        && !low.contains("majuscules")
                        && !low.starts_with("nom_")
                        && !low.contains("toponyme")
                        && !low.contains("libel"),
                    "{} : {name}",
                    t.key
                );
            }
            assert!(fill.len() + 1 >= WIDE_MIN, "{} : trop peu de champs", t.key);
        }
    }

    #[test]
    fn wide_layers_hold_15_to_20_fields_with_the_name_in_the_middle() {
        let mut rng = Rng::new(11);
        for key in [
            "communes",
            "routes",
            "batiments",
            "gares",
            "parcelles",
            "zonage",
        ] {
            let t = theme(key);
            let require: Vec<&str> = t.fields.iter().map(|f| f.role).collect();
            let (mut wide, n) = (0, 400);
            for _ in 0..n {
                let mut b = Builder::new();
                let layer = b.add(&mut rng, key, &require, &[], &[]);
                let count = layer.fields.len();
                if count < WIDE_MIN {
                    assert!(count <= 6, "{key} : {count} champs");
                    continue;
                }
                wide += 1;
                assert!(count <= WIDE_MAX, "{key} : {count} champs");
                let mut seen = std::collections::HashSet::new();
                for f in &layer.fields {
                    assert!(seen.insert(f.name.to_lowercase()), "{key} : {}", f.name);
                }
                // Chaque champ de rôle reste, une fois.
                for r in &require {
                    assert_eq!(
                        layer.fields.iter().filter(|f| f.def.role == *r).count(),
                        1,
                        "{key} : {r}"
                    );
                }
                if let Some(i) = layer.fields.iter().position(|f| f.def.role == "nom") {
                    // Dans la moitié du milieu, jamais en tête ni en fin de liste.
                    assert!(
                        i >= 2 && i + 2 < count,
                        "{key} : nom au rang {i} sur {count}"
                    );
                }
            }
            // Environ deux couches sur cinq (`WIDE_CHANCE` sur `WIDE_OUT_OF`).
            assert!((120..=200).contains(&wide), "{key} : {wide} sur {n}");
        }
        let json = {
            let mut b = Builder::new();
            b.wide_next();
            Layer::to_json(&b.add(&mut rng, "batiments", &[], &[], &[]))
        };
        let tys: std::collections::HashSet<&str> = json["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["type"].as_str().unwrap())
            .collect();
        assert!(tys.contains("string") && (tys.contains("integer") || tys.contains("number")));
    }

    #[test]
    fn at_most_two_layers_are_widened_unless_forced() {
        let mut rng = Rng::new(13);
        for _ in 0..300 {
            let mut b = Builder::new();
            for key in [
                "communes",
                "routes",
                "batiments",
                "gares",
                "parcelles",
                "zonage",
            ] {
                b.add(&mut rng, key, &[], &[], &[]);
            }
            let wide = b
                .layers
                .iter()
                .filter(|l| l.fields.len() >= WIDE_MIN)
                .count();
            assert!(wide <= MAX_WIDE_LAYERS, "{wide}");
        }
        let mut b = Builder::new();
        b.wide_next();
        let first = b.add(&mut rng, "communes", &[], &[], &[]);
        assert!(first.fields.len() >= WIDE_MIN);
        // L'élargissement forcé ne vaut que pour la couche suivante.
        let mut plain = 0;
        for _ in 0..50 {
            let mut b = Builder::new();
            b.wide_next();
            b.add(&mut rng, "quartiers", &[], &[], &[]);
            plain +=
                usize::from(b.add(&mut rng, "communes", &[], &[], &[]).fields.len() < WIDE_MIN);
        }
        assert!(plain > 15, "{plain}");
    }

    // -- gros projets (5.47) ---------------------------------------------------------------------

    #[test]
    fn a_learned_inventory_comes_back_through_the_raw_form_of_qgis() {
        use cassini_core::agent_inventory::agent_inventory;
        let (examples, _) = crate::generate(crate::DEFAULT_SEED, 600);
        for e in examples
            .iter()
            .filter(|e| e.inventory.get("index").is_none())
        {
            let layers: Vec<Value> = e.inventory["layers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|l| raw_layer(l, l["layer_id"].as_str().unwrap()))
                .collect();
            let mut raw = serde_json::Map::new();
            raw_project(&e.inventory["project"], &mut raw);
            raw.insert("layers".into(), json!(layers));
            let raw = Value::Object(raw);
            let back = agent_inventory(&raw);
            assert_eq!(back["project"], e.inventory["project"], "{}", e.request);
            let learned = e.inventory["layers"].as_array().unwrap();
            let back = back["layers"].as_array().unwrap();
            assert_eq!(learned.len(), back.len());
            for (l, b) in learned.iter().zip(back) {
                let mut l = l.clone();
                // QGIS ne compte pas les objets d'un raster.
                if l["kind"] == "raster" {
                    l.as_object_mut().unwrap().remove("feature_count");
                }
                assert_eq!(&l, b, "{}", e.request);
            }
        }
    }

    #[test]
    fn qgis_ids_keep_the_short_digits_the_set_shows() {
        use cassini_core::agent_inventory::ShortIds;
        let mut rng = Rng::new(1);
        for (id, short) in [
            ("gares_ab12", "gares_ab12"),
            ("gares_ab12cd34", "gares_ab12"),
            ("MNT_5_m_0f0f", "MNT_5_m_0f0f"),
        ] {
            let full = qgis_layer_id(&mut rng, id);
            assert!(is_qgis_uuid(&full), "{full}");
            let table = ShortIds::of([full.as_str()]);
            let shown = table.shorten(json!({"layers": [{"layer_id": full}]}));
            assert_eq!(shown["layers"][0]["layer_id"], short);
        }
        let uuid = "gares_0123abcd_0123_4567_89ab_0123456789ab";
        assert_eq!(qgis_layer_id(&mut rng, uuid), uuid);
    }

    #[test]
    fn a_big_project_holds_20_to_60_layers_and_no_filler_named_by_the_request() {
        let mut rng = Rng::new(4);
        let mut b = Builder::new();
        b.add(&mut rng, "gares", &[], &[], &[]);
        let learned = b.finish(&mut rng);
        let request = "Tampon de 300 m autour des gares et des plans d'eau";
        for total in [20, 40, 60] {
            let (raw, ids) = big_project(&mut rng, &learned, request, total).unwrap();
            let layers = raw["layers"].as_array().unwrap();
            // Les remplissages ne suffisent pas toujours à 60 couches autour d'un petit inventaire.
            assert!(
                layers.len() == total || (total == 60 && layers.len() >= 50),
                "{total}"
            );
            assert_eq!(ids.len(), learned["layers"].as_array().unwrap().len());
            assert!(!layers.iter().any(|l| l["name"] == "Plans d'eau"));
            for l in layers.iter().filter(|l| l["kind"] == "vector") {
                let n = l["fields"].as_array().unwrap().len();
                let learned_one = ids.iter().any(|(_, q)| l["layer_id"] == q.as_str());
                assert!(learned_one || (10..=25).contains(&n), "{l}");
            }
        }
        // Les jumelles : même thème, deux noms qualifiés, la copie garde la source.
        let mut b = Builder::new();
        let (a, c) = b.add_twins(&mut rng, "gares", "Gares", "2019", "2024", false);
        assert_eq!(
            (a.name.as_str(), c.name.as_str()),
            ("Gares 2019", "Gares 2024")
        );
        assert!(
            a.source.contains("_2019.") && c.source.contains("_2024."),
            "{a:?} {c:?}"
        );
        assert_eq!(b.focus_ids(), [a.id.clone(), c.id.clone()]);
        let mut b = Builder::new();
        let (a, c) = b.add_twins(&mut rng, "gares", "Gares", "", "copie", true);
        assert_eq!((a.name.as_str(), c.name.as_str()), ("Gares", "Gares copie"));
        assert_eq!(a.source, c.source);
        assert_ne!(a.id, c.id);
    }
}
