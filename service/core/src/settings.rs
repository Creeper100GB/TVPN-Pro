//! Settings whitelist from `contracts/ipc-v0.md` section 6.2.
//!
//! Unknown keys are rejected by serde, invalid values by [`Settings::validate`].
//! A patch is applied all-or-nothing: either the merged settings validate or
//! nothing changes.

use serde::{Deserialize, Serialize};
use std::fmt;

/// What the block heuristic may do with a blocked page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockFallback {
    Off,
    Suggest,
    Auto,
}

/// How the MAC is chosen for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacMode {
    Random,
    Original,
}

/// Network mode. `System` exists in the enum so the value is rejected with a
/// clear error instead of a schema error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetMode {
    Focus,
    System,
}

/// How torrents may be routed. `VpnOnly` never touches Tor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TorrentMode {
    Blocked,
    VpnOnly,
}

/// Complete settings, as returned by `get_settings`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub vpn_enabled: bool,
    pub vpn_country: String,
    pub tor_bridges: bool,
    pub block_fallback: BlockFallback,
    pub mac_mode: MacMode,
    pub torrent_mode: TorrentMode,
    pub net_mode: NetMode,
    pub clipboard_watch: bool,
    pub scanner_hash_lookup: bool,
    pub portal_timeout_sec: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            vpn_enabled: true,
            vpn_country: "random".to_owned(),
            tor_bridges: false,
            block_fallback: BlockFallback::Auto,
            mac_mode: MacMode::Random,
            torrent_mode: TorrentMode::Blocked,
            net_mode: NetMode::Focus,
            clipboard_watch: false,
            scanner_hash_lookup: false,
            portal_timeout_sec: 180,
        }
    }
}

/// Partial update from `set_settings`. Every field is optional.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsPatch {
    pub vpn_enabled: Option<bool>,
    pub vpn_country: Option<String>,
    pub tor_bridges: Option<bool>,
    pub block_fallback: Option<BlockFallback>,
    pub mac_mode: Option<MacMode>,
    pub torrent_mode: Option<TorrentMode>,
    pub net_mode: Option<NetMode>,
    pub clipboard_watch: Option<bool>,
    pub scanner_hash_lookup: Option<bool>,
    pub portal_timeout_sec: Option<u16>,
}

/// Why a settings value or patch was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsError {
    InvalidCountry(String),
    PortalTimeout(u16),
    NetModeUnsupported,
}

impl SettingsError {
    /// Wire code from `contracts/ipc-v0.md` section 9.
    pub fn code(&self) -> &'static str {
        "E_IPC_BAD_SCHEMA"
    }

    /// Short machine detail, never free text with user data.
    pub fn detail(&self) -> &'static str {
        match self {
            SettingsError::InvalidCountry(_) => "vpn_country",
            SettingsError::PortalTimeout(_) => "portal_timeout_sec",
            SettingsError::NetModeUnsupported => "net_mode",
        }
    }
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::InvalidCountry(value) => {
                write!(formatter, "invalid vpn_country: {value:?}")
            }
            SettingsError::PortalTimeout(value) => {
                write!(formatter, "portal_timeout_sec out of range: {value}")
            }
            SettingsError::NetModeUnsupported => {
                formatter.write_str("net_mode \"system\" is not supported in v0")
            }
        }
    }
}

impl std::error::Error for SettingsError {}

impl Settings {
    /// Applies a patch and validates the result.
    pub fn apply(&self, patch: &SettingsPatch) -> Result<Settings, SettingsError> {
        let mut merged = self.clone();

        if let Some(value) = patch.vpn_enabled {
            merged.vpn_enabled = value;
        }
        if let Some(value) = &patch.vpn_country {
            merged.vpn_country = value.clone();
        }
        if let Some(value) = patch.tor_bridges {
            merged.tor_bridges = value;
        }
        if let Some(value) = patch.block_fallback {
            merged.block_fallback = value;
        }
        if let Some(value) = patch.mac_mode {
            merged.mac_mode = value;
        }
        if let Some(value) = patch.torrent_mode {
            merged.torrent_mode = value;
        }
        if let Some(value) = patch.net_mode {
            merged.net_mode = value;
        }
        if let Some(value) = patch.clipboard_watch {
            merged.clipboard_watch = value;
        }
        if let Some(value) = patch.scanner_hash_lookup {
            merged.scanner_hash_lookup = value;
        }
        if let Some(value) = patch.portal_timeout_sec {
            merged.portal_timeout_sec = value;
        }

        merged.validate()?;
        Ok(merged)
    }

