//! Tor Browser and fallback launcher primitives.
//!
//! The service never receives a path from the UI. The installer pins the
//! executable and its SHA-256; these helpers verify that pin and the minimum
//! version before a launch. The actual launch is a separate VM step (S2).

use std::fmt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub const MIN_TOR_BROWSER_VERSION: &str = "13.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchError {
    Missing,
    HashMismatch,
    VersionTooOld,
}

impl fmt::Display for LaunchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => formatter.write_str("executable missing"),
            Self::HashMismatch => formatter.write_str("hash mismatch"),
            Self::VersionTooOld => formatter.write_str("version too old"),
        }
    }
}

impl std::error::Error for LaunchError {}

/// Compares a lowercase or uppercase SHA-256 hex string.
pub fn verify_sha256(path: &Path, expected: &str) -> Result<(), LaunchError> {
    if !path.is_file() {
        return Err(LaunchError::Missing);
    }
    let mut file = std::fs::File::open(path).map_err(|_| LaunchError::Missing)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = std::io::Read::read(&mut file, &mut buffer).map_err(|_| LaunchError::Missing)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(LaunchError::HashMismatch)
    }
}

/// Parses `13.5.9` or `13` and compares the numeric components.
pub fn version_at_least(version: &str, minimum: &str) -> bool {
    let parse = |value: &str| -> Vec<u32> {
        value
            .split('.')
            .map(|part| part.parse::<u32>().unwrap_or(0))
            .collect()
    };
    let actual = parse(version);
    let required = parse(minimum);
    for index in 0..required.len().max(actual.len()) {
        let left = actual.get(index).copied().unwrap_or(0);
        let right = required.get(index).copied().unwrap_or(0);
        if left != right {
            return left > right;
        }
    }
    true
}

/// Fixed launch plan. Ports stay on loopback and match PLAN 4.5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorBrowserPlan {
    pub path: PathBuf,
    pub socks_port: u16,
    pub control_port: u16,
    pub data_dir: PathBuf,
}

impl TorBrowserPlan {
    pub fn new(path: impl Into<PathBuf>, data_dir: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            socks_port: 9150,
            control_port: 9051,
            data_dir: data_dir.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_checked() {
        let path = std::env::temp_dir().join(format!("tvpn-launch-{}.bin", std::process::id()));
        std::fs::write(&path, b"tvpn").unwrap();
        let expected = "a5d5e8b0a5f2b6d4c6f7c2e6e5d4c3b2a1f0e9d8c7b6a5f4e3d2c1b0a9f8e7d6";
        assert_eq!(
            verify_sha256(&path, expected),
            Err(LaunchError::HashMismatch)
        );
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            verify_sha256(Path::new("C:\\does-not-exist"), expected),
            Err(LaunchError::Missing)
        );
    }

    #[test]
    fn versions_are_compared_numerically() {
        assert!(version_at_least("13.5.9", "13.0"));
        assert!(version_at_least("14.0", "13.5.9"));
        assert!(!version_at_least("12.9.9", "13.0"));
        assert!(version_at_least("13", "13.0"));
    }

    #[test]
    fn ports_are_loopback_contract_values() {
        let plan = TorBrowserPlan::new("tor-browser.exe", "data");
        assert_eq!(plan.socks_port, 9150);
        assert_eq!(plan.control_port, 9051);
    }
}
