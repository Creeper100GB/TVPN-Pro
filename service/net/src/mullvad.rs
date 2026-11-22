//! Mullvad CLI wrapper from `PLAN.md` 4.5.
//!
//! Only fixed arguments from the service are passed. The account number is
//! never an argument here; login belongs in the GUI/Credential Manager path.
//! Real integration is tested in the VM with the Mullvad app installed.

use std::fmt;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MullvadStatus {
    Disconnected,
    Connecting { location: Option<String> },
    Connected { location: Option<String> },
    Blocked { reason: String },
    Unknown(String),
}

#[derive(Debug)]
pub enum MullvadError {
    Io(std::io::Error),
    Command(i32),
    InvalidCountry,
}

impl fmt::Display for MullvadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "mullvad io: {error}"),
            Self::Command(code) => write!(formatter, "mullvad exited with {code}"),
            Self::InvalidCountry => formatter.write_str("invalid Mullvad country code"),
        }
    }
}

impl std::error::Error for MullvadError {}

impl From<std::io::Error> for MullvadError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Parses `mullvad status` output without exposing relay names as free text.
pub fn parse_status(output: &str) -> MullvadStatus {
    let first = output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");

    if let Some(rest) = first.strip_prefix("Connected to ") {
        return MullvadStatus::Connected {
            location: Some(rest.to_owned()),
        };
    }
    if let Some(rest) = first.strip_prefix("Connecting to ") {
        return MullvadStatus::Connecting {
            location: Some(rest.to_owned()),
        };
    }
    if first == "Disconnected" || first.starts_with("Disconnected ") {
        return MullvadStatus::Disconnected;
    }
    if let Some(rest) = first.strip_prefix("Blocked") {
        return MullvadStatus::Blocked {
            reason: rest.trim_start_matches(':').trim().to_owned(),
        };
    }

    MullvadStatus::Unknown(first.to_owned())
}

/// Two ASCII letters, lowercased. Anything else is refused before it reaches
/// the command line.
pub fn normalize_country(value: &str) -> Result<String, MullvadError> {
    let lower = value.to_ascii_lowercase();
    if lower.len() == 2
        && lower
            .chars()
            .all(|character| character.is_ascii_lowercase())
    {
        Ok(lower)
    } else {
        Err(MullvadError::InvalidCountry)
    }
}

pub struct MullvadClient {
    executable: PathBuf,
}

impl MullvadClient {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    pub fn status(&self) -> Result<MullvadStatus, MullvadError> {
        Ok(parse_status(&self.run(&["status"])?))
    }

    pub fn connect(&self) -> Result<(), MullvadError> {
        self.run(&["connect"]).map(|_| ())
    }

    pub fn disconnect(&self) -> Result<(), MullvadError> {
        self.run(&["disconnect"]).map(|_| ())
    }

    pub fn set_location(&self, country: &str) -> Result<(), MullvadError> {
        let country = normalize_country(country)?;
        self.run(&["relay", "set", "location", &country])
            .map(|_| ())
    }

    pub fn set_lockdown(&self, enabled: bool) -> Result<(), MullvadError> {
        let mode = if enabled { "on" } else { "off" };
        self.run(&["lockdown-mode", "set", mode]).map(|_| ())
    }

    fn run(&self, arguments: &[&str]) -> Result<String, MullvadError> {
        let output = Command::new(&self.executable).args(arguments).output()?;
        if !output.status.success() {
            return Err(MullvadError::Command(output.status.code().unwrap_or(-1)));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_connected() {
        assert_eq!(
            parse_status("Connected to se-sto-wg-001 in Stockholm, Sweden\n"),
            MullvadStatus::Connected {
                location: Some("se-sto-wg-001 in Stockholm, Sweden".to_owned())
            }
        );
    }

    #[test]
    fn parses_connecting_and_disconnected() {
        assert_eq!(
            parse_status("Connecting to de-fra-wg-002 in Frankfurt, Germany"),
            MullvadStatus::Connecting {
                location: Some("de-fra-wg-002 in Frankfurt, Germany".to_owned())
            }
        );
        assert_eq!(parse_status("Disconnected\n"), MullvadStatus::Disconnected);
    }

    #[test]
    fn parses_blocked() {
        assert_eq!(
            parse_status("Blocked: no account"),
            MullvadStatus::Blocked {
                reason: "no account".to_owned()
            }
        );
    }

    #[test]
    fn keeps_unknown_output_short() {
        assert_eq!(
            parse_status("Something new\nmore"),
            MullvadStatus::Unknown("Something new".to_owned())
        );
    }

    #[test]
    fn countries_are_two_letters_only() {
        assert_eq!(normalize_country("SE").unwrap(), "se");
        assert!(matches!(
            normalize_country("se; rm -rf"),
            Err(MullvadError::InvalidCountry)
        ));
        assert!(matches!(
            normalize_country("random"),
            Err(MullvadError::InvalidCountry)
        ));
    }
}
