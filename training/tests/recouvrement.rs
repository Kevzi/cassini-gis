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

use cassini_training::families::COMMUNES;
use cassini_training::{generate, normalize, DEFAULT_COUNT, DEFAULT_SEED};
use serde_json::Value;

const HELD_OUT: &str = include_str!("../../ai/tests/tenu_a_l_ecart/sentences.json");
/// Les phrases de `plan_reel` : celles de l'épreuve du modèle local, qu'il lit telles quelles.
const PLAN_REEL: &str = include_str!("../../tests/epreuve-modele-local/sentences.json");
const QUESTION_REEL: &str = include_str!("../../ai/tests/question_reel.rs");
const DUREE_REEL: &str = include_str!("../../ai/tests/duree_reel.rs");
const ECHELLE_UTM_REEL: &str = include_str!("../../ai/tests/echelle_utm_reel.rs");
const TERRAIN_REEL: &str = include_str!("../../ai/tests/terrain_reel.rs");
const EPIC11_REEL: &str = include_str!("../../ai/tests/epic11_reel.rs");
const STYLE_REEL: &str = include_str!("../../ai/tests/style_reel.rs");
/// Les phrases de la recette QGIS d'Annecy (5.45), écrites à la main, jouées sous QGIS.
const RECETTE_ANNECY: &str = include_str!("../../qgis-plugin/tests/recette/annecy.json");
/// Les phrases de la barre par commande (5.53), écrites à la main : `{id, command, phrase, …}`.
const PAR_COMMANDE: &str = include_str!("../../ai/tests/par_commande/phrases.json");
/// Les phrases neuves de `plan_reel` (5.53, livrable 3), écrites à la main : `{id, phrase, …}`.
const PLAN_REEL_PHRASES: &str = include_str!("../../ai/tests/plan_reel/phrases.json");
/// Les phrases neuves de `question_reel` (5.53, livrable 3), écrites à la main.
const QUESTION_REEL_PHRASES: &str = include_str!("../../ai/tests/question_reel/phrases.json");
/// Les phrases neuves de `echelle_utm_reel` (5.53, livrable 3, lot 2), écrites à la main.
const ECHELLE_UTM_REEL_PHRASES: &str = include_str!("../../ai/tests/echelle_utm_reel/phrases.json");
/// Les phrases neuves de `terrain_reel` (5.53, livrable 3, lot 2), écrites à la main.
const TERRAIN_REEL_PHRASES: &str = include_str!("../../ai/tests/terrain_reel/phrases.json");
/// Les phrases neuves d'`epic11_reel` (5.53, livrable 3, lot 3), écrites à la main.
const EPIC11_REEL_PHRASES: &str = include_str!("../../ai/tests/epic11_reel/phrases.json");
/// Les phrases neuves de `style_reel` (5.53, livrable 3, lot 3), écrites à la main.
const STYLE_REEL_PHRASES: &str = include_str!("../../ai/tests/style_reel/phrases.json");

/// Les fichiers `phrases.json` écrits à la main pour 5.53, chacun avec son nom et son compte.
const HAND_WRITTEN: [(&str, &str, usize); 7] = [
    ("par_commande", PAR_COMMANDE, 99),
    ("plan_reel", PLAN_REEL_PHRASES, 23),
    ("question_reel", QUESTION_REEL_PHRASES, 35),
    ("echelle_utm_reel", ECHELLE_UTM_REEL_PHRASES, 36),
    ("terrain_reel", TERRAIN_REEL_PHRASES, 36),
    // Nommées à part des constantes de leur essai (`epic11_reel`, `style_reel`), comparées à elles.
    ("epic11_reel:phrases", EPIC11_REEL_PHRASES, 20),
    ("style_reel:phrases", STYLE_REEL_PHRASES, 32),
];

/// Texte normalisé, chiffres masqués et communes remplacées par un même repère.
fn masked(text: &str) -> String {
    let mut places: Vec<String> = COMMUNES.iter().map(|c| normalize(c)).collect();
    places.push("villeneuve le roi".to_string());
    // Les noms les plus longs d'abord : « villeneuve le roi » avant un nom plus court inclus.
    places.sort_by_key(|p| std::cmp::Reverse(p.len()));
    let mut out = format!(" {} ", normalize(text));
    for place in &places {
        out = out.replace(&format!(" {place} "), " lieu ");
    }
    let mut masked = String::new();
    let mut in_digits = false;
    for c in out.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                masked.push('0');
            }
            in_digits = true;
        } else {
            masked.push(c);
            in_digits = false;
        }
    }
    // « 1 200 » est un seul nombre : des nombres consécutifs n'en font qu'un.
    let mut words: Vec<&str> = masked.split_whitespace().collect();
    words.dedup_by(|a, b| *a == "0" && *b == "0");
    words.join(" ")
}

fn held_out() -> Vec<(String, String)> {
    from_json(HELD_OUT)
}

