use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Set working directory
    #[arg(short, long)]
    pub workdir: Option<PathBuf>,

    /// Run in dev mode. Will use current working directory for internal files and output.
    #[arg(long)]
    pub dev_mode: bool,

    /// Log to file
    #[arg(long)]
    #[clap(group = "logging")]
    pub log_file: bool,

    /// Disable logging
    #[arg(long)]
    #[clap(group = "logging")]
    pub log_none: bool,

    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Subcommand, Debug, Clone, Copy)]
pub enum CliCommand {
    /// Run server
    Run {
        /// Run as Windows service (Windows only)
        #[arg(long)]
        service: bool,
    },
    /// Print API token and exit
    GetToken,
    /// Install itself as Windows service
    InstallService {
        /// As a service, write log files
        #[arg(long)]
        verbose: bool,
    },
    /// Uninstall itself as Windows service
    UninstallService,
}
