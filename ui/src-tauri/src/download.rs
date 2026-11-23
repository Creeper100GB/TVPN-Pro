//! Real download engine for the desktop shell.
//!
//! This is shell-side on purpose until `tvpnd` exists (Phase 2). The routes are
//! honest: `direct` is unprotected, `tor` requires a local SOCKS proxy on
//! 127.0.0.1:9250, `vpn` is refused because the service owns that path.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
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
        Some("socks5h://127.0.0.1:9250")
    } else {
        None
    };

    let staging = staging_path(app, id)?;
    let folder = download_folder(app)?;
    // Reserve the final name before the transfer starts: two concurrent
    // downloads must not both settle on the same path.
    let (path, reservation) = reserve_destination(&folder, url)?;
    drop(reservation);
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
            let _ = tokio::fs::remove_file(&path).await;
            return Err(error);
        }
    };

    let mut verdict = scan(&staging).await;
    if verdict == "threat" {
        // A detected threat that cannot be moved is still a detected threat:
        // the file is deleted and the verdict says what went wrong instead of
        // reporting a plain failed download.
        match quarantine(app, id, &staging) {
            Ok(()) => verdict = "quarantined".to_owned(),
            Err(_) => {
                let _ = std::fs::remove_file(&staging);
                verdict = "quarantine_failed".to_owned();
            }
        }
        let _ = std::fs::remove_file(&path);
    } else {
        if move_into_place(&staging, &path).is_err() {
            let _ = std::fs::remove_file(&staging);
            let _ = std::fs::remove_file(&path);
            return Err("move".to_owned());
        }
        // Mark-of-the-Web for every file that lands in the normal download
        // folder, not only for "clean": when the scanner could not classify
        // the file, SmartScreen and Office should warn all the more.
        let _ = mark_of_the_web(&path);
    }

    let _ = app.emit(
        "download-done",
        Done {
            id: id.to_owned(),
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

/// One DNS answer for the download engine. Injected so the address checks can be
/// tested without any network traffic.
pub type Resolver = Arc<dyn Fn(&str, u16) -> Result<Vec<IpAddr>, String> + Send + Sync>;

/// Which addresses a hop may reach.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AddressPolicy {
    /// Only globally routable addresses. Every real download uses this.
    PublicOnly,
    /// Loopback stays reachable so the engine can be tested against a local
    /// server. Not reachable from the `download_start` command.
    #[cfg(test)]
    AllowLoopback,
}

fn system_resolver() -> Resolver {
    Arc::new(|host: &str, port: u16| {
        (host, port)
            .to_socket_addrs()
            .map(|addresses| addresses.map(|address| address.ip()).collect())
            .map_err(|_| "network".to_owned())
    })
}

/// Addresses a download must never reach: the host itself, the local network or
/// the link layer.
///
/// A public host name that resolves here points at services on this machine or
/// in the local network; the user asked for the internet, not for those.
fn is_blocked_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(v4) => is_blocked_v4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_blocked_v4(v4),
            None => {
                let segments = v6.segments();
                // `::/96` (IPv4-compatible, withdrawn) and NAT64 `64:ff9b::/96`
                // both carry an IPv4 address that can reach this machine: the
                // Windows CLAT maps NAT64 onto IPv4, loopback included.
                let embeds_v4 = segments[0..6].iter().all(|segment| *segment == 0)
                    || (segments[0] == 0x0064
                        && segments[1] == 0xff9b
                        && segments[2..6].iter().all(|segment| *segment == 0));
                let octets = v6.octets();
                let embedded = Ipv4Addr::new(octets[12], octets[13], octets[14], octets[15]);
                (embeds_v4 && is_blocked_v4(embedded))
                    || v6.is_loopback()
                    || v6.is_unspecified()
                    || v6.is_multicast()
                    || v6.is_unique_local()
                    || v6.is_unicast_link_local()
                    // fec0::/10 is the withdrawn site-local prefix.
                    || (segments[0] & 0xffc0) == 0xfec0
                    // 6to4 tunnels carry the IPv4 address of their endpoint.
                    || segments[0] == 0x2002
                    // Teredo 2001::/32 tunnels through UDPv4.
                    || (segments[0] == 0x2001 && segments[1] == 0x0000)
            }
        },
    }
}

