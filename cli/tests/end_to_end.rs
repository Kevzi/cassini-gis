//! Suite de bout en bout du journal (R4, R5) sur le vrai binaire : exécuter,
//! fermer, rouvrir, rejouer, comparer. Aucun QGIS.
//!
//! Les commandes `test.*` n'existent pas dans le binaire : la suite emploie
//! `system.about`. Les entrées et sorties de fichiers sont couvertes dans `core`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{json, Value};

const BIN: &str = env!("CARGO_BIN_EXE_cassini");

/// Projet de test : (dossier, chemin du projet, journal à côté du projet).
fn temp_project() -> (PathBuf, String, PathBuf) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "cassini-e2e-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let project = dir.join("p.qgz").to_string_lossy().replace('\\', "/");
    let journal = dir.join("p.cassini").join("journal.jsonl");
    (dir, project, journal)
}

/// Une session `run` : une commande, puis le processus se ferme. Rend le résultat.
fn run_session(project: &str) -> Value {
    let out = Command::new(BIN)
        .args(["run", "system.about", "--project", project])
        .output()
        .expect("le binaire se lance");
    assert!(out.status.success(), "run a échoué");
    serde_json::from_slice(&out.stdout).expect("run rend du JSON")
}

/// Une session `serve` : hello puis `calls` appels, puis l'entrée se ferme.
/// Rend les résultats des appels.
fn serve_session(project: &str, calls: usize) -> Vec<Value> {
    let mut child = Command::new(BIN)
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("le binaire se lance");
    let mut stdin = child.stdin.take().unwrap();
    let hello = json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
        "params":{"protocol_version":1,"commands":[],"project":project}});
    writeln!(stdin, "{hello}").unwrap();
    for n in 0..calls {
        let call = json!({"jsonrpc":"2.0","id":n + 2,"method":"command.run",
            "params":{"command":"system.about","params":{}}});
        writeln!(stdin, "{call}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "serve a échoué");
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .skip(1)
        .map(|l| {
            let reply: Value = serde_json::from_str(l).expect("serve rend du JSON");
            assert!(reply.get("error").is_none(), "serve : {reply}");
            reply["result"].clone()
        })
        .collect()
}

fn replay(project: &str) -> Output {
    Command::new(BIN)
        .args(["replay", project])
        .output()
        .expect("le binaire se lance")
}

/// Le journal est du texte lisible : UTF-8, sans octet de contrôle, une ligne = un
/// objet JSON numéroté de 1 à n. Rend les lignes lues.
fn read_journal_as_text(bytes: &[u8]) -> Result<Vec<Value>, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("pas de l'UTF-8 : {e}"))?;
    if text
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r'))
    {
        return Err("octets de contrôle dans le journal".into());
    }
    let lines: Vec<Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).map_err(|e| format!("ligne illisible : {e}")))
        .collect::<Result<_, _>>()?;
    for (i, line) in lines.iter().enumerate() {
        if line["step"] != json!(i + 1) {
            return Err(format!("l'étape {} est mal numérotée", i + 1));
        }
    }
    Ok(lines)
}

fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn run_close_reopen_replay_and_compare() {
    let (dir, project, journal) = temp_project();
    // Session 1 (run), session 2 (run), puis session 3 (serve) qui rouvre le projet.
    let mut original = vec![run_session(&project), run_session(&project)];
    original.extend(serve_session(&project, 2));
    assert_eq!(original.len(), 4);

    // Le journal est un texte lisible, à côté du projet, sans chemin absolu.
    assert_eq!(journal.parent().unwrap().file_name().unwrap(), "p.cassini");
    let bytes = std::fs::read(&journal).unwrap();
    let lines = read_journal_as_text(&bytes).expect("journal lisible");
    let text = String::from_utf8(bytes.clone()).unwrap();
    let dir_text = dir.to_string_lossy();
    assert!(!text.contains(&*dir_text) && !text.contains(&dir_text.replace('\\', "/")));

    // Ce que le journal a gardé est ce que chaque session a rendu.
    assert_eq!(lines.len(), 4);
    for (line, result) in lines.iter().zip(&original) {
        assert_eq!(&line["result"], result);
    }

    // Rejeu après fermeture : identique, et le journal n'a pas bougé.
    let out = replay(&project);
    assert!(out.status.success(), "replay a échoué");
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report, json!({"steps": 4, "identical": true}));
    assert_eq!(std::fs::read(&journal).unwrap(), bytes);
    cleanup(&dir);
}

#[test]
fn a_replayed_result_that_differs_fails_the_replay() {
    let (dir, project, journal) = temp_project();
    run_session(&project);
    let text = std::fs::read_to_string(&journal).unwrap();
    let tampered = text.replace(cassini_core::VERSION, "9.9.9");
    assert_ne!(text, tampered, "la version doit figurer au journal");
    std::fs::write(&journal, tampered).unwrap();
    let out = replay(&project);
    assert!(!out.status.success());
    let err: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["code"], "replay.diverged");
    cleanup(&dir);
}

#[test]
fn a_journal_that_is_not_text_is_refused() {
    assert!(read_journal_as_text(&[0xff, 0xfe, 0x00, 0x01]).is_err());
    assert!(read_journal_as_text(b"{\"step\":2}\n").is_err());
    assert!(read_journal_as_text(b"{\"step\":1}\n").is_ok());
}
