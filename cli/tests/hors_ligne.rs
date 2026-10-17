//! Preuve hors réseau : le binaire n'importe aucune bibliothèque réseau de Windows.
//! cpp-httplib est compilé dans llama-common mais écarté au lien.

#[test]
fn the_binary_imports_no_network_dll() {
    if !cfg!(windows) {
        return;
    }
    let bytes = std::fs::read(env!("CARGO_BIN_EXE_cassini")).unwrap();
    let lower: Vec<u8> = bytes.iter().map(u8::to_ascii_lowercase).collect();
    for dll in ["ws2_32", "winhttp", "wininet", "wsock32"] {
        assert!(
            !lower.windows(dll.len()).any(|w| w == dll.as_bytes()),
            "le binaire importe {dll}"
        );
    }
}
