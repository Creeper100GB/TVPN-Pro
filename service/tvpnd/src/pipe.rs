//! Windows named-pipe transport for `tvpnd`.
//!
//! Security boundary:
//! - the pipe and the token file get a DACL for SYSTEM and the installing user,
//! - the first request on every connection must be `hello` with the token,
//! - a message is one NDJSON line, at most 64 KiB,
//! - one connection is served at a time in this slice. The contract's four
//!   parallel connections need a shared state and are the next step.

use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, LocalFree, ERROR_BROKEN_PIPE, ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE,
    HANDLE, HLOCAL, INVALID_HANDLE_VALUE,
};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
    ConvertStringSidToSidW,
};
use windows::Win32::Security::{
    GetTokenInformation, SetFileSecurityW, TokenUser, DACL_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FlushFileBuffers, MoveFileExW, ReadFile, WriteFile, CREATE_NEW,
    FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, FILE_SHARE_READ, MOVEFILE_REPLACE_EXISTING,
    MOVEFILE_WRITE_THROUGH, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_WAIT,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use crate::Service;
use tvpn_core::ipc::{parse_request, Response, MAX_MESSAGE_BYTES};

const BUFFER_SIZE: usize = 64 * 1024;

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Generates the 32-byte session token, base64url encoded as the contract asks.
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("CSPRNG must be available");
    URL_SAFE_NO_PAD.encode(bytes)
}

fn token_path() -> PathBuf {
    if let Ok(path) = std::env::var("TVPN_TOKEN_PATH") {
        return PathBuf::from(path);
    }
    let base = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_owned());
    PathBuf::from(base).join("TVPN").join("session.token")
}

fn pipe_name() -> String {
    std::env::var("TVPN_PIPE_NAME").unwrap_or_else(|_| r"\\.\pipe\tvpn-pro-v0".to_owned())
}

fn owner_path() -> PathBuf {
    token_path().with_file_name("owner.sid")
}

/// The SID of the installing user. The service runs as SYSTEM, so it cannot
/// derive the interactive user from its own token.
fn owner_sid() -> Option<String> {
    if let Ok(value) = std::env::var("TVPN_OWNER_SID") {
        let value = value.trim().to_owned();
        if valid_sid(&value) {
            return Some(value);
        }
    }
    let value = std::fs::read_to_string(owner_path()).ok()?;
    let value = value.trim().to_owned();
    if valid_sid(&value) {
        Some(value)
    } else {
        None
    }
}

/// Persists the installing user's SID before the service is created.
pub fn save_owner_sid(sid: &str) -> io::Result<()> {
    if !valid_sid(sid) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid SID"));
    }
    let path = owner_path();
    secure_write_inner(&path, sid.as_bytes(), Some(sid), false)
}

/// Writes the token with a DACL for SYSTEM, Administrators and the optional
/// install owner. The file is created with its final DACL in one step.
pub fn write_token(token: &str) -> io::Result<PathBuf> {
    let path = token_path();
    let owner = owner_sid();
    secure_write_inner(&path, token.as_bytes(), owner.as_deref(), true)?;
    Ok(path)
}

pub(crate) fn current_user_sid() -> io::Result<String> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)
            .map_err(|_| io::Error::last_os_error())?;

        let mut length = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut length);
        if length == 0 {
            let _ = CloseHandle(token);
            return Err(io::Error::last_os_error());
        }

        let words =
            (length as usize + std::mem::size_of::<usize>() - 1) / std::mem::size_of::<usize>();
        let mut buffer = vec![0usize; words];
        let result = GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr() as *mut core::ffi::c_void),
            length,
            &mut length,
        );
        let _ = CloseHandle(token);
        result.map_err(|_| io::Error::last_os_error())?;

        let user = &*(buffer.as_ptr() as *const TOKEN_USER);
        let mut string = PWSTR::null();
        ConvertSidToStringSidW(user.User.Sid, &mut string)
            .map_err(|_| io::Error::last_os_error())?;
        let sid = string.to_string().map_err(|_| io::Error::last_os_error())?;
        let _ = LocalFree(Some(HLOCAL(string.0 as *mut core::ffi::c_void)));
        Ok(sid)
    }
}

