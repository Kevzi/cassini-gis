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

/// `plan.propose` sur le vrai binaire sans modèle : une erreur typée, sans chemin absolu,
/// aucun ordre envoyé, et le canal sert toujours `command.run` dans la même session.
#[test]
fn plan_propose_without_a_model_is_a_typed_error_and_the_session_goes_on() {
    let (dir, project, journal) = temp_project();
    let models = dir.join("modeles-vides");
    std::fs::create_dir_all(&models).unwrap();
    let mut child = Command::new(BIN)
        .arg("serve")
        .env("CASSINI_MODELS", &models)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("le binaire se lance");
    let mut stdin = child.stdin.take().unwrap();
    for message in [
        json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
            "params":{"protocol_version":1,"commands":["project.inventory"],"project":project}}),
        json!({"jsonrpc":"2.0","id":2,"method":"plan.propose",
            "params":{"request":"Ouvre les gares."}}),
        json!({"jsonrpc":"2.0","id":3,"method":"command.run",
            "params":{"command":"system.about","params":{}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"plan.propose",
            "params":{"request":"  "}}),
    ] {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "serve a échoué");
    let text = String::from_utf8(out.stdout).unwrap();
    let replies: Vec<Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    // Quatre réponses, aucun ordre : l'inventaire n'est demandé ni sans modèle ni pour une
    // phrase vide.
    assert_eq!(replies.len(), 4, "{text}");
    assert!(replies.iter().all(|r| r.get("method").is_none()));
    assert_eq!(replies[1]["id"], 2);
    assert_eq!(replies[1]["error"]["data"]["code"], "ai.no_model");
    let dir_text = dir.to_string_lossy();
    assert!(!text.contains(&*dir_text) && !text.contains(&dir_text.replace('\\', "/")));
    assert!(!text.contains("modeles-vides"));
    assert_eq!(replies[2]["result"]["protocol_version"], 1);
    assert_eq!(replies[3]["id"], 4);
    assert_eq!(replies[3]["error"]["data"]["code"], "ai.empty_prompt");
    // Seul `command.run` est journalisé.
    assert_eq!(
        std::fs::read_to_string(&journal).unwrap().lines().count(),
        1
    );
    cleanup(&dir);
}

/// Un dossier de modèles désigné qui n'existe pas : l'erreur ne le recopie pas.
#[test]
fn plan_propose_with_a_missing_models_folder_hides_the_folder() {
    let (dir, project, _) = temp_project();
    let models = dir.join("absent");
    let mut child = Command::new(BIN)
        .arg("serve")
        .env("CASSINI_MODELS", &models)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("le binaire se lance");
    let mut stdin = child.stdin.take().unwrap();
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
            "params":{"protocol_version":1,"commands":[],"project":project}})
    )
    .unwrap();
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","id":2,"method":"plan.propose","params":{"request":"x"}})
    )
    .unwrap();
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let reply: Value = serde_json::from_str(text.lines().nth(1).unwrap()).unwrap();
    assert_eq!(reply["error"]["data"]["code"], "models.no_folder");
    let dir_text = dir.to_string_lossy();
    assert!(!text.contains(&*dir_text) && !text.contains(&dir_text.replace('\\', "/")));
    cleanup(&dir);
}

/// Lance `serve` sur le dossier de modèles `models`, propose « Ouvre les gares. », répond à la
/// demande d'inventaire, et rend la réponse à la proposition avec tout le texte reçu.
fn propose_with_models(project: &str, models: &Path) -> (Value, String) {
    use std::io::{BufRead, BufReader};
    let mut child = Command::new(BIN)
        .arg("serve")
        .env("CASSINI_MODELS", models)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("le binaire se lance");
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut text = String::new();
    let mut next = |text: &mut String| -> Value {
        let line = lines.next().expect("le noyau répond").unwrap();
        text.push_str(&line);
        text.push('\n');
        serde_json::from_str(&line).unwrap()
    };
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
            "params":{"protocol_version":1,"commands":["project.inventory"],"project":project}})
    )
    .unwrap();
    assert!(next(&mut text)["result"].is_object());
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","id":2,"method":"plan.propose","params":{"request":"Ouvre les gares."}})
    )
    .unwrap();
    let order = next(&mut text);
    assert_eq!(order["params"]["command"], "project.inventory");
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","id":order["id"],
            "result":{"layers":[],"project_crs":"EPSG:2154","extent":null}})
    )
    .unwrap();
    let reply = next(&mut text);
    drop(stdin);
    assert!(child.wait().unwrap().success());
    assert_eq!(reply["id"], 2);
    (reply, text)
}

/// Le texte reçu ne porte aucun des dossiers `folders`.
fn assert_hides(text: &str, folders: &[&Path]) {
    for folder in folders {
        let f = folder.to_string_lossy();
        assert!(
            !text.contains(&*f) && !text.contains(&f.replace('\\', "/")),
            "{text}"
        );
    }
}

