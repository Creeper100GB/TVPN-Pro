//! Client for the local `tvpnd` named pipe.
//!
//! Every UI command opens one short-lived connection, sends `hello` and the
//! command, reads one response and closes. The service serialises connections,
//! so a mutex here prevents the UI from opening two at once.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tauri::State;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, OPEN_EXISTING,
};

/// Serializes pipe calls from the Tauri commands.
#[derive(Default, Clone)]
pub struct ServiceClient(pub Arc<Mutex<()>>);

const ALLOWED: &[&str] = &[
    "enable",
    "disable",
    "panic",
    "recover",
    "get_status",
    "get_settings",
    "set_settings",
];

fn programdata() -> PathBuf {
    std::env::var("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\ProgramData"))
        .join("TVPN")
}

fn token_path() -> PathBuf {
    #[cfg(debug_assertions)]
    if let Ok(path) = std::env::var("TVPN_TOKEN_PATH") {
        return PathBuf::from(path);
    }
    programdata().join("session.token")
}

fn pipe_name() -> String {
    #[cfg(debug_assertions)]
    if let Ok(name) = std::env::var("TVPN_PIPE_NAME") {
        return name;
    }
    r"\\.\pipe\tvpn-pro-v0".to_owned()
}

#[cfg(windows)]
fn connect() -> Result<HANDLE, String> {
    let name: Vec<u16> = pipe_name()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        CreateFileW(
            PCWSTR(name.as_ptr()),
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_NONE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
        .map_err(|_| "service_unavailable".to_owned())
    }
}

#[cfg(not(windows))]
fn connect() -> Result<(), String> {
    Err("service_unavailable".to_owned())
}

#[cfg(windows)]
fn send(handle: HANDLE, value: &Value) -> Result<(), String> {
    let mut line = serde_json::to_vec(value).map_err(|_| "service_json".to_owned())?;
    line.push(b'\n');
    let mut written = 0u32;
    unsafe {
        WriteFile(handle, Some(&line), Some(&mut written), None)
            .map_err(|_| "service_write".to_owned())?;
    }
    Ok(())
}

#[cfg(windows)]
fn read_line(handle: HANDLE) -> Result<String, String> {
    let mut pending = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let mut read = 0u32;
        unsafe {
            ReadFile(handle, Some(&mut buffer), Some(&mut read), None)
                .map_err(|_| "service_read".to_owned())?;
        }
        if read == 0 {
            return Err("service_closed".to_owned());
        }
        pending.extend_from_slice(&buffer[..read as usize]);
        if let Some(position) = pending.iter().position(|byte| *byte == b'\n') {
            pending.truncate(position);
            return String::from_utf8(pending).map_err(|_| "service_utf8".to_owned());
        }
        if pending.len() > 64 * 1024 {
            return Err("service_too_large".to_owned());
        }
    }
}

#[cfg(windows)]
fn call(command: &str, args: Value) -> Result<Value, String> {
    let token = std::fs::read_to_string(token_path()).map_err(|_| "service_token".to_owned())?;
    let token = token.trim();
    let handle = connect()?;

    let result = (|| {
        send(
            handle,
            &json!({
                "v": 0,
                "id": "hello",
                "cmd": "hello",
                "args": { "token": token, "client": "ui", "client_version": env!("CARGO_PKG_VERSION") }
            }),
        )?;
        let hello: Value =
            serde_json::from_str(&read_line(handle)?).map_err(|_| "service_json".to_owned())?;
        if hello["ok"].as_bool() != Some(true) {
            return Err(hello["error"]["code"]
                .as_str()
                .unwrap_or("E_IPC_AUTH")
                .to_owned());
        }

        send(
            handle,
            &json!({ "v": 0, "id": "ui", "cmd": command, "args": args }),
        )?;
        let response: Value =
            serde_json::from_str(&read_line(handle)?).map_err(|_| "service_json".to_owned())?;
        if response["ok"].as_bool() == Some(true) {
            Ok(response["data"].clone())
        } else {
            Err(response["error"]["code"]
                .as_str()
                .unwrap_or("service_error")
                .to_owned())
        }
    })();

    unsafe {
        let _ = CloseHandle(handle);
    }
    result
}

#[cfg(not(windows))]
fn call(_command: &str, _args: Value) -> Result<Value, String> {
    Err("service_unavailable".to_owned())
}

/// Snapshot for the UI. Never returns paths or secrets.
#[tauri::command]
pub async fn service_status(state: State<'_, ServiceClient>) -> Result<Value, String> {
    let lock = Arc::clone(&state.0);
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _guard = lock.lock().map_err(|_| "service_lock".to_owned())?;
        call("get_status", json!({}))
    })
    .await;
    match result {
        Ok(value) => value,
        Err(_) => Err("service_task".to_owned()),
    }
}

/// Fixed command whitelist. The shell never forwards arbitrary commands.
#[tauri::command]
pub async fn service_command(
    state: State<'_, ServiceClient>,
    command: String,
    args: Value,
) -> Result<Value, String> {
    if !ALLOWED.contains(&command.as_str()) {
        return Err("E_IPC_UNKNOWN_COMMAND".to_owned());
    }
    let lock = Arc::clone(&state.0);
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _guard = lock.lock().map_err(|_| "service_lock".to_owned())?;
        call(&command, args)
    })
    .await;
    match result {
        Ok(value) => value,
        Err(_) => Err("service_task".to_owned()),
    }
}

/// Synchronous panic used by the tray icon and the global shortcut. It works
/// even when the webview is closed.
pub fn panic_now(client: &ServiceClient) -> Result<Value, String> {
    let _guard = client.0.lock().map_err(|_| "service_lock".to_owned())?;
    call("panic", json!({}))
}