fn security_descriptor(sddl: &str) -> io::Result<PSECURITY_DESCRIPTOR> {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let wide_sddl = wide(sddl);
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(wide_sddl.as_ptr()),
            1,
            &mut descriptor,
            None,
        )
        .map_err(|_| io::Error::last_os_error())?;
    }
    Ok(descriptor)
}

fn set_dacl(path: &Path, sddl: &str) -> io::Result<()> {
    let descriptor = security_descriptor(sddl)?;
    let wide_path = wide(&path.to_string_lossy());
    let result = unsafe {
        SetFileSecurityW(
            PCWSTR(wide_path.as_ptr()),
            DACL_SECURITY_INFORMATION,
            descriptor,
        )
    };
    let _ = unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
    if result.as_bool() {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// True for the explicit test paths. Production uses `%ProgramData%\TVPN`.
fn is_test_path(path: &Path) -> bool {
    #[cfg(debug_assertions)]
    {
        if std::env::var_os("TVPN_TOKEN_PATH").is_some()
            || std::env::var_os("TVPN_SETTINGS_PATH").is_some()
        {
            return true;
        }
    }
    path.starts_with(std::env::temp_dir())
}

fn directory_sddl(owner: Option<&str>) -> String {
    let mut sddl = "D:P(A;OICI;GA;;;SY)(A;OICI;GA;;;BA)".to_owned();
    if let Some(owner) = owner {
        sddl.push_str(&format!("(A;OICI;GR;;;{owner})"));
    }
    sddl
}

fn file_sddl(owner: Option<&str>, owner_read: bool) -> String {
    let mut sddl = "D:P(A;;GA;;;SY)(A;;GA;;;BA)".to_owned();
    if owner_read {
        if let Some(owner) = owner {
            sddl.push_str(&format!("(A;;GR;;;{owner})"));
        }
    }
    sddl
}

/// A syntactically valid SID. Rejecting garbage here prevents SDDL injection
/// through the owner file.
fn valid_sid(value: &str) -> bool {
    if value.is_empty() || value.len() > 184 {
        return false;
    }
    let wide_value = wide(value);
    let mut sid = windows::Win32::Security::PSID::default();
    let ok = unsafe { ConvertStringSidToSidW(PCWSTR(wide_value.as_ptr()), &mut sid) };
    if ok.is_err() {
        return false;
    }
    unsafe {
        let _ = LocalFree(Some(HLOCAL(sid.0)));
    }
    true
}

fn secure_dir_with_owner(owner: Option<&str>) -> io::Result<()> {
    let dir = token_path()
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no parent"))?;
    if is_test_path(&dir) {
        std::fs::create_dir_all(&dir)?;
        return Ok(());
    }
    std::fs::create_dir_all(&dir)?;
    set_dacl(&dir, &directory_sddl(owner))
}

fn secure_write_inner(
    path: &Path,
    bytes: &[u8],
    owner: Option<&str>,
    owner_read: bool,
) -> io::Result<()> {
    if is_test_path(path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        return std::fs::write(path, bytes);
    }

    let owner_string = owner.map(str::to_owned).or_else(owner_sid);
    let owner = owner_string.as_deref();
    secure_dir_with_owner(owner)?;

    let mut random = [0u8; 6];
    getrandom::getrandom(&mut random).map_err(|_| io::Error::other("random"))?;
    let suffix = random
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("tvpn");
    let temp = path.with_file_name(format!(".{file_name}.{suffix}.tmp"));

    let descriptor = security_descriptor(&file_sddl(owner, owner_read))?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };
    let wide_temp = wide(&temp.to_string_lossy());
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide_temp.as_ptr()),
            GENERIC_WRITE.0,
            FILE_SHARE_READ,
            Some(&attributes),
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    };
    let _ = unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
    let handle = match handle {
        Ok(handle) => handle,
        Err(error) => {
            let _ = std::fs::remove_file(&temp);
            return Err(error.into());
        }
    };

    let write_result = (|| -> io::Result<()> {
        let mut written = 0usize;
        while written < bytes.len() {
            let mut chunk = 0u32;
            unsafe {
                WriteFile(handle, Some(&bytes[written..]), Some(&mut chunk), None)
                    .map_err(|_| io::Error::last_os_error())?;
            }
            if chunk == 0 {
                return Err(io::Error::new(io::ErrorKind::WriteZero, "short write"));
            }
            written += chunk as usize;
        }
        unsafe {
            FlushFileBuffers(handle).map_err(|_| io::Error::last_os_error())?;
        }
        Ok(())
    })();

    if let Err(error) = write_result {
        unsafe {
            let _ = CloseHandle(handle);
        }
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    unsafe {
        let _ = CloseHandle(handle);
    }

    let wide_path = wide(&path.to_string_lossy());
    let moved = unsafe {
        MoveFileExW(
            PCWSTR(wide_temp.as_ptr()),
            PCWSTR(wide_path.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved.is_err() {
        let error = io::Error::last_os_error();
        let _ = std::fs::remove_file(&temp);
        return Err(error);
    }
    Ok(())
}

/// Writes a small security-critical file with a restrictive DACL and an
/// atomic replace. Test paths keep plain writes so unit tests can read them.
pub(crate) fn secure_write(path: &Path, bytes: &[u8], owner_read: bool) -> io::Result<()> {
    secure_write_inner(path, bytes, None, owner_read)
}

fn create_pipe(name: &str) -> io::Result<HANDLE> {
    let sid = current_user_sid()?;
    let owner = owner_sid();
    let mut sddl = format!("D:P(A;;GA;;;SY)(A;;GA;;;{sid})");
    if let Some(owner) = owner {
        if owner != sid {
            sddl.push_str(&format!("(A;;GA;;;{owner})"));
        }
    }
    let descriptor = security_descriptor(&sddl)?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };
    let wide_name = wide(name);
    let handle = unsafe {
        CreateNamedPipeW(
            PCWSTR(wide_name.as_ptr()),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            1,
            BUFFER_SIZE as u32,
            BUFFER_SIZE as u32,
            0,
            Some(&attributes),
        )
    };
    let _ = unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };

    if handle == INVALID_HANDLE_VALUE {
        Err(io::Error::last_os_error())
    } else {
        Ok(handle)
    }
}

/// Connects once to unblock `ConnectNamedPipe` when the SCM asks the service
/// to stop while no UI is connected.
pub fn wake() {
    let name = wide(&pipe_name());
    let result = unsafe {
        CreateFileW(
            PCWSTR(name.as_ptr()),
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_NONE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    };
    if let Ok(handle) = result {
        unsafe {
            let _ = CloseHandle(handle);
        }
    }
}

/// Runs the pipe server until the process is stopped.
pub fn run(token: &str) -> io::Result<()> {
    run_with_stop(token, Arc::new(AtomicBool::new(false)))
}

/// Runs the pipe server until `stop` is set between two connections.
pub fn run_with_stop(token: &str, stop: Arc<AtomicBool>) -> io::Result<()> {
    let path = write_token(token)?;
    println!("tvpnd: pipe {} token {}", pipe_name(), path.display());

    let service = Arc::new(Mutex::new(Service::new(token)));

    let watchdog = Arc::clone(&service);
    let watchdog_stop = Arc::clone(&stop);
    std::thread::spawn(move || {
        while !watchdog_stop.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_secs(5));
            if let Ok(mut guard) = watchdog.lock() {
                guard.watchdog_tick();
            }
        }
    });

    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let handle = create_pipe(&pipe_name())?;
        let connected = unsafe { ConnectNamedPipe(handle, None) };
        if let Err(error) = connected {
            if error.code() != ERROR_PIPE_CONNECTED.to_hresult() {
                unsafe {
                    let _ = CloseHandle(handle);
                }
                continue;
            }
        }

        if stop.load(Ordering::SeqCst) {
            unsafe {
                let _ = CloseHandle(handle);
            }
            break;
        }

        if let Ok(mut guard) = service.lock() {
            guard.reset_auth();
        }
        let result = serve_connection(handle, &service);
        unsafe {
            let _ = DisconnectNamedPipe(handle);
            let _ = CloseHandle(handle);
        }
        if let Err(error) = result {
            if error.raw_os_error() != Some(ERROR_BROKEN_PIPE.0 as i32) {
                eprintln!("tvpnd: connection ended: {error}");
            }
        }
    }
    Ok(())
}

