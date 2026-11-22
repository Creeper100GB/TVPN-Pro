//! System-wide panic hotkey.
//!
//! Tauri's global-shortcut plugin registered the combination but did not
//! deliver the callback reliably in the installed app. This small Win32
//! message loop owns RegisterHotKey directly and calls the service without
//! depending on the webview.

#[cfg(windows)]
pub fn spawn(app: tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_SHIFT, VK_F12,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

    std::thread::spawn(move || {
        let modifiers = MOD_CONTROL | MOD_ALT | MOD_SHIFT;
        if unsafe { RegisterHotKey(None, 1, modifiers, u32::from(VK_F12.0)) }.is_err() {
            return;
        }

        let mut message = MSG::default();
        while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
            if message.message == WM_HOTKEY {
                if let Some(client) = app.try_state::<crate::service::ServiceClient>() {
                    let _ = crate::service::panic_now(client.inner());
                }
                let _ = app.emit("panic-requested", ());
            }
        }

        let _ = unsafe { UnregisterHotKey(None, 1) };
    });
}

#[cfg(not(windows))]
pub fn spawn(_app: tauri::AppHandle) {}
