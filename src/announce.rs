use crate::database::ServerDatabase;
use crate::job::{Job, JobStatus};
use crate::task::{TaskKind, TaskResult, TaskStatus};
use notify_rust::Notification;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug)]
pub struct AnnounceSystem {
    enabled: bool,
    db: Arc<Mutex<ServerDatabase>>,
}

impl AnnounceSystem {
    pub fn new(enabled: bool, db: Arc<Mutex<ServerDatabase>>) -> Self {
        Self { enabled, db }
    }

    pub async fn on_task_result(&self, task_result: &TaskResult) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }
        if task_result.task.kind == TaskKind::DownloadAndConvert {
            if self
                .check_needs_completion_announcement(task_result.task.owner_job_id)
                .await?
            {
                self.show_completion_announcement(task_result.task.owner_job_id)
                    .await?;
            }
        } else if task_result.status == TaskStatus::Failed {
            self.show_urlfetch_failed_announcement(task_result.task.owner_job_id)
                .await?;
        }

        Ok(())
    }

    async fn check_needs_completion_announcement(&self, job_id: Uuid) -> anyhow::Result<bool> {
        let stats = self.db.lock().await.get_job_task_stats(job_id).await?;
        Ok(stats.num_total > 1 && stats.num_waiting == 0 && stats.num_active == 0)
    }

    async fn show_completion_announcement(&self, job_id: Uuid) -> anyhow::Result<()> {
        let job: Job = self.db.lock().await.get_job(job_id).await?;
        tokio::task::spawn_blocking(move || {
            let text = match job.status {
                JobStatus::Done => "download complete",
                JobStatus::PartiallyDone => "download partially complete",
                JobStatus::Paused => "download paused",
                JobStatus::Failed => "download failed",
                JobStatus::Cancelled => "download cancelled",
                _ => "ERROR_STATUS",
            };
            Notification::new()
                .summary(&format!("YouTube {text}"))
                .body(&job.title)
                .show()
                .unwrap();
        });
        Ok(())
    }

    async fn show_urlfetch_failed_announcement(&self, job_id: Uuid) -> anyhow::Result<()> {
        let job: Job = self.db.lock().await.get_job(job_id).await?;
        tokio::task::spawn_blocking(move || {
            let text = "Failed to fetch video data";
            Notification::new()
                .summary(text)
                .body(&job.url)
                .show()
                .unwrap();
        });
        Ok(())
    }

    async fn show_no_avail_videos_announcement(&self, job_id: Uuid) -> anyhow::Result<()> {
        let job: Job = self.db.lock().await.get_job(job_id).await?;
        tokio::task::spawn_blocking(move || {
            let text = "No videos available.";
            Notification::new()
                .summary(text)
                .body(&job.url)
                .show()
                .unwrap();
        });
        Ok(())
    }

    pub async fn on_contents_empty(&self, job_id: Uuid) -> anyhow::Result<()> {
        if self.enabled {
            self.show_no_avail_videos_announcement(job_id).await?;
        }
        Ok(())
    }
}
