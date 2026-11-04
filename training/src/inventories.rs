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

const NOM: FieldDef = FieldDef {
    role: "nom",
    names: &["nom", "libelle", "nom_usuel"],
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
                names: &["nom", "nom_commune", "libelle"],
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
                names: &["nom", "nom_voie"],
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
                names: &["nom", "toponyme"],
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
                names: &["nom", "nom_etab"],
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
];

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
            fields.push(FieldInst { def, name, values });
        }
        let layer = Layer {
            theme: t,
            id: layer_id(rng, slug),
            name: (*rng.pick(t.names)).to_string(),
            source: format!("donnees/{file}.{ext}"),
            epsg: 2154,
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
        self.sourced.push(self.layers.len());
        let mut layer = self.add(rng, "mnt", &[], &[], &[]);
        let file = if epsg == 2154 {
            (*rng.pick(theme("mnt").files)).to_string()
        } else {
            (*rng.pick(&["mnt", "mnt_5m", "mnt_25m"])).to_string()
        };
        let dir = *rng.pick(&["donnees/", "donnees/", "dem/", "../donnees/", ""]);
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

    /// Termine l'inventaire : distractions par défaut, ordre mélangé. Les fichiers non ouverts
    /// sont ceux que la famille a posés (`add_file`), et seulement eux : dans QGIS, l'extension
    /// ne parcourt pas le disque et `unloaded_files` est toujours vide.
    ///
    /// Une part des inventaires prend les formes que rend le produit depuis QGIS
    /// (`core/src/agent_inventory.rs`, clé absente plutôt que `null`) : raster sans nombre
    /// d'objets, couche temporaire sans `source`, SCR qui n'est pas EPSG (sans `epsg`), table
    /// sans géométrie (`geometry: null` sur un vecteur), projet non enregistré (sans `name`).
    pub fn finish(mut self, rng: &mut Rng) -> Value {
        let n = rng.range(1, 4) as usize;
        self.distract(rng, n, |t| t.geom != Geom::Raster);
        if rng.chance(1, 4) {
            self.distract(rng, 1, |t| t.geom == Geom::Raster);
        }
        let mut layers: Vec<Value> = self.layers.iter().map(Layer::to_json).collect();
        let mut project = json!({"name": rng.pick(PROJECTS), "epsg": 2154});
        if rng.chance(1, 3) {
            qgis_shapes(
                rng,
                &mut layers,
                &self.distractors,
                &self.sourced,
                &mut project,
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
) {
    let project = project.as_object_mut().expect("projet objet");
    if rng.chance(1, 4) {
        project.remove("name");
    }
    if rng.chance(1, 8) {
        project.remove("epsg");
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
        if rng.chance(1, 12) {
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
    fn two_themes_of_the_same_group_never_meet() {
        let mut rng = Rng::new(5);
        for _ in 0..200 {
            let mut b = Builder::new();
            b.add(&mut rng, "routes", &[], &[], &[]);
            b.distract(&mut rng, 10, |_| true);
            assert!(b.layers.iter().all(|l| l.theme.key != "reseau"));
        }
    }
}
