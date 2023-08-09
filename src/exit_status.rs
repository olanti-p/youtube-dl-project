use crate::env::Config;
use std::sync::Arc;
use tokio::sync::Mutex;

pub enum ExitStatus {
    ChangeConfig(Config),
}

#[derive(Clone)]
pub struct ExitStatusHandle {
    data: Arc<Mutex<Option<ExitStatus>>>,
}

impl ExitStatusHandle {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn store(&self, status: ExitStatus) {
        *self.data.lock().await = Some(status);
    }

    pub async fn take(&self) -> Option<ExitStatus> {
        self.data.lock().await.take()
    }
}
