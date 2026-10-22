//! Test d'intégration de la commande `cassini-training` (ticket 5.10, tâche 2) : la chaîne
//! d'entraînement dépend du fichier qu'elle écrit. Même graine, même fichier à l'octet près ;
//! autre graine, autre fichier ; arguments manquants ou faux, code non nul.

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_cassini-training");

/// Un dossier temporaire propre à ce test, vidé au début.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "cassini-training-commande-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Lance la commande et rend le contenu du fichier écrit.
fn run(dir: &std::path::Path, file: &str, args: &[&str]) -> Vec<u8> {
    let out = dir.join(file);
    let status = Command::new(BIN)
        .args(args)
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    std::fs::read(&out).unwrap()
}

#[test]
fn the_same_seed_gives_the_same_bytes_and_another_seed_others() {
    let dir = scratch("graine");
    for format in ["structured", "chat"] {
        let args = |seed: &'static str| ["--seed", seed, "--count", "300", "--format", format];
        let a = run(&dir, "a.jsonl", &args("1"));
        let b = run(&dir, "b.jsonl", &args("1"));
        let c = run(&dir, "c.jsonl", &args("2"));
        assert!(!a.is_empty());
        assert_eq!(a, b, "{format} : même graine, même fichier");
        assert_ne!(a, c, "{format} : autre graine, autre fichier");
        assert!(
            !dir.join("a.jsonl.tmp").exists(),
            "aucun fichier temporaire laissé"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_chat_format_writes_prompt_and_completion_only() {
    let dir = scratch("chat");
    let bytes = run(&dir, "chat.jsonl", &["--count", "50", "--format", "chat"]);
    let text = String::from_utf8(bytes).unwrap();
    assert_eq!(text.lines().count(), 50);
    for line in text.lines() {
        let record: Value = serde_json::from_str(line).unwrap();
        let object = record.as_object().unwrap();
        assert_eq!(object.len(), 2, "{line}");
        let prompt = record["prompt"].as_str().unwrap();
        let completion = record["completion"].as_str().unwrap();
        assert!(prompt.ends_with("<|im_start|>assistant\n<think>\n\n</think>\n\n"));
        assert!(completion.starts_with("{\"kind\":\""));
        assert!(completion.ends_with("}<|im_end|>"));
    }
    // Le format par défaut reste l'enregistrement structuré de la tâche 1.
    let structured = run(&dir, "structured.jsonl", &["--count", "5"]);
    let first: Value = serde_json::from_str(
        String::from_utf8(structured)
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    assert!(first.get("inventory").is_some() && first.get("expected").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_or_wrong_arguments_give_a_non_zero_code() {
    for args in [
        vec!["--seed", "1"],
        vec!["--format", "texte", "--out", "jamais.jsonl"],
        vec!["--seed", "abc", "--out", "jamais.jsonl"],
        vec!["--out"],
    ] {
        let status = Command::new(BIN).args(&args).output().unwrap();
        assert!(!status.status.success(), "{args:?}");
        assert!(String::from_utf8_lossy(&status.stderr).contains("usage"));
    }
    assert!(!std::path::Path::new("jamais.jsonl").exists());
}
