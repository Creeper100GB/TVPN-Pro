//! Tauri shell for the TVPN Pro UI.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod download;
mod hotkey;
mod service;

use download::Downloads;
use service::ServiceClient;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};

fn main() {
    tauri::Builder::default()
        .manage(Downloads::default())
        .manage(ServiceClient::default())
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Öffnen", true, None::<&str>)?;
            let panic = MenuItem::with_id(
                app,
                "panic",
                "Panik (Strg+Alt+Shift+F12)",
                true,
                None::<&str>,
            )?;
            let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &panic, &quit])?;

            let mut tray = TrayIconBuilder::new()
                .tooltip("TVPN Pro")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_main(app),
                    "panic" => {
                        if let Some(client) = app.try_state::<ServiceClient>() {
                            let _ = service::panic_now(client.inner());
                        }
                        let _ = app.emit("panic-requested", ());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            if let Err(error) = tray.build(app) {
                eprintln!("tvpn-pro: tray unavailable: {error}");
            }

            hotkey::spawn(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
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

fn show_main<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
