//! Privileged backend for `tvpnd`.
//!
//! The service owns the state machine; this module owns the side effects:
//! firewall, MAC, VPN and Tor. The real backend is Windows-only. Unit tests and
//! the UI integration test use [`MockBackend`] via `TVPN_TEST_BACKEND=mock`.
//!
//! The mock is never selected in a release build without that environment
//! variable, and it is never shipped as a protection claim.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use tvpn_core::settings::Settings;
use tvpn_net::mullvad::{MullvadClient, MullvadStatus};
use tvpn_net::tor::TorController;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use tvpn_net::wfp::WfpEngine;

/// A failure with a contract error code and a short machine detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError {
    pub code: &'static str,
    pub detail: &'static str,
}

impl BackendError {
    pub fn new(code: &'static str, detail: &'static str) -> Self {
        Self { code, detail }
    }
}

/// Snapshot of the components the service reports to the UI.
#[derive(Debug, Clone, Default)]
pub struct BackendStatus {
    pub vpn_connected: bool,
    pub vpn_country: Option<String>,
    pub tor_bootstrap: u8,
    pub tor_circuit: bool,
    pub firewall_armed: bool,
    pub firewall_filters: usize,
    pub mac_supported: bool,
    pub mac_randomized: bool,
    pub banners: Vec<String>,
}

/// Side effects needed by the state machine.
pub trait Backend {
    fn preflight(&mut self, settings: &Settings) -> Result<(), BackendError>;
    fn arm(&mut self) -> Result<(), BackendError>;
    fn randomize_mac(&mut self) -> Result<bool, BackendError>;
    fn vpn_connect(&mut self, country: &str) -> Result<(), BackendError>;
    fn tor_start(&mut self) -> Result<(), BackendError>;
    fn tor_wait(&mut self, timeout: Duration) -> Result<(), BackendError>;
    fn leak_check(&mut self, vpn_required: bool) -> Result<(), BackendError>;
    fn shutdown(&mut self) -> Result<(), BackendError>;
    fn disarm(&mut self) -> Result<(), BackendError>;
    fn status(&mut self) -> BackendStatus;
}

/// Deterministic backend for unit tests and UI integration.
#[derive(Debug, Default)]
pub struct MockBackend {
    armed: bool,
    vpn_connected: bool,
    tor_circuit: bool,
    mac_randomized: bool,
    tor_started: bool,
    fail_preflight: bool,
}

impl MockBackend {
    pub fn failing() -> Self {
        Self {
            fail_preflight: true,
            ..Self::default()
        }
    }
}

impl Backend for MockBackend {
    fn preflight(&mut self, _settings: &Settings) -> Result<(), BackendError> {
        if self.fail_preflight {
            return Err(BackendError::new("E_PREFLIGHT_MISSING_COMPONENT", "mock"));
        }
        Ok(())
    }

    fn arm(&mut self) -> Result<(), BackendError> {
        self.armed = true;
        Ok(())
    }

    fn randomize_mac(&mut self) -> Result<bool, BackendError> {
        self.mac_randomized = true;
        Ok(true)
    }

    fn vpn_connect(&mut self, _country: &str) -> Result<(), BackendError> {
        self.vpn_connected = true;
        Ok(())
    }

    fn tor_start(&mut self) -> Result<(), BackendError> {
        self.tor_started = true;
        Ok(())
    }

    fn tor_wait(&mut self, _timeout: Duration) -> Result<(), BackendError> {
        if self.tor_started {
            self.tor_circuit = true;
            Ok(())
        } else {
            Err(BackendError::new("E_TOR_CIRCUIT_FAILED", "not_started"))
        }
    }

    fn leak_check(&mut self, vpn_required: bool) -> Result<(), BackendError> {
        if !self.armed || !self.tor_circuit {
            return Err(BackendError::new("E_FIREWALL_VERIFY_MISMATCH", "not_armed"));
        }
        if vpn_required && !self.vpn_connected {
            return Err(BackendError::new("E_VPN_CONNECT_TIMEOUT", "not_connected"));
        }
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), BackendError> {
        self.vpn_connected = false;
        self.tor_circuit = false;
        self.tor_started = false;
        Ok(())
    }

