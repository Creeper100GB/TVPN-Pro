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

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, LocalFree, ERROR_BROKEN_PIPE, ERROR_PIPE_CONNECTED, HANDLE, HLOCAL,
    INVALID_HANDLE_VALUE,
};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
};
use windows::Win32::Security::{
    GetTokenInformation, SetFileSecurityW, TokenUser, DACL_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
};
use windows::Win32::Storage::FileSystem::{
    FlushFileBuffers, ReadFile, WriteFile, PIPE_ACCESS_DUPLEX,
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

/// Writes the token with a DACL for SYSTEM, the current account and the
/// optional install owner from `TVPN_OWNER_SID`.
pub fn write_token(token: &str) -> io::Result<PathBuf> {
    let path = token_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, token)?;

    let sid = current_user_sid()?;
    let owner = std::env::var("TVPN_OWNER_SID")
        .ok()
        .filter(|value| !value.is_empty());
    let mut sddl = format!("D:P(A;;GA;;;SY)(A;;GA;;;{sid})");
    if let Some(owner) = owner {
        if owner != sid {
            sddl.push_str(&format!("(A;;GR;;;{owner})"));
        }
    }

    set_dacl(&path, &sddl)?;
    Ok(path)
}

fn current_user_sid() -> io::Result<String> {
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

fn create_pipe(name: &str) -> io::Result<HANDLE> {
    let sid = current_user_sid()?;
    let owner = std::env::var("TVPN_OWNER_SID")
        .ok()
        .filter(|value| !value.is_empty());
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

/// Runs the pipe server until the process is stopped.
pub fn run(token: &str) -> io::Result<()> {
    let path = write_token(token)?;
    println!("tvpnd: pipe {} token {}", pipe_name(), path.display());

    let mut service = Service::new(token);
    loop {
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

        service.reset_auth();
        let result = serve_connection(handle, &mut service);
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
}

fn serve_connection(handle: HANDLE, service: &mut Service) -> io::Result<()> {
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
                Ok(request) => service.handle(&request),
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
}
