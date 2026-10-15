//! Logique de `build.rs`, séparée pour pouvoir être testée : `build.rs` ne passe
//! pas par `cargo test`, ce fichier si (inclus par `lib.rs` sous `cfg(test)`).

use std::fs;
use std::path::{Path, PathBuf};

/// Mots réservés de Rust : `pub mod type;` ne se compile pas.
const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen", "_",
];

fn is_rust_ident(name: &str) -> bool {
    if KEYWORDS.contains(&name) {
        return false;
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// Les modules de `dir` : `nom.rs` ou `nom/mod.rs`, triés par nom.
/// Un nom qui n'est pas un identifiant Rust est une erreur nommant le fichier.
pub fn command_modules(dir: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Impossible de lire le dossier {} : {e}", dir.display()))?;
    let mut modules: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|e| format!("Entrée de dossier illisible : {e}"))?
            .path();
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (name, target) = if path.is_dir() {
            let m = path.join("mod.rs");
            if !m.is_file() {
                return Err(format!(
                    "Le dossier « {file_name} » de src/commands/ n'a pas de fichier mod.rs : \
                     ses commandes ne seraient jamais compilées. Ajoutez-lui un mod.rs, ou retirez-le."
                ));
            }
            (file_name.clone(), m)
        } else if let Some(stem) = file_name.strip_suffix(".rs") {
            if file_name == "mod.rs" {
                continue;
            }
            (stem.to_string(), path.clone())
        } else {
            continue;
        };
        if !is_rust_ident(&name) {
            let suggestion = name.replace(|c: char| !(c == '_' || c.is_ascii_alphanumeric()), "_");
            let hint = if is_rust_ident(&suggestion) {
                format!(" (par exemple « {suggestion} »)")
            } else {
                String::new()
            };
            return Err(format!(
                "Le fichier « {file_name} » de src/commands/ n'a pas un nom valide : \
                 utilisez des lettres, chiffres et « _ », sans commencer par un chiffre ni reprendre \
                 un mot réservé de Rust{hint}."
            ));
        }
        modules.push((name, target));
    }
    modules.sort();
    Ok(modules)
}

/// Le texte de `commands_mods.rs` : une déclaration `pub mod` par module.
pub fn render_modules(modules: &[(String, PathBuf)]) -> String {
    let mut out = String::new();
    for (name, target) in modules {
        let p = target.to_string_lossy().replace('\\', "/");
        out.push_str(&format!("#[path = {p:?}]\npub mod {name};\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un dossier de travail neuf, propre à chaque test.
    fn scratch(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cassini-build-support-{}-{label}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn files_and_mod_folders_become_sorted_modules() {
        let dir = scratch("ok");
        fs::write(dir.join("zeta.rs"), "").unwrap();
        fs::write(dir.join("alpha.rs"), "").unwrap();
        fs::write(dir.join("mod.rs"), "").unwrap();
        fs::write(dir.join("notes.txt"), "").unwrap();
        fs::create_dir(dir.join("geo")).unwrap();
        fs::write(dir.join("geo").join("mod.rs"), "").unwrap();

        let names: Vec<String> = command_modules(&dir)
            .unwrap()
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert_eq!(names, ["alpha", "geo", "zeta"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_invalid_file_name_is_refused_and_named() {
        let dir = scratch("bad");
        fs::write(dir.join("system-about.rs"), "").unwrap();
        let err = command_modules(&dir).unwrap_err();
        assert!(err.contains("system-about.rs"), "{err}");
        assert!(err.contains("system_about"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_without_mod_rs_is_refused_and_named() {
        let dir = scratch("nomod");
        fs::create_dir(dir.join("geo")).unwrap();
        let err = command_modules(&dir).unwrap_err();
        assert!(err.contains("geo") && err.contains("mod.rs"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_rust_keyword_is_refused() {
        let dir = scratch("kw");
        fs::write(dir.join("type.rs"), "").unwrap();
        let err = command_modules(&dir).unwrap_err();
        assert!(err.contains("type.rs"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_name_starting_with_a_digit_is_refused() {
        let dir = scratch("digit");
        fs::write(dir.join("2fast.rs"), "").unwrap();
        assert!(command_modules(&dir).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rendered_modules_use_forward_slashes() {
        let text = render_modules(&[("geo".to_string(), PathBuf::from(r"C:\x\geo\mod.rs"))]);
        assert_eq!(text, "#[path = \"C:/x/geo/mod.rs\"]\npub mod geo;\n");
    }
}
