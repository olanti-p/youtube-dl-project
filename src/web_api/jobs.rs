use crate::auth::User;
use crate::job::Job;
use crate::job_manager::JobManagerHandle;
use crate::queue_command::QueueCommand;
use rocket::form::Form;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{get, post, FromForm, State};
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromForm)]
pub struct NewJobForm {
    pub url: String,
    pub format: String,
}

#[post("/jobs/new", data = "<data>")]
pub async fn new_job(
    state: &State<JobManagerHandle>,
    _user: User,
    data: Form<NewJobForm>,
) -> (Status, Option<Json<Job>>) {
    match state.create_job(&data).await {
        Ok(val) => (Status::Accepted, Some(Json(val))),
        Err(e) => {
            warn!(
                "Failed to start job for url={:?} format={:?}: {e}",
                data.url, data.format
            );
            (Status::InternalServerError, None)
        }
    }
}

#[get("/jobs/get/<job_id>")]
pub async fn get_job(
    state: &State<JobManagerHandle>,
    _user: User,
    job_id: Uuid,
) -> (Status, Option<Json<Job>>) {
    match state.get_job(job_id).await {
        Ok(val) => (Status::Ok, Some(Json(val))),
        Err(e) => {
            warn!("Failed to get job {job_id}: {e}");
            (Status::InternalServerError, None)
        }
    }
}

#[post("/jobs/pause/<job_id>")]
pub async fn pause_job(state: &State<JobManagerHandle>, _user: User, job_id: Uuid) -> (Status, ()) {
    match state.modify_job(job_id, QueueCommand::Pause).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to pause job {job_id}: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[post("/jobs/resume/<job_id>")]
pub async fn resume_job(
    state: &State<JobManagerHandle>,
    _user: User,
    job_id: Uuid,
) -> (Status, ()) {
    match state.modify_job(job_id, QueueCommand::Resume).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to resume job {job_id}: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[post("/jobs/cancel/<job_id>")]
pub async fn cancel_job(
    state: &State<JobManagerHandle>,
    _user: User,
    job_id: Uuid,
) -> (Status, ()) {
    match state.modify_job(job_id, QueueCommand::Cancel).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to cancel job {job_id}: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[post("/jobs/retry/<job_id>")]
pub async fn retry_job(state: &State<JobManagerHandle>, _user: User, job_id: Uuid) -> (Status, ()) {
    match state.modify_job(job_id, QueueCommand::Retry).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to retry job {job_id}: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[post("/jobs/delete/<job_id>")]
pub async fn delete_job(
    state: &State<JobManagerHandle>,
    _user: User,
    job_id: Uuid,
) -> (Status, ()) {
    match state.modify_job(job_id, QueueCommand::Delete).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to delete job {job_id}: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[get("/jobs/get_all")]
pub async fn get_all_jobs(
    state: &State<JobManagerHandle>,
    _user: User,
) -> (Status, Option<Json<Vec<Job>>>) {
    match state.get_all_jobs().await {
        Ok(val) => (Status::Ok, Some(Json(val))),
        Err(e) => {
            warn!("Failed to get all jobs: {e}");
            (Status::InternalServerError, None)
        }
    }
}

#[post("/jobs/pause_all")]
pub async fn pause_all_jobs(state: &State<JobManagerHandle>, _user: User) -> (Status, ()) {
    match state.modify_all_jobs(QueueCommand::Pause).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to pause all jobs: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[post("/jobs/resume_all")]
pub async fn resume_all_jobs(state: &State<JobManagerHandle>, _user: User) -> (Status, ()) {
    match state.modify_all_jobs(QueueCommand::Resume).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to resume all jobs: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[post("/jobs/cancel_all")]
pub async fn cancel_all_jobs(state: &State<JobManagerHandle>, _user: User) -> (Status, ()) {
    match state.modify_all_jobs(QueueCommand::Cancel).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to cancel all jobs: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[post("/jobs/retry_all")]
pub async fn retry_all_jobs(state: &State<JobManagerHandle>, _user: User) -> (Status, ()) {
    match state.modify_all_jobs(QueueCommand::Retry).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to retry all jobs: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[post("/jobs/delete_all")]
pub async fn delete_all_jobs(state: &State<JobManagerHandle>, _user: User) -> (Status, ()) {
    match state.modify_all_jobs(QueueCommand::Delete).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to delete all jobs: {e}");
            (Status::InternalServerError, ())
        }
    }
}
