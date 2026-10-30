//! Où poser le modèle : sur une carte graphique dédiée s'il y en a une, sur le processeur sinon
//! (ticket 5.21). Aucun réglage de l'utilisateur.
//!
//! Avec la feature `gpu` (le paquet livré), llama.cpp ne lie aucun moteur de calcul : il les
//! charge au démarrage, comme des bibliothèques posées à côté de l'exécutable. Le moteur
//! processeur (une variante par jeu d'instructions, la meilleure est prise seule) est toujours
//! là ; le moteur carte (Vulkan) est ignoré s'il ne se charge pas (pas de `vulkan-1.dll`, pas de
//! pilote) ou s'il ne trouve aucune carte. Une carte intégrée n'est jamais prise.

use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::{list_llama_ggml_backend_devices, LlamaBackendDeviceType};

use crate::AiError;

/// Où le modèle est posé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Placement {
    /// Tout sur le processeur ; aucune carte ne reçoit rien, pas même les gros lots.
    Cpu,
    /// Tout sur ces cartes dédiées (index des périphériques de ggml).
    Gpu(Vec<usize>),
}

impl Placement {
    /// La décision, sans modèle : les cartes dédiées seules, le processeur faute d'en avoir.
    pub(crate) fn decide(
        devices: impl IntoIterator<Item = (usize, LlamaBackendDeviceType)>,
    ) -> Self {
        let gpus: Vec<usize> = devices
            .into_iter()
            .filter(|(_, kind)| *kind == LlamaBackendDeviceType::Gpu)
            .map(|(index, _)| index)
            .collect();
        if gpus.is_empty() {
            Placement::Cpu
        } else {
            Placement::Gpu(gpus)
        }
    }

    /// La décision pour les périphériques que ggml voit maintenant.
    pub(crate) fn current() -> Self {
        Self::decide(
            list_llama_ggml_backend_devices()
                .into_iter()
                .map(|d| (d.index, d.device_type)),
        )
    }

    pub(crate) fn on_gpu(&self) -> bool {
        matches!(self, Placement::Gpu(_))
    }

    /// Paramètres de chargement. Une liste de périphériques donnée, même vide, remplace le choix
    /// par défaut de llama.cpp, qui prendrait une carte intégrée faute de carte dédiée : vide,
    /// le modèle et ses calculs restent sur le processeur.
    pub(crate) fn params(&self) -> LlamaModelParams {
        let cpu = || {
            LlamaModelParams::default()
                .with_n_gpu_layers(0)
                .with_devices(&[])
                .unwrap_or_default()
        };
        match self {
            Placement::Cpu => cpu(),
            // Couches par défaut de llama.cpp : toutes sur la carte.
            Placement::Gpu(gpus) => LlamaModelParams::default()
                .with_devices(gpus)
                .unwrap_or_else(|_| cpu()),
        }
    }
}

/// Essaie `attempt` sur `placement`, puis une seule fois sur le processeur si la carte refuse
/// (mémoire trop petite, pilote). Rend le résultat et s'il est sur la carte ; l'erreur rendue
/// est celle du dernier essai.
pub(crate) fn with_cpu_fallback<T, E>(
    placement: &Placement,
    mut attempt: impl FnMut(&Placement) -> Result<T, E>,
) -> Result<(T, bool), E> {
    match attempt(placement) {
        Ok(value) => Ok((value, placement.on_gpu())),
        Err(_) if placement.on_gpu() => attempt(&Placement::Cpu).map(|value| (value, false)),
        Err(e) => Err(e),
    }
}

/// Après un premier essai `first` : si la carte a pris le modèle (`on_gpu`) mais pas son
/// contexte (mémoire trop petite), `retry` (recharger sur le processeur) est tenté une fois.
/// Jamais pour un modèle déjà sur le processeur, jamais pour une autre erreur.
pub(crate) fn retry_context_on_cpu<T>(
    first: Result<T, AiError>,
    on_gpu: bool,
    retry: impl FnOnce() -> Result<T, AiError>,
) -> Result<T, AiError> {
    match first {
        Err(AiError::Context(_)) if on_gpu => retry(),
        other => other,
    }
}

/// Un moteur processeur est-il chargé ? Sans lui, aucun modèle ne peut tourner.
#[cfg(feature = "gpu")]
fn has_cpu_device() -> bool {
    list_llama_ggml_backend_devices()
        .iter()
        .any(|d| d.device_type == LlamaBackendDeviceType::Cpu)
}

