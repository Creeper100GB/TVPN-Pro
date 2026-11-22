//! Real download engine for the desktop shell.
//!
//! This is shell-side on purpose until `tvpnd` exists (Phase 2). The routes are
//! honest: `direct` is unprotected, `tor` requires a local SOCKS proxy on
//! 127.0.0.1:9150, `vpn` is refused because the service owns that path.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::AsyncWriteExt;

/// Cancel switches, keyed by download id.
#[derive(Default)]
pub struct Downloads(Mutex<HashMap<String, Arc<AtomicBool>>>);

impl Downloads {
    pub fn cancel_all(&self) {
        if let Ok(map) = self.0.lock() {
            for flag in map.values() {
                flag.store(true, Ordering::Relaxed);
            }
        }
    }
}

/// Hard ceiling for a single download. Prevents a malicious server from
/// filling the disk before the scan runs.
const MAX_DOWNLOAD_BYTES: u64 = 8 * 1024 * 1024 * 1024;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    id: String,
    received: u64,
    total: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Done {
    id: String,
    path: String,
    bytes: u64,
    sha256: String,
    verdict: String,
}

#[derive(Clone, Serialize)]
struct Failure {
    id: String,
    code: String,
}

#[tauri::command]
pub fn download_cancel(state: State<'_, Downloads>, id: String) {
    if let Ok(map) = state.0.lock() {
        if let Some(flag) = map.get(&id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

#[tauri::command]
pub fn download_cancel_all(state: State<'_, Downloads>) {
    state.cancel_all();
}

#[tauri::command]
pub async fn download_start(
    app: AppHandle,
    state: State<'_, Downloads>,
    id: String,
    url: String,
    route: String,
) -> Result<(), String> {
    tvpn_core::url::validate(&url).map_err(|error| error.detail().to_string())?;

    match route.as_str() {
        "direct" | "tor" => {}
        "vpn" => return Err("vpn_requires_service".to_owned()),
        _ => return Err("route".to_owned()),
    }

    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut map) = state.0.lock() {
        map.insert(id.clone(), cancel.clone());
    }

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let emitter = handle.clone();
        if let Err(code) = transfer(&emitter, &id, &url, &route, cancel).await {
            let _ = emitter.emit(
                "download-error",
                Failure {
                    id: id.clone(),
                    code,
                },
            );
        }
        if let Some(downloads) = emitter.try_state::<Downloads>() {
            if let Ok(mut map) = downloads.0.lock() {
                map.remove(&id);
            }
        }
    });

    Ok(())
}

async fn transfer(
    app: &AppHandle,
    id: &str,
    url: &str,
    route: &str,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let proxy = if route == "tor" {
        Some("socks5h://127.0.0.1:9150")
    } else {
        None
    };

    let path = destination(app, url)?;
    let staging = staging_path(app, id)?;
    let progress_id = id.to_owned();

    let fetched = fetch_to_file(url, &staging, proxy, cancel, |bytes, total| {
        let _ = app.emit(
            "download-progress",
            Progress {
                id: progress_id.clone(),
                received: bytes,
                total,
            },
        );
    })
    .await;

    let (received, digest) = match fetched {
        Ok(value) => value,
        Err(error) => {
            let _ = tokio::fs::remove_file(&staging).await;
            return Err(error);
        }
    };

    let mut final_path = path.clone();
    let mut verdict = scan(&staging).await;
    if verdict == "threat" {
        match quarantine(app, id, &staging) {
            Ok(target) => final_path = target,
            Err(_) => {
                let _ = std::fs::remove_file(&staging);
                verdict = "failed".to_owned();
            }
        }
    } else {
        if move_into_place(&staging, &path).is_err() {
            let _ = std::fs::remove_file(&staging);
            return Err("move".to_owned());
        }
        if verdict == "clean" {
            let _ = mark_of_the_web(&path);
        }
    }

    let _ = app.emit(
        "download-done",
        Done {
            id: id.to_owned(),
            path: final_path.to_string_lossy().into_owned(),
            bytes: received,
            sha256: digest,
            verdict,
        },
    );
    let _ = app.emit(
        "download-progress",
        Progress {
            id: id.to_owned(),
            received,
            total: Some(received),
        },
    );

    Ok(())
}

