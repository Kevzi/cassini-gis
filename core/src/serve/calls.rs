//! Appels d'un relais du canal d'IA externe (AD-29, ticket 5.28) : le `command.run` d'un relais
//! passe par `Session::call_from`, comme celui de l'extension (journal, chemins, ordre dicté à
//! l'extension, annulation), avec l'origine `external` au journal.
//!
//! Le fil lecteur du relais a déjà réservé l'occupation : elle est libérée ici quelle que soit
//! l'issue. L'appel est annoncé à l'extension par la notification `external.call`, au début
//! (`started`) et à la fin (`finished`, résultat ou erreur, durée). Aucun chemin absolu ne passe
//! vers le relais (AD-9) : un paramètre de chemin absolu ou qui sort du projet est refusé avant
//! toute exécution, et le résultat comme l'erreur sont réduits avant de partir.

use std::path::Path;
use std::time::Instant;

use serde_json::{json, Map, Value};

use super::{check_available, respond, run_params, Current, RpcError, Source, State};
use crate::error::ErrorReport;
use crate::journal::relativize;
use crate::registry::RegistryError;

/// L'origine notée au journal pour l'étape d'un relais ; le rejeu l'ignore.
const ORIGIN: &str = "external";

/// Exécute le `command.run` `id` du relais `conn`, dont l'occupation est réservée, et lui répond.
pub(super) fn run(state: &mut State<'_>, conn: u64, id: Value, params: Option<Value>) {
    let link = state.link.clone();
    let hub = &link.external;
    // La connexion a disparu pendant que l'appel attendait en file (interrupteur coupé) : rien
    // n'est exécuté, l'occupation est libérée.
    if !hub.has_conn(conn) {
        hub.release();
        return;
    }
    let outcome = call(state, conn, &id, params.as_ref());
    hub.release();
    // Session perdue (entrée fermée) : aucune réponse, le relais recevra `external.session_lost`.
    // Relais parti ou coupé par l'interrupteur : sa connexion n'est plus inscrite, rien ne part.
    if link.input_closed.get() {
        return;
    }
    if let Some(response) = respond(Some(id), outcome) {
        hub.send(conn, &response);
    }
}

fn call(
    state: &mut State<'_>,
    conn: u64,
    id: &Value,
    params: Option<&Value>,
) -> Result<Value, RpcError> {
    let link = state.link.clone();
    // La validation de chaque appel (`external.approve`) est un ticket à part : d'ici là, la
    // case allumée refuse tout appel (décision du propriétaire, 2026-10-30).
    if link.external.ask() {
        return Err(RpcError::coded(
            "external.not_approved",
            "La case « Me demander avant chaque appel » est cochée dans QGIS : cet appel n'est \
             pas exécuté."
                .to_string(),
        ));
    }
    let (command, args) = run_params(params)?;
    let Some(spec) = state.registry.spec(&command) else {
        return Err(RpcError::business(ErrorReport::from(
            RegistryError::UnknownCommand(command),
        )));
    };
    check_available(state, &command)?;
    for name in &spec.path_params {
        if args.get(name).is_some_and(escapes_project) {
            return Err(RpcError::coded(
                "external.path_refused",
                format!(
                    "Le paramètre « {name} » doit être un chemin relatif au projet, qui n'en sort \
                     pas : rien n'est exécuté."
                ),
            ));
        }
    }
    let Some(project) = state.project.clone() else {
        return Err(RpcError::coded(
            "session.not_started",
            "La session QGIS n'est pas ouverte.".to_string(),
        ));
    };
    state.next_call += 1;
    let call_id = format!("external-{}", state.next_call);
    notify(
        state,
        json!({"call_id": call_id, "event": "started", "command": command, "params": args}),
    );
    *link.current.borrow_mut() = Some(Current {
        source: Source::External(conn),
        id: Some(id.clone()),
        call_id: Some(call_id.clone()),
    });
    let started = Instant::now();
    let session = state
        .session
        .as_mut()
        .expect("session ouverte avec le projet");
    let outcome = session.call_from(&command, args, Some(ORIGIN));
    *link.current.borrow_mut() = None;
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let work_dir = session.work_dir().to_path_buf();
    let dirs = known_dirs(&work_dir, Path::new(&project));
    let outcome = match outcome {
        Ok(value) => Ok(scrub(&relativize(&value, &work_dir), &dirs)),
        Err(report) => Err(ErrorReport {
            message: scrub_text(&report.message, &dirs),
            ..report
        }),
    };
    let mut finished = Map::new();
    finished.insert("call_id".into(), json!(call_id));
    finished.insert("event".into(), json!("finished"));
    finished.insert("command".into(), json!(command));
    match &outcome {
        Ok(value) => finished.insert("result".into(), value.clone()),
        Err(report) => finished.insert(
            "error".into(),
            json!({"code": report.code, "message": report.message}),
        ),
    };
    finished.insert("duration_ms".into(), json!(duration_ms));
    notify(state, Value::Object(finished));
    outcome.map_err(RpcError::business)
}