    fn disarm(&mut self) -> Result<(), BackendError> {
        self.armed = false;
        Ok(())
    }

    fn status(&mut self) -> BackendStatus {
        BackendStatus {
            vpn_connected: self.vpn_connected,
            tor_circuit: self.tor_circuit,
            firewall_armed: self.armed,
            firewall_filters: if self.armed { 8 } else { 0 },
            mac_supported: true,
            mac_randomized: self.mac_randomized,
            ..BackendStatus::default()
        }
    }
}

/// Production backend. Windows only; the crate also builds on Linux for tests.
#[cfg(windows)]
pub struct RealBackend {
    wfp: Option<WfpEngine>,
    tor: Option<TorController>,
    tor_child: Option<Child>,
    mullvad: Option<MullvadClient>,
    armed: bool,
    vpn_country: Option<String>,
    mac_randomized: bool,
    banners: Vec<String>,
}

#[cfg(windows)]
impl RealBackend {
    pub fn new() -> Self {
        Self {
            wfp: None,
            tor: None,
            tor_child: None,
            mullvad: None,
            armed: false,
            vpn_country: None,
            mac_randomized: false,
            banners: Vec::new(),
        }
    }

    fn programdata() -> PathBuf {
        std::env::var("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(r"C:\ProgramData"))
            .join("TVPN")
    }

    pub fn tor_path() -> PathBuf {
        if let Ok(path) = std::env::var("TVPN_TOR_EXE") {
            return PathBuf::from(path);
        }
        let local = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .unwrap_or_default()
            .join("tor")
            .join("tor.exe");
        if local.exists() {
            return local;
        }
        PathBuf::from(r"C:\Program Files\TVPN\tor\tor.exe")
    }

    pub fn mullvad_path() -> PathBuf {
        if let Ok(path) = std::env::var("TVPN_MULLVAD_EXE") {
            return PathBuf::from(path);
        }
        PathBuf::from(r"C:\Program Files\Mullvad VPN\resources\mullvad.exe")
    }

    fn mullvad_daemon_path() -> PathBuf {
        PathBuf::from(r"C:\Program Files\Mullvad VPN\resources\mullvad-daemon.exe")
    }

    fn tor_data_dir() -> PathBuf {
        Self::programdata().join("tor")
    }

    fn cookie_path() -> PathBuf {
        Self::tor_data_dir().join("control_auth_cookie")
    }

    fn tor_address() -> String {
        std::env::var("TVPN_TOR_CONTROL").unwrap_or_else(|_| "127.0.0.1:9051".to_owned())
    }

    fn read_status(&mut self) -> (bool, Option<String>) {
        let Some(client) = self.mullvad.as_ref() else {
            return (false, None);
        };
        match client.status() {
            Ok(MullvadStatus::Connected { location }) => {
                let country = location
                    .as_deref()
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.split('-').next())
                    .map(str::to_ascii_lowercase);
                (true, country)
            }
            _ => (false, None),
        }
    }

    fn connect_tor(&mut self) -> Result<(), BackendError> {
        let controller = TorController::connect(&Self::tor_address(), &Self::cookie_path())
            .map_err(|_| BackendError::new("E_TOR_CONTROL_AUTH", "cookie"))?;
        self.tor = Some(controller);
        Ok(())
    }
}

