use crate::main_sync_wrapper;
///
/// Adapted from examples provided by `windows_service` crate.
/// https://github.com/mullvad/windows-service-rs
///
use crate::process::generic_stop_handle::GenericStopHandle;
use std::thread::sleep;
use std::time::Instant;
use std::{ffi::OsString, time::Duration};
use tracing::warn;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};
use windows_service::{
    service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType},
    service_manager::{ServiceManager, ServiceManagerAccess},
};
use windows_sys::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;

// Internal service name
const SERVICE_NAME: &str = "youtube-dl-server";
// Human-readable service name
const SERVICE_DISPLAY_NAME: &str = "Youtube-DL Server";
// Human-readable description
const SERVICE_DESCRIPTION: &str = "Downloads YouTube videos from submitted URLs.";
// Service will be run in its own process
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

fn control_event_handler(
    stop_handle: &GenericStopHandle,
    control_event: ServiceControl,
) -> ServiceControlHandlerResult {
    match control_event {
        // Notifies a service to report its current status information to the service
        // control manager. We always return NoError even if not implemented.
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

        ServiceControl::Stop => {
            warn!("Received service stop event.");
            stop_handle.stop();
            ServiceControlHandlerResult::NoError
        }

        _ => ServiceControlHandlerResult::NotImplemented,
    }
}

fn run_service_internal() -> Result<(), Box<dyn std::error::Error>> {
    // When running as a service, we must report to the SCM as soon as possible.
    // Initializing logging takes too much time, so we have to defer it.

    let stop_handle = GenericStopHandle::new();
    let stop_handle_copy = stop_handle.clone();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        control_event_handler(&stop_handle, control_event)
    };

    // Register system service event handler.
    // The returned status handle should be used to report service status changes to the system.
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    // Tell the system that service is running
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    let ret = main_sync_wrapper(Some(stop_handle_copy));

    // Tell the system that service has stopped.
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    ret
}

// Generate the windows service boilerplate.
// The boilerplate contains the low-level service entry function (ffi_service_main) that parses
// incoming service arguments into Vec<OsString> and passes them to user defined service
// entry (my_service_main).
define_windows_service!(ffi_service_main, my_service_main);

// Service entry function which is called on background thread by the system with service
// parameters. There is no stdout or stderr at this point so make sure to configure the log
// output to file if needed.
fn my_service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service_internal() {
        eprintln!("Runtime service failure: {e}");
    }
}

/// Run current executable as service
pub fn run_as_service() -> anyhow::Result<()> {
    // Register generated `ffi_service_main` with the system and start the service, blocking
    // this thread until the service is stopped.
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}

/// Install current executable as Windows service
pub fn install_service(verbose: bool) -> anyhow::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let current_binary_path = std::env::current_exe().unwrap();
    let workdir_path = std::env::current_dir().unwrap();

    let launch_arguments = vec![
        OsString::from(if verbose { "--log-file" } else { "--log-none" }),
        OsString::from("--workdir"),
        OsString::from(workdir_path),
        OsString::from("run"),
        OsString::from("--service"),
    ];

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: SERVICE_TYPE,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: current_binary_path,
        launch_arguments,
        dependencies: vec![],
        // Use least privileged access level for security reasons
        account_name: Some(OsString::from("NT AUTHORITY\\LocalService")),
        account_password: None,
    };
    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description(SERVICE_DESCRIPTION)?;
    println!("Successfully installed as '{SERVICE_NAME}' service.");
    Ok(())
}

/// Uninstall current executable from Windows services
pub fn uninstall_service() -> anyhow::Result<()> {
    println!("Uninstalling as service.  Please wait...");
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    // The service will be marked for deletion as long as this function call succeeds.
    // However, it will not be deleted from the database until it is stopped and all open handles to it are closed.
    service.delete()?;
    // Our handle to it is not closed yet. So we can still query it.
    if service.query_status()?.current_state != ServiceState::Stopped {
        // If the service cannot be stopped, it will be deleted when the system restarts.
        service.stop()?;
    }
    // Explicitly close our open handle to the service. This is automatically called when `service` goes out of scope.
    drop(service);

    // Win32 API does not give us a way to wait for service deletion.
    // To check if the service is deleted from the database, we have to poll it ourselves.
    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    while start.elapsed() < timeout {
        if let Err(windows_service::Error::Winapi(e)) =
            service_manager.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS)
        {
            if e.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) {
                println!("Service '{SERVICE_NAME}' does not exist.");
                return Ok(());
            }
        }
        sleep(Duration::from_secs(1));
    }
    println!("Service '{SERVICE_NAME}' is marked for deletion.");

    Ok(())
}
