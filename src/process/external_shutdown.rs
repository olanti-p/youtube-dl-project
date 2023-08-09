use crate::process::generic_stop_handle::GenericStopHandle;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Orbit, Rocket, Shutdown};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct ExternalShutdownFairing {
    external_handle: GenericStopHandle,
    internal_handle: GenericStopHandle,
    stop_handle: Arc<Mutex<Option<Shutdown>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl ExternalShutdownFairing {
    pub fn new(external_handle: GenericStopHandle, internal_handle: GenericStopHandle) -> Self {
        Self {
            external_handle,
            internal_handle,
            stop_handle: Default::default(),
            worker: Default::default(),
        }
    }
}

#[rocket::async_trait]
impl Fairing for ExternalShutdownFairing {
    fn info(&self) -> Info {
        Info {
            name: "ExternalShutdown",
            kind: Kind::Liftoff | Kind::Singleton,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let external_handle = self.external_handle.clone();
        let internal_handle = self.internal_handle.clone();
        let stop_handle = self.stop_handle.clone();
        *stop_handle.lock().await = Some(rocket.shutdown());
        *self.worker.lock().await = Some(rocket::tokio::task::spawn(async move {
            while !internal_handle.is_stopped() {
                if external_handle.is_stopped() {
                    internal_handle.stop();
                    let mut handle_ref = stop_handle.lock().await;
                    if let Some(shutdown) = handle_ref.take() {
                        shutdown.notify();
                    }
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }));
    }
}
