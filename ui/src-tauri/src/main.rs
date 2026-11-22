//! Tauri shell for the TVPN Pro UI.
//!
//! Until `tvpnd` exists, the shell owns the download engine. The service will
//! take over the protected routes in Phase 2.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod download;
mod service;

use download::Downloads;
use service::ServiceClient;

fn main() {
    tauri::Builder::default()
        .manage(Downloads::default())
        .manage(ServiceClient::default())
        .invoke_handler(tauri::generate_handler![
            download::download_start,
            download::download_cancel,
            service::service_status,
            service::service_command,
            service::service_install
        ])
        .run(tauri::generate_context!())
        .expect("failed to run TVPN Pro");
}
