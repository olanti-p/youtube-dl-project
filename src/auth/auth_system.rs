use crate::database::ServerDatabase;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct AuthSystem {
    db: Arc<Mutex<ServerDatabase>>,
}

impl AuthSystem {
    pub fn new(db: Arc<Mutex<ServerDatabase>>) -> Self {
        Self { db }
    }

    pub async fn attempt_new_session(&self, api_token: &str) -> anyhow::Result<Uuid> {
        let session_token = self.db.lock().await.new_session(api_token).await?;
        Ok(session_token)
    }

    pub async fn expire_all_sessions(&self) -> anyhow::Result<()> {
        self.db.lock().await.expire_all_sessions().await
    }
}