fn is_blocked_v4(v4: Ipv4Addr) -> bool {
    let octets = v4.octets();
    // 100.64.0.0/10 is carrier-grade NAT, never a public host.
    let shared = octets[0] == 100 && (64..=127).contains(&octets[1]);
    // 198.18.0.0/15 is the benchmarking range Windows clients use for local
    // fake-IP tunnels (WSL, split tunnels), so it can point at this machine.
    let benchmark = octets[0] == 198 && (octets[1] == 18 || octets[1] == 19);
    v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_unspecified()
        || v4.is_multicast()
        || v4.is_broadcast()
        || v4.is_documentation()
        // 0.0.0.0/8 means "this network", down to 0.1.2.3.
        || octets[0] == 0
        // 240.0.0.0/4 is reserved apart from the limited broadcast address.
        || octets[0] >= 240
        // 192.0.0.0/24: IETF protocol assignments, never a content host.
        || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
        // 192.88.99.0/24: the withdrawn 6to4 relay anycast prefix.
        || (octets[0] == 192 && octets[1] == 88 && octets[2] == 99)
        || shared
        || benchmark
}

/// A request target that passed the address check and is pinned to one address.
struct Pinned {
    host: String,
    address: SocketAddr,
}

/// Resolves one hop once, rejects blocked answers and returns the address the
/// client has to connect to.
///
/// Pinning matters: without it the HTTP client would resolve the name a second
/// time at connect time, and a DNS server that changed its answer in between
/// (rebinding) would pick the endpoint.
async fn pin_hop(url: &str, resolver: &Resolver, policy: AddressPolicy) -> Result<Pinned, String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| "url".to_owned())?;
    let host = parsed
        .host_str()
        .ok_or_else(|| "url".to_owned())?
        .to_owned();
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| "url".to_owned())?;

    let literal = host.trim_start_matches('[').trim_end_matches(']');
    let addresses = match literal.parse::<IpAddr>() {
        Ok(address) => vec![address],
        Err(_) => {
            let resolver = Arc::clone(resolver);
            let name = host.clone();
            tokio::task::spawn_blocking(move || resolver(&name, port))
                .await
                .map_err(|_| "network".to_owned())??
        }
    };

    if addresses.is_empty() {
        return Err("network".to_owned());
    }
    let blocked = |address: &IpAddr| match policy {
        AddressPolicy::PublicOnly => is_blocked_address(*address),
        #[cfg(test)]
        AddressPolicy::AllowLoopback => is_blocked_address(*address) && !address.is_loopback(),
    };
    // One blocked answer is enough to refuse the hop: a name that resolves to
    // this machine and to a public host must not become reachable through the
    // public answer.
    if addresses.iter().any(blocked) {
        return Err("blocked_address".to_owned());
    }

    Ok(Pinned {
        // The client matches its DNS override against the host name it is handed.
        // An IPv6 literal keeps its brackets in the URL, so pinning `[::1]` would
        // never match the `::1` it connects to. `host` stays bracketed for the
        // resolver call only.
        host: literal.to_owned(),
        address: SocketAddr::new(addresses[0], port),
    })
}

/// Redirect hops the engine follows before it fails closed.
const MAX_REDIRECTS: usize = 8;

/// Absolute, validated target of a redirect response.
fn redirect_target(current: &str, location: &str) -> Result<String, String> {
    let next = reqwest::Url::parse(current)
        .and_then(|base| base.join(location))
        .map_err(|_| "url".to_owned())?;
    tvpn_core::url::validate(next.as_str()).map_err(|error| error.detail().to_owned())?;
    Ok(next.to_string())
}

fn client_builder(proxy: Option<&str>) -> Result<reqwest::ClientBuilder, String> {
    let builder = reqwest::Client::builder()
        .user_agent("TVPN-Pro/0.0.0")
        // Redirects are followed by hand: every hop has to pass the same
        // address check and gets its own pinned client.
        .redirect(reqwest::redirect::Policy::none());
    match proxy {
        Some(proxy) => {
            let proxy = reqwest::Proxy::all(proxy).map_err(|_| "proxy".to_owned())?;
            Ok(builder.proxy(proxy))
        }
        // The direct route has to connect to the address `pin_hop` checked.
        // reqwest uses the system proxy by default (HTTP_PROXY, HTTPS_PROXY,
        // ALL_PROXY and the Windows proxy settings), which would bypass the pin
        // and send the request, host name included, to that proxy instead.
        None => Ok(builder.no_proxy()),
    }
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
    fetch_to_file_with(
        url,
        dest,
        proxy,
        cancel,
        on_tick,
        &system_resolver(),
        AddressPolicy::PublicOnly,
    )
    .await
}