/// La notification `external.call` vers l'extension ; rien ne part une fois l'entrée fermée.
fn notify(state: &State<'_>, params: Value) {
    if state.link.input_closed.get() {
        return;
    }
    let notice = json!({"jsonrpc": "2.0", "method": "external.call", "params": params});
    state.link.send_or_fail(&notice);
}

/// Un paramètre de chemin (une chaîne ou une liste de chaînes) est-il absolu, ou sort-il du
/// projet par `..` ? Toute autre valeur est laissée à la validation du schéma.
fn escapes_project(value: &Value) -> bool {
    match value {
        Value::String(raw) => {
            looks_absolute(raw) || raw.split(['/', '\\']).any(|part| part == "..")
        }
        Value::Array(items) => items.iter().any(escapes_project),
        _ => false,
    }
}

/// Absolu sur l'un ou l'autre système : racine, partage réseau, lettre de lecteur.
fn looks_absolute(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    raw.starts_with(['/', '\\']) || drive || Path::new(raw).is_absolute()
}

/// Les dossiers du poste qu'un texte peut citer, sous leurs deux écritures de séparateurs.
struct Dirs {
    /// Le dossier du projet : un chemin absolu qui y mène redevient relatif à lui.
    project: Vec<String>,
    /// Le dossier de travail et celui du projet, le plus long d'abord, pour qu'il soit remplacé en
    /// entier ; une racine (`/`, `C:\`) n'y entre pas, elle couvrirait tout séparateur.
    all: Vec<String>,
}

fn spellings(dir: &Path) -> Vec<String> {
    let native = dir.to_string_lossy().into_owned();
    if is_root(&native) {
        return Vec::new();
    }
    let mut out = vec![native.replace('\\', "/"), native.replace('/', "\\")];
    out.dedup();
    out
}

/// Une racine de système de fichiers : rien d'autre qu'une lettre de lecteur et des séparateurs.
fn is_root(dir: &str) -> bool {
    let bytes = dir.as_bytes();
    let rest = if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        &dir[2..]
    } else {
        dir
    };
    rest.split(['/', '\\']).all(str::is_empty)
}

fn known_dirs(work_dir: &Path, project: &Path) -> Dirs {
    let project = project.parent().map(spellings).unwrap_or_default();
    let mut all = spellings(work_dir);
    all.extend(project.iter().cloned());
    all.sort_by_key(|d| std::cmp::Reverse(d.len()));
    all.dedup();
    Dirs { project, all }
}

