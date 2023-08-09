use crate::auth::User;
use crate::database::TaskStats;
use crate::job_manager::JobManagerHandle;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, State};
use tracing::warn;

#[get("/status")]
pub async fn get_status(
    state: &State<JobManagerHandle>,
    _user: User,
) -> (Status, Option<Json<TaskStats>>) {
    match state.get_overall_stats().await {
        Ok(val) => (Status::Ok, Some(Json(val))),
        Err(e) => {
            warn!("Failed to get queue status: {e}");
            (Status::InternalServerError, None)
        }
    }
}
