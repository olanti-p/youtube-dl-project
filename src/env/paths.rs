use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Paths {
    pub database_file: PathBuf,
    pub server_config_file: PathBuf,
    pub ytdlp_config_file: PathBuf,
    pub worker_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub output_dir: PathBuf,
}