fn serve_connection(handle: HANDLE, service: &Arc<Mutex<Service>>) -> io::Result<()> {
    let mut pending = Vec::new();
    let mut buffer = [0u8; 8 * 1024];

    loop {
        let mut read = 0u32;
        unsafe {
            ReadFile(handle, Some(&mut buffer), Some(&mut read), None)
                .map_err(|_| io::Error::last_os_error())?;
        }
        if read == 0 {
            return Ok(());
        }
        pending.extend_from_slice(&buffer[..read as usize]);

        while let Some(position) = pending.iter().position(|byte| *byte == b'\n') {
            let mut line: Vec<u8> = pending.drain(..=position).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if line.is_empty() {
                continue;
            }
            if line.len() > MAX_MESSAGE_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "message too large",
                ));
            }
            let text = match std::str::from_utf8(&line) {
                Ok(text) => text,
                Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, "not UTF-8")),
            };
            let response = match parse_request(text) {
                Ok(request) => {
                    let mut guard = service
                        .lock()
                        .map_err(|_| io::Error::other("service state poisoned"))?;
                    guard.handle(&request)
                }
                Err(error) => Response::error("?", error.code(), Some(error.detail())),
            };
            write_response(handle, &response)?;
        }

        if pending.len() > MAX_MESSAGE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "message too large",
            ));
        }
    }
}