/// `fetch_to_file` with an injected resolver and address policy.
#[allow(clippy::too_many_arguments)]
pub async fn fetch_to_file_with<F>(
    url: &str,
    dest: &Path,
    proxy: Option<&str>,
    cancel: Arc<AtomicBool>,
    on_tick: F,
    resolver: &Resolver,
    policy: AddressPolicy,
) -> Result<(u64, String), String>
where
    F: Fn(u64, Option<u64>) + Send + Sync,
{
    // A queued download must not open a connection after a cancel; otherwise
    // it could still reach the network with the real IP once the firewall is
    // gone. Checked before `send()` and again before the body is read.
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_owned());
    }

    // A request without a proxy resolves the host here, so the checked address
    // is the address that gets connected. With the Tor route the proxy does the
    // lookup (socks5h): resolving it locally would leak the name to the system
    // resolver and the pin would point at an address the proxy never uses.
    let pin_locally = proxy.is_none();

    let mut current = url.to_owned();
    let mut hops = 0usize;
    let response = loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_owned());
        }

        let mut builder = client_builder(proxy)?;
        if pin_locally {
            let pinned = pin_hop(&current, resolver, policy).await?;
            builder = builder.resolve(&pinned.host, pinned.address);
        }
        let client = builder.build().map_err(|_| "client".to_owned())?;
        let response = client
            .get(&current)
            .send()
            .await
            .map_err(|_| "network".to_owned())?;

        if !response.status().is_redirection() {
            break response;
        }
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let Some(location) = location else {
            break response;
        };
        if hops >= MAX_REDIRECTS {
            return Err("redirect".to_owned());
        }
        hops += 1;

        // The same rules apply to a redirect target as to the first URL: the
        // text is validated and the address is checked before it is used.
        current = redirect_target(&current, &location)?;
    };

    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_owned());
    }

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
fn quarantine(app: &AppHandle, id: &str, path: &Path) -> Result<(), String> {
    let folder = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "quarantine_dir".to_owned())?
        .join("quarantine");
    std::fs::create_dir_all(&folder).map_err(|_| "quarantine_dir".to_owned())?;
    let target = folder.join(format!("{}.bin", safe_id(id)));

    match std::fs::rename(path, &target) {
        Ok(()) => {}
        Err(_) => {
            std::fs::copy(path, &target).map_err(|_| "quarantine_move".to_owned())?;
            let _ = std::fs::remove_file(path);
        }
    }

    // A quarantined copy is stored like every other stored file: without the
    // zone identifier, opening it from the quarantine folder would not warn.
    let _ = mark_of_the_web(&target);
    Ok(())
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
    // `to` is the placeholder `reserve_destination` created. `rename` replaces an
    // existing file on Windows, so the placeholder keeps holding the name until
    // the move succeeds; deleting it first would let a concurrent download reserve
    // the finished name and take it.
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    // The copy fallback writes over the placeholder, so it has to go - but only
    // after the rename failed, not before.
    let _ = std::fs::remove_file(to);
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

fn download_folder(app: &AppHandle) -> Result<PathBuf, String> {
    let folder = app
        .path()
        .download_dir()
        .map_err(|_| "no_download_dir".to_owned())?
        .join("TVPN");
    std::fs::create_dir_all(&folder).map_err(|_| "mkdir".to_owned())?;
    Ok(folder)
}

/// Reserves a free name in `folder` by creating it with `create_new`.
///
/// Checking `exists()` and writing later would let two concurrent downloads
/// pick the same target; the created file holds the name instead.
fn reserve_destination(folder: &Path, url: &str) -> Result<(PathBuf, std::fs::File), String> {
    let name = sanitized_destination_name(url);
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

    for counter in 0..10_000usize {
        let candidate = if counter == 0 {
            folder.join(&name)
        } else {
            folder.join(format!("{stem}-{counter}.{extension}"))
        };
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("create_file".to_owned()),
        }
    }
    Err("create_file".to_owned())
}

/// File name for a URL: last segment, restricted to characters that cannot
/// escape the folder, Windows-reserved names escaped, length bounded.
fn sanitized_destination_name(url: &str) -> String {
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
    name
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

/// Upper bound for one Defender scan. A hanging scanner must not block the
/// download task (and with it the cancel path) forever.
const SCAN_TIMEOUT: Duration = Duration::from_secs(120);

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

    match tauri::async_runtime::spawn_blocking(move || run_scanner(&program, &target)).await {
        Ok(verdict) => verdict,
        Err(_) => "unknown".to_owned(),
    }
}

