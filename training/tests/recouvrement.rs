//! Garde-fou du jeu tenu à l'écart (ticket 5.10) : aucune de ses phrases ne doit figurer
//! dans le jeu engendré avec la graine par défaut. Depuis 5.15, il couvre aussi les phrases
//! des essais réels de `ai/tests/` : `plan_reel` (les phrases de l'épreuve), `question_reel`,
//! `duree_reel` (le scénario de démo et son amorce), depuis 5.39 `echelle_utm_reel`, depuis 5.41
//! `terrain_reel`, depuis 11.16, `epic11_reel` et, depuis 11.23, `style_reel` et les points d'un
//! autre système de `epic11_reel` et, depuis 5.45, les 20 phrases de la recette QGIS d'Annecy
//! (`qgis-plugin/tests/recette/annecy.json`), puis depuis 5.53 la barre par commande
//! (`ai/tests/par_commande/phrases.json`) et les phrases neuves de `plan_reel` et de
//! `question_reel` (`ai/tests/plan_reel/phrases.json`, `ai/tests/question_reel/phrases.json`,
//! livrable 3, lot 1), puis celles de `echelle_utm_reel` et de `terrain_reel`
//! (`ai/tests/echelle_utm_reel/phrases.json`, `ai/tests/terrain_reel/phrases.json`, lot 2), puis
//! celles d'`epic11_reel` et de `style_reel` (`ai/tests/epic11_reel/phrases.json`,
//! `ai/tests/style_reel/phrases.json`, lot 3). Les familles visent leur sorte de demande
//! avec d'autres tournures ; elles ne les recopient jamais. Deux comparaisons, sur le texte normalisé
//! (minuscules, sans accents ni ponctuation) : à l'identique, puis à un nombre ou une commune
//! près (chiffres masqués, noms de communes remplacés par un même repère, des deux côtés).
//!
//! Le jeu tenu à l'écart a été écrit par un autre auteur que le générateur, qui ne l'a pas lu.
//! En cas d'échec, seul l'identifiant de la phrase est imprimé. On retire alors du générateur
//! la tournure fautive et on le signale ; on ne retouche jamais une famille pour coller au jeu
//! tenu à l'écart.

use std::collections::HashSet;

use cassini_training::barres::{
    self, held_out, masked, real_trials, real_trials_by_source, HAND_WRITTEN,
};
use cassini_training::families::FAMILIES;
use cassini_training::{
    generate, generate_families_unchecked, normalize, DEFAULT_COUNT, DEFAULT_SEED, FOCUS_COUNT,
    FOCUS_EXCLUDED, FOCUS_SEED,
};

/// Identifiants des phrases de `sentences` dont la forme `key` figure dans le jeu engendré.
fn overlap(sentences: Vec<(String, String)>, key: fn(&str) -> String) -> Vec<String> {
    let (examples, _) = generate(DEFAULT_SEED, DEFAULT_COUNT);
    assert_eq!(examples.len(), DEFAULT_COUNT);
    let generated: HashSet<String> = examples.iter().map(|e| key(&e.request)).collect();
    sentences
        .into_iter()
        .filter(|(_, text)| generated.contains(&key(text)))
        .map(|(id, _)| id)
        .collect()
}

#[test]
fn no_held_out_sentence_appears_in_the_generated_set() {
    let ids = overlap(held_out(), normalize);
    assert!(
        ids.is_empty(),
        "phrases tenues à l'écart engendrées telles quelles : {}",
        ids.join(", ")
    );
}

#[test]
fn no_held_out_sentence_appears_up_to_a_number_or_a_commune() {
    let ids = overlap(held_out(), masked);
    assert!(
        ids.is_empty(),
        "phrases tenues à l'écart engendrées à un nombre ou une commune près : {}",
        ids.join(", ")
    );
}

#[test]
fn no_sentence_of_the_real_trials_appears_in_the_generated_set() {
    let ids = overlap(real_trials(), masked);
    assert!(
        ids.is_empty(),
        "phrases des essais réels engendrées à un nombre ou une commune près : {}",
        ids.join(", ")
    );
}

/// Le jeu ciblé d'une séance d'ajustement continu (5.54) est tiré à `FOCUS_SEED` sur les seules
/// familles choisies : chaque famille, tirée seule à cette graine jusqu'à `FOCUS_COUNT`
/// exemples, passe les mêmes contrôles que le jeu complet (jeu tenu à l'écart à l'identique et à
/// un nombre ou une commune près, essais réels à un nombre ou une commune près). Une famille
/// trop petite pour `FOCUS_COUNT` exemples neufs rend ce qu'elle peut : tout est contrôlé. Les
/// familles qui ne les passent pas sont exactement celles de `FOCUS_EXCLUDED`, que le
/// générateur refuse au jeu ciblé ; une famille réparée en sort, une famille neuve qui copie
/// une phrase y entre ou se répare (seul l'identifiant de la phrase est imprimé).
#[test]
fn no_family_drawn_alone_at_the_focus_seed_copies_a_held_out_or_real_trial_sentence() {
    let mut copies = Vec::new();
    let mut leaking = Vec::new();
    for family in FAMILIES {
        let (examples, _) =
            generate_families_unchecked(FOCUS_SEED, FOCUS_COUNT, &[family.name]).unwrap();
        let found = barres::copies(&examples);
        if !found.is_empty() {
            leaking.push(family.name);
            copies.extend(found.into_iter().map(|c| format!("{} : {c}", family.name)));
        }
    }
    let mut excluded = FOCUS_EXCLUDED.to_vec();
    excluded.sort_unstable();
    leaking.sort_unstable();
    assert_eq!(
        leaking,
        excluded,
        "familles qui, tirées seules, engendrent une phrase tenue à l'écart ou d'un essai réel \
         ({}), contre FOCUS_EXCLUDED",
        copies.join(", ")
    );
}

