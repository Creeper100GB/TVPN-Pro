//! Tauri shell for the TVPN Pro UI.
//!
//! There is no IPC yet: the front end runs against mock data. The service and
//! its named pipe arrive in Phase 2, together with the state machine.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run TVPN Pro");
}
