mod capture;
mod external_shutdown;
mod generic_stop_handle;
mod kill_child;

#[cfg(windows)]
mod service_win;
#[cfg(windows)]
pub use service_win::*;

#[cfg(not(windows))]
mod service_linux;
#[cfg(not(windows))]
pub use service_linux::*;

pub use capture::*;
pub use external_shutdown::ExternalShutdownFairing;
pub use generic_stop_handle::GenericStopHandle;
pub use kill_child::kill_child_process;
