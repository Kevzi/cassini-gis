//! Chemins du journal : normalisation lexicale, forme relative au projet, écriture POSIX.

use std::path::{Component, Path, PathBuf};

use serde_json::Value;

/// Copie où tout chemin sous `out_dir` devient son nom de fichier : le dossier de sortie
/// diffère entre l'exécution et le rejeu, et le journal ne garde aucun chemin absolu (AD-9).
pub(crate) fn relativize(value: &Value, out_dir: &Path) -> Value {
    match value {
        Value::String(s) => {
            let path = Path::new(s);
            match path
                .is_absolute()
                .then(|| path.strip_prefix(out_dir).ok())
                .flatten()
            {
                Some(rest) if !rest.as_os_str().is_empty() => Value::String(posix(rest)),
                _ => value.clone(),
            }
        }
        Value::Array(items) => Value::Array(items.iter().map(|v| relativize(v, out_dir)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), relativize(v, out_dir)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Normalisation lexicale : retire `.` et résout `..` sans toucher au disque.
pub(super) fn normalize(path: &Path) -> PathBuf {
    let mut out: Vec<Component> = Vec::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => match out.last() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                _ => out.push(c),
            },
            other => out.push(other),
        }
    }
    out.iter().collect()
}

/// Chemin de `target` relatif à `base`, tous deux absolus et normalisés ;
/// `None` si leurs racines diffèrent (autre disque).
pub(super) fn relative_to(base: &Path, target: &Path) -> Option<PathBuf> {
    let b: Vec<Component> = base.components().collect();
    let t: Vec<Component> = target.components().collect();
    let root_len = b
        .iter()
        .take_while(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
        .count();
    let common = b.iter().zip(t.iter()).take_while(|(x, y)| x == y).count();
    if common < root_len {
        return None;
    }
    let mut out = PathBuf::new();
    for _ in common..b.len() {
        out.push("..");
    }
    for c in &t[common..] {
        out.push(c.as_os_str());
    }
    Some(out)
}

/// Écriture POSIX d'un chemin relatif ; `.` pour le dossier du projet lui-même.
pub(super) fn posix(relative: &Path) -> String {
    let parts: Vec<String> = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}
