//! `tvpnd` protocol core.
//!
//! This module is pure logic: it handles the contract commands that do not
//! need a privileged backend yet. The transport (named pipe, token file, ACL)
//! lives next to it and is tested on Windows.

use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use tvpn_core::ipc::{Command, Request, Response};
use tvpn_core::settings::{Settings, SettingsPatch};
use tvpn_core::state::State;

/// Version reported to the UI. Keep in sync with the workspace version.
pub const SERVICE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Protocol version from `contracts/ipc-v0.md`.
pub const PROTOCOL_VERSION: u8 = 0;

/// One service session. The caller owns the transport and the token.
pub struct Service {
    state: State,
    settings: Settings,
    session_id: String,
    started_at: u64,
    authorized: bool,
    token: String,
}

impl Service {
    /// Creates a service that expects `token` in the `hello` request.
    pub fn new(token: impl Into<String>) -> Self {
        let started_at = now();
        Self {
            state: State::Off,
            settings: Settings::default(),
            session_id: format!("{:x}-{:x}", std::process::id(), started_at),
            started_at,
            authorized: false,
            token: token.into(),
        }
    }

    /// Handles exactly one request and returns exactly one response.
    pub fn handle(&mut self, request: &Request) -> Response {
        if !self.authorized && request.command != Command::Hello {
            return Response::error(request.id.as_str(), "E_IPC_AUTH", Some("auth"));
        }

        match request.command {
            Command::Hello => self.hello(request),
            Command::GetStatus => Response::ok(request.id.as_str(), self.status()),
            Command::GetSettings => Response::ok(
                request.id.as_str(),
                serde_json::to_value(&self.settings).unwrap_or(Value::Null),
            ),
            Command::SetSettings => self.set_settings(request),
            // The privileged backend is added in the next phase. Until then the
            // state machine and the UI must not believe that anything is armed.
            _ => Response::error(
                request.id.as_str(),
                "E_STATE_CONFLICT",
                Some("not_implemented"),
            ),
        }
    }

    fn hello(&mut self, request: &Request) -> Response {
        let token = request
            .args
            .get("token")
            .and_then(Value::as_str)
            .unwrap_or("");

        if token != self.token {
            return Response::error(request.id.as_str(), "E_IPC_AUTH", Some("auth"));
        }

        self.authorized = true;
        let states: Vec<&str> = State::ALL.iter().map(|state| state.as_str()).collect();
        Response::ok(
            request.id.as_str(),
            json!({
                "session_id": &self.session_id,
                "service_version": SERVICE_VERSION,
                "protocol": PROTOCOL_VERSION,
                "states": states,
            }),
        )
    }

    fn set_settings(&mut self, request: &Request) -> Response {
        let raw = request.args.get("settings").cloned().unwrap_or(Value::Null);
        let patch: SettingsPatch = match serde_json::from_value(raw) {
            Ok(patch) => patch,
            Err(_) => {
                return Response::error(request.id.as_str(), "E_IPC_BAD_SCHEMA", Some("schema"))
            }
        };

        match self.settings.apply(&patch) {
            Ok(settings) => {
                self.settings = settings;
                Response::ok(
                    request.id.as_str(),
                    serde_json::to_value(&self.settings).unwrap_or(Value::Null),
                )
            }
            Err(error) => Response::error(request.id.as_str(), error.code(), Some(error.detail())),
        }
    }

    fn status(&self) -> Value {
        json!({
            "state": self.state.as_str(),
            "since": iso8601(self.started_at),
            "net_mode": "focus",
            "protect_mode": "tor_vpn",
            "vpn": { "state": "disconnected", "country": null },
            "tor": { "bootstrap": 0, "circuit": false },
            "watchdog": { "level": "ok", "last_check_at": null },
            "banners": [],
            "downloads": { "active": 0, "quarantined": 0 },
            "session": { "id": &self.session_id, "started_at": iso8601(self.started_at) },
            "service": { "version": SERVICE_VERSION, "protocol": PROTOCOL_VERSION },
        })
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Minimal RFC 3339 formatter for UTC seconds, without an extra dependency.
pub fn iso8601(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let remainder = seconds % 86_400;

    // Howard Hinnant's civil-from-days algorithm.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };

    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        remainder / 3_600,
        (remainder % 3_600) / 60,
        remainder % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tvpn_core::ipc::parse_request;

    fn request(command: &str, id: &str, args: Value) -> Request {
        let line = json!({ "v": 0, "id": id, "cmd": command, "args": args }).to_string();
        parse_request(&line).expect("test request must parse")
    }

    fn authorized() -> Service {
        let mut service = Service::new("secret");
        let response = service.handle(&request("hello", "h", json!({ "token": "secret" })));
        assert!(response.ok);
        service
    }

    #[test]
    fn hello_requires_the_token() {
        let mut service = Service::new("secret");
        let response = service.handle(&request("hello", "h", json!({ "token": "wrong" })));
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "E_IPC_AUTH");
    }

    #[test]
    fn commands_before_hello_are_refused() {
        let mut service = Service::new("secret");
        let response = service.handle(&request("get_status", "s", Value::Null));
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "E_IPC_AUTH");
    }

    #[test]
    fn hello_returns_the_session_and_states() {
        let mut service = Service::new("secret");
        let response = service.handle(&request("hello", "h", json!({ "token": "secret" })));
        assert!(response.ok);
        let data = response.data.unwrap();
        assert_eq!(data["protocol"].as_u64(), Some(0));
        assert_eq!(data["service_version"].as_str(), Some(SERVICE_VERSION));
        assert_eq!(data["states"].as_array().unwrap().len(), 11);
    }

    #[test]
    fn settings_round_trip() {
        let mut service = authorized();
        let response = service.handle(&request(
            "set_settings",
            "set",
            json!({ "settings": { "vpn_country": "NL" } }),
        ));
        assert!(response.ok);
        assert_eq!(response.data.unwrap()["vpn_country"], "NL");

        let response = service.handle(&request("get_settings", "get", Value::Null));
        assert_eq!(response.data.unwrap()["vpn_country"], "NL");
    }

    #[test]
    fn invalid_settings_keep_the_old_value() {
        let mut service = authorized();
        let response = service.handle(&request(
            "set_settings",
            "set",
            json!({ "settings": { "portal_timeout_sec": 5 } }),
        ));
        assert!(!response.ok);
        let error = response.error.unwrap();
        assert_eq!(error.code, "E_IPC_BAD_SCHEMA");
        assert_eq!(error.detail.as_deref(), Some("portal_timeout_sec"));

        let response = service.handle(&request("get_settings", "get", Value::Null));
        assert_eq!(response.data.unwrap()["portal_timeout_sec"], 180);
    }

    #[test]
    fn status_has_the_contract_shape() {
        let mut service = authorized();
        let response = service.handle(&request("get_status", "s", Value::Null));
        let data = response.data.unwrap();
        assert_eq!(data["state"], "OFF");
        assert_eq!(data["service"]["protocol"].as_u64(), Some(0));
        assert!(data["since"].as_str().unwrap().ends_with('Z'));
    }

    #[test]
    fn iso8601_formats_epoch() {
        assert_eq!(iso8601(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
    }
}
