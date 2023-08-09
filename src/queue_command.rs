use crate::task::{Task, TaskStatus};
use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueueCommand {
    Pause,
    Resume,
    Cancel,
    Retry,
    Delete,
    SetPrioritized(bool),
    TaskStatusChange(TaskStatus),
    JobUpdated {
        job_title: String,
        job_thumbnail: String,
        new_tasks: Vec<Task>,
    },
}
