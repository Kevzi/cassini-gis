//! `cassini-training` : régénère le jeu d'exemples d'entraînement.
//!
//!   cargo run -p cassini-training --release -- --seed 1 --count 8000 --out exemples.jsonl
//!
//!   cargo run -p cassini-training --release -- --format chat --seed 1 --out entrainement.jsonl
//!
//! `--format structured` (par défaut) écrit `{inventory, request, expected}` ; `--format chat`
//! écrit `{prompt, completion}`, le texte exact vu par le modèle (module `prompt`).
//! Le résumé par famille (gardés, écartés, doublons) s'imprime sur la sortie d'erreur.

use std::process::ExitCode;

use cassini_training::{generate, to_chat_jsonl, to_jsonl, DEFAULT_COUNT, DEFAULT_SEED};

const USAGE: &str =
    "usage : cassini-training [--seed N] [--count M] [--format structured|chat] --out <fichier.jsonl>";

fn main() -> ExitCode {
    let mut seed = DEFAULT_SEED;
    let mut count = DEFAULT_COUNT;
    let mut out = None;
    let mut chat = false;
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
    let (examples, summary) = generate(seed, count);
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
