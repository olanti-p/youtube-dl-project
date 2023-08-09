use crate::auth::User;
use crate::filesystem::FilesystemDriver;
use crate::job_manager::JobManagerHandle;
use crate::queue_command::QueueCommand;
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::response::status::NotFound;
use rocket::{get, post, State};
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

#[get("/tasks/get_stdout/<task_id>")]
pub async fn get_task_stdout(
    fs: &State<Arc<FilesystemDriver>>,
    _user: User,
    task_id: Uuid,
) -> Result<NamedFile, NotFound<String>> {
    let path = fs.get_ytdlp_stdout_file(task_id);
    NamedFile::open(path)
        .await
        .map_err(|_| NotFound("Log not available.".to_string()))
}

#[get("/tasks/get_stderr/<task_id>")]
pub async fn get_task_stderr(
    fs: &State<Arc<FilesystemDriver>>,
    _user: User,
    task_id: Uuid,
) -> Result<NamedFile, NotFound<String>> {
    let path = fs.get_ytdlp_stderr_file(task_id);
    NamedFile::open(path)
        .await
        .map_err(|_| NotFound("Log not available.".to_string()))
}

#[post("/tasks/pause/<task_id>")]
pub async fn pause_task(
    state: &State<JobManagerHandle>,
    _user: User,
    task_id: Uuid,
) -> (Status, ()) {
    match state.modify_task(task_id, QueueCommand::Pause).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to pause task {task_id}: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[post("/tasks/resume/<task_id>")]
pub async fn resume_task(
    state: &State<JobManagerHandle>,
    _user: User,
    task_id: Uuid,
) -> (Status, ()) {
    match state.modify_task(task_id, QueueCommand::Resume).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to resume task {task_id}: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[post("/tasks/cancel/<task_id>")]
pub async fn cancel_task(
    state: &State<JobManagerHandle>,
    _user: User,
    task_id: Uuid,
) -> (Status, ()) {
    match state.modify_task(task_id, QueueCommand::Cancel).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to cancel task {task_id}: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[post("/tasks/retry/<task_id>")]
pub async fn retry_task(
    state: &State<JobManagerHandle>,
    _user: User,
    task_id: Uuid,
) -> (Status, ()) {
    match state.modify_task(task_id, QueueCommand::Retry).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to retry task {task_id}: {e}");
            (Status::InternalServerError, ())
        }
    }
}

#[post("/tasks/delete/<task_id>")]
pub async fn delete_task(
    state: &State<JobManagerHandle>,
    _user: User,
    task_id: Uuid,
) -> (Status, ()) {
    match state.modify_task(task_id, QueueCommand::Delete).await {
        Ok(_) => (Status::Accepted, ()),
        Err(e) => {
            warn!("Failed to delete task {task_id}: {e}");
            (Status::InternalServerError, ())
        }
    }
}