/// Les phrases `{id, text}` d'un fichier `sentences.json`.
fn from_json(text: &str) -> Vec<(String, String)> {
    let sentences: Vec<Value> = serde_json::from_str(text).expect("sentences.json lisible");
    assert!(!sentences.is_empty());
    sentences
        .iter()
        .map(|s| {
            (
                s["id"].as_str().expect("id").to_string(),
                s["text"].as_str().expect("texte").to_string(),
            )
        })
        .collect()
}

/// Les phrases `{id, phrase}` d'un `phrases.json` écrit à la main (5.53).
fn from_phrases(text: &str) -> Vec<(String, String)> {
    let phrases: Vec<Value> = serde_json::from_str(text).expect("phrases.json lisible");
    phrases
        .iter()
        .map(|p| {
            (
                p["id"].as_str().expect("id").to_string(),
                p["phrase"].as_str().expect("phrase").to_string(),
            )
        })
        .collect()
}

/// Les littéraux de chaîne de la valeur de la constante `start` (après son `=`, jusqu'au
/// premier `];` ou `);`) : les paires `("id", "texte")` d'un tableau de phrases.
fn literals(source: &str, start: &str) -> Vec<String> {
    let from = source
        .find(start)
        .unwrap_or_else(|| panic!("bloc « {start} » introuvable"));
    let block = &source[from + start.len()..];
    let block = &block[block.find('=').expect("valeur de la constante") + 1..];
    let end = block
        .find("];")
        .into_iter()
        .chain(block.find(");"))
        .min()
        .expect("fin du bloc");
    let mut out = Vec::new();
    let mut chars = block[..end].chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut text = String::new();
        while let Some(c) = chars.next() {
            match c {
                '\\' => text.extend(chars.next()),
                '"' => break,
                c => text.push(c),
            }
        }
        out.push(text);
    }
    out
}

/// Les paires `(id, texte)` d'un tableau de phrases écrit dans un fichier Rust.
fn pairs(source: &str, start: &str) -> Vec<(String, String)> {
    let found = literals(source, start);
    assert!(
        !found.is_empty() && found.len().is_multiple_of(2),
        "« {start} » : {found:?}"
    );
    found
        .chunks(2)
        .map(|p| (p[0].clone(), p[1].clone()))
        .collect()
}

/// Les phrases des essais réels de `ai/tests/` : `plan_reel`, `question_reel`, `duree_reel`,
/// `echelle_utm_reel`, `terrain_reel`, `epic11_reel`, `style_reel`, la recette QGIS d'Annecy et,
/// depuis 5.53, la barre par commande puis les phrases neuves de `plan_reel`, `question_reel`,
/// `echelle_utm_reel`, `terrain_reel`, `epic11_reel` et `style_reel`.
fn real_trials() -> Vec<(String, String)> {
    real_trials_by_source()
        .into_iter()
        .flat_map(|(_, s)| s)
        .collect()
}

/// Les phrases des essais réels, groupées par source (fichier ou constante qui les porte).
fn real_trials_by_source() -> Vec<(&'static str, Vec<(String, String)>)> {
    let mut all = vec![("plan_reel:epreuve", from_json(PLAN_REEL))];
    let questions = pairs(QUESTION_REEL, "const AMBIGUOUS");
    assert_eq!(questions.len(), 12, "{questions:?}");
    all.push(("question_reel:ambiguous", questions));
    let scenario = pairs(DUREE_REEL, "const SCENARIO");
    assert_eq!(scenario.len(), 8, "{scenario:?}");
    all.push(("duree_reel:scenario", scenario));
    all.push(("duree_reel:warmup", pairs(DUREE_REEL, "const WARMUP")));
    let scale_utm = pairs(ECHELLE_UTM_REEL, "const SENTENCES");
    assert_eq!(scale_utm.len(), 8, "{scale_utm:?}");
    all.push(("echelle_utm_reel:sentences", scale_utm));
    let terrain = pairs(TERRAIN_REEL, "const SENTENCES");
    assert_eq!(terrain.len(), 8, "{terrain:?}");
    all.push(("terrain_reel:sentences", terrain));
    let epic11 = pairs(EPIC11_REEL, "const SENTENCES");
    assert_eq!(epic11.len(), 28, "{epic11:?}");
    all.push(("epic11_reel", epic11));
    all.push((
        "epic11_reel:grid_question",
        pairs(EPIC11_REEL, "const GRID_QUESTION"),
    ));
    let foreign = pairs(EPIC11_REEL, "const FOREIGN_POINTS");
    assert_eq!(foreign.len(), 4, "{foreign:?}");
    all.push(("epic11_reel:foreign_points", foreign));
    let style = pairs(STYLE_REEL, "const SENTENCES");
    assert_eq!(style.len(), 10, "{style:?}");
    all.push(("style_reel", style));
    let annecy = from_json(RECETTE_ANNECY);
    assert_eq!(annecy.len(), 20, "{annecy:?}");
    all.push(("recette:annecy", annecy));
    for (name, text, count) in HAND_WRITTEN {
        let phrases = from_phrases(text);
        assert_eq!(phrases.len(), count, "{name} : {phrases:?}");
        all.push((name, phrases));
    }
    all
}

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
