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
    SourceReseau(String),
    /// Le chemin est vide.
    CheminVide,
    /// Le fichier n'existe pas sur le disque.
    FichierAbsent(PathBuf),
    /// La plage demandée sort du fichier.
    ///
    /// Jamais de troncature silencieuse : un lecteur qui reçoit moins d'octets
    /// qu'il n'en a demandé échoue plus loin, là où la cause n'est plus visible.
    PlageHorsFichier {
        /// Premier octet demandé.
        decalage: u64,
        /// Nombre d'octets demandés.
        longueur: u64,
        /// Taille réelle du fichier.
        taille: u64,
    },
    /// Aucun des emplacements candidats ne portait de fond de carte utilisable.
    AucunEmplacement {
        /// Pourquoi le **premier** candidat a échoué : c'est l'emplacement
        /// normal, les suivants ne sont que des replis.
        cause: String,
        /// Tous les emplacements essayés, dans l'ordre.
        essayes: Vec<PathBuf>,
    },
    /// Le fichier existe mais sa lecture a échoué.
    LectureImpossible {
        /// Fichier concerné.
        chemin: PathBuf,
        /// Message du système.
        cause: String,
    },
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
            BasemapError::PlageHorsFichier {
                decalage,
                longueur,
                taille,
            } => write!(
                f,
                "plage de fond de carte hors fichier : {longueur} octets \
                 à partir du décalage {decalage}, alors que le fichier \
                 en compte {taille}"
            ),
            BasemapError::AucunEmplacement { cause, essayes } => {
                let liste: Vec<String> = essayes.iter().map(|p| p.display().to_string()).collect();
                write!(
                    f,
                    "fond de carte introuvable : {cause} ; emplacements essayés : {}",
                    if liste.is_empty() {
                        "aucun".to_string()
                    } else {
                        liste.join(", ")
                    }
                )
            }
            BasemapError::LectureImpossible { chemin, cause } => write!(
                f,
                "lecture du fond de carte impossible ({}) : {cause}",
                chemin.display()
            ),
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
    pub fn depuis_le_premier_emplacement(candidats: &[PathBuf]) -> Result<Self, BasemapError> {
        let mut cause = None;
        for candidat in candidats {
            match Self::depuis_chemin_local(&candidat.to_string_lossy()) {
                Ok(source) => return Ok(source),
                Err(erreur) => {
                    if cause.is_none() {
                        cause = Some(erreur.to_string());
                    }
                }
            }
        }
        Err(BasemapError::AucunEmplacement {
            cause: cause.unwrap_or_else(|| "aucun emplacement candidat".to_string()),
            essayes: candidats.to_vec(),
        })
    }

    /// Le chemin du fichier de fond de carte.
    pub fn chemin(&self) -> &Path {
        &self.chemin
    }

    /// Lit `longueur` octets à partir de `decalage`, bruts.
    ///
    /// C'est la seule porte d'entrée des octets du fond de carte : le lecteur
    /// PMTiles de l'interface n'ouvre aucun fichier, il demande des plages.
    ///
    /// Une plage qui dépasse la fin du fichier est **refusée**, jamais tronquée :
    /// un lecteur qui reçoit moins d'octets qu'il n'en a demandé se casse plus
    /// loin, à un endroit où la cause n'est plus visible.
    pub fn lire_plage(&self, decalage: u64, longueur: u64) -> Result<Vec<u8>, BasemapError> {
        let mut fichier = File::open(&self.chemin).map_err(|e| self.echec_de_lecture(&e))?;
        let taille = fichier
            .metadata()
            .map_err(|e| self.echec_de_lecture(&e))?
            .len();

        let hors_fichier = BasemapError::PlageHorsFichier {
            decalage,
            longueur,
            taille,
        };
        // `checked_add` plutôt que `decalage + longueur` : un débordement
        // d'entier donnerait une fin plus petite que le décalage, donc une
        // plage absurde qui passerait le contrôle.
        if decalage
            .checked_add(longueur)
            .is_none_or(|fin| fin > taille)
        {
            return Err(hors_fichier);
        }
        // Sur une cible 32 bits, une longueur légitime au regard du fichier peut
        // tout de même dépasser la taille adressable d'un tampon.
        let taille_du_tampon = usize::try_from(longueur).map_err(|_| hors_fichier)?;

        fichier
            .seek(SeekFrom::Start(decalage))
            .map_err(|e| self.echec_de_lecture(&e))?;
        let mut octets = vec![0u8; taille_du_tampon];
        fichier
            .read_exact(&mut octets)
            .map_err(|e| self.echec_de_lecture(&e))?;
        Ok(octets)
    }

    fn echec_de_lecture(&self, cause: &std::io::Error) -> BasemapError {
        BasemapError::LectureImpossible {
            chemin: self.chemin.clone(),
            cause: cause.to_string(),
        }
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

    fn fichier_temporaire_avec(nom: &str, contenu: &[u8]) -> PathBuf {
        let p = std::env::temp_dir().join(nom);
        fs::write(&p, contenu).expect("écriture du fichier de test");
        p
    }

    fn fichier_temporaire(nom: &str) -> PathBuf {
        fichier_temporaire_avec(nom, b"fond de carte de test")
    }

    /// Une source sur un fichier de contenu connu, pour éprouver `lire_plage`.
    fn source_de_test(nom: &str, contenu: &[u8]) -> (BasemapSource, PathBuf) {
        let p = fichier_temporaire_avec(nom, contenu);
        let source = BasemapSource::depuis_chemin_local(p.to_str().unwrap())
            .expect("le fichier de test doit être accepté");
        (source, p)
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

    #[test]
    fn lit_une_plage_dans_les_bornes() {
        let (source, p) = source_de_test("cassini-plage-ok.pmtiles", b"0123456789");
        assert_eq!(source.lire_plage(3, 4).unwrap(), b"3456");
        let _ = fs::remove_file(p);
    }

    #[test]
    fn lit_la_plage_qui_finit_exactement_a_la_fin() {
        let (source, p) = source_de_test("cassini-plage-fin.pmtiles", b"0123456789");
        assert_eq!(source.lire_plage(0, 10).unwrap(), b"0123456789");
        assert_eq!(source.lire_plage(9, 1).unwrap(), b"9");
        let _ = fs::remove_file(p);
    }

    #[test]
    fn lit_une_plage_vide_sans_erreur() {
        let (source, p) = source_de_test("cassini-plage-vide.pmtiles", b"0123456789");
        assert!(source.lire_plage(10, 0).unwrap().is_empty());
        let _ = fs::remove_file(p);
    }

    #[test]
    fn refuse_une_plage_qui_depasse_la_fin() {
        let (source, p) = source_de_test("cassini-plage-depasse.pmtiles", b"0123456789");
        assert_eq!(
            source.lire_plage(9, 2).unwrap_err(),
            BasemapError::PlageHorsFichier {
                decalage: 9,
                longueur: 2,
                taille: 10,
            }
        );
        let _ = fs::remove_file(p);
    }

    #[test]
    fn le_message_de_plage_hors_fichier_nomme_les_trois_nombres() {
        let (source, p) = source_de_test("cassini-plage-message.pmtiles", b"0123456789");
        let message = source.lire_plage(8, 5).unwrap_err().to_string();
        for nombre in ["8", "5", "10"] {
            assert!(
                message.contains(nombre),
                "le message devrait nommer {nombre} : {message}"
            );
        }
        let _ = fs::remove_file(p);
    }

    #[test]
    fn refuse_un_debordement_dentier_au_lieu_de_paniquer() {
        let (source, p) = source_de_test("cassini-plage-debordement.pmtiles", b"0123456789");
        assert_eq!(
            source.lire_plage(u64::MAX, 2).unwrap_err(),
            BasemapError::PlageHorsFichier {
                decalage: u64::MAX,
                longueur: 2,
                taille: 10,
            }
        );
        let _ = fs::remove_file(p);
    }

    #[test]
    fn retient_le_premier_emplacement_candidat_quand_il_existe() {
        let premier = fichier_temporaire("cassini-emplacement-1.pmtiles");
        let second = fichier_temporaire("cassini-emplacement-2.pmtiles");
        let source =
            BasemapSource::depuis_le_premier_emplacement(&[premier.clone(), second.clone()])
                .expect("le premier candidat existe, il doit être retenu");
        assert_eq!(source.chemin(), premier.as_path());
        let _ = fs::remove_file(premier);
        let _ = fs::remove_file(second);
    }

    #[test]
    fn se_replie_sur_le_candidat_suivant() {
        let absent = std::env::temp_dir().join("cassini-emplacement-absent.pmtiles");
        let _ = fs::remove_file(&absent);
        let repli = fichier_temporaire("cassini-emplacement-repli.pmtiles");
        let source = BasemapSource::depuis_le_premier_emplacement(&[absent, repli.clone()])
            .expect("le repli existe, il doit être retenu");
        assert_eq!(source.chemin(), repli.as_path());
        let _ = fs::remove_file(repli);
    }

    #[test]
    fn sans_aucun_candidat_valable_la_cause_est_celle_du_premier() {
        let normal = std::env::temp_dir().join("cassini-emplacement-normal.pmtiles");
        let repli = std::env::temp_dir().join("cassini-emplacement-second.pmtiles");
        let _ = fs::remove_file(&normal);
        let _ = fs::remove_file(&repli);
        let erreur = BasemapSource::depuis_le_premier_emplacement(&[normal.clone(), repli.clone()])
            .unwrap_err();
        let BasemapError::AucunEmplacement { cause, essayes } = &erreur else {
            panic!("variante inattendue : {erreur:?}");
        };
        assert!(
            cause.contains(&normal.display().to_string()),
            "la cause devrait être celle de l'emplacement normal : {cause}"
        );
        assert_eq!(essayes, &[normal.clone(), repli.clone()]);
        // Le message nomme tous les emplacements essayés, pas seulement le premier.
        let message = erreur.to_string();
        for candidat in [&normal, &repli] {
            assert!(
                message.contains(&candidat.display().to_string()),
                "le message devrait nommer {} : {message}",
                candidat.display()
            );
        }
    }

    #[test]
    fn refuse_une_liste_de_candidats_vide() {
        let erreur = BasemapSource::depuis_le_premier_emplacement(&[]).unwrap_err();
        assert!(matches!(erreur, BasemapError::AucunEmplacement { .. }));
        assert!(erreur.to_string().contains("aucun"));
    }

    #[test]
    fn refuse_la_lecture_dun_fichier_disparu_apres_la_declaration() {
        let (source, p) = source_de_test("cassini-plage-disparu.pmtiles", b"0123456789");
        fs::remove_file(&p).expect("suppression du fichier de test");
        assert!(matches!(
            source.lire_plage(0, 1).unwrap_err(),
            BasemapError::LectureImpossible { .. }
        ));
    }
}
