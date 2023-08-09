use crate::env::EnvironmentManager;
use crate::task::Task;
use anyhow::Context;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::File;
use uuid::Uuid;

const MAIN_FILE_NAME: &'static str = "main";
const WORKER_DATA_DIR: &'static str = "data";
const WORKER_LOG_DIR: &'static str = "log";
const WORKER_STDOUT_FILE: &'static str = "stdout.log";
const WORKER_STDERR_FILE: &'static str = "stderr.log";

#[derive(Debug)]
pub struct FilesystemDriver {
    env: Arc<EnvironmentManager>,
}

impl FilesystemDriver {
    pub fn new(env: Arc<EnvironmentManager>) -> Self {
        Self { env }
    }

    pub async fn init_directories(&self) -> anyhow::Result<()> {
        ensure_writable_dir_exists(&self.env.paths.worker_dir).await?;
        ensure_writable_dir_exists(&self.env.paths.output_dir).await?;
        ensure_writable_dir_exists(&self.env.paths.database_file.parent().unwrap()).await?;
        Ok(())
    }

    pub fn get_database_file(&self) -> &Path {
        &self.env.paths.database_file
    }

    pub async fn remove_worker_root_dir(&self, task_id: Uuid) -> anyhow::Result<()> {
        let path = self.get_worker_root_dir_for_task(task_id);
        tokio::fs::remove_dir_all(&path).await?;
        Ok(())
    }

    fn get_worker_root_dir_for_task(&self, task_id: Uuid) -> PathBuf {
        self.env.paths.worker_dir.join(task_id.to_string())
    }

    fn get_worker_data_dir_for_task(&self, task_id: Uuid) -> PathBuf {
        self.get_worker_root_dir_for_task(task_id)
            .join(WORKER_DATA_DIR)
    }

    fn get_worker_log_dir_for_task(&self, task_id: Uuid) -> PathBuf {
        self.get_worker_root_dir_for_task(task_id)
            .join(WORKER_LOG_DIR)
    }

    pub async fn create_worker_data_dir(&self, task_id: Uuid) -> anyhow::Result<()> {
        let path = self.get_worker_data_dir_for_task(task_id);
        tokio::fs::create_dir_all(&path).await?;
        Ok(())
    }

    pub async fn create_worker_log_dir(&self, task_id: Uuid) -> anyhow::Result<()> {
        let path = self.get_worker_log_dir_for_task(task_id);
        tokio::fs::create_dir_all(&path).await?;
        Ok(())
    }

    pub async fn remove_worker_data_dir(&self, task_id: Uuid) -> anyhow::Result<()> {
        let path = self.get_worker_data_dir_for_task(task_id);
        tokio::fs::remove_dir_all(&path).await?;
        Ok(())
    }

    pub async fn remove_worker_output_file(&self, task: &Task) -> anyhow::Result<()> {
        let path = self.get_ytdlp_output_file(task);
        tokio::fs::remove_file(&path).await?;
        Ok(())
    }

    pub fn get_ytdlp_output_template(&self, task_id: Uuid) -> PathBuf {
        self.get_worker_data_dir_for_task(task_id)
            .join(format!("{MAIN_FILE_NAME}.%(ext)s"))
    }

    pub fn get_ytdlp_output_file(&self, task: &Task) -> PathBuf {
        let format = self.env.ytdlp.get_format(&task.format);
        self.get_worker_data_dir_for_task(task.task_id)
            .join(format!("{MAIN_FILE_NAME}.{}", format.ext))
    }

    pub fn get_ytdlp_stdout_file(&self, task_id: Uuid) -> PathBuf {
        self.get_worker_log_dir_for_task(task_id)
            .join(WORKER_STDOUT_FILE)
    }

    pub fn get_ytdlp_stderr_file(&self, task_id: Uuid) -> PathBuf {
        self.get_worker_log_dir_for_task(task_id)
            .join(WORKER_STDERR_FILE)
    }

    pub async fn move_output_file(&self, task: &Task) -> anyhow::Result<()> {
        let filename_unsafe: &str = if task.title.is_empty() {
            &task.url
        } else {
            &task.title
        };
        let filename = filenamify::filenamify(filename_unsafe);

        let source_path = self.get_ytdlp_output_file(task);
        let source_ext = source_path
            .extension()
            .context("expected file produced by yt_dlp to have extension")?
            .to_str()
            .context("expected file produced by yt_dlp to have extension expressed in UTF-8")?;

        let destination_path_unsafe = self.env.paths.output_dir.join(filename + "." + source_ext);
        let destination_path = pick_free_file_name(&destination_path_unsafe).await;

        tokio::fs::rename(source_path, destination_path).await?;

        Ok(())
    }

    pub async fn make_log_files(&self, task_id: Uuid) -> anyhow::Result<(File, File)> {
        let stdout_file = {
            let path = self.get_ytdlp_stdout_file(task_id);
            let mut opts = tokio::fs::OpenOptions::new();
            opts.create(true).append(true);
            opts.open(path).await?
        };
        let stderr_file = {
            let path = self.get_ytdlp_stderr_file(task_id);
            let mut opts = tokio::fs::OpenOptions::new();
            opts.create(true).append(true);
            opts.open(path).await?
        };
        Ok((stdout_file, stderr_file))
    }
}

pub async fn pick_free_file_name(original_path: &Path) -> PathBuf {
    let original_file_stem = original_path
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap()
        .to_owned();
    let original_file_ext = original_path
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap()
        .to_owned();
    let mut i = 0;
    loop {
        let mut candidate = original_path.to_owned();
        if i != 0 {
            candidate.set_file_name(&format!("{original_file_stem} ({i}).{original_file_ext}"));
        }
        match tokio::fs::try_exists(&candidate).await {
            Ok(true) => {
                i = i + 1;
            }
            _ => return candidate,
        }
    }
}

pub async fn ensure_writable_dir_exists(path: &Path) -> anyhow::Result<()> {
    if tokio::fs::metadata(path).await.is_err() {
        // Path does not exist, is ill-formed OR we don't have permissions.
        // In the latter cases, simple way to check is to create the folder.
        tokio::fs::create_dir_all(path).await?;
    }
    Ok(())
}

pub async fn path_exists(path: &Path) -> bool {
    matches!(tokio::fs::try_exists(path).await, Ok(true))
}
