//! IPC envelope from `contracts/ipc-v0.md` sections 1 to 4.
//!
//! Only the envelope is modelled here: version, id, command whitelist, size
//! limit and the response/event shapes. Command arguments are validated by the
//! handlers, not by the transport.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Protocol version carried in every message.
pub const PROTOCOL_VERSION: u8 = 0;

/// Hard limit per message, before parsing.
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;

/// Longest accepted request id.
pub const MAX_ID_BYTES: usize = 64;

/// The fixed command whitelist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    #[serde(rename = "hello")]
    Hello,
    #[serde(rename = "enable")]
    Enable,
    #[serde(rename = "disable")]
    Disable,
    #[serde(rename = "panic")]
    Panic,
    #[serde(rename = "recover")]
    Recover,
    #[serde(rename = "get_status")]
    GetStatus,
    #[serde(rename = "get_settings")]
    GetSettings,
    #[serde(rename = "set_settings")]
    SetSettings,
    #[serde(rename = "downloads.add")]
    DownloadsAdd,
    #[serde(rename = "downloads.control")]
    DownloadsControl,
    #[serde(rename = "open_in_fallback")]
    OpenInFallback,
    #[serde(rename = "scan.report")]
    ScanReport,
    #[serde(rename = "export_logs")]
    ExportLogs,
    #[serde(rename = "get_session_summary")]
    GetSessionSummary,
}

impl Command {
    /// Parses the wire name. Unknown names are not guessed.
    pub fn from_wire(value: &str) -> Option<Self> {
        let command = match value {
            "hello" => Self::Hello,
            "enable" => Self::Enable,
            "disable" => Self::Disable,
            "panic" => Self::Panic,
            "recover" => Self::Recover,
            "get_status" => Self::GetStatus,
            "get_settings" => Self::GetSettings,
            "set_settings" => Self::SetSettings,
            "downloads.add" => Self::DownloadsAdd,
            "downloads.control" => Self::DownloadsControl,
            "open_in_fallback" => Self::OpenInFallback,
            "scan.report" => Self::ScanReport,
            "export_logs" => Self::ExportLogs,
            "get_session_summary" => Self::GetSessionSummary,
            _ => return None,
        };
        Some(command)
    }

    /// Wire name used in requests and logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hello => "hello",
            Self::Enable => "enable",
            Self::Disable => "disable",
            Self::Panic => "panic",
            Self::Recover => "recover",
            Self::GetStatus => "get_status",
            Self::GetSettings => "get_settings",
            Self::SetSettings => "set_settings",
            Self::DownloadsAdd => "downloads.add",
            Self::DownloadsControl => "downloads.control",
            Self::OpenInFallback => "open_in_fallback",
            Self::ScanReport => "scan.report",
            Self::ExportLogs => "export_logs",
            Self::GetSessionSummary => "get_session_summary",
        }
    }
}

/// A validated request.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub id: String,
    pub command: Command,
    pub args: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRequest {
    v: u8,
    id: String,
    cmd: String,
    #[serde(default)]
    args: Value,
}

/// Why a message was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcError {
    TooLarge,
    BadVersion,
    BadSchema,
    BadId,
    UnknownCommand,
    RateLimit,
    Auth,
}

impl IpcError {
    /// Wire code from `contracts/ipc-v0.md` section 9.
    pub fn code(self) -> &'static str {
        match self {
            Self::TooLarge => "E_IPC_TOO_LARGE",
            Self::BadVersion => "E_IPC_BAD_VERSION",
            Self::BadSchema => "E_IPC_BAD_SCHEMA",
            Self::BadId => "E_IPC_BAD_SCHEMA",
            Self::UnknownCommand => "E_IPC_UNKNOWN_COMMAND",
            Self::RateLimit => "E_IPC_RATE_LIMIT",
            Self::Auth => "E_IPC_AUTH",
        }
    }

    /// Short machine detail, never message content.
    pub fn detail(self) -> &'static str {
        match self {
            Self::TooLarge => "too_large",
            Self::BadVersion => "version",
            Self::BadSchema => "schema",
            Self::BadId => "id",
            Self::UnknownCommand => "command",
            Self::RateLimit => "rate_limit",
            Self::Auth => "auth",
        }
    }
}

impl fmt::Display for IpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.detail())
    }
}

impl std::error::Error for IpcError {}

/// Parses and validates one NDJSON line.
pub fn parse_request(line: &str) -> Result<Request, IpcError> {
    if line.len() > MAX_MESSAGE_BYTES {
        return Err(IpcError::TooLarge);
    }

    let raw: RawRequest = serde_json::from_str(line).map_err(|_| IpcError::BadSchema)?;

    if raw.v != PROTOCOL_VERSION {
        return Err(IpcError::BadVersion);
    }
    if !is_valid_id(&raw.id) {
        return Err(IpcError::BadId);
    }
    let command = Command::from_wire(&raw.cmd).ok_or(IpcError::UnknownCommand)?;

    Ok(Request {
        id: raw.id,
        command,
        args: raw.args,
    })
}

fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_BYTES
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Error body of a failed response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Response to exactly one request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub v: u8,
    pub id: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
}

