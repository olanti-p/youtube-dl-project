use crate::auth::{AuthSystem, User};
use rocket::form::Form;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{post, FromForm, State};
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromForm)]
pub struct NewSessionForm {
    pub api_token: String,
}

#[post("/sessions/new", data = "<data>")]
pub async fn new_session(
    auth: &State<AuthSystem>,
    data: Form<NewSessionForm>,
) -> (Status, Option<Json<Uuid>>) {
    match auth.attempt_new_session(&data.api_token).await {
        Ok(session_token) => (Status::Ok, Some(Json(session_token))),
        Err(e) => {
            warn!(
                "Failed to create new session with token {}: {e}",
                data.api_token
            );
            (Status::BadRequest, None)
        }
    }
}

#[post("/sessions/expire_all")]
pub async fn expire_all_sessions(auth: &State<AuthSystem>, _user: User) -> Status {
    match auth.expire_all_sessions().await {
        Ok(_) => Status::Ok,
        Err(e) => {
            warn!("Failed to expire all sessions: {e}");
            Status::InternalServerError
        }
    }
}
