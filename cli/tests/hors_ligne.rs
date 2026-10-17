//! Preuve hors réseau : le binaire n'importe aucune bibliothèque réseau de Windows.
//! cpp-httplib n'est pas compilé (llama-cpp-2 sans la feature `common`) ; ce test
//! lit la table d'imports du PE, pour que la garantie ne régresse pas en silence.
//! Il ne s'exécute que sous Windows, où la recette a un job dédié.

use object::read::pe::PeFile64;
use object::Object;

const FORBIDDEN: [&str; 6] = [
    "ws2_32", "wsock32", "winhttp", "wininet", "mswsock", "dnsapi",
];

#[test]
fn the_binary_imports_no_network_dll() {
    if !cfg!(windows) {
        return;
    }
    let bytes = std::fs::read(env!("CARGO_BIN_EXE_cassini")).unwrap();
    let pe = PeFile64::parse(&*bytes).expect("le binaire est un PE 64 bits");
    let libraries: Vec<String> = pe
        .imports()
        .expect("table d'imports lisible")
        .map(|i| String::from_utf8_lossy(i.expect("import lisible").library()).to_lowercase())
        .collect();
    assert!(
        libraries.iter().any(|l| l.starts_with("kernel32")),
        "la table d'imports est vide : {libraries:?}"
    );
    for lib in &libraries {
        for dll in FORBIDDEN {
            assert!(!lib.starts_with(dll), "le binaire importe {lib}");
        }
    }
}