impl Response {
    /// Successful response.
    pub fn ok(id: impl Into<String>, data: Value) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id: id.into(),
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    /// Failed response with a contract error code.
    pub fn error(id: impl Into<String>, code: &str, detail: Option<&str>) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            id: id.into(),
            ok: false,
            data: None,
            error: Some(ErrorBody {
                code: code.to_owned(),
                detail: detail.map(str::to_owned),
            }),
        }
    }
}

/// Event names from `contracts/ipc-v0.md` section 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventName {
    #[serde(rename = "state_changed")]
    StateChanged,
    #[serde(rename = "progress")]
    Progress,
    #[serde(rename = "banner")]
    Banner,
    #[serde(rename = "alarm")]
    Alarm,
    #[serde(rename = "download_event")]
    DownloadEvent,
    #[serde(rename = "scan_result")]
    ScanResult,
    #[serde(rename = "block_detected")]
    BlockDetected,
    #[serde(rename = "watchdog")]
    Watchdog,
    #[serde(rename = "settings_changed")]
    SettingsChanged,
    #[serde(rename = "session_summary")]
    SessionSummary,
    #[serde(rename = "resync_required")]
    ResyncRequired,
}

impl EventName {
    /// Wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StateChanged => "state_changed",
            Self::Progress => "progress",
            Self::Banner => "banner",
            Self::Alarm => "alarm",
            Self::DownloadEvent => "download_event",
            Self::ScanResult => "scan_result",
            Self::BlockDetected => "block_detected",
            Self::Watchdog => "watchdog",
            Self::SettingsChanged => "settings_changed",
            Self::SessionSummary => "session_summary",
            Self::ResyncRequired => "resync_required",
        }
    }
}

/// Unsolicited message from the service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub v: u8,
    pub evt: EventName,
    pub seq: u64,
    pub data: Value,
}

impl Event {
    /// Event with the current protocol version.
    pub fn new(seq: u64, name: EventName, data: Value) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            evt: name,
            seq,
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn line(command: &str, id: &str, version: u8) -> String {
        format!(r#"{{"v":{version},"id":"{id}","cmd":"{command}"}}"#)
    }

    #[test]
    fn parses_a_valid_request() {
        let request = parse_request(&line("enable", "req-1", 0)).unwrap();
        assert_eq!(request.command, Command::Enable);
        assert_eq!(request.id, "req-1");
        assert_eq!(request.args, Value::Null);
    }

    #[test]
    fn parses_dotted_command_names() {
        let request = parse_request(&line("downloads.add", "a", 0)).unwrap();
        assert_eq!(request.command, Command::DownloadsAdd);
        assert_eq!(request.command.as_str(), "downloads.add");
    }

    #[test]
    fn rejects_unknown_commands() {
        assert_eq!(
            parse_request(&line("shell.exec", "a", 0)),
            Err(IpcError::UnknownCommand)
        );
    }

    #[test]
    fn rejects_wrong_version() {
        assert_eq!(
            parse_request(&line("enable", "a", 1)),
            Err(IpcError::BadVersion)
        );
    }

    #[test]
    fn rejects_bad_ids() {
        for id in ["", "has space", "with/slash"] {
            assert_eq!(
                parse_request(&line("enable", id, 0)),
                Err(IpcError::BadId),
                "id {id:?}"
            );
        }
        let long = "a".repeat(MAX_ID_BYTES + 1);
        assert_eq!(
            parse_request(&line("enable", &long, 0)),
            Err(IpcError::BadId)
        );
    }

    #[test]
    fn rejects_unknown_fields() {
        let raw = r#"{"v":0,"id":"a","cmd":"enable","path":"C:\\evil.exe"}"#;
        assert_eq!(parse_request(raw), Err(IpcError::BadSchema));
    }

    #[test]
    fn rejects_oversized_messages() {
        let raw = format!(
            "{}{}",
            line("enable", "a", 0),
            " ".repeat(MAX_MESSAGE_BYTES)
        );
        assert_eq!(parse_request(&raw), Err(IpcError::TooLarge));
    }

    #[test]
    fn responses_omit_absent_parts() {
        let ok = serde_json::to_value(Response::ok("a", json!({"accepted": true}))).unwrap();
        assert_eq!(ok["ok"], true);
        assert_eq!(ok["data"]["accepted"], true);
        assert!(ok.get("error").is_none());

        let error = serde_json::to_value(Response::error("a", "E_IPC_AUTH", None)).unwrap();
        assert_eq!(error["ok"], false);
        assert_eq!(error["error"]["code"], "E_IPC_AUTH");
        assert!(error.get("data").is_none());
    }

    #[test]
    fn events_use_contract_names() {
        let event = Event::new(7, EventName::StateChanged, json!({"to": "READY"}));
        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["evt"], "state_changed");
        assert_eq!(value["seq"], 7);
        assert_eq!(value["v"], 0);
    }

    #[test]
    fn error_codes_match_the_contract() {
        assert_eq!(IpcError::TooLarge.code(), "E_IPC_TOO_LARGE");
        assert_eq!(IpcError::Auth.code(), "E_IPC_AUTH");
        assert_eq!(IpcError::UnknownCommand.code(), "E_IPC_UNKNOWN_COMMAND");
    }
}
