use crate::env::config_trait::ConfigTrait;
use crate::filesystem::ensure_writable_dir_exists;
use rocket::async_trait;
use rocket::serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::warn;

pub const MAX_AUTOMATIC_RETRIES: u32 = 50;
pub const MAX_DOWNLOAD_WORKERS: u32 = 32;
pub const MAX_RETRY_TIMEOUT: u32 = 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub download_folder: PathBuf,
    pub temp_folder: PathBuf,
    pub start_with_os: bool,
    pub show_announcements: bool,
    pub num_automatic_retries: u32,
    pub timeout_before_retry: u32,
    pub num_download_workers: u32,
}

impl Config {
    pub(super) fn new(download_folder: PathBuf, temp_folder: PathBuf) -> Self {
        let mut default_cfg: Self = serde_yaml::from_str(include_str!("config.yaml")).unwrap();

        default_cfg.download_folder = download_folder;
        default_cfg.temp_folder = temp_folder;

        default_cfg
    }
}

#[async_trait]
impl ConfigTrait for Config {
    async fn check_validity(config: &Config) -> bool {
        if config.num_download_workers > MAX_DOWNLOAD_WORKERS {
            warn!("Rejecting config: num_download_workers = {} exceeds hardcoded limit {MAX_DOWNLOAD_WORKERS}", config.num_download_workers);
            return false;
        }
        if config.num_automatic_retries > MAX_AUTOMATIC_RETRIES {
            warn!("Rejecting config: num_automatic_retries = {} exceeds hardcoded limit {MAX_AUTOMATIC_RETRIES}", config.num_automatic_retries);
            return false;
        }
        if config.timeout_before_retry > MAX_RETRY_TIMEOUT {
            warn!("Rejecting config: timeout_before_retry = {} exceeds hardcoded limit {MAX_RETRY_TIMEOUT}", config.timeout_before_retry);
            return false;
        }
        if let Err(e) = ensure_writable_dir_exists(&config.download_folder).await {
            warn!(
                "Rejecting config: cannot access folder \"{}\": {e}",
                config.download_folder.to_string_lossy()
            );
            return false;
        }
        if let Err(e) = ensure_writable_dir_exists(&config.temp_folder).await {
            warn!(
                "Rejecting config: cannot access folder \"{}\": {e}",
                config.temp_folder.to_string_lossy()
            );
            return false;
        }
        true
    }
}