/// Runs `MpCmdRun` with a deadline; a scanner that outlives it is killed and
/// the file counts as unclassified.
fn run_scanner(program: &Path, target: &Path) -> String {
    run_scanner_with_timeout(
        program,
        &[
            "-Scan".as_ref(),
            "-ScanType".as_ref(),
            "3".as_ref(),
            "-File".as_ref(),
            target.as_os_str(),
            "-DisableRemediation".as_ref(),
        ],
        SCAN_TIMEOUT,
    )
}

/// How long the pipe readers may take after the scanner exited.
const READER_GRACE: Duration = Duration::from_secs(5);

/// Joins a reader thread, or gives up once `deadline` has passed.
///
/// A thread still blocked on a pipe a grandchild holds open is detached and
/// counts as no output: an unclassified verdict is better than a scan that never
/// returns.
fn join_until(handle: std::thread::JoinHandle<Vec<u8>>, deadline: Instant) -> Option<Vec<u8>> {
    while !handle.is_finished() {
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    handle.join().ok()
}

/// Runs `program` with piped output and a deadline.
///
/// The pipes are drained by two reader threads while the process runs. Waiting
/// first and reading afterwards would deadlock on a chatty scanner: once its
/// pipe buffer is full it blocks on write, and the parent would wait for an exit
/// that never comes.
fn run_scanner_with_timeout(
    program: &Path,
    args: &[&std::ffi::OsStr],
    timeout: Duration,
) -> String {
    let mut child = match std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return "unknown".to_owned(),
    };

    fn drain<R: std::io::Read + Send + 'static>(
        pipe: Option<R>,
    ) -> std::thread::JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut buffer = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut buffer);
            }
            buffer
        })
    }

    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());

    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                // The readers finish on their own once the pipes are closed.
                return "unknown".to_owned();
            }
        }
    }

    let status = match child.wait() {
        Ok(status) => status,
        Err(_) => return "unknown".to_owned(),
    };
    // The scanner is gone, but a grandchild can still hold the write end of the
    // pipes; an unbounded `join` would hang past the deadline and the scan would
    // never return. The readers get a short grace period instead.
    let reader_deadline = Instant::now() + READER_GRACE;
    let Some(out) = join_until(stdout, reader_deadline) else {
        return "unknown".to_owned();
    };
    let Some(err) = join_until(stderr, reader_deadline) else {
        return "unknown".to_owned();
    };
    let mut text = String::from_utf8_lossy(&out).to_ascii_lowercase();
    text.push_str(&String::from_utf8_lossy(&err).to_ascii_lowercase());

    // MpCmdRun also returns 2 when the scan itself failed (0x80004005), for
    // example when Defender is disabled or replaced by another AV.
    if text.contains("failed with hr") {
        return "unknown".to_owned();
    }

    match status.code() {
        Some(0) => "clean".to_owned(),
        Some(2) => "threat".to_owned(),
        _ => "unknown".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        fetch_to_file_with, filename_from_url, pin_hop, redirect_target, reserve_destination,
        run_scanner_with_timeout, sanitized_destination_name, AddressPolicy, Resolver,
    };
    use std::io::{Read, Write};
    use std::net::{IpAddr, Ipv4Addr, TcpListener};
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex, MutexGuard};

    /// Serialises every test that builds an HTTP client.
    ///
    /// `one_test_sets_proxy_env` changes process-wide proxy variables, which the
    /// other network tests would pick up if they ran at the same time.
    static NETWORK_ENV: Mutex<()> = Mutex::new(());

    fn network_env_guard() -> MutexGuard<'static, ()> {
        NETWORK_ENV
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// A resolver that maps one name to one address. No network involved.
    fn fixed_resolver(name: &'static str, address: IpAddr) -> Resolver {
        Arc::new(move |host: &str, _port: u16| {
            if host == name {
                Ok(vec![address])
            } else {
                Err("network".to_owned())
            }
        })
    }

    fn unresolvable() -> Resolver {
        // The literal-address tests must not resolve anything at all.
        Arc::new(|_host: &str, _port: u16| Err("network".to_owned()))
    }

    #[tokio::test]
    async fn a_name_that_resolves_to_loopback_is_refused() {
        // `nip.io` style name: the URL rules like it, the address does not.
        let resolver = fixed_resolver("loopback.nip.io", IpAddr::V4(Ipv4Addr::LOCALHOST));

        let refused = pin_hop(
            "https://loopback.nip.io/file.bin",
            &resolver,
            AddressPolicy::PublicOnly,
        )
        .await;

        assert_eq!(refused.err(), Some("blocked_address".to_owned()));
    }

    #[tokio::test]
    async fn a_public_answer_is_accepted_and_pinned() {
        let address = IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34));
        let resolver = fixed_resolver("example.org", address);

        let pinned = pin_hop(
            "https://example.org/file.bin",
            &resolver,
            AddressPolicy::PublicOnly,
        )
        .await
        .expect("public host is allowed");

        // The checked address is the address the client has to connect to.
        assert_eq!(pinned.host, "example.org");
        assert_eq!(pinned.address, (address, 443).into());
    }

    #[tokio::test]
    async fn an_ipv6_literal_is_pinned_without_its_brackets() {
        // The URL keeps the brackets; the client's DNS override does not.
        let pinned = pin_hop(
            "https://[2606:4700:4700::1111]/file.bin",
            &unresolvable(),
            AddressPolicy::PublicOnly,
        )
        .await
        .expect("public literal is allowed");

        assert_eq!(pinned.host, "2606:4700:4700::1111");
        assert_eq!(
            pinned.address,
            (IpAddr::V6("2606:4700:4700::1111".parse().unwrap()), 443).into()
        );
    }

    #[tokio::test]
    async fn private_link_local_and_unspecified_answers_are_refused() {
        for address in [
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
            IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1)),
            // 0.0.0.0/8, 198.18.0.0/15, 240.0.0.0/4, 192.0.0.0/24, 192.88.99.0/24.
            IpAddr::V4(Ipv4Addr::new(0, 1, 2, 3)),
            IpAddr::V4(Ipv4Addr::new(198, 18, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(198, 19, 255, 254)),
            IpAddr::V4(Ipv4Addr::new(240, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 0, 0, 8)),
            IpAddr::V4(Ipv4Addr::new(192, 88, 99, 1)),
            IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
            "fd00::1".parse().expect("ula"),
            "fe80::1".parse().expect("link local"),
            "::ffff:127.0.0.1".parse().expect("mapped"),
            // IPv4-compatible `::/96` and NAT64 `64:ff9b::/96`.
            "::7f00:1".parse().expect("ipv4 compatible"),
            "64:ff9b::7f00:1".parse().expect("nat64 loopback"),
            "64:ff9b::a00:1".parse().expect("nat64 private"),
            // 6to4, Teredo and the withdrawn site-local prefix.
            "2002:7f00:1::1".parse().expect("6to4"),
            "2001:0:4136:e378::1".parse().expect("teredo"),
            "fec0::1".parse().expect("site local"),
        ] {
            let resolver = fixed_resolver("internal.example.org", address);
            let refused = pin_hop(
                "https://internal.example.org/file.bin",
                &resolver,
                AddressPolicy::PublicOnly,
            )
            .await;

            assert_eq!(
                refused.err(),
                Some("blocked_address".to_owned()),
                "{address} must be refused"
            );
        }
    }

    #[tokio::test]
    async fn a_redirect_to_a_loopback_name_is_refused() {
        let resolver = fixed_resolver("rebind.nip.io", IpAddr::V4(Ipv4Addr::LOCALHOST));
        let target = redirect_target(
            "https://example.org/file.bin",
            "https://rebind.nip.io/x.bin",
        )
        .expect("an https target passes the URL rules");

        let refused = pin_hop(&target, &resolver, AddressPolicy::PublicOnly).await;

        assert_eq!(refused.err(), Some("blocked_address".to_owned()));
    }

    #[tokio::test]
    async fn a_redirect_target_must_keep_the_url_rules() {
        assert_eq!(
            redirect_target("https://example.org/file.bin", "http://example.org/x.bin"),
            Err("not_https".to_owned())
        );
        assert_eq!(
            redirect_target("https://example.org/file.bin", "https://127.0.0.1/x.bin"),
            Err("ip_literal".to_owned())
        );
        assert_eq!(
            redirect_target("https://example.org/file.bin", "/relative/x.bin"),
            Ok("https://example.org/relative/x.bin".to_owned())
        );
    }

    /// A server that answers every request with a redirect to `location`.
    fn redirecting_server(location: &'static str) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("addr");
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buffer = [0u8; 1024];
            let _ = stream.read(&mut buffer);
            write!(
                stream,
                "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .expect("redirect");
        });
        (format!("http://{address}/start.bin"), handle)
    }

    #[tokio::test]
    // The guard only serialises test threads, and `#[tokio::test]` drives a
    // current-thread runtime: nothing else on this thread can wait for it.
    #[allow(clippy::await_holding_lock)]
    async fn a_redirect_hop_is_checked_before_it_is_followed() {
        let _env = network_env_guard();
        // The first hop is the local test server (allowed for tests); the
        // redirect target resolves into the private network and is refused.
        let (url, server) = redirecting_server("https://rebind.nip.io/x.bin");
        let resolver = fixed_resolver("rebind.nip.io", IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7)));
        let folder = std::env::temp_dir().join(format!(
            "tvpn-redirect-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&folder).expect("temp dir");
        let destination = folder.join("redirected.bin");

        let result = fetch_to_file_with(
            &url,
            &destination,
            None,
            Arc::new(AtomicBool::new(false)),
            |_, _| {},
            &resolver,
            AddressPolicy::AllowLoopback,
        )
        .await;

        server.join().expect("server");
        assert_eq!(result, Err("blocked_address".to_owned()));
        assert!(!destination.exists() || std::fs::metadata(&destination).unwrap().len() == 0);

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }

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

    #[test]
    fn reserved_windows_names_are_escaped() {
        assert_eq!(
            sanitized_destination_name("https://example.org/CON"),
            "download-CON"
        );
        assert_eq!(
            sanitized_destination_name("https://example.org/com1.txt"),
            "download-com1.txt"
        );
        assert_eq!(
            sanitized_destination_name("https://example.org/console.txt"),
            "console.txt"
        );
    }

    #[test]
    fn two_reservations_get_different_paths() {
        let folder = std::env::temp_dir().join(format!(
            "tvpn-reserve-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&folder).expect("temp dir");

        let (first, first_file) =
            reserve_destination(&folder, "https://example.org/sample.bin").expect("first");
        drop(first_file);
        let (second, second_file) =
            reserve_destination(&folder, "https://example.org/sample.bin").expect("second");
        drop(second_file);

        assert_ne!(first, second);
        assert_eq!(first.file_name().unwrap(), "sample.bin");
        assert_eq!(second.file_name().unwrap(), "sample-1.bin");
        assert!(first.exists() && second.exists());

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[tokio::test]
    // The guard only serialises test threads, and `#[tokio::test]` drives a
    // current-thread runtime: nothing else on this thread can wait for it.
    #[allow(clippy::await_holding_lock)]
    async fn cancelled_transfer_does_not_connect() {
        let _env = network_env_guard();
        // Nothing listens here: the cancel flag has to stop the request before
        // it is even attempted.
        let folder = std::env::temp_dir().join(format!(
            "tvpn-cancel-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&folder).expect("temp dir");
        let destination = folder.join("cancelled.bin");

        let result = fetch_to_file_with(
            "http://127.0.0.1:9/never.bin",
            &destination,
            None,
            Arc::new(AtomicBool::new(true)),
            |_, _| {},
            &unresolvable(),
            AddressPolicy::AllowLoopback,
        )
        .await;

        assert_eq!(result, Err("cancelled".to_owned()));
        assert!(!destination.exists());

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[tokio::test]
    // The guard only serialises test threads, and `#[tokio::test]` drives a
    // current-thread runtime: nothing else on this thread can wait for it.
    #[allow(clippy::await_holding_lock)]
    async fn fetches_and_hashes() {
        let _env = network_env_guard();
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

        let folder =
            std::env::temp_dir().join(format!("tvpn-download-test-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("temp dir");
        let destination = folder.join("sample.bin");

        let result = fetch_to_file_with(
            &format!("http://{address}/sample.bin"),
            &destination,
            None,
            Arc::new(AtomicBool::new(false)),
            |_, _| {},
            &unresolvable(),
            AddressPolicy::AllowLoopback,
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

    /// How long a test may wait for its local server.
    const SERVER_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

    /// A local server that answers one request with `body`.
    ///
    /// The accept has a deadline of its own: a request that never arrives (the
    /// regression these tests cover) has to fail the test instead of blocking
    /// `join` forever.
    fn one_shot_server(body: &'static [u8]) -> (u16, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        listener.set_nonblocking(true).expect("nonblocking");
        let handle = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + SERVER_WAIT;
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "no request arrived at the local server"
                        );
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    Err(error) => panic!("accept: {error}"),
                }
            };
            let _ = stream.set_nonblocking(false);
            let mut buffer = [0u8; 1024];
            let _ = stream.read(&mut buffer);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("header");
            stream.write_all(body).expect("body");
        });
        (port, handle)
    }

    fn scratch_dir(prefix: &str) -> std::path::PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "tvpn-{prefix}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&folder).expect("temp dir");
        folder
    }

    /// The DNS override has to be wired: without it the client resolves the name
    /// itself, and `pinned.test` does not exist in the system resolver.
    #[tokio::test]
    // The guard only serialises test threads, and `#[tokio::test]` drives a
    // current-thread runtime: nothing else on this thread can wait for it.
    #[allow(clippy::await_holding_lock)]
    async fn a_pinned_name_connects_to_the_checked_address() {
        let _env = network_env_guard();
        let (port, server) = one_shot_server(b"pinned body");
        let resolver = fixed_resolver("pinned.test", IpAddr::V4(Ipv4Addr::LOCALHOST));
        let folder = scratch_dir("pinned-test");
        let destination = folder.join("pinned.bin");

        let result = tokio::time::timeout(
            SERVER_WAIT,
            fetch_to_file_with(
                &format!("http://pinned.test:{port}/ok"),
                &destination,
                None,
                Arc::new(AtomicBool::new(false)),
                |_, _| {},
                &resolver,
                AddressPolicy::AllowLoopback,
            ),
        )
        .await
        .expect("the pinned fetch must not hang");

        server.join().expect("server");
        assert_eq!(result.expect("download through the pin").0, 11);
        assert_eq!(std::fs::read(&destination).expect("read"), b"pinned body");

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }

    /// A system proxy must not swallow the direct route: it would connect to the
    /// proxy instead of the address the pin checked, so the SSRF guard and the
    /// name pinning would both be moot.
    #[tokio::test]
    // The guard only serialises test threads, and `#[tokio::test]` drives a
    // current-thread runtime: nothing else on this thread can wait for it.
    #[allow(clippy::await_holding_lock)]
    async fn the_direct_route_ignores_a_system_proxy() {
        let _env = network_env_guard();
        let (port, server) = one_shot_server(b"direct body");
        let folder = scratch_dir("proxy-test");
        let destination = folder.join("direct.bin");

        // Nothing listens on port 1, so a proxied request fails instead of
        // leaving the machine.
        let proxy = "http://127.0.0.1:1";
        let saved = ["HTTPS_PROXY", "ALL_PROXY"].map(|key| (key, std::env::var(key).ok()));
        // Only this test writes the variables and every client-building test
        // holds the same lock.
        std::env::set_var("HTTPS_PROXY", proxy);
        std::env::set_var("ALL_PROXY", proxy);

        let result = tokio::time::timeout(
            SERVER_WAIT,
            fetch_to_file_with(
                &format!("http://127.0.0.1:{port}/ok"),
                &destination,
                None,
                Arc::new(AtomicBool::new(false)),
                |_, _| {},
                &unresolvable(),
                AddressPolicy::AllowLoopback,
            ),
        )
        .await
        .expect("the direct route waited for a system proxy instead of connecting");

        for (key, value) in saved {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }

        server.join().expect("server");
        assert_eq!(result.expect("download despite the proxy env").0, 11);
        assert_eq!(std::fs::read(&destination).expect("read"), b"direct body");

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }

    /// A chatty scanner must not deadlock: the pipes are drained while it runs.
    #[cfg(windows)]
    #[test]
    fn a_chatty_scanner_is_drained_while_it_runs() {
        // 20k lines are far more than a pipe buffer holds. Reading only after
        // the exit would block here until the deadline.
        let result = run_scanner_with_timeout(
            std::path::Path::new("cmd.exe"),
            &[
                "/C".as_ref(),
                "for /L %i in (1,1,20000) do @echo aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".as_ref(),
            ],
            std::time::Duration::from_secs(60),
        );

        // Exit code 0 maps to "clean"; the deadline would map to "unknown".
        assert_eq!(result, "clean");
    }
}
