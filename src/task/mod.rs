use crate::download_manager::WorkerControlHandle;
use crate::env::YtdlpConfig;
use crate::filesystem::FilesystemDriver;
use crate::job::Job;
use crate::playlist::{VideoInfo, VideoOrPlaylist};
use chrono::{DateTime, Utc};
use fetch_url_contents::run_task_fetch_url_contents;
use rocket::serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

mod download_and_convert;
mod fetch_url_contents;

pub struct TaskResult {
    pub task: Task,
    pub status: TaskStatus,
    pub data: Option<TaskResultData>,
}

#[derive(Debug)]
pub enum TaskResultData {
    DownloadAndConvert,
    FetchUrlContents(VideoOrPlaylist),
}

pub async fn run_task(
    task: Task,
    ytdlp: Arc<YtdlpConfig>,
    fs: Arc<FilesystemDriver>,
    progress: Arc<Mutex<TaskProgress>>,
    control_handle: WorkerControlHandle,
) -> anyhow::Result<TaskResultData> {
    match task.kind {
        TaskKind::FetchUrlContents => {
            run_task_fetch_url_contents(task, ytdlp, fs, progress, control_handle).await
        }
        TaskKind::DownloadAndConvert => {
            download_and_convert::run_task_download_and_convert(
                task,
                ytdlp,
                fs,
                progress,
                control_handle,
            )
            .await
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq)]
pub enum TaskStatus {
    Waiting,
    Processing,
    Done,
    Paused,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq)]
pub enum TaskKind {
    FetchUrlContents,
    DownloadAndConvert,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct Task {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub kind: TaskKind,
    pub thumbnail: String,
    pub owner_job_id: Uuid,
    pub url: String,
    pub format: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub prioritized: bool,
    pub is_resumed: bool,
    pub pending_cleanup: bool,
    pub pending_delete: bool,
    pub title: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskProgress {
    pub percent: i32,
    pub bytes_estimate: i32,
    pub bytes_downloaded: i32,
}

impl Task {
    pub fn new_from_video_info(owner_job_id: Uuid, video: &VideoInfo, format: String) -> Self {
        let created_at = Utc::now();
        let task_id = Uuid::new_v4();
        Self {
            task_id,
            status: TaskStatus::Waiting,
            kind: TaskKind::DownloadAndConvert,
            thumbnail: video
                .thumbnail
                .as_ref()
                .map_or_else(String::default, |x| x.to_string()),
            owner_job_id,
            url: video.url.clone(),
            format,
            created_at,
            started_at: None,
            finished_at: None,
            prioritized: false,
            is_resumed: false,
            pending_delete: false,
            pending_cleanup: false,
            title: video.title.clone(),
        }
    }

    pub fn new_fetch_url_contents(job: &Job) -> Self {
        let created_at = Utc::now();
        let task_id = Uuid::new_v4();
        Self {
            task_id,
            status: TaskStatus::Waiting,
            kind: TaskKind::FetchUrlContents,
            thumbnail: job.thumbnail.clone(),
            owner_job_id: job.job_id,
            url: job.url.clone(),
            format: job.format.clone(),
            created_at,
            started_at: None,
            finished_at: None,
            prioritized: false,
            is_resumed: false,
            pending_delete: false,
            pending_cleanup: false,
            title: "[Fetch Contents]".to_string(),
        }
    }
}
