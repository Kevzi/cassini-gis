//! Coquille de bureau.
//!
//! Aucune logique métier ici : la coquille ouvre une fenêtre et relaie les
//! appels vers `cassini-core` (CLAUDE.md §2.5). Tant que le registre de
//! commandes n'existe pas (étape 2), elle n'expose que la version du moteur.

/// Version du moteur, pour vérifier de bout en bout que le pont fonctionne.
#[tauri::command]
fn version_du_moteur() -> String {
    cassini_core::VERSION.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![version_du_moteur])
        .run(tauri::generate_context!())
        .expect("le lancement de la fenêtre Cassini a échoué");
}
