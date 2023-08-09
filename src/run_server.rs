use crate::announce::AnnounceSystem;
use crate::auth::{AuthFairing, AuthSystem};
use crate::database::ServerDatabase;
use crate::env::{CliCommand, EnvironmentManager};
use crate::exit_status::{ExitStatus, ExitStatusHandle};
use crate::filesystem::FilesystemDriver;
use crate::job_manager::JobManagerHandle;
use crate::process::{ExternalShutdownFairing, GenericStopHandle};
use crate::web_api::{get_api_routes, get_index_html_redirect};
use rocket::fs::FileServer;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

pub async fn run_server(
    env: Arc<EnvironmentManager>,
    service_stop_handle: Option<GenericStopHandle>,
) -> anyhow::Result<Option<ExitStatus>> {
    if let CliCommand::Run { service: true } = env.cli.command {
        info!("Running as a service.");
    }
    info!("Current env: {:#?}", env);

    let fs = Arc::new(FilesystemDriver::new(env.clone()));
    fs.init_directories().await?;

    let db = Arc::new(Mutex::new(ServerDatabase::open(&fs).await?));
    let auth_fairing = AuthFairing::new(db.clone());
    let auth_system = AuthSystem::new(db.clone());
    let announcements = Arc::new(AnnounceSystem::new(
        env.config.show_announcements,
        db.clone(),
    ));
    let job_manager =
        JobManagerHandle::new(announcements, env.clone(), fs.clone(), db.clone()).await?;

    let external_shutdown_monitor = {
        let internal_handle = job_manager.get_stop_handle();
        let external_handle = service_stop_handle.unwrap_or_default();
        ExternalShutdownFairing::new(external_handle, internal_handle)
    };

    let job_manager_join_handle = {
        let job_manager = job_manager.clone();
        rocket::tokio::task::spawn(async move {
            job_manager.run().await.unwrap();
        })
    };

    let job_manager_stop_handle = job_manager.get_stop_handle();

    let exit_state = ExitStatusHandle::new();
    let _rocket = rocket::build()
        .attach(auth_fairing)
        .attach(external_shutdown_monitor)
        .mount("/api", get_api_routes())
        .mount("/", get_index_html_redirect())
        .mount("/", FileServer::from("webui/"))
        .manage(auth_system)
        .manage(job_manager)
        .manage(env)
        .manage(exit_state.clone())
        .manage(fs)
        .launch()
        .await?;

    job_manager_stop_handle.stop();
    job_manager_join_handle.await.unwrap();

    Ok(exit_state.take().await)
}
