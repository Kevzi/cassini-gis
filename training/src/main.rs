//! `cassini-training` : régénère le jeu d'exemples d'entraînement.
//!
//!   cargo run -p cassini-training --release -- --seed 1 --count 8600 --out exemples.jsonl
//!
//!   cargo run -p cassini-training --release -- --format chat --seed 1 --out entrainement.jsonl
//!
//! `--format structured` (par défaut) écrit `{inventory, request, expected}` ; `--format chat`
//! écrit `{prompt, completion}`, le texte exact vu par le modèle (module `prompt`).
//! Le résumé par famille (gardés, écartés, doublons) s'imprime sur la sortie d'erreur.
//!
//! `--familles a,b` (5.54) restreint le tirage pondéré aux familles nommées : le jeu ciblé
//! d'une séance d'ajustement continu. Un nom inconnu sort en code 2, avec la liste des noms ;
//! un jeu ciblé qui reprend une phrase d'une barre (module `barres`) n'est pas écrit et sort en
//! code 3, en nommant l'identifiant de la phrase. `--familles-connues` imprime les noms des
//! familles, un par ligne.

use std::process::ExitCode;

use cassini_training::families::FAMILIES;
use cassini_training::{
    barres, generate, generate_families, to_chat_jsonl, to_jsonl, DEFAULT_COUNT, DEFAULT_SEED,
};

const USAGE: &str = "usage : cassini-training [--seed N] [--count M] [--format structured|chat] \
     [--familles a,b] --out <fichier.jsonl> | cassini-training --familles-connues";

fn main() -> ExitCode {
    let mut seed = DEFAULT_SEED;
    let mut count = DEFAULT_COUNT;
    let mut out = None;
    let mut chat = false;
    let mut families: Option<String> = None;
    if std::env::args()
        .skip(1)
        .eq(["--familles-connues".to_string()])
    {
        for family in FAMILIES {
            println!("{}", family.name);
        }
        return ExitCode::SUCCESS;
    }
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next();
        let parsed = match (arg.as_str(), value) {
            ("--seed", Some(v)) => v.parse().map(|v| seed = v).is_ok(),
            ("--count", Some(v)) => v.parse().map(|v| count = v).is_ok(),
            ("--format", Some(v)) => match v.as_str() {
                "structured" => {
                    chat = false;
                    true
                }
                "chat" => {
                    chat = true;
                    true
                }
                _ => false,
            },
            ("--familles", Some(v)) => {
                families = Some(v);
                true
            }
            ("--out", Some(v)) => {
                out = Some(v);
                true
            }
            _ => false,
        };
        if !parsed {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    }
    let Some(out) = out else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let (examples, summary) = match &families {
        None => generate(seed, count),
        Some(list) => {
            let names: Vec<&str> = list.split(',').map(str::trim).collect();
            match generate_families(seed, count, &names) {
                Ok(generated) => {
                    let copies = barres::copies(&generated.0);
                    if !copies.is_empty() {
                        eprintln!(
                            "jeu ciblé refusé, il reprend une phrase de barre : {}",
                            copies.join(", ")
                        );
                        return ExitCode::from(3);
                    }
                    generated
                }
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            }
        }
    };
    let text = if chat {
        to_chat_jsonl(&examples)
    } else {
        to_jsonl(&examples)
    };
    // Écrit sous `<out>.tmp`, puis renomme : un fichier coupé en pleine écriture ne porte
    // jamais le nom final, que la chaîne d'entraînement reprend sur `--reprendre`.
    let partial = format!("{out}.tmp");
    if let Err(e) = std::fs::write(&partial, text).and_then(|()| std::fs::rename(&partial, &out)) {
        eprintln!("écriture impossible de {out} : {e}");
        let _ = std::fs::remove_file(&partial);
        return ExitCode::FAILURE;
    }
    eprintln!(
        "graine {seed} : {} exemples écrits dans {out}",
        examples.len()
    );
    eprintln!(
        "{:<28} {:>7} {:>8} {:>9}",
        "famille", "gardés", "écartés", "doublons"
    );
    for (name, t) in &summary.families {
        eprintln!(
            "{name:<28} {:>7} {:>8} {:>9}",
            t.kept, t.rejected, t.duplicates
        );
    }
    for reason in &summary.reasons {
        eprintln!("écarté : {reason}");
    }
    if examples.len() < count {
        eprintln!("seulement {} exemples sur {count} demandés", examples.len());
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