fn write_response(handle: HANDLE, response: &Response) -> io::Result<()> {
    let mut line = serde_json::to_vec(response)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "serialize"))?;
    line.push(b'\n');
    let mut written = 0u32;
    unsafe {
        WriteFile(handle, Some(&line), Some(&mut written), None)
            .map_err(|_| io::Error::last_os_error())?;
        FlushFileBuffers(handle).map_err(|_| io::Error::last_os_error())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_url_safe_and_32_bytes() {
        let token = generate_token();
        assert_eq!(URL_SAFE_NO_PAD.decode(token.as_bytes()).unwrap().len(), 32);
        assert!(!token.contains('+'));
        assert!(!token.contains('/'));
        assert!(!token.contains('='));
    }

    #[test]
    fn sd_accepts_the_system_and_user_aces() {
        let descriptor = security_descriptor("D:P(A;;GA;;;SY)(A;;GA;;;S-1-5-18)").unwrap();
        assert!(!descriptor.0.is_null());
        unsafe {
            let _ = LocalFree(Some(HLOCAL(descriptor.0)));
        }
    }

    #[test]
    fn owner_sid_rejects_sddl_injection() {
        assert!(valid_sid("S-1-5-18"));
        assert!(valid_sid("S-1-5-21-1-2-3-1001"));
        assert!(!valid_sid("S-1-5-21-1-2-3-1001)(A;;GA;;;WD"));
        assert!(!valid_sid(""));
        assert!(!valid_sid("not-a-sid"));
    }
}