/// Fetches `url` into `dest`, hashing on the fly.
///
/// Kept free of Tauri types so the engine can be tested against a local server.
pub async fn fetch_to_file<F>(
    url: &str,
    dest: &Path,
    proxy: Option<&str>,
    cancel: Arc<AtomicBool>,
    on_tick: F,
) -> Result<(u64, String), String>
where
    F: Fn(u64, Option<u64>) + Send + Sync,
{
    let mut builder = reqwest::Client::builder()
        .user_agent("TVPN-Pro/0.0.0")
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > 8 {
                return attempt.stop();
            }
            if tvpn_core::url::validate(attempt.url().as_str()).is_ok() {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }));
    if let Some(proxy) = proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(|_| "proxy".to_owned())?);
    }
    let client = builder.build().map_err(|_| "client".to_owned())?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| "network".to_owned())?;

    if !response.status().is_success() {
        return Err(format!("http_{}", response.status().as_u16()));
    }

    let total = response.content_length();
    if total.is_some_and(|value| value > MAX_DOWNLOAD_BYTES) {
        return Err("too_large".to_owned());
    }
    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|_| "create_file".to_owned())?;

    let mut hasher = Sha256::new();
    let mut received: u64 = 0;
    let mut last_emit = Instant::now();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            let _ = tokio::fs::remove_file(dest).await;
            return Err("cancelled".to_owned());
        }
        let chunk = chunk.map_err(|_| "read".to_owned())?;
        if received.saturating_add(chunk.len() as u64) > MAX_DOWNLOAD_BYTES {
            drop(file);
            let _ = tokio::fs::remove_file(dest).await;
            return Err("too_large".to_owned());
        }
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|_| "write".to_owned())?;
        received += chunk.len() as u64;

        if last_emit.elapsed() >= Duration::from_millis(250) {
            on_tick(received, total);
            last_emit = Instant::now();
        }
    }

    file.flush().await.map_err(|_| "flush".to_owned())?;
    drop(file);

    let digest: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();

    Ok((received, digest))
}

/// Mark-of-the-Web so SmartScreen and Office still warn after the download.
fn mark_of_the_web(path: &Path) -> std::io::Result<()> {
    std::fs::write(
        format!("{}:Zone.Identifier", path.display()),
        b"[ZoneTransfer]\r\nZoneId=3\r\n",
    )
}

/// Moves a known threat out of the download folder under a generated name.
fn quarantine(app: &AppHandle, id: &str, path: &Path) -> Result<PathBuf, String> {
    let safe: String = id
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || *character == '-' || *character == '_'
        })
        .collect();
    let safe = if safe.is_empty() {
        "download".to_owned()
    } else {
        safe
    };

    let folder = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "quarantine_dir".to_owned())?
        .join("quarantine");
    std::fs::create_dir_all(&folder).map_err(|_| "quarantine_dir".to_owned())?;
    let target = folder.join(format!("{safe}.bin"));

    match std::fs::rename(path, &target) {
        Ok(()) => Ok(target),
        Err(_) => {
            std::fs::copy(path, &target).map_err(|_| "quarantine_move".to_owned())?;
            let _ = std::fs::remove_file(path);
            Ok(target)
        }
    }
}

fn safe_id(id: &str) -> String {
    let safe: String = id
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || *character == '-' || *character == '_'
        })
        .collect();
    if safe.is_empty() {
        "download".to_owned()
    } else {
        safe
    }
}

fn staging_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    let folder = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "staging_dir".to_owned())?
        .join("staging");
    std::fs::create_dir_all(&folder).map_err(|_| "staging_dir".to_owned())?;
    Ok(folder.join(format!("{}.part", safe_id(id))))
}

fn move_into_place(from: &Path, to: &Path) -> Result<(), ()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to).map_err(|_| ())?;
    std::fs::remove_file(from).map_err(|_| ())?;
    Ok(())
}

fn is_reserved_windows_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0')
}

