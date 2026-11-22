//! Garde-fou hors catalogue (ticket 35) : refuser au lieu d'inventer un plan.
//!
//! Quand la demande porte sur une opération que le catalogue n'a pas, le modèle rend parfois un
//! plan inventé, un plan invalide ou une question sans rapport. Une table de familles d'opérations
//! absentes, lue avant toute génération, refuse alors tout de suite, sans appel au modèle.
//!
//! Chaque famille associe des motifs de la demande (texte normalisé : sans casse ni accents, les
//! apostrophes et les traits d'union lus comme des espaces) à un mot-clé `missing`. Une famille
//! ne vaut que tant qu'**aucun identifiant du registre** ne porte son mot-clé : si le catalogue
//! l'acquiert, le garde se tait pour elle (même règle que `refusals_now_wrong` des essais).
//! Les motifs sont étroits (verbe et objet) : une phrase couverte voisine n'est jamais refusée.
//! L'ordre des familles est fixe ; la première qui correspond répond.

use crate::registry::Registry;

/// Une famille d'opérations absentes du catalogue.
struct Family {
    /// Mot-clé `missing` : absent de tout identifiant du registre tant que la famille vaut.
    missing: &'static str,
    /// L'opération absente, en français, pour la phrase de refus.
    label: &'static str,
    /// Vrai si la demande normalisée relève de la famille.
    matches: fn(&str) -> bool,
}

fn any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(w))
}

const FAMILIES: [Family; 10] = [
    Family {
        missing: "label",
        label: "l'étiquetage de la carte (afficher un champ en étiquette)",
        // Le contenu à afficher est exigé : une « étiquette » de légende ou de mise en page n'est
        // pas l'étiquetage des entités.
        matches: |t| {
            any(t, &["etiquet", "labeliser"])
                && any(
                    t,
                    &["nom de", "noms", "valeur", "champ", "attribut", "chaque"],
                )
        },
    },
    Family {
        missing: "clip",
        label: "la découpe d'un modèle de terrain (ou d'un raster) selon une emprise",
        matches: |t| {
            // Une emprise est exigée : « Découpe le raster de coût en deux classes » est une
            // reclassification, couverte (raster.reclassify).
            any(t, &["decoup", "clip", "recadr"])
                && any(t, &["emprise", "etendue", "masque", "limites de"])
                && any(
                    t,
                    &[
                        "modele de terrain",
                        "modele numerique",
                        "mnt",
                        "raster",
                        "altitude",
                    ],
                )
        },
    },
    Family {
        missing: "profile",
        label: "le profil d'altitude le long d'une ligne",
        matches: |t| {
            any(
                t,
                &[
                    "profil d altitude",
                    "profil altimetrique",
                    "profil en long",
                    "profil topographique",
                    "profil d elevation",
                    "profil de terrain",
                ],
            )
        },
    },
    Family {
        missing: "raster_calculator",
        label: "la calculatrice raster (opération cellule à cellule entre deux rasters)",
        matches: |t| {
            (any(
                t,
                &[
                    "soustrai",
                    "retranch",
                    "additionn",
                    "multipli",
                    "calculatrice raster",
                ],
            ) || t.contains("difference entre"))
                && any(
                    t,
                    &[
                        "modele de terrain",
                        "modele numerique",
                        "mnt",
                        "raster",
                        "releve de terrain",
                    ],
                )
        },
    },
    Family {
        missing: "watershed",
        label: "la délimitation d'un bassin versant",
        matches: |t| any(t, &["bassin versant", "bassin hydrographique", "watershed"]),
    },
    Family {
        missing: "visibility_stat",
        label: "la statistique par zone d'un champ de vision",
        matches: |t| {
            any(t, &["champ de vision", "viewshed", "visibilite"])
                && any(t, &["moyenne", "statistique"])
        },
    },
    // Quatre familles de terrain de plus (2026-11-22, `base-22-2` : le modèle entraîné de zéro ne
    // refusait plus TR33 à TR35, EP18 et EP20). Même règle : verbe et objet, jamais une couche
    // du projet (« zones inondables » est une couche du PLU, couverte).
    Family {
        missing: "flood",
        label: "la simulation d'une crue ou d'une inondation sur le terrain",
        matches: |t| {
            any(
                t,
                &[
                    "simul",
                    "propagation",
                    "extension de la crue",
                    "extension d une crue",
                ],
            ) && any(t, &["crue", "inondation"])
                && any(
                    t,
                    &[
                        "modele de terrain",
                        "modele numerique",
                        "mnt",
                        "raster",
                        "terrain",
                    ],
                )
        },
    },
    Family {
        missing: "flow_accum",
        label: "l'accumulation des écoulements",
        matches: |t| t.contains("accumulation") && t.contains("ecoulement"),
    },
    Family {
        missing: "cut_fill",
        label: "la cubature de déblai et de remblai",
        matches: |t| {
            any(t, &["cubature", "araser"]) || (t.contains("deblai") && t.contains("remblai"))
        },
    },
    Family {
        missing: "rough",
        label: "l'indice de rugosité du terrain",
        matches: |t| {
            any(t, &["rugosite", "roughness"])
                && any(
                    t,
                    &[
                        "modele de terrain",
                        "modele numerique",
                        "mnt",
                        "raster",
                        "terrain",
                        "relief",
                    ],
                )
        },
    },
];

