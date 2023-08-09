use crate::process::{run_command_to_end, RunResult};
use handlebars::Handlebars;
use serde::Serialize;
use std::path::PathBuf;
use tokio::process::Command;

const SERVICE_NAME: &'static str = "youtube-dl-server";
const SERVICE_UNIT_FILENAME: &'static str = "youtube-dl-server.service";
const SERVICE_UNIT_TEMPLATE: &'static str = include_str!("youtube-dl-server.service");
const SERVICE_TARGET_DIR: &'static str = "/lib/systemd/system";

#[derive(Serialize)]
struct SystemdUnitData {
    binary_path: PathBuf,
    workdir_path: PathBuf,
    logging_scheme: &'static str,
}

fn get_full_unit_filename() -> PathBuf {
    PathBuf::from(SERVICE_TARGET_DIR).join(SERVICE_UNIT_FILENAME)
}

fn generate_unit_data(verbose: bool) -> SystemdUnitData {
    let binary_path = std::env::current_exe().unwrap();
    let workdir_path = std::env::current_dir().unwrap();
    let logging_scheme = if verbose { "--log-file" } else { "--log-none" };

    SystemdUnitData {
        binary_path,
        workdir_path,
        logging_scheme,
    }
}

fn write_service_file(verbose: bool) -> anyhow::Result<()> {
    let handlebars = Handlebars::new();
    let data = generate_unit_data(verbose);
    let rendered = handlebars.render_template(SERVICE_UNIT_TEMPLATE, &data)?;
    std::fs::write(get_full_unit_filename(), rendered)?;
    Ok(())
}

fn reload_services() -> anyhow::Result<()> {
    let result: RunResult = tokio::runtime::Runtime::new().unwrap().block_on(async {
        let mut command = Command::new("systemctl");
        command.arg("daemon-reload");
        run_command_to_end(command).await
    })?;

    if result.exit_code != Some(0) {
        eprintln!(
            "Failed to reload systemctl. Output:\n{:?}{:02X?}\n{:?}{:02X?}",
            String::from_utf8_lossy(&result.stdout),
            result.stdout,
            String::from_utf8_lossy(&result.stderr),
            result.stderr,
        );
    }
    Ok(())
}

fn enable_service() -> anyhow::Result<()> {
    let result: RunResult = tokio::runtime::Runtime::new().unwrap().block_on(async {
        let mut command = Command::new("systemctl");
        command.arg("enable");
        command.arg(SERVICE_NAME);
        run_command_to_end(command).await
    })?;

    if result.exit_code != Some(0) {
        eprintln!(
            "Failed to enable service '{SERVICE_NAME}'. Output:\n{:?}{:02X?}\n{:?}{:02X?}",
            String::from_utf8_lossy(&result.stdout),
            result.stdout,
            String::from_utf8_lossy(&result.stderr),
            result.stderr,
        );
    }
    Ok(())
}

fn disable_service() -> anyhow::Result<()> {
    let result: RunResult = tokio::runtime::Runtime::new().unwrap().block_on(async {
        let mut command = Command::new("systemctl");
        command.arg("disable");
        command.arg(SERVICE_NAME);
        run_command_to_end(command).await
    })?;

    if result.exit_code != Some(0) {
        eprintln!(
            "Failed to disable service '{SERVICE_NAME}'. Output:\n{:?}{:02X?}\n{:?}{:02X?}",
            String::from_utf8_lossy(&result.stdout),
            result.stdout,
            String::from_utf8_lossy(&result.stderr),
            result.stderr,
        );
    }
    Ok(())
}

fn remove_service_file() -> anyhow::Result<()> {
    std::fs::remove_file(get_full_unit_filename())?;
    Ok(())
}

pub fn install_service(verbose: bool) -> anyhow::Result<()> {
    write_service_file(verbose)?;
    reload_services()?;
    enable_service()?;
    println!("Successfully installed service '{SERVICE_NAME}'.");
    Ok(())
}

pub fn uninstall_service() -> anyhow::Result<()> {
    disable_service()?;
    remove_service_file()?;
    println!("Successfully uninstalled service '{SERVICE_NAME}'.");
    Ok(())
}
