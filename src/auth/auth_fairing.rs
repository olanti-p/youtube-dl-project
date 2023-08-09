use crate::auth::auth_request_state::AuthRequestState;
use crate::auth::user::User;
use crate::database::ServerDatabase;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Data, Request};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::warn;
use uuid::Uuid;

pub struct AuthFairing {
    db: Arc<Mutex<ServerDatabase>>,
}

impl AuthFairing {
    pub fn new(db: Arc<Mutex<ServerDatabase>>) -> Self {
        Self { db }
    }

    fn extract_api_token_headers<'r>(req: &'r Request<'_>) -> Vec<&'r str> {
        req.headers().get("api-token").collect()
    }

    fn extract_session_token_headers<'r>(req: &'r Request<'_>) -> Vec<&'r str> {
        req.headers().get("session-token").collect()
    }

    fn authorization_provided(api_token_headers: &[&str], session_token_headers: &[&str]) -> bool {
        !api_token_headers.is_empty() || !session_token_headers.is_empty()
    }

    fn validate_api_token_headers(api_token_headers: Vec<&str>) -> anyhow::Result<&str> {
        if api_token_headers.len() != 1 {
            return Err(anyhow!("Expected exactly 1 'api-token' header."));
        }
        let api_token: &str = api_token_headers[0];
        Ok(api_token)
    }

    fn validate_session_token_headers<'a>(
        session_token_headers: Vec<&str>,
    ) -> anyhow::Result<Uuid> {
        if session_token_headers.len() != 1 {
            return Err(anyhow!("Expected exactly 1 'session-token' header."));
        }
        let session_token: Uuid = serde_json::from_str(session_token_headers[0])?;
        Ok(session_token)
    }

    async fn resolve_tokens(
        &self,
        api_token: &str,
        session_token: Uuid,
    ) -> anyhow::Result<Option<User>> {
        self.db
            .lock()
            .await
            .validate_session(api_token, session_token)
            .await
    }

    async fn on_request_inner(&self, req: &mut Request<'_>) -> anyhow::Result<Option<User>> {
        let api_token_hdr = Self::extract_api_token_headers(req);
        let session_token_hdr = Self::extract_session_token_headers(req);
        if !Self::authorization_provided(&api_token_hdr, &session_token_hdr) {
            return Ok(None);
        }
        let api_token = Self::validate_api_token_headers(api_token_hdr)?;
        let session_token = Self::validate_session_token_headers(session_token_hdr)?;
        let user_opt = self.resolve_tokens(api_token, session_token).await?;
        Ok(Some(user_opt.ok_or(anyhow!("Invalid session."))?))
    }
}

#[rocket::async_trait]
impl Fairing for AuthFairing {
    fn info(&self) -> Info {
        Info {
            name: "Auth",
            kind: Kind::Request | Kind::Singleton,
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, _: &mut Data<'_>) {
        match self.on_request_inner(req).await {
            Ok(None) => {
                req.local_cache(|| AuthRequestState::None);
                warn!("No auth.");
            }
            Ok(Some(user)) => {
                warn!("Authorized as: {:?}", user);
                req.local_cache(move || AuthRequestState::Authorized(user));
            }
            Err(e) => {
                warn!("Auth error: {e}. Request headers: {:?}", req.headers());
                req.local_cache(|| AuthRequestState::Error(e));
            }
        }
    }
}
