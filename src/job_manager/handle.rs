use crate::announce::AnnounceSystem;
use crate::database::ServerDatabase;
use crate::env::EnvironmentManager;
use crate::filesystem::FilesystemDriver;
use crate::job_manager::JobManager;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct JobManagerHandle {
    ptr: Arc<JobManager>,
}

impl Deref for JobManagerHandle {
    type Target = JobManager;

    fn deref(&self) -> &Self::Target {
        self.ptr.deref()
    }
}

impl JobManagerHandle {
    pub async fn new(
        announcements: Arc<AnnounceSystem>,
        env: Arc<EnvironmentManager>,
        fs: Arc<FilesystemDriver>,
        db: Arc<Mutex<ServerDatabase>>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            ptr: Arc::new(JobManager::new(announcements, env, fs, db).await?),
        })
    }
}
