//! Fond de carte : déclaration d'une source **locale**, et lecture de ses octets.
//!
//! Le principe « hors ligne total » (CLAUDE.md §2.3) est appliqué ici, pas dans
//! l'interface : une source de fond de carte qui désigne le réseau est refusée
//! à la construction. Aucune fonction du MVP ne doit pouvoir sortir sur le
//! réseau par simple configuration.
//!
//! Le moteur garde aussi la lecture du disque (CLAUDE.md §2.5) : l'interface ne
//! connaît jamais le chemin du fichier, elle demande des plages d'octets.

use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Pourquoi une source de fond de carte a été refusée.
#[derive(Debug, PartialEq, Eq)]
pub enum BasemapError {
    /// La source désigne le réseau au lieu d'un fichier local.
    NetworkSource(String),
    /// Le chemin est vide.
    EmptyPath,
    /// Le fichier n'existe pas sur le disque.
    MissingFile(PathBuf),
    /// La plage demandée sort du fichier.
    ///
    /// Jamais de troncature silencieuse : un lecteur qui reçoit moins d'octets
    /// qu'il n'en a demandé échoue plus loin, là où la cause n'est plus visible.
    RangeOutsideFile {
        /// Premier octet demandé.
        offset: u64,
        /// Nombre d'octets demandés.
        length: u64,
        /// Taille réelle du fichier.
        size: u64,
    },
    /// Aucun des emplacements candidats ne portait de fond de carte utilisable.
    NoLocation {
        /// Pourquoi le **premier** candidat a échoué : c'est l'emplacement
        /// normal, les suivants ne sont que des replis.
        cause: String,
        /// Tous les emplacements essayés, dans l'ordre.
        tried: Vec<PathBuf>,
    },
    /// Le fichier existe mais sa lecture a échoué.
    ReadFailed {
        /// Fichier concerné.
        path: PathBuf,
        /// Message du système.
        cause: String,
    },
}

impl fmt::Display for BasemapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BasemapError::NetworkSource(s) => write!(
                f,
                "source de fond de carte refusée, elle désigne le réseau : {s}"
            ),
            BasemapError::EmptyPath => write!(f, "chemin de fond de carte vide"),
            BasemapError::MissingFile(p) => {
                write!(f, "fichier de fond de carte absent : {}", p.display())
            }
            BasemapError::RangeOutsideFile {
                offset,
                length,
                size,
            } => write!(
                f,
                "plage de fond de carte hors fichier : {length} octets \
                 à partir du décalage {offset}, alors que le fichier \
                 en compte {size}"
            ),
            BasemapError::NoLocation { cause, tried } => {
                let list: Vec<String> = tried.iter().map(|p| p.display().to_string()).collect();
                write!(
                    f,
                    "fond de carte introuvable : {cause} ; emplacements essayés : {}",
                    if list.is_empty() {
                        "aucun".to_string()
                    } else {
                        list.join(", ")
                    }
                )
            }
            BasemapError::ReadFailed { path, cause } => write!(
                f,
                "lecture du fond de carte impossible ({}) : {cause}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for BasemapError {}

/// Un fond de carte lu depuis un fichier local, et rien d'autre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasemapSource {
    path: PathBuf,
}

