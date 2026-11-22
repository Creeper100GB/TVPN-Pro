//! Hostname plan from spike S3.
//!
//! Windows applies `Rename-Computer` only after a reboot. The setup uses this
//! module once to generate a neutral name; a running session must not claim
//! that it changed the visible hostname.

use std::fmt;

use getrandom::getrandom;

pub const MIN_HOSTNAME_LEN: usize = 3;
pub const MAX_HOSTNAME_LEN: usize = 63;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostnameError {
    Invalid,
}

impl fmt::Display for HostnameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid hostname")
    }
}

impl std::error::Error for HostnameError {}

/// `tvpn-` plus eight lowercase hex characters.
pub fn neutral_name() -> String {
    let mut bytes = [0u8; 4];
    getrandom(&mut bytes).expect("CSPRNG must be available");
    format!(
        "tvpn-{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3]
    )
}

/// Validates the subset of RFC 1123 hostnames Windows accepts here.
pub fn validate(value: &str) -> Result<(), HostnameError> {
    if !(MIN_HOSTNAME_LEN..=MAX_HOSTNAME_LEN).contains(&value.len()) {
        return Err(HostnameError::Invalid);
    }
    if value.starts_with('-') || value.ends_with('-') {
        return Err(HostnameError::Invalid);
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(HostnameError::Invalid);
    }
    Ok(())
}

/// What the setup must do. S3 measured that a live rename needs a reboot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostnamePlan {
    pub name: String,
    pub requires_reboot: bool,
}

impl HostnamePlan {
    pub fn new() -> Self {
        Self {
            name: neutral_name(),
            requires_reboot: true,
        }
    }
}

impl Default for HostnamePlan {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_name_is_valid() {
        for _ in 0..32 {
            let plan = HostnamePlan::new();
            assert!(validate(&plan.name).is_ok(), "{}", plan.name);
            assert!(plan.name.starts_with("tvpn-"));
            assert!(plan.requires_reboot);
        }
    }

    #[test]
    fn rejects_bad_names() {
        for value in ["", "ab", "-abc", "abc-", "ABC", "ab_c", "a.b"] {
            assert!(validate(value).is_err(), "{value:?}");
        }
    }

    #[test]
    fn accepts_length_bounds() {
        assert!(validate("abc").is_ok());
        assert!(validate(&"a".repeat(63)).is_ok());
        assert!(validate(&"a".repeat(64)).is_err());
    }
}