fn destination(app: &AppHandle, url: &str) -> Result<PathBuf, String> {
    let folder = app
        .path()
        .download_dir()
        .map_err(|_| "no_download_dir".to_owned())?
        .join("TVPN");
    std::fs::create_dir_all(&folder).map_err(|_| "mkdir".to_owned())?;

    let mut name = filename_from_url(url);
    if is_reserved_windows_name(&name) {
        name = format!("download-{name}");
    }
    if name.len() > 180 {
        let extension = Path::new(&name)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let stem = Path::new(&name)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("download");
        let keep = 180usize.saturating_sub(extension.len() + 1);
        let stem: String = stem.chars().take(keep).collect();
        name = if extension.is_empty() {
            stem
        } else {
            format!("{stem}.{extension}")
        };
    }
    let stem = Path::new(&name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("download")
        .to_owned();
    let extension = Path::new(&name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bin")
        .to_owned();

    let mut candidate = folder.join(&name);
    let mut counter = 1;
    while candidate.exists() {
        candidate = folder.join(format!("{stem}-{counter}.{extension}"));
        counter += 1;
    }
    Ok(candidate)
}

/// Last path segment, restricted to characters that cannot escape the folder.
pub fn filename_from_url(url: &str) -> String {
    let without_query = url.split(['?', '#']).next().unwrap_or(url);
    let last = without_query.rsplit('/').next().unwrap_or("");

    let sanitized: String = last
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();

    let trimmed = sanitized.trim_matches('.').trim();
    if trimmed.is_empty() {
        "download.bin".to_owned()
    } else {
        trimmed.to_owned()
    }
}

async fn scan(path: &Path) -> String {
    let candidates = [
        r"C:\Program Files\Windows Defender\MpCmdRun.exe",
        r"C:\Program Files (x86)\Windows Defender\MpCmdRun.exe",
    ];
    let Some(program) = candidates
        .iter()
        .map(Path::new)
        .find(|candidate| candidate.exists())
    else {
        return "unknown".to_owned();
    };

    let program = program.to_path_buf();
    let target = path.to_path_buf();

    let result = tauri::async_runtime::spawn_blocking(move || {
        std::process::Command::new(program)
            .args(["-Scan", "-ScanType", "3", "-File"])
            .arg(target)
            .arg("-DisableRemediation")
            .output()
    })
    .await;

    match result {
        Ok(Ok(output)) => {
            let mut text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
            text.push_str(&String::from_utf8_lossy(&output.stderr).to_ascii_lowercase());

            // MpCmdRun also returns 2 when the scan itself failed (0x80004005),
            // for example when Defender is disabled or replaced by another AV.
            if text.contains("failed with hr") {
                return "unknown".to_owned();
            }

            match output.status.code() {
                Some(0) => "clean".to_owned(),
                Some(2) => "threat".to_owned(),
                _ => "unknown".to_owned(),
            }
        }
        _ => "unknown".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{fetch_to_file, filename_from_url};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    fn keeps_the_last_segment() {
        assert_eq!(
            filename_from_url("https://example.org/files/debian.iso"),
            "debian.iso"
        );
        assert_eq!(
            filename_from_url("https://example.org/files/debian.iso?token=1#part"),
            "debian.iso"
        );
    }

    #[test]
    fn falls_back_without_a_name() {
        assert_eq!(filename_from_url("https://example.org/"), "download.bin");
        assert_eq!(filename_from_url("https://example.org"), "example.org");
    }

    #[test]
    fn strips_path_characters() {
        let name = filename_from_url("https://example.org/a/..%2F..%2Fevil.exe");
        assert!(!name.contains(['/', '\\', ':']));
        assert!(!name.starts_with('.'));
        assert!(name.ends_with(".exe"));

        assert_eq!(
            filename_from_url("https://example.org/a/b_c-d.e"),
            "b_c-d.e"
        );
    }

    #[tokio::test]
    async fn fetches_and_hashes() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("addr");
        let body = b"hello tvpn".to_vec();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buffer = [0u8; 1024];
            let _ = stream.read(&mut buffer);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("header");
            stream.write_all(&body).expect("body");
        });

        let folder = std::env::temp_dir().join(format!("tvpn-download-test-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("temp dir");
        let destination = folder.join("sample.bin");

        let result = fetch_to_file(
            &format!("http://{address}/sample.bin"),
            &destination,
            None,
            Arc::new(AtomicBool::new(false)),
            |_, _| {},
        )
        .await;

        server.join().expect("server");
        let (received, digest) = result.expect("download");
        assert_eq!(received, 10);
        assert_eq!(std::fs::read(&destination).expect("read"), b"hello tvpn");
        assert_eq!(
            digest,
            "0431704069adb90135d05ba5203025dd49dacc947e0909c9b2779717d48ea7db"
        );

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }
}
