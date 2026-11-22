//! `tvpnd` protocol core.
//!
//! This module owns the state machine and the fixed command whitelist. The
//! privileged side effects live in [`backend`]; the transport (named pipe,
//! token file, ACL) lives next to it and is tested on Windows.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tvpn_core::ipc::{Command, Request, Response};
use tvpn_core::settings::{Settings, SettingsPatch};
use tvpn_core::state::{Event, State};

use crate::backend::{default_backend, Backend, BackendError, BackendStatus};

/// Version reported to the UI. Keep in sync with the workspace version.
pub const SERVICE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Protocol version from `contracts/ipc-v0.md`.
pub const PROTOCOL_VERSION: u8 = 0;

#[cfg(windows)]
pub mod pipe;

#[cfg(windows)]
pub mod service;

pub mod backend;

/// One service session. The caller owns the transport and the token.
pub struct Service {
    state: State,
    state_since: u64,
    settings: Settings,
    session_id: String,
    started_at: u64,
    authorized: bool,
    token: String,
    backend: Box<dyn Backend>,
    banners: Vec<String>,
    failure_code: Option<String>,
    last_summary: Option<Value>,
    reached_ready: bool,
    failclosed_count: u32,
}

impl Service {
    /// Creates a service that expects `token` in the `hello` request.
    pub fn new(token: impl Into<String>) -> Self {
        Self::with_backend(token, default_backend())
    }

    /// Test and integration constructor.
    pub fn with_backend(token: impl Into<String>, backend: Box<dyn Backend>) -> Self {
        let started_at = now();
        Self {
            state: State::Off,
            state_since: started_at,
            settings: Settings::default(),
            session_id: format!("{:x}-{:x}", std::process::id(), started_at),
            started_at,
            authorized: false,
            token: token.into(),
            backend,
            banners: Vec::new(),
            failure_code: None,
            last_summary: None,
            reached_ready: false,
            failclosed_count: 0,
        }
    }

    /// Ends the authentication of the previous pipe connection.
    pub fn reset_auth(&mut self) {
        self.authorized = false;
    }

