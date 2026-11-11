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

#[test]
fn the_families_option_draws_only_the_named_families_and_refuses_an_unknown_one() {
    // 5.54 : le jeu ciblé d'une séance continue. Le résumé par famille, sur la sortie d'erreur,
    // ne nomme que les familles demandées ; le fichier n'a que des exemples de celles-ci.
    let dir = scratch("familles");
    let out = dir.join("cible.jsonl");
    let status = Command::new(BIN)
        .args(["--seed", "3", "--count", "60", "--format", "chat"])
        .args(["--familles", "tampon,refus", "--out"])
        .arg(&out)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&status.stderr);
    assert!(status.status.success(), "{stderr}");
    assert_eq!(std::fs::read_to_string(&out).unwrap().lines().count(), 60);
    let table: Vec<&str> = stderr
        .lines()
        .skip_while(|l| !l.starts_with("famille "))
        .skip(1)
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    assert_eq!(table, ["refus", "tampon"], "{stderr}");
    let known: Vec<&str> = cassini_training::families::FAMILIES
        .iter()
        .map(|f| f.name)
        .collect();
    // Une famille de `FOCUS_EXCLUDED` est refusée au jeu ciblé ; la liste est vide depuis 5.54,
    // lot 2 (le contrôle reste pour les familles futures).
    let mut refusals = vec![("tampon,nexiste_pas", "nexiste_pas")];
    if let Some(excluded) = cassini_training::FOCUS_EXCLUDED.first() {
        refusals.push((excluded, "refusée au jeu ciblé"));
    }
    for (families, needle) in refusals {
        let refused = dir.join("refuse.jsonl");
        let status = Command::new(BIN)
            .args(["--count", "10", "--familles", families, "--out"])
            .arg(&refused)
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&status.stderr);
        assert_eq!(status.status.code(), Some(2), "{families} : {stderr}");
        assert!(stderr.contains(needle), "{stderr}");
        assert!(!refused.exists());
    }
    let listed = Command::new(BIN)
        .arg("--familles-connues")
        .output()
        .unwrap();
    assert!(listed.status.success());
    assert_eq!(
        String::from_utf8(listed.stdout)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        known
    );
    let _ = std::fs::remove_dir_all(&dir);
}
