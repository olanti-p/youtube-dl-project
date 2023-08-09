#![feature(async_closure)]

mod announce;
mod auth;
mod database;
mod download_manager;
mod env;
mod exit_status;
mod filesystem;
mod job;
mod job_manager;
mod playlist;
mod process;
mod queue_command;
mod run_server;
mod task;
mod web_api;

#[macro_use]
extern crate anyhow;
extern crate rocket;

use crate::database::ServerDatabase;
use crate::env::EnvironmentManager;
use crate::filesystem::FilesystemDriver;
use auth::User;
use clap::Parser;
use env::{Cli, CliCommand};
use exit_status::ExitStatus;
use process::GenericStopHandle;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

async fn async_main(
    service_stop_handle: Option<GenericStopHandle>,
) -> Result<(), Box<dyn std::error::Error>> {
    let env = Arc::new(EnvironmentManager::init().await);
    init_logging(&env);

    if let CliCommand::GetToken = env.cli.command {
        let fs = Arc::new(FilesystemDriver::new(env.clone()));
        let db = ServerDatabase::open(&fs).await?;
        let user = db.get_user_by_name(User::admin_user_name()).await?.unwrap();
        println!("{}", user.get_api_token());
    } else if let CliCommand::Run { .. } = env.cli.command {
        let mut env = env;
        loop {
            let ret = run_server::run_server(env.clone(), service_stop_handle.clone()).await?;
            if let Some(ExitStatus::ChangeConfig(config)) = ret {
                env = Arc::new(env.save_config(&config).await?);
                info!("Config changed!  Waiting 5 seconds for the OS to free up the socket...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            } else {
                break;
            }
        }
    }

    Ok(())
}

fn main_sync_wrapper(
    service_stop_handle: Option<GenericStopHandle>,
) -> Result<(), Box<dyn std::error::Error>> {
    rocket::async_main(async_main(service_stop_handle))?;
    Ok(())
}

fn init_logging(env: &EnvironmentManager) {
    if env.cli.log_file {
        let logger = tracing_appender::rolling::hourly(&env.paths.logs_dir, "server.log");
        tracing_subscriber::fmt()
            .with_writer(logger)
            .with_ansi(false) // Don't print colors to file
            .init();
    } else if env.cli.log_none {
        // Write into /dev/null
        tracing_subscriber::fmt()
            .with_writer(|| std::io::sink())
            .init();
    } else {
        tracing_subscriber::fmt::init();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if let Some(workdir) = cli.workdir {
        std::env::set_current_dir(workdir)?;
    }

    match cli.command {
        #[allow(unused_variables)]
        CliCommand::Run { service } => {
            #[cfg(windows)]
            {
                if service {
                    process::run_as_service()?;
                } else {
                    main_sync_wrapper(None)?;
                }
            }
            #[cfg(not(windows))]
            {
                main_sync_wrapper(None)?;
            }
        }
        CliCommand::GetToken => {
            main_sync_wrapper(None)?;
        }
        #[allow(unused_variables)]
        CliCommand::InstallService { verbose } => {
            process::install_service(verbose)?;
        }
        CliCommand::UninstallService => {
            process::uninstall_service()?;
        }
    }
    Ok(())
}
