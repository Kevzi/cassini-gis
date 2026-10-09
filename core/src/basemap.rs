//! Fond de carte : déclaration d'une source **locale**.
//!
//! Le principe « hors ligne total » (CLAUDE.md §2.3) est appliqué ici, pas dans
//! l'interface : une source de fond de carte qui désigne le réseau est refusée
//! à la construction. Aucune fonction du MVP ne doit pouvoir sortir sur le
//! réseau par simple configuration.

use std::fmt;
use std::path::{Path, PathBuf};

/// Pourquoi une source de fond de carte a été refusée.
#[derive(Debug, PartialEq, Eq)]
pub enum BasemapError {
    /// La source désigne le réseau au lieu d'un fichier local.
    SourceReseau(String),
    /// Le chemin est vide.
    CheminVide,
    /// Le fichier n'existe pas sur le disque.
    FichierAbsent(PathBuf),
}

impl fmt::Display for BasemapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BasemapError::SourceReseau(s) => write!(
                f,
                "source de fond de carte refusée, elle désigne le réseau : {s}"
            ),
            BasemapError::CheminVide => write!(f, "chemin de fond de carte vide"),
            BasemapError::FichierAbsent(p) => {
                write!(f, "fichier de fond de carte absent : {}", p.display())
            }
        }
    }
}

impl std::error::Error for BasemapError {}

/// Un fond de carte lu depuis un fichier local, et rien d'autre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasemapSource {
    chemin: PathBuf,
}

impl BasemapSource {
    /// Déclare un fond de carte à partir d'un chemin local existant.
    ///
    /// Refuse toute chaîne qui ressemble à une adresse réseau, même si un
    /// fichier du même nom existait par ailleurs.
    pub fn depuis_chemin_local(brut: &str) -> Result<Self, BasemapError> {
        let brut = brut.trim();
        if brut.is_empty() {
            return Err(BasemapError::CheminVide);
        }
        if designe_le_reseau(brut) {
            return Err(BasemapError::SourceReseau(brut.to_string()));
        }
        let chemin = PathBuf::from(brut);
        if !chemin.is_file() {
            return Err(BasemapError::FichierAbsent(chemin));
        }
        Ok(Self { chemin })
    }

    /// Le chemin du fichier de fond de carte.
    pub fn chemin(&self) -> &Path {
        &self.chemin
    }
}

/// Vrai si la chaîne désigne une ressource réseau plutôt qu'un fichier local.
///
/// `file://` est refusé aussi : le MVP prend des chemins de fichiers, pas des
/// URL, ce qui évite d'avoir à distinguer les schémas à l'usage.
fn designe_le_reseau(brut: &str) -> bool {
    let bas = brut.to_ascii_lowercase();
    bas.contains("://") || bas.starts_with("//")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fichier_temporaire(nom: &str) -> PathBuf {
        let p = std::env::temp_dir().join(nom);
        fs::write(&p, b"fond de carte de test").expect("écriture du fichier de test");
        p
    }

    #[test]
    fn accepte_un_fichier_local_existant() {
        let p = fichier_temporaire("cassini-fond-ok.pmtiles");
        let source = BasemapSource::depuis_chemin_local(p.to_str().unwrap())
            .expect("un fichier local existant doit être accepté");
        assert_eq!(source.chemin(), p.as_path());
        let _ = fs::remove_file(p);
    }

    #[test]
    fn refuse_une_adresse_http() {
        let erreur = BasemapSource::depuis_chemin_local("https://exemple.fr/fond.pmtiles")
            .expect_err("une adresse réseau doit être refusée");
        assert!(matches!(erreur, BasemapError::SourceReseau(_)));
    }

    #[test]
    fn refuse_tout_schema_durl_y_compris_file() {
        for brut in [
            "file:///tmp/fond.pmtiles",
            "HTTP://exemple.fr/f.pmtiles",
            "//serveur/part",
        ] {
            let erreur = BasemapSource::depuis_chemin_local(brut).unwrap_err();
            assert!(
                matches!(erreur, BasemapError::SourceReseau(_)),
                "{brut} aurait dû être refusé comme source réseau"
            );
        }
    }

    #[test]
    fn refuse_un_fichier_absent() {
        let inexistant = std::env::temp_dir().join("cassini-fond-qui-nexiste-pas.pmtiles");
        let _ = fs::remove_file(&inexistant);
        let erreur = BasemapSource::depuis_chemin_local(inexistant.to_str().unwrap()).unwrap_err();
        assert!(matches!(erreur, BasemapError::FichierAbsent(_)));
    }

    #[test]
    fn refuse_un_chemin_vide() {
        assert_eq!(
            BasemapSource::depuis_chemin_local("   ").unwrap_err(),
            BasemapError::CheminVide
        );
    }
}
