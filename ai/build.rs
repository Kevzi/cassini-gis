//! Ticket 5.21 : avec la feature `gpu`, llama.cpp construit ses moteurs de calcul (processeur,
//! par variante de jeu d'instructions, et carte graphique Vulkan) comme des bibliothèques à part,
//! posées dans le dossier de construction de `llama-cpp-sys-2`. Le noyau les charge depuis le
//! dossier de son exécutable : on les copie donc à côté des binaires du profil (`target/<profil>`)
//! et des tests (`target/<profil>/deps`), avec les bibliothèques partagées que le binaire importe
//! (`llama`, `ggml`, `ggml-base`). `llama-common` n'est jamais copiée : le binaire ne l'importe
//! pas, et elle embarque un client HTTP (elle importe `ws2_32`). Sans la feature, rien n'est fait.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=DEP_LLAMA_BACKENDS_DIR");
    if env::var_os("CARGO_FEATURE_GPU").is_none() {
        return;
    }
    let Some(backends) = env::var_os("DEP_LLAMA_BACKENDS_DIR").map(PathBuf::from) else {
        panic!(
            "la feature `gpu` attend DEP_LLAMA_BACKENDS_DIR de llama-cpp-sys-2 (dynamic-backends)"
        );
    };
    println!("cargo:rerun-if-changed={}", backends.display());
    // Les bibliothèques partagées : `<racine>/bin` sous Windows, `<racine>/lib` ailleurs.
    let root =
        PathBuf::from(env::var_os("DEP_LLAMA_ROOT").expect("DEP_LLAMA_ROOT de llama-cpp-sys-2"));
    let shared = root.join(if cfg!(windows) { "bin" } else { "lib" });
    // OUT_DIR = <cible>/<profil>/build/cassini-ai-<empreinte>/out
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR posé par cargo"));
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR assez profond")
        .to_path_buf();
    let engines = modules_in(&backends);
    assert!(
        !engines.is_empty(),
        "aucun moteur de calcul dans {}",
        backends.display()
    );
    let mut modules = modules_in(&shared);
    modules.retain(|p| {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        !name.contains("llama-common")
    });
    modules.extend(engines);
    for dir in [profile_dir.clone(), profile_dir.join("deps")] {
        if !dir.is_dir() {
            continue;
        }
        for module in &modules {
            let dst = dir.join(module.file_name().expect("nom de fichier"));
            copy_if_changed(module, &dst);
        }
    }
}

/// Les bibliothèques chargeables du dossier `dir` (vide s'il n'existe pas).
fn modules_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| is_module(p))
        .collect()
}

/// Une bibliothèque chargeable : `.dll` sous Windows, `.so` ailleurs.
fn is_module(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("dll" | "so" | "dylib")
    )
}

/// Copie `src` sur `dst` si la taille ou la date diffère. Une destination verrouillée (un
/// `cassini.exe` en cours d'exécution la tient) n'arrête pas la construction : un avertissement.
fn copy_if_changed(src: &Path, dst: &Path) {
    let same = match (fs::metadata(src), fs::metadata(dst)) {
        (Ok(a), Ok(b)) => a.len() == b.len() && a.modified().ok() <= b.modified().ok(),
        _ => false,
    };
    if same {
        return;
    }
    if let Err(e) = fs::copy(src, dst) {
        println!(
            "cargo:warning=copie de {} vers {} impossible : {e}",
            src.display(),
            dst.display()
        );
    }
}
