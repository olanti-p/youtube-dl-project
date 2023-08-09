use crate::announce::AnnounceSystem;
use crate::database::{ServerDatabase, TaskStats};
use crate::download_manager::DownloadManager;
use crate::env::EnvironmentManager;
use crate::filesystem::FilesystemDriver;
use crate::job::Job;
use crate::process::GenericStopHandle;
use crate::queue_command::QueueCommand;
use crate::task::{Task, TaskKind, TaskResult, TaskResultData, TaskStatus};
use crate::web_api::NewJobForm;
use dirty_marker::DirtyMarker;
pub use handle::JobManagerHandle;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, MutexGuard};
use tracing::{info, warn};
use uuid::Uuid;

mod dirty_marker;
mod handle;

#[derive(Debug)]
pub struct JobManager {
    announcements: Arc<AnnounceSystem>,
    db: Arc<Mutex<ServerDatabase>>,
    dload_manager: (Mutex<DownloadManager>,),
    stop_handle: GenericStopHandle,
    jobs_dirty: DirtyMarker,
    cleanup_dirty: DirtyMarker,
}

impl JobManager {
    pub async fn new(
        announcements: Arc<AnnounceSystem>,
        env: Arc<EnvironmentManager>,
        fs: Arc<FilesystemDriver>,
        db: Arc<Mutex<ServerDatabase>>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            announcements,
            db,
            dload_manager: (Mutex::new(DownloadManager::new(env.clone(), fs)),),
            stop_handle: GenericStopHandle::new(),
            jobs_dirty: DirtyMarker::new(),
            cleanup_dirty: DirtyMarker::new(),
        })
    }

    async fn lock_downloads(&self) -> (MutexGuard<DownloadManager>, MutexGuard<ServerDatabase>) {
        let db_lock = self.db.lock().await;
        let dm_lock = self.dload_manager.0.lock().await;
        (dm_lock, db_lock)
    }

    pub async fn create_job(&self, data: &NewJobForm) -> anyhow::Result<Job> {
        let db_lock = self.db.lock().await;

        let ret = db_lock.create_job(data).await;
        self.mark_dirty();
        ret
    }

    pub async fn modify_all_jobs(&self, command: QueueCommand) -> anyhow::Result<()> {
        let (mut dload_manager, db_lock) = self.lock_downloads().await;

        dload_manager.modify_all_tasks(&command);
        db_lock.modify_all_jobs(command).await?;

        self.mark_dirty();
        Ok(())
    }

    pub async fn modify_job(&self, id: Uuid, command: QueueCommand) -> anyhow::Result<()> {
        let (mut dload_manager, db_lock) = self.lock_downloads().await;

        dload_manager.modify_tasks_by_job(id, &command);
        db_lock.modify_job(id, command).await?;

        self.mark_dirty();
        Ok(())
    }

    pub async fn modify_task(&self, id: Uuid, command: QueueCommand) -> anyhow::Result<()> {
        let (mut dload_manager, db_lock) = self.lock_downloads().await;

        dload_manager.modify_task(id, &command);
        db_lock.modify_task(id, command).await?;

        self.mark_dirty();
        Ok(())
    }

    async fn populate_job_progress(dload_manager: &DownloadManager, job: &mut Job) {
        for task in &job.tasks {
            if task.status != TaskStatus::Processing {
                continue;
            }
            if let Some(progress) = dload_manager.get_worker_progress(task.task_id).await {
                job.progress.insert(task.task_id, progress);
            }
        }
    }

    pub async fn get_job(&self, id: Uuid) -> anyhow::Result<Job> {
        let (dload_manager, db_lock) = self.lock_downloads().await;

        let mut job = db_lock.get_job(id).await?;
        Self::populate_job_progress(&dload_manager, &mut job).await;
        Ok(job)
    }

    pub async fn get_all_jobs(&self) -> anyhow::Result<Vec<Job>> {
        let (dload_manager, db_lock) = self.lock_downloads().await;

        let mut jobs = db_lock.get_all_jobs().await?;
        for job in &mut jobs {
            Self::populate_job_progress(&dload_manager, job).await;
        }
        Ok(jobs)
    }

    pub async fn get_overall_stats(&self) -> anyhow::Result<TaskStats> {
        let db_lock = self.db.lock().await;
        db_lock.get_global_task_stats().await
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        info!("Started job manager.");
        let mut do_stop = false;
        let mut did_send_stop_signals = false;
        while !do_stop {
            rocket::tokio::time::sleep(Duration::from_millis(100)).await;

            do_stop = self.stop_handle.is_stopped();

            if do_stop && !did_send_stop_signals {
                did_send_stop_signals = true;
                self.modify_all_jobs(QueueCommand::Cancel).await?;
            }

            if self.cleanup_dirty.is_dirty() && !self.poll_pending_operations().await? {
                self.cleanup_dirty.mark_clean();
            }

            if self.poll_done().await? {
                self.cleanup_dirty.mark_dirty();
                self.jobs_dirty.mark_dirty();
            }

            if !do_stop && self.jobs_dirty.is_dirty() {
                self.poll_start().await?;
                self.jobs_dirty.mark_clean();
            }
        }
        info!("Shutdown job manager.");
        Ok(())
    }

    async fn handle_result_fetch_url_contents(&self, result: TaskResult) -> anyhow::Result<()> {
        let format = result.task.format;
        let content = if let Some(TaskResultData::FetchUrlContents(content)) = result.data {
            content
        } else {
            return Ok(());
        };

        if content.is_empty() {
            self.announcements
                .on_contents_empty(result.task.owner_job_id)
                .await?;
            return Ok(());
        }

        let job_id = result.task.owner_job_id;
        let job_title = content.title().to_string();
        let job_thumbnail = content.thumbnail().unwrap_or_default().to_string();

        let new_tasks: Vec<Task> = content
            .videos()
            .into_iter()
            .map(|video| Task::new_from_video_info(job_id, video, format.clone()))
            .collect();

        let command = QueueCommand::JobUpdated {
            job_title,
            job_thumbnail,
            new_tasks,
        };

        self.db.lock().await.modify_job(job_id, command).await?;

        Ok(())
    }

    async fn handle_task_result(&self, result: TaskResult) -> anyhow::Result<()> {
        self.announcements.on_task_result(&result).await?;

        if result.task.kind == TaskKind::FetchUrlContents {
            self.handle_result_fetch_url_contents(result).await?;
        }

        Ok(())
    }

    // Returns `true` if there is potentially more work to be done.
    async fn poll_pending_operations(&self) -> anyhow::Result<bool> {
        // We must be mindful not to lock DB *and then simultaneously*
        // download manager in case webapi decides to modify some entries,
        // and locks download manager *and then simultaneously* DB.
        //
        // So, we first lock db, then release it, then lock down_man and db again.
        let pending_ops = {
            let db_lock = self.db.lock().await;
            db_lock.get_pending_operations().await?
        };
        if !pending_ops.is_empty() {
            let (mut dload_manager, db_lock) = self.lock_downloads().await;

            for task in &pending_ops.cleanup {
                warn!("Cleaning up after {task}...");
                dload_manager.clean_up_after_worker(*task).await;
            }
            db_lock.confirm_cleanup(&pending_ops.cleanup).await?;
            db_lock.confirm_deletion(&pending_ops.delete).await?;
        }
        Ok(pending_ops.num_busy > 0)
    }

    async fn collect_finished_tasks(&self) -> anyhow::Result<Vec<TaskResult>> {
        let (mut dload_manager, db_lock) = self.lock_downloads().await;
        let task_results = dload_manager.poll_done().await;
        for task_result in &task_results {
            let db_command = QueueCommand::TaskStatusChange(task_result.status);
            db_lock
                .modify_task(task_result.task.task_id, db_command)
                .await?;
        }
        Ok(task_results)
    }

    async fn handle_task_results(&self, results: Vec<TaskResult>) -> anyhow::Result<()> {
        for result in results {
            self.handle_task_result(result).await?;
        }
        Ok(())
    }

    async fn poll_done(&self) -> anyhow::Result<bool> {
        let task_results = self.collect_finished_tasks().await?;
        if task_results.is_empty() {
            Ok(false)
        } else {
            self.handle_task_results(task_results).await?;
            Ok(true)
        }
    }

    async fn poll_start(&self) -> anyhow::Result<()> {
        let (mut dload_manager, db_lock) = self.lock_downloads().await;

        let free_slots = dload_manager.num_free_workers();
        let tasks = db_lock.acquire_tasks(free_slots).await?;
        if tasks.is_empty() {
            return Ok(());
        }
        for task in tasks {
            dload_manager.start_task(task);
        }

        Ok(())
    }

    pub fn get_stop_handle(&self) -> GenericStopHandle {
        self.stop_handle.clone()
    }

    fn mark_dirty(&self) {
        self.jobs_dirty.mark_dirty();
        self.cleanup_dirty.mark_dirty();
    }
}