/// Un faux modèle (`x.gguf` qui n'est pas un GGUF) : `compose_plan` échoue après
/// l'inventaire, en `ai.*`, et la réponse ne porte ni le dossier ni le chemin du modèle.
#[test]
fn plan_propose_with_an_unreadable_model_hides_its_path() {
    let (dir, project, _) = temp_project();
    let models = dir.join("modeles");
    std::fs::create_dir_all(&models).unwrap();
    std::fs::write(models.join("x.gguf"), b"pas un modele").unwrap();
    let (reply, text) = propose_with_models(&project, &models);
    let code = reply["error"]["data"]["code"].as_str().unwrap();
    assert!(code.starts_with("ai."), "{reply}");
    assert_hides(&text, &[&dir, &models]);
    cleanup(&dir);
}

/// Lance `serve` sur le dossier de modèles `models`, ouvre la session, envoie `messages`,
/// ferme l'entrée et rend tout le texte reçu après `session.hello`.
fn serve_with_models(project: &str, models: &Path, messages: &[Value]) -> String {
    let mut child = Command::new(BIN)
        .arg("serve")
        .env("CASSINI_MODELS", models)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("le binaire se lance");
    let mut stdin = child.stdin.take().unwrap();
    let hello = json!({"jsonrpc":"2.0","id":1,"method":"session.hello",
        "params":{"protocol_version":1,"commands":["project.inventory"],"project":project}});
    for message in std::iter::once(&hello).chain(messages) {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "serve a échoué");
    let text = String::from_utf8(out.stdout).unwrap();
    text.lines().skip(1).map(|l| format!("{l}\n")).collect()
}

/// Une déclaration de modèle (ticket 5.10, tâche 3) qui nomme une base absente : `agent.models`
/// la montre avec son problème, sans chemin ; sans choix, le noyau ne la prend pas ; choisie,
/// elle est refusée `models.unusable`, nommée par son seul fichier, avant tout inventaire.
#[test]
fn plan_propose_with_a_declaration_naming_a_missing_base_names_the_file_only() {
    let (dir, project, _) = temp_project();
    let models = dir.join("modeles");
    std::fs::create_dir_all(&models).unwrap();
    // L'adaptateur présent n'est jamais listé seul.
    std::fs::write(models.join("lora.gguf"), b"pas un adaptateur").unwrap();
    std::fs::write(
        models.join("ajuste.modele.json"),
        r#"{"model":"base-absente.gguf","adapter":"lora.gguf","prompt":"short"}"#,
    )
    .unwrap();
    let text = serve_with_models(
        &project,
        &models,
        &[
            json!({"jsonrpc":"2.0","id":2,"method":"agent.models"}),
            json!({"jsonrpc":"2.0","id":3,"method":"plan.propose",
                "params":{"request":"Ouvre les gares."}}),
            json!({"jsonrpc":"2.0","id":4,"method":"plan.propose",
                "params":{"request":"Ouvre les gares.","model":"ajuste.modele.json"}}),
        ],
    );
    let replies: Vec<Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    // Trois réponses, aucun ordre d'inventaire.
    assert_eq!(replies.len(), 3, "{text}");
    assert!(replies.iter().all(|r| r.get("method").is_none()), "{text}");
    assert_eq!(
        replies[0]["result"]["models"],
        json!([{"file": "ajuste.modele.json", "size_bytes": 17,
            "problem": "fichier manquant : base-absente.gguf"}])
    );
    assert_eq!(replies[1]["error"]["data"]["code"], "ai.no_model", "{text}");
    assert_eq!(
        replies[2]["error"]["data"]["code"], "models.unusable",
        "{text}"
    );
    assert!(text.contains("base-absente.gguf"), "{text}");
    assert_hides(&text, &[&dir, &models]);
    cleanup(&dir);
}

/// `agent.models` sur deux GGUF : la liste par nom et taille, rien au journal ; un modèle
/// inconnu est refusé `models.unknown` sans ordre d'inventaire.
#[test]
fn agent_models_lists_two_models_and_an_unknown_choice_is_refused() {
    let (dir, project, journal) = temp_project();
    let models = dir.join("modeles");
    std::fs::create_dir_all(&models).unwrap();
    std::fs::write(models.join("a.gguf"), b"12345").unwrap();
    std::fs::write(models.join("b.gguf"), b"123").unwrap();
    let text = serve_with_models(
        &project,
        &models,
        &[
            json!({"jsonrpc":"2.0","id":2,"method":"agent.models"}),
            json!({"jsonrpc":"2.0","id":3,"method":"plan.propose",
                "params":{"request":"Ouvre les gares.","model":"x.gguf"}}),
        ],
    );
    let replies: Vec<Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(replies.len(), 2, "{text}");
    assert_eq!(
        replies[0]["result"],
        json!({"models": [{"file": "a.gguf", "size_bytes": 5}, {"file": "b.gguf", "size_bytes": 3}]})
    );
    assert_eq!(
        replies[1]["error"]["data"]["code"], "models.unknown",
        "{text}"
    );
    assert_hides(&text, &[&dir, &models]);
    // Rien n'est journalisé.
    assert!(std::fs::read_to_string(&journal)
        .unwrap_or_default()
        .is_empty());
    cleanup(&dir);
}