/// Copie de `value` sans chemin absolu (AD-9) : voir `scrub_text`.
fn scrub(value: &Value, dirs: &Dirs) -> Value {
    match value {
        Value::String(s) => Value::String(scrub_text(s, dirs)),
        Value::Array(items) => Value::Array(items.iter().map(|v| scrub(v, dirs)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), scrub(v, dirs)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Un texte commence-t-il par un chemin absolu : racine, partage réseau, ou lettre de lecteur
/// suivie d'un séparateur (`A: voir` n'en est pas un) ?
fn starts_absolute(text: &str) -> bool {
    let bytes = text.as_bytes();
    text.starts_with(['/', '\\'])
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
}

/// Le dernier segment non vide d'un chemin ; le chemin tel quel s'il n'en a pas (`/`).
fn last_segment(path: &str) -> &str {
    path.split(['/', '\\'])
        .rfind(|part| !part.is_empty())
        .unwrap_or(path)
}

/// `dir` préfixe-t-il `text` à une limite de chemin ? Rend la suite, sans ses séparateurs.
fn under<'t>(text: &'t str, dir: &str) -> Option<&'t str> {
    let rest = text.strip_prefix(dir)?;
    (rest.is_empty() || rest.starts_with(['/', '\\'])).then(|| rest.trim_start_matches(['/', '\\']))
}

/// Un texte qui est en entier un chemin du projet devient relatif à lui (écriture POSIX) ; un
/// autre chemin absolu entier, son nom de fichier. Dans un autre texte (un message), les dossiers du projet et de
/// travail deviennent `.`, puis tout chemin absolu qui reste est réduit à son nom de fichier.
fn scrub_text(text: &str, dirs: &Dirs) -> String {
    for dir in &dirs.project {
        if let Some(rest) = under(text, dir) {
            return if rest.is_empty() {
                ".".to_string()
            } else {
                rest.replace('\\', "/")
            };
        }
    }
    if starts_absolute(text) && !text.contains(char::is_whitespace) {
        return last_segment(text).to_string();
    }
    let mut text = text.to_string();
    for dir in &dirs.all {
        text = replace_bounded(&text, dir);
    }
    reduce_tokens(&text)
}

/// Remplace `dir` par `.` là où il est suivi d'un séparateur ou de la fin du texte.
fn replace_bounded(text: &str, dir: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(dir) {
        let after = &rest[at + dir.len()..];
        out.push_str(&rest[..at]);
        if after.is_empty() || after.starts_with(['/', '\\']) {
            out.push('.');
        } else {
            out.push_str(dir);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Réduit à son nom de fichier chaque chemin absolu d'un texte : un mot qui commence (en début de
/// texte, après une espace, un guillemet ou une parenthèse) par un chemin absolu, jusqu'à une
/// espace ou une fermeture.
fn reduce_tokens(text: &str) -> String {
    let opens = |c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '«' | '(');
    let closes = |c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '»' | ')');
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut at_boundary = true;
    while let Some(c) = rest.chars().next() {
        if at_boundary && starts_absolute(rest) {
            let end = rest.find(closes).unwrap_or(rest.len());
            out.push_str(last_segment(&rest[..end]));
            rest = &rest[end..];
            at_boundary = false;
            continue;
        }
        out.push(c);
        at_boundary = opens(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_leaving_the_project_is_refused() {
        for bad in [
            "/abs/x.gpkg",
            "\\\\serveur\\partage\\x.gpkg",
            "C:\\x.gpkg",
            "c:x.gpkg",
            "../x.gpkg",
            "donnees/../../x.gpkg",
            "donnees\\..\\..\\x",
        ] {
            assert!(escapes_project(&json!(bad)), "{bad}");
        }
        assert!(escapes_project(&json!(["a.gpkg", "/b.gpkg"])));
        for good in ["donnees/x.gpkg", "x.gpkg", "out:1.1", "a..b.gpkg"] {
            assert!(!escapes_project(&json!(good)), "{good}");
        }
        assert!(!escapes_project(&json!(["a.gpkg", "b/c.gpkg"])));
        assert!(!escapes_project(&json!(3)));
    }

    #[test]
    fn no_absolute_path_is_left_in_a_result_or_a_message() {
        let dirs = known_dirs(
            Path::new("/home/u/Projet code/p.cassini"),
            Path::new("/home/u/Projet code/p.qgz"),
        );
        let value = json!({
            "file": "/ailleurs/carte.pdf",
            "list": ["C:\\Users\\u\\x.gpkg", "relatif.gpkg"],
            "under": "/home/u/Projet code/donnees/x.gpkg",
            "project": "/home/u/Projet code",
            "root": "/",
            "message": "Échec sur /home/u/Projet code/donnees/x.gpkg et /home/u/Projet code/p.cassini/3-a.gpkg",
            "n": 3,
        });
        assert_eq!(
            scrub(&value, &dirs),
            json!({
                "file": "carte.pdf",
                "list": ["x.gpkg", "relatif.gpkg"],
                "under": "donnees/x.gpkg",
                "project": ".",
                "root": "/",
                "message": "Échec sur ./donnees/x.gpkg et ./3-a.gpkg",
                "n": 3,
            })
        );
        for (message, expected) in [
            (
                "Fichier introuvable : D:\\SIG\\cadastre.gpkg",
                "Fichier introuvable : cadastre.gpkg",
            ),
            (
                "Fichier introuvable : /ailleurs/x.gpkg.",
                "Fichier introuvable : x.gpkg.",
            ),
            (
                "Voir « /a/b.gpkg » et (C:/c/d.shp)",
                "Voir « b.gpkg » et (d.shp)",
            ),
            // Pas de remplacement sans limite de chemin (pas de « .2/x ») ; le mot absolu est réduit.
            ("Voisin /home/u/Projet code2/x", "Voisin Projet code2/x"),
            ("A: voir a/b", "A: voir a/b"),
            ("ratio 1/2 et ./rel/x", "ratio 1/2 et ./rel/x"),
            ("/", "/"),
        ] {
            assert_eq!(scrub_text(message, &dirs), expected, "{message}");
        }
    }

    #[test]
    fn a_project_at_a_root_does_not_mangle_separators() {
        let dirs = known_dirs(Path::new("/p.cassini"), Path::new("/p.qgz"));
        assert!(dirs.project.is_empty());
        assert_eq!(scrub_text("a/b et c\\d", &dirs), "a/b et c\\d");
        assert_eq!(scrub_text("/p.cassini/x.gpkg", &dirs), "x.gpkg");
        assert!(is_root("C:\\") && is_root("C:/") && is_root("/") && !is_root("C:\\x"));
    }
}