/// Charge les moteurs de calcul posés à côté de l'exécutable, jamais ceux du dossier de
/// construction. Ceux qui ne se chargent pas sont ignorés ; il faut au moins le processeur.
#[cfg(feature = "gpu")]
pub(crate) fn load_backends() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe
        .parent()
        .ok_or_else(|| "dossier de l'exécutable introuvable".to_string())?;
    if dir.to_str().is_none() {
        return Err(format!(
            "le dossier de l'exécutable n'est pas un chemin lisible : {}",
            dir.display()
        ));
    }
    // Journaux coupés avant le chargement : il se fait avant `LlamaBackend::init`, donc avant
    // `void_logs`.
    unsafe extern "C" fn silent(
        _level: llama_cpp_sys_2::ggml_log_level,
        _text: *const std::os::raw::c_char,
        _user_data: *mut std::os::raw::c_void,
    ) {
    }
    if std::env::var_os("CASSINI_DEBUG_LLAMA").is_none() {
        // SAFETY: rappel sans effet, valable pour toute la durée du processus.
        unsafe { llama_cpp_sys_2::llama_log_set(Some(silent), std::ptr::null_mut()) };
    }
    llama_cpp_2::llama_backend::load_backends_from_path(dir);
    if has_cpu_device() {
        Ok(())
    } else {
        Err(format!(
            "aucun moteur de calcul processeur (ggml-cpu-*) à côté de l'exécutable, dans {}",
            dir.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LlamaBackendDeviceType::{Accelerator, Cpu, Gpu, IntegratedGpu, Unknown};

    #[test]
    fn a_dedicated_card_takes_the_model() {
        let p = Placement::decide([(0, Gpu), (1, Cpu)]);
        assert_eq!(p, Placement::Gpu(vec![0]));
        assert!(p.on_gpu());
    }

    #[test]
    fn no_card_means_processor() {
        assert_eq!(Placement::decide([(0, Cpu)]), Placement::Cpu);
        assert_eq!(Placement::decide([]), Placement::Cpu);
        assert!(!Placement::Cpu.on_gpu());
    }

    #[test]
    fn an_integrated_card_alone_is_never_taken() {
        let p = Placement::decide([(0, IntegratedGpu), (1, Cpu), (2, Accelerator), (3, Unknown)]);
        assert_eq!(p, Placement::Cpu);
    }

    #[test]
    fn with_both_only_the_dedicated_card_is_taken() {
        let p = Placement::decide([(0, IntegratedGpu), (1, Gpu), (2, Cpu)]);
        assert_eq!(p, Placement::Gpu(vec![1]));
    }

    #[test]
    fn a_card_that_accepts_keeps_the_model() {
        let mut tried = Vec::new();
        let r = with_cpu_fallback(&Placement::Gpu(vec![0]), |p| {
            tried.push(p.clone());
            Ok::<_, &str>("carte")
        });
        assert_eq!(r, Ok(("carte", true)));
        assert_eq!(tried, [Placement::Gpu(vec![0])]);
    }

    #[test]
    fn a_card_too_small_falls_back_to_the_processor_once() {
        let mut tried = Vec::new();
        let r = with_cpu_fallback(&Placement::Gpu(vec![0]), |p| {
            tried.push(p.clone());
            if p.on_gpu() {
                Err("mémoire")
            } else {
                Ok("processeur")
            }
        });
        assert_eq!(r, Ok(("processeur", false)));
        assert_eq!(tried, [Placement::Gpu(vec![0]), Placement::Cpu]);
    }

    #[test]
    fn the_error_is_shown_only_if_the_processor_fails_too() {
        let mut calls = 0;
        let r = with_cpu_fallback(&Placement::Gpu(vec![0]), |p| {
            calls += 1;
            Err::<(), _>(if p.on_gpu() { "carte" } else { "processeur" })
        });
        assert_eq!(r, Err("processeur"));
        assert_eq!(calls, 2);
    }

    #[test]
    fn a_processor_failure_is_not_retried() {
        let mut calls = 0;
        let r = with_cpu_fallback(&Placement::Cpu, |_| {
            calls += 1;
            Err::<(), _>("processeur")
        });
        assert_eq!(r, Err("processeur"));
        assert_eq!(calls, 1);
    }

    #[test]
    fn a_context_refused_by_the_card_is_retried_once_on_the_processor() {
        let mut calls = 0;
        let r = retry_context_on_cpu(
            Err::<&str, _>(AiError::Context("mémoire".into())),
            true,
            || {
                calls += 1;
                Ok("processeur")
            },
        );
        assert_eq!(r, Ok("processeur"));
        assert_eq!(calls, 1);
    }

    #[test]
    fn a_context_refused_on_the_processor_is_not_retried() {
        let mut calls = 0;
        let first = Err::<&str, _>(AiError::Context("mémoire".into()));
        let r = retry_context_on_cpu(first.clone(), false, || {
            calls += 1;
            Ok("encore")
        });
        assert_eq!(r, first);
        assert_eq!(calls, 0);
    }

    #[test]
    fn another_error_on_the_card_is_not_retried() {
        let mut calls = 0;
        for first in [
            Err::<&str, _>(AiError::Decode("décodage".into())),
            Err(AiError::Load("chargement".into())),
            Err(AiError::PromptTooLong),
        ] {
            let r = retry_context_on_cpu(first.clone(), true, || {
                calls += 1;
                Ok("encore")
            });
            assert_eq!(r, first);
        }
        assert_eq!(calls, 0);
    }

    #[test]
    fn a_success_on_the_card_is_kept() {
        let mut calls = 0;
        let r = retry_context_on_cpu(Ok::<_, AiError>("carte"), true, || {
            calls += 1;
            Ok("processeur")
        });
        assert_eq!(r, Ok("carte"));
        assert_eq!(calls, 0);
    }

    /// Ne prouve pas que la liste vide est passée : `devices()` est vide aussi pour
    /// `LlamaModelParams::default()` (tableau rempli de pointeurs nuls). La règle de la carte
    /// intégrée se vérifie à la main (`CASSINI_DEBUG_LLAMA=1`, aucune couche sur la carte).
    #[test]
    fn processor_parameters_offload_nothing() {
        assert_eq!(Placement::Cpu.params().n_gpu_layers(), 0);
        assert!(Placement::Cpu.params().devices().is_empty());
    }
}
