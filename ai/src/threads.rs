//! Nombre de fils par défaut du moteur (ticket 5.20) : les cœurs physiques dont au moins un
//! processeur logique est autorisé au processus (masque d'affinité), sous Windows et sous Linux.
//!
//! Le comptage est un calcul pur, testé sur des masques fabriqués ; seule la lecture de la
//! topologie et du masque dépend de la plateforme. Si la lecture échoue, ou sur une machine à
//! plus de 64 processeurs logiques, on retombe sans bruit sur l'ancienne formule : les cœurs
//! physiques de la machine (`num_cpus`). Sous Linux, un cœur physique est un ensemble distinct de
//! processeurs frères (`thread_siblings_list`) ; sous Windows, une entrée `RelationProcessorCore`.

use std::num::NonZeroU32;

/// Nombre de fils par défaut : les cœurs physiques que le processus a le droit d'utiliser,
/// sans dépasser les fils qu'il peut utiliser (`available_parallelism`, qui voit les quotas),
/// au moins un.
pub fn default_threads() -> NonZeroU32 {
    let usable = std::thread::available_parallelism().map_or(usize::MAX, |n| n.get());
    pick_threads(allowed_physical_cores(), num_cpus::get_physical, usable)
}

/// Choix pur : les cœurs autorisés lus (`read`), ou l'ancienne formule (`fallback`) si la
/// lecture a échoué ou rend 0 ; borné par `usable`, au moins un.
fn pick_threads(
    read: Option<usize>,
    fallback: impl FnOnce() -> usize,
    usable: usize,
) -> NonZeroU32 {
    let physical = read.filter(|&n| n > 0).unwrap_or_else(fallback);
    u32::try_from(physical.min(usable))
        .ok()
        .and_then(NonZeroU32::new)
        .unwrap_or(NonZeroU32::MIN)
}

/// Nombre de cœurs physiques dont le masque de processeurs logiques (`cores`, un masque par
/// cœur) croise le masque des processeurs autorisés `allowed`.
pub(crate) fn cores_allowed(cores: &[u64], allowed: u64) -> usize {
    cores.iter().filter(|&&core| core & allowed != 0).count()
}

/// Cœurs physiques autorisés au processus, lus sur la plateforme ; `None` si la lecture échoue.
#[cfg(target_os = "linux")]
fn allowed_physical_cores() -> Option<usize> {
    linux::allowed_physical_cores()
}

/// Cœurs physiques autorisés au processus, lus sur la plateforme ; `None` si la lecture échoue.
#[cfg(windows)]
fn allowed_physical_cores() -> Option<usize> {
    windows::allowed_physical_cores()
}

/// Ailleurs, pas de lecture : l'ancienne formule s'applique.
#[cfg(not(any(target_os = "linux", windows)))]
fn allowed_physical_cores() -> Option<usize> {
    None
}

/// Lit une liste de processeurs au format du noyau Linux (`0-3,8,10-11`) en masque ; `None` si
/// le texte est mal formé ou nomme un processeur au-delà du 64e.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_cpu_list(text: &str) -> Option<u64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let mut mask = 0u64;
    for part in text.split(',') {
        let (lo, hi) = match part.split_once('-') {
            Some((a, b)) => (a.trim().parse::<u32>().ok()?, b.trim().parse::<u32>().ok()?),
            None => {
                let n = part.trim().parse::<u32>().ok()?;
                (n, n)
            }
        };
        if lo > hi || hi >= 64 {
            return None;
        }
        for cpu in lo..=hi {
            mask |= 1 << cpu;
        }
    }
    Some(mask)
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{cores_allowed, parse_cpu_list};
    use std::collections::BTreeSet;
    use std::fs;

    /// Masque autorisé dans `/proc/self/status` (`Cpus_allowed_list`) ; un cœur physique par
    /// masque distinct de `/sys/devices/system/cpu/cpuN/topology/thread_siblings_list`, pour
    /// chaque processeur en ligne (`core_id` n'est pas unique d'une puce à l'autre).
    pub(super) fn allowed_physical_cores() -> Option<usize> {
        let status = fs::read_to_string("/proc/self/status").ok()?;
        let allowed = status
            .lines()
            .find_map(|l| l.strip_prefix("Cpus_allowed_list:"))
            .and_then(parse_cpu_list)?;
        let online = parse_cpu_list(&fs::read_to_string("/sys/devices/system/cpu/online").ok()?)?;
        let mut cores = BTreeSet::new();
        for cpu in (0..64).filter(|cpu| online & (1u64 << cpu) != 0) {
            let siblings = fs::read_to_string(format!(
                "/sys/devices/system/cpu/cpu{cpu}/topology/thread_siblings_list"
            ))
            .ok()?;
            cores.insert(parse_cpu_list(&siblings)?);
        }
        let cores: Vec<u64> = cores.into_iter().collect();
        Some(cores_allowed(&cores, allowed))
    }
}

