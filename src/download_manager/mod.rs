use crate::env::EnvironmentManager;
use crate::filesystem::FilesystemDriver;
use crate::queue_command::QueueCommand;
use crate::task::{run_task, Task, TaskProgress, TaskResult, TaskResultData, TaskStatus};
use command::WorkerError;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{error, info};
use uuid::Uuid;
use worker::DownloadWorker;
pub use worker_handle::WorkerControlHandle;

pub mod command;
mod worker;
mod worker_handle;

#[derive(Debug)]
pub struct DownloadManager {
    workers: Vec<DownloadWorker>,
    progress_by_id: HashMap<Uuid, Arc<Mutex<TaskProgress>>>,
    handle_by_task_id: HashMap<Uuid, WorkerControlHandle>,
    task_by_job_id: HashMap<Uuid, Vec<Uuid>>,
    env: Arc<EnvironmentManager>,
    fs: Arc<FilesystemDriver>,
}

enum WorkerCollectResult {
    Ok(TaskResultData),
    Cancelled,
    Paused,
    Failed(anyhow::Error),
    Crashed(anyhow::Error),
}

impl WorkerCollectResult {
    pub fn task_status(&self) -> TaskStatus {
        match self {
            WorkerCollectResult::Ok(_) => TaskStatus::Done,
            WorkerCollectResult::Cancelled => TaskStatus::Cancelled,
            WorkerCollectResult::Paused => TaskStatus::Paused,
            _ => TaskStatus::Failed,
        }
    }

    pub fn task_data(self) -> Option<TaskResultData> {
        match self {
            WorkerCollectResult::Ok(data) => Some(data),
            _ => None,
        }
    }
}

impl DownloadManager {
    pub fn new(env: Arc<EnvironmentManager>, fs: Arc<FilesystemDriver>) -> Self {
        Self {
            workers: Default::default(),
            progress_by_id: Default::default(),
            handle_by_task_id: Default::default(),
            task_by_job_id: Default::default(),
            env,
            fs,
        }
    }

    pub fn num_free_workers(&self) -> u32 {
        self.env.config.num_download_workers - self.workers.len() as u32
    }

    fn register_worker_handle(
        &mut self,
        task_id: Uuid,
        owner_job_id: Uuid,
        control_handle: WorkerControlHandle,
    ) {
        self.handle_by_task_id.insert(task_id, control_handle);
        if let Some(job_tasks) = self.task_by_job_id.get_mut(&owner_job_id) {
            job_tasks.push(task_id);
        } else {
            self.task_by_job_id.insert(owner_job_id, vec![task_id]);
        }
    }

    fn unregister_worker_handle(&mut self, task_id: Uuid, owner_job_id: Uuid) {
        self.handle_by_task_id.remove(&task_id);
        let job_tasks = self.task_by_job_id.get_mut(&owner_job_id).unwrap();
        let index = job_tasks.iter().position(|x| *x == task_id).unwrap();
        job_tasks.remove(index);
        if job_tasks.is_empty() {
            self.task_by_job_id.remove(&owner_job_id);
        }
    }

    fn register_worker_progress(&mut self, task_id: Uuid, progress: Arc<Mutex<TaskProgress>>) {
        self.progress_by_id.insert(task_id, progress);
    }

    fn unregister_worker_progress(&mut self, task_id: Uuid) {
        self.progress_by_id.remove(&task_id);
    }

    fn spawn_worker_thread(
        &mut self,
        task: &Task,
        progress: Arc<Mutex<TaskProgress>>,
        control_handle: WorkerControlHandle,
    ) -> JoinHandle<anyhow::Result<TaskResultData>> {
        let task = task.clone();
        let fs = self.fs.clone();
        let ytdlp = self.env.ytdlp.clone();
        tokio::task::spawn(async move { run_task(task, ytdlp, fs, progress, control_handle).await })
    }

    pub fn start_task(&mut self, task: Task) {
        let progress = Arc::new(Mutex::new(TaskProgress::default()));
        let control_handle = WorkerControlHandle::new();

        self.register_worker_handle(task.task_id, task.owner_job_id, control_handle.clone());
        self.register_worker_progress(task.task_id, progress.clone());

        let join_handle = self.spawn_worker_thread(&task, progress.clone(), control_handle.clone());

        self.workers.push(DownloadWorker {
            task,
            progress,
            join_handle,
            control_handle,
        });
    }

    pub async fn get_worker_progress(&self, task_id: Uuid) -> Option<TaskProgress> {
        if let Some(x) = self.progress_by_id.get(&task_id) {
            Some(x.lock().await.clone())
        } else {
            None
        }
    }

    fn is_worker_finished(&self, idx: usize) -> bool {
        self.workers[idx].join_handle.is_finished()
    }