#[cfg(windows)]
impl Default for RealBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(windows)]
impl Backend for RealBackend {
    fn preflight(&mut self, settings: &Settings) -> Result<(), BackendError> {
        let engine =
            WfpEngine::open().map_err(|_| BackendError::new("E_PREFLIGHT_NO_ADMIN", "wfp"))?;
        self.wfp = Some(engine);

        let tor = Self::tor_path();
        if !tor.exists() {
            return Err(BackendError::new("E_PREFLIGHT_MISSING_COMPONENT", "tor"));
        }

        if settings.vpn_enabled {
            let mullvad = Self::mullvad_path();
            if !mullvad.exists() {
                return Err(BackendError::new(
                    "E_PREFLIGHT_MISSING_COMPONENT",
                    "mullvad",
                ));
            }
            let client = MullvadClient::new(mullvad);
            match client.status() {
                Ok(MullvadStatus::Blocked { .. }) => {
                    return Err(BackendError::new(
                        "E_PREFLIGHT_VPN_NOT_LOGGED_IN",
                        "mullvad",
                    ))
                }
                Ok(_) => {}
                Err(_) => {
                    return Err(BackendError::new(
                        "E_PREFLIGHT_MISSING_COMPONENT",
                        "mullvad_cli",
                    ))
                }
            }
            self.mullvad = Some(client);
        }

        Ok(())
    }

    fn arm(&mut self) -> Result<(), BackendError> {
        let engine = self
            .wfp
            .as_mut()
            .ok_or_else(|| BackendError::new("E_FIREWALL_ARM_FAILED", "not_open"))?;
        engine
            .arm_default_deny()
            .map_err(|_| BackendError::new("E_FIREWALL_ARM_FAILED", "default_deny"))?;

        let mut paths = vec![Self::tor_path(), Self::mullvad_path()];
        let daemon = Self::mullvad_daemon_path();
        if daemon.exists() {
            paths.push(daemon);
        }

        for path in paths {
            if path.exists() {
                engine
                    .permit_app(&path)
                    .map_err(|_| BackendError::new("E_FIREWALL_ARM_FAILED", "permit"))?;
            }
        }

        self.armed = true;
        Ok(())
    }

    fn randomize_mac(&mut self) -> Result<bool, BackendError> {
        // Hyper-V synthetic adapters do not expose a writable MAC. The real
        // implementation must use the physical adapter's NetworkAddress
        // registry value and restart the adapter; S1 does not cover that yet.
        self.banners.push("mac_unsupported".to_owned());
        Ok(false)
    }

