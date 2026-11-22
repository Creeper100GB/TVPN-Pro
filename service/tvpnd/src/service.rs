//! Windows service wrapper for the pipe server.

use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use windows_service::define_windows_service;
use windows_service::service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use windows_service::Result;

const SERVICE_NAME: &str = "tvpnd";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

/// Installs and starts the service for the current user.
///
/// The caller must be elevated. The current token's SID becomes the owner SID
/// so the unprivileged UI can read the session token from `%ProgramData%`.
pub fn install() -> Result<()> {
    let owner = crate::pipe::current_user_sid().map_err(windows_service::Error::Winapi)?;
    crate::pipe::save_owner_sid(&owner).map_err(windows_service::Error::Winapi)?;

    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )?;
    let info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from("TVPN Pro Service"),
        service_type: SERVICE_TYPE,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: std::env::current_exe().map_err(windows_service::Error::Winapi)?,
        launch_arguments: vec![OsString::from("--service")],
        dependencies: Vec::new(),
        account_name: None,
        account_password: None,
    };
    let service = manager.create_service(
        &info,
        ServiceAccess::CHANGE_CONFIG | ServiceAccess::QUERY_STATUS | ServiceAccess::START,
    )?;
    service.set_description("TVPN Pro network guard and local IPC service")?;
    let _ = service.start(&[] as &[OsString]);
    Ok(())
}

/// Stops and deletes the service.
pub fn uninstall() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(
        SERVICE_NAME,
        ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
    )?;
    let _ = service.stop();

    // DeleteService fails while the SCM still considers the service running.
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        match service.query_status() {
            Ok(status) if status.current_state == ServiceState::Stopped => break,
            _ => std::thread::sleep(Duration::from_millis(200)),
        }
    }
    service.delete()?;
    Ok(())
}

/// Registers with the service control manager and blocks until the service stops.
pub fn run() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

define_windows_service!(ffi_service_main, service_main);

fn service_main(_arguments: Vec<OsString>) {
    let _ = run_service();
}

fn run_service() -> Result<()> {
    let stop = Arc::new(AtomicBool::new(false));
    let handler_stop = Arc::clone(&stop);

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            ServiceControl::Stop => {
                handler_stop.store(true, Ordering::SeqCst);
                crate::pipe::wake();
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    let token = crate::pipe::generate_token();
    let result = crate::pipe::run_with_stop(&token, Arc::clone(&stop));
    let exit_code = if result.is_ok() {
        ServiceExitCode::Win32(0)
    } else {
        ServiceExitCode::ServiceSpecific(1)
    };

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code,
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}