    async fn join_worker_thread(
        join_handle: JoinHandle<anyhow::Result<TaskResultData>>,
    ) -> WorkerCollectResult {
        let join_result = join_handle.await;
        match join_result {
            Ok(Ok(data)) => WorkerCollectResult::Ok(data),
            Ok(Err(e)) => match e.downcast::<WorkerError>() {
                Ok(WorkerError::Aborted) => WorkerCollectResult::Cancelled,
                Ok(WorkerError::Paused) => WorkerCollectResult::Paused,
                Ok(e) => WorkerCollectResult::Failed(anyhow::Error::from(e)),
                Err(e) => WorkerCollectResult::Failed(e),
            },
            Err(e) => WorkerCollectResult::Crashed(anyhow::Error::from(e)),
        }
    }

    fn report_worker_result(task_id: Uuid, result: &WorkerCollectResult) {
        match result {
            WorkerCollectResult::Ok(_) => {
                info!("task finished successfully (id={task_id})");
            }
            WorkerCollectResult::Paused => {
                info!("task has been paused (id={task_id})");
            }
            WorkerCollectResult::Cancelled => {
                info!("task has been cancelled (id={task_id})");
            }
            WorkerCollectResult::Failed(e) => {
                error!("task has failed (id={task_id}): {e}");
            }
            WorkerCollectResult::Crashed(e) => {
                error!("worker has crashed (id={task_id}): {e}");
            }
        }
    }

    async fn collect_worker(&mut self, idx: usize) -> TaskResult {
        let DownloadWorker {
            task, join_handle, ..
        } = self.workers.remove(idx);
        let task_id = task.task_id;
        let owner_job_id = task.owner_job_id;

        let worker_result = Self::join_worker_thread(join_handle).await;

        Self::report_worker_result(task_id, &worker_result);
        self.unregister_worker_handle(task_id, owner_job_id);
        self.unregister_worker_progress(task_id);

        TaskResult {
            task,
            status: worker_result.task_status(),
            data: worker_result.task_data(),
        }
    }

    pub async fn poll_done(&mut self) -> Vec<TaskResult> {
        let mut res = Vec::<TaskResult>::new();
        // 1. We use indices and not iterators because we
        //    can't to take ownership of `self` here, as `self`
        //    is modified down the line when removing handles.
        // 2. Iterate in reverse so we can safely remove elements.
        for idx in (0..self.workers.len()).rev() {
            if self.is_worker_finished(idx) {
                res.push(self.collect_worker(idx).await);
            }
        }
        res
    }

    fn signal_worker(
        &mut self,
        command: &QueueCommand,
        _task_id: Uuid,
        worker_handle: WorkerControlHandle,
    ) {
        match command {
            QueueCommand::Pause => {
                worker_handle.pause();
            }
            QueueCommand::Cancel => {
                worker_handle.stop();
            }
            QueueCommand::Delete => {
                worker_handle.stop();
            }
            _ => {}
        }
    }

    fn get_worker_by_task_id(&self, task_id: Uuid) -> Option<(Uuid, WorkerControlHandle)> {
        self.handle_by_task_id
            .get(&task_id)
            .cloned()
            .map(|x| (task_id, x))
    }

    fn get_workers_by_job_id(&self, job_id: Uuid) -> Vec<(Uuid, WorkerControlHandle)> {
        self.task_by_job_id
            .get(&job_id)
            .map(|tasks| {
                tasks
                    .iter()
                    .map(|task_id| (*task_id, self.handle_by_task_id[task_id].clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_all_workers(&self) -> Vec<(Uuid, WorkerControlHandle)> {
        self.handle_by_task_id
            .iter()
            .map(|(&x, y)| (x, y.clone()))
            .collect()
    }

    pub fn modify_tasks_by_job<QC: Borrow<QueueCommand>>(&mut self, job_id: Uuid, command: QC) {
        for (task_id, worker_handle) in self.get_workers_by_job_id(job_id) {
            self.signal_worker(command.borrow(), task_id, worker_handle);
        }
    }

    pub fn modify_task<QC: Borrow<QueueCommand>>(&mut self, task_id: Uuid, command: QC) {
        if let Some((task_id, worker_handle)) = self.get_worker_by_task_id(task_id) {
            self.signal_worker(command.borrow(), task_id, worker_handle);
        }
    }

    pub fn modify_all_tasks<QC: Borrow<QueueCommand>>(&mut self, command: QC) {
        for (task_id, worker_handle) in self.get_all_workers() {
            self.signal_worker(command.borrow(), task_id, worker_handle);
        }
    }

    pub async fn clean_up_after_worker(&mut self, task_id: Uuid) {
        let _ = self.fs.remove_worker_root_dir(task_id).await;
    }
}
