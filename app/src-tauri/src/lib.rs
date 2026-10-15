//! Coquille de bureau.
//!
//! Aucune logique métier ici : la coquille ouvre une fenêtre et relaie les
//! appels vers `cassini-core` (CLAUDE.md §2.5). Tant qu'elle n'appelle pas le
//! registre de commandes, elle n'expose que la version du moteur et la lecture
//! du fond de carte.

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
/// développement. `app/scripts/verify-protocol.mjs` vérifie cette concordance.
const ARCHIVE_PATH: &str = "fixtures/basemap-villeneuve-le-roi.pmtiles";

/// Version du moteur, pour vérifier de bout en bout que le pont fonctionne.
#[tauri::command]
fn engine_version() -> String {
    cassini_core::VERSION.to_string()
}

/// Les emplacements où chercher l'archive, dans l'ordre.
///
/// Le choix lui-même vit dans le moteur
/// (`BasemapSource::from_first_location`), qui est testé ; ici il n'y
/// a que la construction de la liste, parce qu'elle dépend de Tauri.
fn candidate_locations(app: &tauri::AppHandle) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(in_resources) = app.path().resolve(ARCHIVE_PATH, BaseDirectory::Resource) {
        candidates.push(in_resources);
    }
    // Repli de développement seulement : `cargo tauri dev` ne passe pas par
    // l'installeur. Ce chemin vient de `CARGO_MANIFEST_DIR`, donc de la machine
    // de compilation ; il n'a aucun sens dans un binaire distribué et n'a rien
    // à faire dans un message montré à l'utilisateur, d'où la condition.
    if cfg!(debug_assertions) {
        // `app/src-tauri` → `..` → `app` → `..` → racine du dépôt.
        candidates.push(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join(ARCHIVE_PATH),
        );
    }
    candidates
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
fn read_basemap(app: tauri::AppHandle, offset: u64, length: u64) -> Result<Response, String> {
    let source = BasemapSource::from_first_location(&candidate_locations(&app))
        .map_err(|error| error.to_string())?;
    let bytes = source
        .read_range(offset, length)
        .map_err(|error| error.to_string())?;
    Ok(Response::new(bytes))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![engine_version, read_basemap])
        .run(tauri::generate_context!())
        .expect("le lancement de la fenêtre Cassini a échoué");
}
