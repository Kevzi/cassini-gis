//! Garde réseau à l'exécution : les sockets TCP et UDP qu'un processus tient. Le canal local
//! (socket Unix sous Linux, tube nommé sous Windows) n'en est pas une, et reste permis (AD-29).
//! Aucune dépendance : `/proc` sous Linux, `netstat -ano` sous Windows. C'est un instantané pris
//! aux moments où les tests l'appellent : une socket ouverte puis fermée entre deux appels n'est
//! pas vue ; le contrôle statique des imports (`hors_ligne.rs`) couvre le binaire entier.

#![allow(dead_code)]

/// Les sockets TCP ou UDP tenues par le processus `pid`, une ligne lisible par socket. Vide si
/// le processus n'en tient aucune ; vide aussi, avec une note, sur un système non pris en charge.
pub fn inet_sockets(pid: u32) -> Vec<String> {
    sockets_of(pid)
}

/// Vérifie que le processus `pid` (nommé `who`) ne tient aucune socket réseau ; sinon échoue en
/// listant ce qui a été trouvé.
pub fn assert_offline(who: &str, pid: u32) {
    let found = inet_sockets(pid);
    assert!(
        found.is_empty(),
        "{who} (pid {pid}) tient {} socket(s) réseau, alors qu'aucune fonction ne doit utiliser \
         le réseau :\n{}",
        found.len(),
        found.join("\n")
    );
}

#[cfg(target_os = "linux")]
fn sockets_of(pid: u32) -> Vec<String> {
    use std::collections::HashSet;

    // Les inodes des sockets ouvertes par le processus : liens `socket:[inode]` de `fd`.
    let Ok(entries) = std::fs::read_dir(format!("/proc/{pid}/fd")) else {
        panic!("impossible de lire /proc/{pid}/fd : le processus est-il encore vivant ?");
    };
    let inodes: HashSet<String> = entries
        .filter_map(Result::ok)
        .filter_map(|e| std::fs::read_link(e.path()).ok())
        .filter_map(|target| {
            let target = target.to_string_lossy().into_owned();
            target
                .strip_prefix("socket:[")
                .and_then(|rest| rest.strip_suffix(']'))
                .map(str::to_string)
        })
        .collect();
    let mut found = Vec::new();
    if inodes.is_empty() {
        return found;
    }
    // Les tables réseau de l'espace de noms du processus ; colonne 10 : l'inode.
    for table in ["tcp", "tcp6", "udp", "udp6"] {
        let path = format!("/proc/{pid}/net/{table}");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("impossible de lire {path} : {e}"));
        for line in text.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() > 9 && inodes.contains(cols[9]) {
                found.push(format!(
                    "{table} local {} distante {} (inode {})",
                    cols[1], cols[2], cols[9]
                ));
            }
        }
    }
    found
}

#[cfg(windows)]
fn sockets_of(pid: u32) -> Vec<String> {
    let out = std::process::Command::new("netstat")
        .arg("-ano")
        .output()
        .expect("netstat se lance");
    assert!(
        out.status.success(),
        "netstat -ano a échoué ({}) : {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let pid = pid.to_string();
    text.lines()
        .filter(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            matches!(cols.first(), Some(&"TCP") | Some(&"UDP")) && cols.last() == Some(&&*pid)
        })
        .map(|line| line.trim().to_string())
        .collect()
}

#[cfg(not(any(target_os = "linux", windows)))]
fn sockets_of(_pid: u32) -> Vec<String> {
    eprintln!("note : la garde réseau ne sait pas lire les sockets sur ce système ; non vérifié.");
    Vec::new()
}