/// Texte de comparaison : minuscules, sans accents, apostrophes et traits d'union en espaces.
fn normalize(request: &str) -> String {
    request
        .chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            '\'' | '’' | '‘' | '`' | '-' | '\u{2011}' | '\u{a0}' => ' ',
            other => other,
        })
        .collect()
}

/// La raison d'un refus si `request` demande une opération que le catalogue n'a pas, sinon `None`.
/// Pure : la demande et le registre seuls, sans réseau ni modèle.
pub fn guard_out_of_catalogue(registry: &Registry, request: &str) -> Option<String> {
    let ids: Vec<&str> = registry.specs().map(|s| s.id.as_str()).collect();
    guard_with_ids(&ids, request)
}

fn guard_with_ids(ids: &[&str], request: &str) -> Option<String> {
    let text = normalize(request);
    FAMILIES
        .iter()
        .filter(|f| !ids.iter().any(|id| id.contains(f.missing)))
        .find(|f| (f.matches)(&text))
        .map(|f| {
            format!(
                "Cette opération n'existe pas dans le catalogue de Cassini : {}.",
                f.label
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(request: &str) -> Option<String> {
        guard_out_of_catalogue(&Registry::standard(), request)
    }

    #[test]
    fn each_absent_family_is_refused_with_its_name() {
        for (request, name) in [
            ("Mets en étiquette le nom de chaque commune sur la carte.", "étiquetage"),
            ("Découpe le modèle de terrain selon l'emprise de la commune.", "découpe"),
            ("Fais un profil d'altitude le long de la route nationale.", "profil d'altitude"),
            (
                "Soustrais le relevé de terrain de 2019 au modèle de terrain actuel.",
                "calculatrice raster",
            ),
            (
                "Délimite le bassin versant du ruisseau qui passe derrière la gare.",
                "bassin versant",
            ),
            (
                "Calcule le champ de vision depuis le point 656 120 / 6 848 060 (Lambert-93), à 25 m de haut, sur 2 km, puis sa moyenne par commune.",
                "champ de vision",
            ),
            ("Simule l'extension d'une crue de 2 m sur le MNT.", "crue"),
            ("Calcule l'accumulation d'écoulement sur le MNT.", "écoulements"),
            ("Calcule l'accumulation des écoulements sur le MNT.", "écoulements"),
            (
                "Calcule la cubature de déblai et de remblai pour araser le terrain à la cote 42 m.",
                "cubature",
            ),
            ("Calcule l'indice de rugosité du terrain à partir du MNT.", "rugosité"),
        ] {
            let reason = refused(request).unwrap_or_else(|| panic!("non refusée : {request}"));
            assert!(reason.contains(name), "{request} -> {reason}");
        }
    }

    #[test]
    fn covered_neighbours_are_left_to_the_model() {
        for request in [
            "Découpe les parcelles par la commune.",
            "Soustrais la zone bâtie des parcelles.",
            "Découpe les parcelles selon les zones naturelles du PLU.",
            "Découpe le raster de coût en deux classes : 1 sous 20, 2 à partir de 20.",
            "Affiche la légende et l'étiquette de la couche dans la mise en page.",
            "Quels sont les moyens de visibilité depuis ce point ?",
            "Donne l'altitude moyenne de chaque commune d'après le modèle de terrain.",
            "Calcule la pente en degrés puis sa moyenne par commune.",
            "Donne-moi l'altitude moyenne et maximale par commune.",
            "Calcule le champ de vision depuis ce point sur 2 km.",
            "Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à Annecy.",
            "Montre les zones inondables du PLU sur la carte.",
            "Superposition pondérée : accessibilité poids 2, calme poids 5, hors zones inondables.",
            "Intitule la carte « Accumulation de population par commune ».",
            "Quel est le terrain de sport le plus proche de la gare ?",
        ] {
            assert_eq!(refused(request), None, "{request}");
        }
    }

    #[test]
    fn matching_ignores_case_accents_and_apostrophes() {
        assert!(refused("FAIS UN PROFIL D’ALTITUDE").is_some());
        assert!(refused("fais un profil d'altitude").is_some());
        assert!(refused("Delimite le BASSIN-VERSANT").is_some());
    }

    #[test]
    fn a_family_is_silent_when_the_catalogue_has_its_keyword() {
        let request = "Découpe le modèle de terrain selon l'emprise de la commune.";
        assert!(guard_with_ids(&["analysis.buffer"], request).is_some());
        assert_eq!(
            guard_with_ids(&["analysis.buffer", "raster.clip"], request),
            None
        );
        // Seule la famille concernée se tait.
        assert!(guard_with_ids(&["raster.clip"], "Fais un profil d'altitude.").is_some());
        // Le catalogue actuel ne porte aucun des six mots-clés.
        let registry = Registry::standard();
        for f in &FAMILIES {
            assert!(
                !registry.specs().any(|s| s.id.contains(f.missing)),
                "{}",
                f.missing
            );
        }
    }

    #[test]
    fn already_refused_requests_are_not_touched() {
        assert_eq!(
            refused("Transforme les bâtiments de cette image satellite en polygones."),
            None
        );
        assert_eq!(refused("Télécharge le cadastre de la commune."), None);
    }
}