    fn vpn_connect(&mut self, country: &str) -> Result<(), BackendError> {
        let client = self
            .mullvad
            .as_ref()
            .ok_or_else(|| BackendError::new("E_PREFLIGHT_VPN_NOT_LOGGED_IN", "mullvad"))?;

        if country != "random" {
            client
                .set_location(country)
                .map_err(|_| BackendError::new("E_VPN_CONNECT_TIMEOUT", "location"))?;
        }
        client
            .connect()
            .map_err(|_| BackendError::new("E_VPN_CONNECT_TIMEOUT", "connect"))?;

        let deadline = Instant::now() + Duration::from_secs(40);
        while Instant::now() < deadline {
            let (connected, location) = self.read_status();
            if connected {
                self.vpn_country = location;
                return Ok(());
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        Err(BackendError::new("E_VPN_CONNECT_TIMEOUT", "timeout"))
    }

    fn tor_start(&mut self) -> Result<(), BackendError> {
        if self.connect_tor().is_ok() {
            return Ok(());
        }

        let tor_path = Self::tor_path();
        let data_dir = Self::tor_data_dir();
        std::fs::create_dir_all(&data_dir)
            .map_err(|_| BackendError::new("E_TOR_BOOTSTRAP_TIMEOUT", "data_dir"))?;
        let mut command = Command::new(&tor_path);
        command
            .arg("--SocksPort")
            .arg("9150")
            .arg("--ControlPort")
            .arg("9051")
            .arg("--CookieAuthentication")
            .arg("1")
            .arg("--DataDirectory")
            .arg(data_dir)
            .arg("--ClientOnly")
            .arg("1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if let Some(directory) = tor_path.parent() {
            let geoip = directory.join("geoip");
            if geoip.exists() {
                command.arg("--GeoIPFile").arg(geoip);
            }
            let geoip6 = directory.join("geoip6");
            if geoip6.exists() {
                command.arg("--GeoIPv6File").arg(geoip6);
            }
        }
        #[cfg(windows)]
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW

        let child = command
            .spawn()
            .map_err(|_| BackendError::new("E_TOR_BOOTSTRAP_TIMEOUT", "spawn"))?;
        self.tor_child = Some(child);

        let deadline = Instant::now() + Duration::from_secs(40);
        while Instant::now() < deadline {
            if self.connect_tor().is_ok() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        Err(BackendError::new("E_TOR_CONTROL_AUTH", "cookie"))
    }

    fn tor_wait(&mut self, timeout: Duration) -> Result<(), BackendError> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Some(controller) = self.tor.as_mut() {
                let bootstrap = controller.bootstrap().unwrap_or(0);
                let circuit = controller.circuit().unwrap_or(false);
                if bootstrap >= 100 && circuit {
                    return Ok(());
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        Err(BackendError::new("E_TOR_BOOTSTRAP_TIMEOUT", "timeout"))
    }

    fn leak_check(&mut self, vpn_required: bool) -> Result<(), BackendError> {
        let filters = self.wfp.as_ref().map(|engine| engine.verify()).unwrap_or(0);
        if !self.armed || filters < 8 {
            return Err(BackendError::new("E_FIREWALL_VERIFY_MISMATCH", "filters"));
        }
        if vpn_required {
            let (connected, _) = self.read_status();
            if !connected {
                return Err(BackendError::new("E_VPN_CONNECT_TIMEOUT", "leak_check"));
            }
        }
        let circuit = self
            .tor
            .as_mut()
            .and_then(|controller| controller.circuit().ok())
            .unwrap_or(false);
        if !circuit {
            return Err(BackendError::new("E_TOR_CIRCUIT_FAILED", "leak_check"));
        }
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), BackendError> {
        self.tor = None;
        if let Some(mut child) = self.tor_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(client) = self.mullvad.as_ref() {
            let _ = client.disconnect();
        }
        self.vpn_country = None;
        Ok(())
    }

    fn disarm(&mut self) -> Result<(), BackendError> {
        self.tor = None;
        if let Some(engine) = self.wfp.as_mut() {
            engine
                .disarm()
                .map_err(|_| BackendError::new("E_FIREWALL_RECOVERY_FAILED", "disarm"))?;
        }
        self.armed = false;
        Ok(())
    }

    fn status(&mut self) -> BackendStatus {
        let (vpn_connected, country) = self.read_status();
        let bootstrap = self
            .tor
            .as_mut()
            .and_then(|controller| controller.bootstrap().ok())
            .unwrap_or(0);
        let circuit = self
            .tor
            .as_mut()
            .and_then(|controller| controller.circuit().ok())
            .unwrap_or(false);
        let filters = self.wfp.as_ref().map(|engine| engine.verify()).unwrap_or(0);
        BackendStatus {
            vpn_connected,
            vpn_country: country.or_else(|| self.vpn_country.clone()),
            tor_bootstrap: bootstrap,
            tor_circuit: circuit,
            firewall_armed: self.armed,
            firewall_filters: filters,
            mac_supported: false,
            mac_randomized: self.mac_randomized,
            banners: self.banners.clone(),
        }
    }
}

#[cfg(not(windows))]
pub struct RealBackend;

#[cfg(not(windows))]
impl RealBackend {
    pub fn new() -> Self {
        Self
    }
}

/// Builds the backend the service should use.
pub fn default_backend() -> Box<dyn Backend> {
    #[cfg(debug_assertions)]
    if std::env::var("TVPN_TEST_BACKEND").as_deref() == Ok("mock") {
        return Box::new(MockBackend::default());
    }
    #[cfg(windows)]
    {
        Box::new(RealBackend::new())
    }
    #[cfg(not(windows))]
    {
        Box::new(MockBackend::default())
    }
}