#[cfg(windows)]
mod windows {
    use super::cores_allowed;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_INSUFFICIENT_BUFFER};
    use windows_sys::Win32::System::SystemInformation::{
        GetLogicalProcessorInformation, RelationProcessorCore, SYSTEM_LOGICAL_PROCESSOR_INFORMATION,
    };
    use windows_sys::Win32::System::Threading::{
        GetActiveProcessorGroupCount, GetCurrentProcess, GetProcessAffinityMask,
    };

    /// Masque autorisé par `GetProcessAffinityMask`, cœurs physiques par
    /// `GetLogicalProcessorInformation` (entrées `RelationProcessorCore`). Plus d'un groupe de
    /// processeurs (plus de 64 processeurs logiques) : `None`, l'ancienne formule s'applique.
    pub(super) fn allowed_physical_cores() -> Option<usize> {
        // SAFETY : appels Win32 sans précondition ; les pointeurs passés désignent des variables
        // et un tampon locaux, valides et assez grands pour la longueur annoncée.
        unsafe {
            if GetActiveProcessorGroupCount() != 1 {
                return None;
            }
            let (mut process, mut system) = (0usize, 0usize);
            if GetProcessAffinityMask(GetCurrentProcess(), &mut process, &mut system) == 0 {
                return None;
            }
            let size = std::mem::size_of::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION>();
            let mut len = 0u32;
            if GetLogicalProcessorInformation(std::ptr::null_mut(), &mut len) != 0
                || GetLastError() != ERROR_INSUFFICIENT_BUFFER
            {
                return None;
            }
            let count = (len as usize).div_ceil(size);
            let mut buffer: Vec<SYSTEM_LOGICAL_PROCESSOR_INFORMATION> =
                vec![std::mem::zeroed(); count];
            let mut len = u32::try_from(count * size).ok()?;
            if GetLogicalProcessorInformation(buffer.as_mut_ptr(), &mut len) == 0 {
                return None;
            }
            buffer.truncate((len as usize / size).min(count));
            let cores: Vec<u64> = buffer
                .iter()
                .filter(|info| info.Relationship == RelationProcessorCore)
                .map(|info| info.ProcessorMask as u64)
                .collect();
            Some(cores_allowed(&cores, process as u64))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le 5800X : 8 cœurs, le cœur k porte les processeurs logiques `2k` et `2k+1`.
    fn ryzen_5800x() -> Vec<u64> {
        (0..8).map(|k| 0b11u64 << (2 * k)).collect()
    }

    #[test]
    fn without_mask_every_core_counts() {
        assert_eq!(cores_allowed(&ryzen_5800x(), 0xFFFF), 8);
    }

    #[test]
    fn one_thread_per_core_counts_each_core() {
        assert_eq!(cores_allowed(&ryzen_5800x(), 0x55), 4);
    }

    #[test]
    fn both_threads_of_one_core_count_once() {
        assert_eq!(cores_allowed(&ryzen_5800x(), 0x3), 1);
    }

    #[test]
    fn a_mask_across_two_cores_counts_two() {
        assert_eq!(cores_allowed(&ryzen_5800x(), 0x7), 2);
    }

    #[test]
    fn a_machine_without_smt_counts_its_cores() {
        let cores: Vec<u64> = (0..4).map(|k| 1u64 << k).collect();
        assert_eq!(cores_allowed(&cores, 0xF), 4);
    }

    #[test]
    fn cpu_lists_parse_as_masks() {
        assert_eq!(parse_cpu_list("0-15\n"), Some(0xFFFF));
        assert_eq!(parse_cpu_list("0,2,4,6"), Some(0x55));
        assert_eq!(parse_cpu_list("0-1,8"), Some(0x103));
        assert_eq!(parse_cpu_list("63"), Some(1 << 63));
        assert_eq!(parse_cpu_list(""), None);
        assert_eq!(parse_cpu_list("0-64"), None, "au-delà du 64e : repli");
        assert_eq!(parse_cpu_list("3-1"), None);
        assert_eq!(parse_cpu_list("a"), None);
    }

    #[test]
    fn a_failed_read_falls_back_to_the_old_formula() {
        assert_eq!(pick_threads(None, || 8, 16).get(), 8);
        assert_eq!(pick_threads(Some(0), || 8, 16).get(), 8);
        assert_eq!(pick_threads(Some(4), || 8, 16).get(), 4);
        assert_eq!(
            pick_threads(Some(8), || 8, 2).get(),
            2,
            "borné par les fils utilisables"
        );
        assert_eq!(pick_threads(None, || 0, 16).get(), 1, "au moins un");
    }

    /// La lecture dépend de la machine qui lance les tests : on vérifie seulement les bornes.
    #[test]
    fn default_threads_stays_within_usable_threads() {
        let usable = std::thread::available_parallelism().map_or(usize::MAX, |n| n.get());
        let n = default_threads().get() as usize;
        assert!((1..=usable).contains(&n), "{n} hors de 1..={usable}");
    }
}
