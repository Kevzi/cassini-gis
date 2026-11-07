//! Aides partagées par les tests des commandes : fixtures versionnées et dossiers temporaires.
//! Aucune commande ici ; le fichier n'est compilé que pour les tests, comme `test_echo.rs`.

#![cfg(test)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::dem::Dem;

/// Chemin d'une fixture versionnée de `/fixtures`.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures")
        .join(name)
}

/// Compteur des dossiers créés par ce processus : deux tests de modules différents peuvent
/// porter le même nom (« trou », « refus ») sans partager de dossier.
static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Dossier temporaire propre à un test, effacé à la fin, y compris s'il échoue.
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new(name: &str) -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cassini-{}-{n}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// Écrit `grid` sous `name` et rend son chemin en texte.
    pub fn write(&self, name: &str, grid: &Dem) -> String {
        let path = self.join(name);
        grid.write_geotiff(&path).unwrap();
        path.to_string_lossy().into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
