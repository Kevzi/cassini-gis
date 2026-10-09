// Sous Windows, en build release, ne pas ouvrir de console derrière la fenêtre.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cassini_app_lib::run()
}