    /// Handles exactly one request and returns exactly one response.
    pub fn handle(&mut self, request: &Request) -> Response {
        if !self.authorized && request.command != Command::Hello {
            return Response::error(request.id.as_str(), "E_IPC_AUTH", Some("auth"));
        }

        match request.command {
            Command::Hello => self.hello(request),
            Command::Enable => self.enable(request),
            Command::Disable => self.disable(request),
            Command::Panic => self.panic(request),
            Command::Recover => self.recover(request),
            Command::GetStatus => Response::ok(request.id.as_str(), self.status()),
            Command::GetSettings => Response::ok(
                request.id.as_str(),
                serde_json::to_value(&self.settings).unwrap_or(Value::Null),
            ),
            Command::SetSettings => self.set_settings(request),
            Command::GetSessionSummary => Response::ok(
                request.id.as_str(),
                self.last_summary.clone().unwrap_or(Value::Null),
            ),
            // The shell owns downloads and scans in v0; the service never
            // accepts paths or URLs from the UI. These commands are reserved.
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

    fn enable(&mut self, request: &Request) -> Response {
        if self.state != State::Off {
            return Response::error(request.id.as_str(), "E_STATE_CONFLICT", Some("not_off"));
        }

        self.failure_code = None;
        self.banners.clear();
        self.last_summary = None;

        if let Err(error) = self.transition(Event::Enable) {
            return error_response(request, error);
        }

        if let Err(error) = self.backend.preflight(&self.settings) {
            return self.fail(request, error);
        }
        if let Err(error) = self.transition(Event::PreflightOk) {
            return error_response(request, error);
        }

        if let Err(error) = self.backend.arm() {
            return self.fail(request, error);
        }

        match self.backend.randomize_mac() {
            Ok(true) => {}
            Ok(false) | Err(_) => self.add_banner("mac_unsupported"),
        }
        // S3 measured that Windows only applies a new hostname after a reboot.
        // The setup can neutralize it once; a session cannot change it.
        self.add_banner("hostname_unchanged");

        if self.settings.vpn_enabled {
            if let Err(error) = self.backend.vpn_connect(&self.settings.vpn_country) {
                return self.fail(request, error);
            }
        } else {
            self.add_banner("degraded");
        }

        if let Err(error) = self.transition(Event::VpnUp) {
            return error_response(request, error);
        }

        if let Err(error) = self.backend.tor_start() {
            return self.fail(request, error);
        }
        if let Err(error) = self.backend.tor_wait(Duration::from_secs(180)) {
            return self.fail(request, error);
        }
        if let Err(error) = self.transition(Event::TorUp) {
            return error_response(request, error);
        }

        if let Err(error) = self.backend.leak_check(self.settings.vpn_enabled) {
            return self.fail(request, error);
        }
        if let Err(error) = self.transition(Event::LeakCheckOk) {
            return error_response(request, error);
        }

        self.reached_ready = true;
        if !self.settings.vpn_enabled {
            let _ = self.transition(Event::WatchdogWarn);
        }

        Response::ok(
            request.id.as_str(),
            json!({ "accepted": true, "state": self.state.as_str() }),
        )
    }

    fn disable(&mut self, request: &Request) -> Response {
        if self.state == State::Off || self.state == State::Teardown {
            return Response::ok(
                request.id.as_str(),
                json!({ "accepted": false, "state": self.state.as_str() }),
            );
        }

        let reached = if self.reached_ready {
            "READY"
        } else {
            self.state.as_str()
        };

        if let Err(error) = self.transition(Event::Disable) {
            return error_response(request, error);
        }

        let _ = self.backend.shutdown();
        if let Err(error) = self.backend.disarm() {
            let _ = self.transition(Event::TeardownIncomplete);
            self.failure_code = Some(error.code.to_owned());
            return Response::error(request.id.as_str(), error.code, Some(error.detail));
        }
        let _ = self.transition(Event::TeardownDone);

        self.last_summary = Some(self.summary(reached, self.state.as_str()));
        self.banners.clear();
        self.failure_code = None;
        self.reached_ready = false;

        Response::ok(
            request.id.as_str(),
            json!({ "accepted": true, "state": self.state.as_str() }),
        )
    }

    fn panic(&mut self, request: &Request) -> Response {
        if let Err(error) = self.transition(Event::Panic) {
            return error_response(request, error);
        }

        let _ = self.backend.shutdown();
        if let Err(error) = self.backend.disarm() {
            let _ = self.transition(Event::TeardownIncomplete);
            self.failure_code = Some(error.code.to_owned());
        } else {
            let _ = self.transition(Event::TeardownDone);
            self.banners.clear();
            self.failure_code = None;
            self.reached_ready = false;
        }

        Response::ok(
            request.id.as_str(),
            json!({ "accepted": true, "state": self.state.as_str() }),
        )
    }

    fn recover(&mut self, request: &Request) -> Response {
        if self.state == State::Off {
            return Response::error(request.id.as_str(), "E_STATE_CONFLICT", Some("already_off"));
        }

        if let Err(error) = self.transition(Event::Recover) {
            return error_response(request, error);
        }

        let _ = self.backend.shutdown();
        if let Err(error) = self.backend.disarm() {
            self.failure_code = Some(error.code.to_owned());
            return Response::error(request.id.as_str(), error.code, Some(error.detail));
        }

        let _ = self.transition(Event::RecoveryDone);
        self.banners.clear();
        self.failure_code = None;
        self.reached_ready = false;

        Response::ok(
            request.id.as_str(),
            json!({ "accepted": true, "state": self.state.as_str() }),
        )
    }

    fn set_settings(&mut self, request: &Request) -> Response {
        if self.state != State::Off {
            return Response::error(request.id.as_str(), "E_STATE_CONFLICT", Some("not_off"));
        }

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

    fn status(&mut self) -> Value {
        let backend = self.backend.status();
        let BackendStatus {
            vpn_connected,
            vpn_country,
            tor_bootstrap,
            tor_circuit,
            firewall_armed,
            firewall_filters,
            mac_supported,
            mac_randomized,
            banners,
        } = backend;

        // Lightweight watchdog: every status poll rechecks the components. A
        // failure while armed moves to FAILCLOSED and keeps the filters.
        if matches!(self.state, State::Ready | State::Degraded) {
            let level = tvpn_watchdog::evaluate(
                firewall_armed,
                tor_circuit,
                self.settings.vpn_enabled,
                vpn_connected,
            );
            if level == tvpn_watchdog::Level::Critical {
                let _ = self.transition(Event::WatchdogCritical);
                self.failure_code = Some("E_WATCHDOG_CRITICAL".to_owned());
                self.add_banner("failclosed");
            }
        }

        let mut all_banners = self.banners.clone();
        for banner in banners {
            if !all_banners.contains(&banner) {
                all_banners.push(banner);
            }
        }
        if self.state == State::FailClosed && !all_banners.iter().any(|b| b == "failclosed") {
            all_banners.push("failclosed".to_owned());
        }

        let watchdog = match self.state {
            State::FailClosed | State::Recovery => "critical",
            State::Degraded => "warn",
            _ => "ok",
        };

        json!({
            "state": self.state.as_str(),
            "since": iso8601(self.state_since),
            "net_mode": "focus",
            "protect_mode": "tor_vpn",
            "vpn": {
                "state": if vpn_connected { "connected" } else { "disconnected" },
                "country": vpn_country,
            },
            "tor": { "bootstrap": tor_bootstrap, "circuit": tor_circuit },
            "watchdog": { "level": watchdog, "last_check_at": iso8601(now()) },
            "banners": all_banners,
            "downloads": { "active": 0, "quarantined": 0 },
            "session": { "id": &self.session_id, "started_at": iso8601(self.started_at) },
            "service": { "version": SERVICE_VERSION, "protocol": PROTOCOL_VERSION },
            "shield": {
                "firewall_armed": firewall_armed,
                "firewall_filters": firewall_filters,
                "mac_supported": mac_supported,
                "mac_randomized": mac_randomized,
            },
        })
    }

    fn summary(&self, reached: &str, ended: &str) -> Value {
        json!({
            "started_at": iso8601(self.started_at),
            "ended_at": iso8601(now()),
            "downloads": { "clean": 0, "warned": 0, "quarantined": 0, "failed": 0 },
            "state_reached": reached,
            "failclosed_count": self.failclosed_count,
            "reasons": self.failure_code.clone().map(|code| vec![code]).unwrap_or_default(),
            "ended_state": ended,
        })
    }

    fn add_banner(&mut self, banner: &str) {
        if !self.banners.iter().any(|value| value == banner) {
            self.banners.push(banner.to_owned());
        }
    }

    fn transition(&mut self, event: Event) -> Result<(), BackendError> {
        let next = tvpn_core::state::next(self.state, event)
            .map_err(|_| BackendError::new("E_STATE_CONFLICT", "transition"))?;
        if next != self.state {
            if next == State::FailClosed {
                self.failclosed_count += 1;
            }
            self.state = next;
            self.state_since = now();
        }
        Ok(())
    }

    fn fail(&mut self, request: &Request, error: BackendError) -> Response {
        let event = if self.state == State::Preflight {
            Event::PreflightFailed
        } else {
            Event::WatchdogCritical
        };
        let _ = self.transition(event);
        self.failure_code = Some(error.code.to_owned());
        self.add_banner("failclosed");
        Response::error(request.id.as_str(), error.code, Some(error.detail))
    }
}

fn error_response(request: &Request, error: BackendError) -> Response {
    Response::error(request.id.as_str(), error.code, Some(error.detail))
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
    use crate::backend::MockBackend;
    use tvpn_core::ipc::parse_request;

    fn request(command: &str, id: &str, args: Value) -> Request {
        let line = json!({ "v": 0, "id": id, "cmd": command, "args": args }).to_string();
        parse_request(&line).expect("test request must parse")
    }

    fn authorized() -> Service {
        authorized_with(Box::new(MockBackend::default()))
    }

    fn authorized_with(backend: Box<dyn Backend>) -> Service {
        let mut service = Service::with_backend("secret", backend);
        let response = service.handle(&request("hello", "h", json!({ "token": "secret" })));
        assert!(response.ok);
        service
    }

    fn enable(service: &mut Service) -> Response {
        service.handle(&request(
            "enable",
            "e",
            json!({ "net_mode": "focus", "protect_mode": "tor_vpn" }),
        ))
    }

    #[test]
    fn hello_requires_the_token() {
        let mut service = Service::with_backend("secret", Box::new(MockBackend::default()));
        let response = service.handle(&request("hello", "h", json!({ "token": "wrong" })));
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "E_IPC_AUTH");
    }

    #[test]
    fn commands_before_hello_are_refused() {
        let mut service = Service::with_backend("secret", Box::new(MockBackend::default()));
        let response = service.handle(&request("get_status", "s", Value::Null));
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "E_IPC_AUTH");
    }

    #[test]
    fn hello_returns_the_session_and_states() {
        let mut service = Service::with_backend("secret", Box::new(MockBackend::default()));
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
    fn enable_reaches_ready_and_disable_returns_off() {
        let mut service = authorized();
        let response = enable(&mut service);
        assert!(response.ok, "{response:?}");
        assert_eq!(service.state, State::Ready);
        assert_eq!(service.status()["state"], "READY");

        let response = service.handle(&request(
            "disable",
            "d",
            json!({ "downloads_policy": "wait" }),
        ));
        assert!(response.ok);
        assert_eq!(service.state, State::Off);
        assert!(service.last_summary.is_some());
    }

    #[test]
    fn enable_twice_is_a_state_conflict() {
        let mut service = authorized();
        assert!(enable(&mut service).ok);
        let response = enable(&mut service);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "E_STATE_CONFLICT");
    }

    #[test]
    fn panic_from_ready_ends_off() {
        let mut service = authorized();
        assert!(enable(&mut service).ok);
        let response = service.handle(&request("panic", "p", Value::Null));
        assert!(response.ok);
        assert_eq!(service.state, State::Off);
    }

    #[test]
    fn recover_from_ready_ends_off() {
        let mut service = authorized();
        assert!(enable(&mut service).ok);
        let response = service.handle(&request("recover", "r", Value::Null));
        assert!(response.ok);
        assert_eq!(service.state, State::Off);
    }

    #[test]
    fn preflight_failure_fails_closed() {
        let mut service = authorized_with(Box::new(MockBackend::failing()));
        let response = enable(&mut service);
        assert!(!response.ok);
        assert_eq!(
            response.error.unwrap().code,
            "E_PREFLIGHT_MISSING_COMPONENT"
        );
        assert_eq!(service.state, State::FailClosed);
        assert_eq!(service.status()["state"], "FAILCLOSED");
    }

    #[test]
    fn settings_are_locked_while_armed() {
        let mut service = authorized();
        assert!(enable(&mut service).ok);
        let response = service.handle(&request(
            "set_settings",
            "set",
            json!({ "settings": { "vpn_country": "NL" } }),
        ));
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "E_STATE_CONFLICT");
    }

    #[test]
    fn iso8601_formats_epoch() {
        assert_eq!(iso8601(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
    }
}
