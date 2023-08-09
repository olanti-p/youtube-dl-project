use crate::download_manager::worker_handle::WorkerControlHandle;
use crate::task::{Task, TaskProgress, TaskResultData};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct DownloadWorker {
    pub task: Task,
    pub progress: Arc<Mutex<TaskProgress>>,
    pub join_handle: JoinHandle<anyhow::Result<TaskResultData>>,
    pub control_handle: WorkerControlHandle,
}