impl BasemapSource {
    /// Déclare un fond de carte à partir d'un chemin local existant.
    ///
    /// Refuse toute chaîne qui ressemble à une adresse réseau, même si un
    /// fichier du même nom existait par ailleurs.
    pub fn from_local_path(raw: &str) -> Result<Self, BasemapError> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(BasemapError::EmptyPath);
        }
        if targets_network(raw) {
            return Err(BasemapError::NetworkSource(raw.to_string()));
        }
        let path = PathBuf::from(raw);
        if !path.is_file() {
            return Err(BasemapError::MissingFile(path));
        }
        Ok(Self { path })
    }

    /// Retient le premier emplacement candidat qui porte un fond de carte
    /// utilisable.
    ///
    /// L'ordre de la liste fait la règle : le premier candidat est
    /// l'emplacement normal, les suivants sont des replis. L'erreur rendue
    /// porte donc la cause du **premier** candidat — celle qui renseigne
    /// l'utilisateur — et nomme tous les emplacements essayés.
    ///
    /// Le choix vit ici, et pas dans la coquille de bureau, pour être testable :
    /// aucune recette ne compile la coquille (CLAUDE.md §8).
    pub fn from_first_location(candidates: &[PathBuf]) -> Result<Self, BasemapError> {
        let mut cause = None;
        for candidate in candidates {
            match Self::from_local_path(&candidate.to_string_lossy()) {
                Ok(source) => return Ok(source),
                Err(error) => {
                    if cause.is_none() {
                        cause = Some(error.to_string());
                    }
                }
            }
        }
        Err(BasemapError::NoLocation {
            cause: cause.unwrap_or_else(|| "aucun emplacement candidat".to_string()),
            tried: candidates.to_vec(),
        })
    }

    /// Le chemin du fichier de fond de carte.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Lit `length` octets à partir de `offset`, bruts.
    ///
    /// C'est la seule porte d'entrée des octets du fond de carte : le lecteur
    /// PMTiles de l'interface n'ouvre aucun fichier, il demande des plages.
    ///
    /// Une plage qui dépasse la fin du fichier est **refusée**, jamais tronquée :
    /// un lecteur qui reçoit moins d'octets qu'il n'en a demandé se casse plus
    /// loin, à un endroit où la cause n'est plus visible.
    pub fn read_range(&self, offset: u64, length: u64) -> Result<Vec<u8>, BasemapError> {
        let mut file = File::open(&self.path).map_err(|e| self.read_failure(&e))?;
        let size = file.metadata().map_err(|e| self.read_failure(&e))?.len();

        let outside_file = BasemapError::RangeOutsideFile {
            offset,
            length,
            size,
        };
        // `checked_add` plutôt que `offset + length` : un débordement
        // d'entier donnerait une fin plus petite que le décalage, donc une
        // plage absurde qui passerait le contrôle.
        if offset.checked_add(length).is_none_or(|end| end > size) {
            return Err(outside_file);
        }
        // Sur une cible 32 bits, une longueur légitime au regard du fichier peut
        // tout de même dépasser la taille adressable d'un tampon.
        let buffer_size = usize::try_from(length).map_err(|_| outside_file)?;

        file.seek(SeekFrom::Start(offset))
            .map_err(|e| self.read_failure(&e))?;
        let mut bytes = vec![0u8; buffer_size];
        file.read_exact(&mut bytes)
            .map_err(|e| self.read_failure(&e))?;
        Ok(bytes)
    }

    fn read_failure(&self, cause: &std::io::Error) -> BasemapError {
        BasemapError::ReadFailed {
            path: self.path.clone(),
            cause: cause.to_string(),
        }
    }
}