    /// Validates every field. Called after a patch and when loading settings.
    pub fn validate(&self) -> Result<(), SettingsError> {
        if !is_valid_country(&self.vpn_country) {
            return Err(SettingsError::InvalidCountry(self.vpn_country.clone()));
        }
        if !(60..=600).contains(&self.portal_timeout_sec) {
            return Err(SettingsError::PortalTimeout(self.portal_timeout_sec));
        }
        if self.net_mode != NetMode::Focus {
            return Err(SettingsError::NetModeUnsupported);
        }
        Ok(())
    }
}

/// `"random"` or two uppercase ASCII letters.
pub fn is_valid_country(value: &str) -> bool {
    if value == "random" {
        return true;
    }
    let bytes = value.as_bytes();
    bytes.len() == 2 && bytes.iter().all(u8::is_ascii_uppercase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_match_the_contract() {
        let settings = Settings::default();
        assert!(settings.vpn_enabled);
        assert_eq!(settings.vpn_country, "random");
        assert!(!settings.tor_bridges);
        assert_eq!(settings.block_fallback, BlockFallback::Auto);
        assert_eq!(settings.mac_mode, MacMode::Random);
        assert_eq!(settings.torrent_mode, TorrentMode::Blocked);
        assert_eq!(settings.net_mode, NetMode::Focus);
        assert_eq!(settings.portal_timeout_sec, 180);
        settings.validate().expect("defaults must validate");
    }

    #[test]
    fn countries_are_random_or_two_uppercase_letters() {
        assert!(is_valid_country("random"));
        assert!(is_valid_country("SE"));
        assert!(!is_valid_country("se"));
        assert!(!is_valid_country("SWE"));
        assert!(!is_valid_country(""));
        assert!(!is_valid_country("S1"));
    }

    #[test]
    fn patch_merges_and_keeps_other_fields() {
        let mut settings = Settings::default();
        let patch: SettingsPatch = serde_json::from_value(json!({ "vpn_country": "NL" })).unwrap();
        settings = settings.apply(&patch).unwrap();
        assert_eq!(settings.vpn_country, "NL");
        assert_eq!(settings.torrent_mode, TorrentMode::Blocked);
    }

    #[test]
    fn invalid_patch_changes_nothing() {
        let settings = Settings::default();
        let patch: SettingsPatch =
            serde_json::from_value(json!({ "portal_timeout_sec": 5 })).unwrap();
        let error = settings.apply(&patch).unwrap_err();
        assert_eq!(error.code(), "E_IPC_BAD_SCHEMA");
        assert_eq!(error.detail(), "portal_timeout_sec");
        // The original is untouched because apply works on a clone.
        assert_eq!(settings.portal_timeout_sec, 180);
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let result: Result<SettingsPatch, _> =
            serde_json::from_value(json!({ "log_level": "debug" }));
        assert!(result.is_err());
    }

    #[test]
    fn system_mode_is_rejected() {
        let settings = Settings::default();
        let patch: SettingsPatch = serde_json::from_value(json!({ "net_mode": "system" })).unwrap();
        assert_eq!(
            settings.apply(&patch).unwrap_err(),
            SettingsError::NetModeUnsupported
        );
    }

    #[test]
    fn torrent_mode_accepts_vpn_only() {
        let settings = Settings::default();
        let patch: SettingsPatch =
            serde_json::from_value(json!({ "torrent_mode": "vpn_only" })).unwrap();
        let merged = settings.apply(&patch).unwrap();
        assert_eq!(merged.torrent_mode, TorrentMode::VpnOnly);
    }

    #[test]
    fn timeout_bounds_are_inclusive() {
        for (value, ok) in [
            (60, true),
            (180, true),
            (600, true),
            (59, false),
            (601, false),
        ] {
            let settings = Settings {
                portal_timeout_sec: value,
                ..Settings::default()
            };
            assert_eq!(settings.validate().is_ok(), ok, "timeout {value}");
        }
    }

    #[test]
    fn round_trip_uses_contract_key_names() {
        let settings = Settings::default();
        let value = serde_json::to_value(&settings).unwrap();
        assert_eq!(value["block_fallback"], "auto");
        assert_eq!(value["mac_mode"], "random");
        assert_eq!(value["net_mode"], "focus");
        assert_eq!(value["torrent_mode"], "blocked");
        assert!(value.get("portal_timeout_sec").is_some());
        let back: Settings = serde_json::from_value(value).unwrap();
        assert_eq!(back, settings);
    }
}
