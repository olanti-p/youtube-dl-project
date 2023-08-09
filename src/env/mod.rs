use clap::Parser;
use directories::{ProjectDirs, UserDirs};
use std::path::PathBuf;
use std::sync::Arc;

mod cli;
mod config;
mod config_trait;
mod paths;
mod ytdlp;

use crate::env::config_trait::ConfigTrait;
pub use cli::{Cli, CliCommand};
pub use config::Config;
pub use paths::Paths;
pub use ytdlp::{DownloadFormat, YtdlpConfig};

#[derive(Debug)]
pub struct EnvironmentManager {
    pub cli: Cli,
    pub config: Config,
    pub ytdlp: Arc<YtdlpConfig>,
    pub dev_mode: bool,
    pub paths: Paths,
}

impl EnvironmentManager {
    pub async fn init() -> Self {
        Self::init_from_cli(Cli::parse()).await
    }

    async fn init_from_cli(cli: Cli) -> Self {
        let dev_mode = cli.dev_mode;

        let server_config_file = Self::get_server_config_file(dev_mode);
        let default_userconf_download_dir = Self::get_default_userconf_download_dir(dev_mode);
        let default_userconf_temp_dir = Self::get_default_userconf_temp_dir(dev_mode);
        let config_default = Config::new(default_userconf_download_dir, default_userconf_temp_dir);
        let config = Config::init(&server_config_file, config_default).await;

        let ytdlp_config_file = Self::get_ytdlp_config_file(dev_mode);
        let ytdlp_default = YtdlpConfig::new();
        let ytdlp = Arc::new(YtdlpConfig::init(&ytdlp_config_file, ytdlp_default).await);

        let paths = Paths {
            database_file: Self::get_database_file(dev_mode),
            server_config_file,
            ytdlp_config_file,
            worker_dir: Self::get_worker_dir(&config, dev_mode),
            logs_dir: Self::get_logs_dir(&config, dev_mode),
            output_dir: Self::get_output_dir(&config),
        };
        Self {
            cli,
            config,
            ytdlp,
            dev_mode,
            paths,
        }
    }

    fn get_project_dirs() -> Option<ProjectDirs> {
        ProjectDirs::from("com", "youtube-dl-server", "youtube-dl-server")
    }

    fn get_config_dir(dev_mode: bool) -> PathBuf {
        if dev_mode {
            PathBuf::from("debug/config/")
        } else {
            Self::get_project_dirs().unwrap().config_dir().to_path_buf()
        }
    }

    fn get_server_config_file(dev_mode: bool) -> PathBuf {
        Self::get_config_dir(dev_mode).join("server.yaml")
    }

    fn get_ytdlp_config_file(dev_mode: bool) -> PathBuf {
        Self::get_config_dir(dev_mode).join("ytdlp.yaml")
    }

    fn get_default_userconf_download_dir(dev_mode: bool) -> PathBuf {
        if dev_mode {
            PathBuf::from("download")
        } else {
            UserDirs::new()
                .unwrap()
                .download_dir()
                .unwrap()
                .to_path_buf()
        }
    }

    fn get_default_userconf_temp_dir(dev_mode: bool) -> PathBuf {
        if dev_mode {
            PathBuf::from("temp")
        } else {
            Self::get_project_dirs().unwrap().data_dir().to_path_buf()
        }
    }

    fn get_output_dir(config: &Config) -> PathBuf {
        config.download_folder.clone()
    }

    fn get_logs_dir(config: &Config, dev_mode: bool) -> PathBuf {
        if dev_mode {
            config.temp_folder.join("debug/logs")
        } else {
            Self::get_project_dirs().unwrap().cache_dir().join("logs")
        }
    }

    fn get_worker_dir(config: &Config, dev_mode: bool) -> PathBuf {
        if dev_mode {
            config.temp_folder.join("workers")
        } else {
            // We don't want to directly use temp_folder supplied by the user
            // because at one moment we may decide to empty it completely,
            // and if the user supplied a non-empty folder here
            // that means we'd be deleting user files. So, add a safety layer.
            config.temp_folder.join("Youtube-DL In-Progress")
        }
    }

    fn get_database_dir(dev_mode: bool) -> PathBuf {
        if dev_mode {
            PathBuf::from("debug/db")
        } else {
            Self::get_project_dirs().unwrap().data_dir().join("db")
        }
    }

    fn get_database_file(dev_mode: bool) -> PathBuf {
        Self::get_database_dir(dev_mode).join("state.db")
    }

    pub async fn check_config_validity(new_config: &Config) -> bool {
        Config::check_validity(new_config).await
    }

    pub async fn save_config(&self, new_config: &Config) -> anyhow::Result<Self> {
        new_config.save(&self.paths.server_config_file).await?;

        let new_env = Self::init_from_cli(self.cli.clone()).await;

        let old_worker_dir = &self.paths.worker_dir;
        let new_worker_dir = &new_env.paths.worker_dir;

        if old_worker_dir != new_worker_dir {
            tokio::fs::remove_dir_all(old_worker_dir).await?;
        }

        Ok(new_env)
    }
}
