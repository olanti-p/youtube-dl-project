use rocket::{routes, Route};

mod config;
mod format;
mod index;
mod jobs;
mod ping;
mod sessions;
mod shutdown;
mod status;
mod tasks;

use config::*;
use format::*;
use index::*;
use jobs::*;
use ping::*;
use sessions::*;
use shutdown::*;
use status::*;
use tasks::*;

pub use jobs::NewJobForm;

pub fn get_api_routes() -> Vec<Route> {
    routes![
        pong,
        new_job,
        get_job,
        pause_job,
        resume_job,
        cancel_job,
        retry_job,
        delete_job,
        get_all_jobs,
        pause_all_jobs,
        resume_all_jobs,
        cancel_all_jobs,
        retry_all_jobs,
        delete_all_jobs,
        get_task_stdout,
        get_task_stderr,
        pause_task,
        resume_task,
        cancel_task,
        retry_task,
        delete_task,
        get_status,
        get_config,
        set_config,
        get_formats,
        shutdown_server,
        new_session,
        expire_all_sessions,
    ]
}

pub fn get_index_html_redirect() -> Vec<Route> {
    routes![root_redirect, index_html_redirect,]
}
