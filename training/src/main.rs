//! `cassini-training` : régénère le jeu d'exemples d'entraînement.
//!
//!   cargo run -p cassini-training --release -- --seed 1 --count 5000 --out exemples.jsonl
//!
//! Le résumé par famille (gardés, écartés, doublons) s'imprime sur la sortie d'erreur.

use std::process::ExitCode;

use cassini_training::{generate, to_jsonl, DEFAULT_COUNT, DEFAULT_SEED};

const USAGE: &str = "usage : cassini-training [--seed N] [--count M] --out <fichier.jsonl>";

fn main() -> ExitCode {
    let mut seed = DEFAULT_SEED;
    let mut count = DEFAULT_COUNT;
    let mut out = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next();
        let parsed = match (arg.as_str(), value) {
            ("--seed", Some(v)) => v.parse().map(|v| seed = v).is_ok(),
            ("--count", Some(v)) => v.parse().map(|v| count = v).is_ok(),
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
    if let Err(e) = std::fs::write(&out, to_jsonl(&examples)) {
        eprintln!("écriture impossible de {out} : {e}");
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
