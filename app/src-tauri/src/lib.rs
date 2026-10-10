//! Coquille de bureau.
//!
//! Aucune logique métier ici : la coquille ouvre une fenêtre et relaie les
//! appels vers `cassini-core` (CLAUDE.md §2.5). Tant que le registre de
//! commandes n'existe pas (étape 2), elle n'expose que la version du moteur et
//! la lecture du fond de carte.

use std::path::PathBuf;

use cassini_core::basemap::BasemapSource;
use tauri::ipc::Response;
use tauri::path::BaseDirectory;
use tauri::Manager;

/// Chemin de l'archive du fond de carte, relatif au répertoire de ressources.
///
/// **Unique endroit du code qui porte ce chemin.** Il vaut aussi pour le dépôt :
/// `tauri.conf.json > bundle > resources` copie `fixtures/…` de la racine du
/// dépôt vers `$RESOURCE/fixtures/…`, donc la même chaîne sert au repli de
/// développement. `app/scripts/verify-protocole.mjs` vérifie cette concordance.
const CHEMIN_DE_LARCHIVE: &str = "fixtures/basemap-villeneuve-le-roi.pmtiles";

/// Version du moteur, pour vérifier de bout en bout que le pont fonctionne.
#[tauri::command]
fn version_du_moteur() -> String {
    cassini_core::VERSION.to_string()
}

/// Les emplacements où chercher l'archive, dans l'ordre.
///
/// Le choix lui-même vit dans le moteur
/// (`BasemapSource::depuis_le_premier_emplacement`), qui est testé ; ici il n'y
/// a que la construction de la liste, parce qu'elle dépend de Tauri.
fn emplacements_candidats(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut candidats: Vec<PathBuf> = Vec::new();
    if let Ok(dans_les_ressources) = app
        .path()
        .resolve(CHEMIN_DE_LARCHIVE, BaseDirectory::Resource)
    {
        candidats.push(dans_les_ressources);
    }
    // Repli de développement seulement : `cargo tauri dev` ne passe pas par
    // l'installeur. Ce chemin vient de `CARGO_MANIFEST_DIR`, donc de la machine
    // de compilation ; il n'a aucun sens dans un binaire distribué et n'a rien
    // à faire dans un message montré à l'utilisateur, d'où la condition.
    if cfg!(debug_assertions) {
        // `app/src-tauri` → `..` → `app` → `..` → racine du dépôt.
        candidats.push(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join(CHEMIN_DE_LARCHIVE),
        );
    }
    candidats
}

/// Lit une plage d'octets de l'archive du fond de carte.
///
/// L'interface ne connaît jamais le chemin du fichier : elle demande des octets,
/// et le moteur décide s'il a le droit de les lire. C'est ce qui remplace, pour
/// le fond de carte, une requête réseau que le MVP s'interdit (CLAUDE.md §2.3).
///
/// Le retour est un `tauri::ipc::Response`, qui traverse l'IPC en binaire, là où
/// un `Vec<u8>` serait sérialisé en tableau JSON.
#[tauri::command]
fn lire_le_fond_de_carte(
    app: tauri::AppHandle,
    decalage: u64,
    longueur: u64,
) -> Result<Response, String> {
    let source = BasemapSource::depuis_le_premier_emplacement(&emplacements_candidats(&app))
        .map_err(|erreur| erreur.to_string())?;
    let octets = source
        .lire_plage(decalage, longueur)
        .map_err(|erreur| erreur.to_string())?;
    Ok(Response::new(octets))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            version_du_moteur,
            lire_le_fond_de_carte
        ])
        .run(tauri::generate_context!())
        .expect("le lancement de la fenêtre Cassini a échoué");
}
