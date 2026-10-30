//! Preuve hors réseau : ni le binaire ni aucune bibliothèque posée à côté de lui n'importe une
//! bibliothèque réseau de Windows. cpp-httplib n'est pas compilé (llama-cpp-2 sans la feature
//! `common`) ; ce test lit la table d'imports de chaque PE, pour que la garantie ne régresse pas
//! en silence. Depuis le ticket 5.21, le paquet livré (feature `gpu`) est un dossier : le
//! binaire et ses moteurs de calcul (`llama.dll`, `ggml*.dll`), tous vérifiés.
//! Il ne s'exécute que sous Windows, où la recette a un job dédié.

use std::path::{Path, PathBuf};

use object::read::pe::PeFile64;
use object::Object;

const FORBIDDEN: [&str; 6] = [
    "ws2_32", "wsock32", "winhttp", "wininet", "mswsock", "dnsapi",
];

/// Les bibliothèques importées par le PE `path`, en minuscules.
fn imports(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let pe = PeFile64::parse(&*bytes)
        .unwrap_or_else(|e| panic!("{} n'est pas un PE 64 bits : {e}", path.display()));
    pe.imports()
        .expect("table d'imports lisible")
        .map(|i| String::from_utf8_lossy(i.expect("import lisible").library()).to_lowercase())
        .collect()
}

fn assert_no_network(path: &Path, libraries: &[String]) {
    for lib in libraries {
        for dll in FORBIDDEN {
            assert!(!lib.starts_with(dll), "{} importe {lib}", path.display());
        }
    }
}

/// Les `.dll` du dossier `dir`.
fn dlls(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("dll"))
        })
        .collect();
    found.sort();
    found
}

fn file_name(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().to_lowercase()
}

#[test]
fn the_binary_and_its_libraries_import_no_network_dll() {
    if !cfg!(windows) {
        return;
    }
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_cassini"));
    let libraries = imports(&exe);
    assert!(
        libraries.iter().any(|l| l.starts_with("kernel32")),
        "la table d'imports est vide : {libraries:?}"
    );
    assert_no_network(&exe, &libraries);
    // Lié directement au chargeur Vulkan, le binaire refuserait de démarrer sans pilote : seul
    // le moteur carte, chargé à part et ignoré s'il échoue, peut l'importer.
    assert!(
        !libraries.iter().any(|l| l.starts_with("vulkan-1")),
        "le binaire importe le chargeur Vulkan : {libraries:?}"
    );
    let dir = exe.parent().unwrap();
    let found = dlls(dir);
    for dll in &found {
        assert_no_network(dll, &imports(dll));
    }
    if cfg!(feature = "gpu") {
        let names: Vec<String> = found.iter().map(|p| file_name(p)).collect();
        // `ggml-cpu-x64` : la variante de base, sans AVX, pour un processeur ancien.
        for expected in [
            "llama.dll",
            "ggml.dll",
            "ggml-base.dll",
            "ggml-vulkan.dll",
            "ggml-cpu-x64.dll",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "{expected} manque dans {} : {names:?}",
                dir.display()
            );
        }
        assert!(
            names.iter().any(|n| n.starts_with("ggml-cpu-")),
            "aucun moteur processeur dans {} : {names:?}",
            dir.display()
        );
        // Chargées au démarrage comme `cassini.exe` : elles ne doivent pas lier le chargeur
        // Vulkan, sinon le noyau ne démarrerait pas sans pilote.
        for shared in ["llama.dll", "ggml.dll", "ggml-base.dll"] {
            let libraries = imports(&dir.join(shared));
            assert!(
                !libraries.iter().any(|l| l.starts_with("vulkan-1")),
                "{shared} importe le chargeur Vulkan : {libraries:?}"
            );
        }
        // Elle embarque un client HTTP (elle importe `ws2_32`) : jamais dans le paquet.
        assert!(
            !names.iter().any(|n| n == "llama-common.dll"),
            "llama-common.dll ne doit pas être dans {}",
            dir.display()
        );
    }
}