/// Vrai si la chaîne désigne une ressource réseau plutôt qu'un fichier local.
///
/// `file://` est refusé aussi : le MVP prend des chemins de fichiers, pas des
/// URL, ce qui évite d'avoir à distinguer les schémas à l'usage.
fn targets_network(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.contains("://") || lower.starts_with("//")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_file_with(name: &str, contents: &[u8]) -> PathBuf {
        let p = std::env::temp_dir().join(name);
        fs::write(&p, contents).expect("écriture du fichier de test");
        p
    }

    fn temp_file(name: &str) -> PathBuf {
        temp_file_with(name, b"fond de carte de test")
    }

    /// Une source sur un fichier de contenu connu, pour éprouver `read_range`.
    fn test_source(name: &str, contents: &[u8]) -> (BasemapSource, PathBuf) {
        let p = temp_file_with(name, contents);
        let source = BasemapSource::from_local_path(p.to_str().unwrap())
            .expect("le fichier de test doit être accepté");
        (source, p)
    }

    #[test]
    fn accepts_an_existing_local_file() {
        let p = temp_file("cassini-fond-ok.pmtiles");
        let source = BasemapSource::from_local_path(p.to_str().unwrap())
            .expect("un fichier local existant doit être accepté");
        assert_eq!(source.path(), p.as_path());
        let _ = fs::remove_file(p);
    }

    #[test]
    fn rejects_an_http_address() {
        let error = BasemapSource::from_local_path("https://exemple.fr/fond.pmtiles")
            .expect_err("une adresse réseau doit être refusée");
        assert!(matches!(error, BasemapError::NetworkSource(_)));
    }

    #[test]
    fn rejects_every_url_scheme_including_file() {
        for raw in [
            "file:///tmp/fond.pmtiles",
            "HTTP://exemple.fr/f.pmtiles",
            "//serveur/part",
        ] {
            let error = BasemapSource::from_local_path(raw).unwrap_err();
            assert!(
                matches!(error, BasemapError::NetworkSource(_)),
                "{raw} aurait dû être refusé comme source réseau"
            );
        }
    }

    #[test]
    fn rejects_a_missing_file() {
        let missing = std::env::temp_dir().join("cassini-fond-qui-nexiste-pas.pmtiles");
        let _ = fs::remove_file(&missing);
        let error = BasemapSource::from_local_path(missing.to_str().unwrap()).unwrap_err();
        assert!(matches!(error, BasemapError::MissingFile(_)));
    }

    #[test]
    fn rejects_an_empty_path() {
        assert_eq!(
            BasemapSource::from_local_path("   ").unwrap_err(),
            BasemapError::EmptyPath
        );
    }

    #[test]
    fn reads_a_range_within_bounds() {
        let (source, p) = test_source("cassini-plage-ok.pmtiles", b"0123456789");
        assert_eq!(source.read_range(3, 4).unwrap(), b"3456");
        let _ = fs::remove_file(p);
    }

    #[test]
    fn reads_the_range_ending_exactly_at_the_end() {
        let (source, p) = test_source("cassini-plage-fin.pmtiles", b"0123456789");
        assert_eq!(source.read_range(0, 10).unwrap(), b"0123456789");
        assert_eq!(source.read_range(9, 1).unwrap(), b"9");
        let _ = fs::remove_file(p);
    }

    #[test]
    fn reads_an_empty_range_without_error() {
        let (source, p) = test_source("cassini-plage-vide.pmtiles", b"0123456789");
        assert!(source.read_range(10, 0).unwrap().is_empty());
        let _ = fs::remove_file(p);
    }

    #[test]
    fn rejects_a_range_past_the_end() {
        let (source, p) = test_source("cassini-plage-depasse.pmtiles", b"0123456789");
        assert_eq!(
            source.read_range(9, 2).unwrap_err(),
            BasemapError::RangeOutsideFile {
                offset: 9,
                length: 2,
                size: 10,
            }
        );
        let _ = fs::remove_file(p);
    }

    #[test]
    fn the_range_outside_file_message_names_the_three_numbers() {
        let (source, p) = test_source("cassini-plage-message.pmtiles", b"0123456789");
        let message = source.read_range(8, 5).unwrap_err().to_string();
        for number in ["8", "5", "10"] {
            assert!(
                message.contains(number),
                "le message devrait nommer {number} : {message}"
            );
        }
        let _ = fs::remove_file(p);
    }

    #[test]
    fn rejects_an_integer_overflow_instead_of_panicking() {
        let (source, p) = test_source("cassini-plage-debordement.pmtiles", b"0123456789");
        assert_eq!(
            source.read_range(u64::MAX, 2).unwrap_err(),
            BasemapError::RangeOutsideFile {
                offset: u64::MAX,
                length: 2,
                size: 10,
            }
        );
        let _ = fs::remove_file(p);
    }

    #[test]
    fn keeps_the_first_candidate_location_when_it_exists() {
        let first = temp_file("cassini-emplacement-1.pmtiles");
        let second = temp_file("cassini-emplacement-2.pmtiles");
        let source = BasemapSource::from_first_location(&[first.clone(), second.clone()])
            .expect("le premier candidat existe, il doit être retenu");
        assert_eq!(source.path(), first.as_path());
        let _ = fs::remove_file(first);
        let _ = fs::remove_file(second);
    }

    #[test]
    fn falls_back_to_the_next_candidate() {
        let missing = std::env::temp_dir().join("cassini-emplacement-absent.pmtiles");
        let _ = fs::remove_file(&missing);
        let fallback = temp_file("cassini-emplacement-repli.pmtiles");
        let source = BasemapSource::from_first_location(&[missing, fallback.clone()])
            .expect("le repli existe, il doit être retenu");
        assert_eq!(source.path(), fallback.as_path());
        let _ = fs::remove_file(fallback);
    }

    #[test]
    fn without_any_usable_candidate_the_cause_is_the_first_one() {
        let normal = std::env::temp_dir().join("cassini-emplacement-normal.pmtiles");
        let fallback = std::env::temp_dir().join("cassini-emplacement-second.pmtiles");
        let _ = fs::remove_file(&normal);
        let _ = fs::remove_file(&fallback);
        let error =
            BasemapSource::from_first_location(&[normal.clone(), fallback.clone()]).unwrap_err();
        let BasemapError::NoLocation { cause, tried } = &error else {
            panic!("variante inattendue : {error:?}");
        };
        assert!(
            cause.contains(&normal.display().to_string()),
            "la cause devrait être celle de l'emplacement normal : {cause}"
        );
        assert_eq!(tried, &[normal.clone(), fallback.clone()]);
        // Le message nomme tous les emplacements essayés, pas seulement le premier.
        let message = error.to_string();
        for candidate in [&normal, &fallback] {
            assert!(
                message.contains(&candidate.display().to_string()),
                "le message devrait nommer {} : {message}",
                candidate.display()
            );
        }
    }

    #[test]
    fn rejects_an_empty_candidate_list() {
        let error = BasemapSource::from_first_location(&[]).unwrap_err();
        assert!(matches!(error, BasemapError::NoLocation { .. }));
        assert!(error.to_string().contains("aucun"));
    }

    #[test]
    fn rejects_reading_a_file_removed_after_declaration() {
        let (source, p) = test_source("cassini-plage-disparu.pmtiles", b"0123456789");
        fs::remove_file(&p).expect("suppression du fichier de test");
        assert!(matches!(
            source.read_range(0, 1).unwrap_err(),
            BasemapError::ReadFailed { .. }
        ));
    }
}