/// Les phrases écrites à la main pour 5.53 (barre par commande, phrases neuves de `plan_reel`,
/// `question_reel`, `echelle_utm_reel`, `terrain_reel`, `epic11_reel` et `style_reel`) sont
/// écrites à neuf : aucune ne reprend, à un nombre ou une commune près, une phrase d'une autre
/// source d'essai réel, ni une autre phrase de son propre fichier.
#[test]
fn no_hand_written_sentence_copies_another_real_trial() {
    let sources = real_trials_by_source();
    let mut copies = Vec::new();
    for (name, _, _) in HAND_WRITTEN {
        let (_, mine) = sources.iter().find(|(n, _)| *n == name).unwrap();
        let others: HashSet<String> = sources
            .iter()
            .filter(|(n, _)| *n != name)
            .flat_map(|(_, s)| s.iter().map(|(_, text)| masked(text)))
            .collect();
        let mut own = HashSet::new();
        for (id, text) in mine {
            let key = masked(text);
            if others.contains(&key) || !own.insert(key) {
                copies.push(format!("{name}:{id}"));
            }
        }
    }
    assert!(
        copies.is_empty(),
        "phrases écrites à la main reprises d'un autre essai ou en double : {}",
        copies.join(", ")
    );
}

/// Le contrôle anti-copie retrouve chaque source par son nom : deux sources du même nom
/// confondraient leurs phrases.
#[test]
fn the_real_trial_sources_have_distinct_names() {
    let sources = real_trials_by_source();
    let mut names = HashSet::new();
    for (name, _) in &sources {
        assert!(names.insert(*name), "source en double : {name}");
    }
    for (name, _, _) in HAND_WRITTEN {
        assert!(
            names.contains(name),
            "source écrite à la main absente : {name}"
        );
    }
}

#[test]
fn the_real_trials_are_read_whole() {
    let all = real_trials();
    assert_eq!(
        all.len(),
        25 + 12
            + 8
            + 1
            + 8
            + 8
            + 28
            + 1
            + 4
            + 10
            + 20
            + HAND_WRITTEN.iter().map(|(_, _, n)| n).sum::<usize>()
    );
    let find = |id: &str| all.iter().find(|(i, _)| i == id).unwrap().1.clone();
    assert_eq!(
        find("Q11"),
        "Garde les parcelles qui appartiennent à la commune."
    );
    assert!(find("A0").ends_with("et sors-moi la carte en PDF."));
    assert_eq!(find("amorce"), "Décris le projet.");
    assert_eq!(find("E01"), "Exporte la carte au 1:25 000.");
    assert_eq!(
        find("X02"),
        "Je voudrais une impression de la carte au 10 000e."
    );
    assert_eq!(
        find("F02"),
        "Sors la carte en PDF au format A2 avec un carroyage Lambert-93 tous les 1 km."
    );
    assert_eq!(
        find("F03"),
        "Fais un PDF de la carte avec un carroyage UTM au kilomètre."
    );
    assert!(find("Y04").contains("UTM 32N"));
    assert_eq!(find("ST10"), "Colore les gares selon leur fréquentation.");
    assert_eq!(find("RA09"), "Fais un tampon autour des routes.");
    assert_eq!(find("PK1"), "Où sont les sommets du MNT ?");
    assert_eq!(
        find("PL23"),
        "Trace les courbes de niveau du MNT tous les 5 m."
    );
    assert_eq!(find("QR35"), "Délimite le bassin versant à partir du MNT.");
    assert_eq!(
        find("EU36"),
        "Donne le temps de trajet en voiture entre le point 451 000 / 5 409 000 et le point 457 000 / 5 401 000, UTM 31N."
    );
    assert_eq!(
        find("TR36"),
        "Calcule le rayonnement solaire reçu par le terrain sur une année."
    );
    assert_eq!(
        find("EP20"),
        "Calcule l'indice de rugosité du terrain à partir du MNT."
    );
    assert_eq!(
        find("SR32"),
        "Représente la population des communes par des cercles proportionnels."
    );
}

#[test]
fn the_normalization_makes_trivial_variants_collide() {
    assert_eq!(
        normalize("Trouve les écoles à moins de 300 m d'une rivière."),
        normalize("trouve les ecoles a moins de 300 m d une riviere")
    );
    assert_eq!(
        masked("Les gares à moins de 500 m, à Villeneuve-le-Roi."),
        masked("les gares a moins de 1 200 m a Orly")
    );
}
