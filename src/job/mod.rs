use crate::database::JobFetch;
use crate::task::{Task, TaskKind, TaskProgress, TaskStatus};
use chrono::{DateTime, Utc};
use rocket::serde::Serialize;
use serde::Deserialize;
use std::collections::HashMap;
use std::default::Default;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
pub enum JobStatus {
    Waiting,
    Processing,
    Done,
    PartiallyDone,
    Paused,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub tasks: Vec<Task>,
    pub thumbnail: String,
    pub url: String,
    pub format: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub progress: HashMap<Uuid, TaskProgress>,
    pub prioritized: bool,
    pub title: String,
}

impl Job {
    pub fn status_from_tasks(tasks: &[Task]) -> JobStatus {
        // HACK: since we know there's only 1 FetchUrlContents task,
        //       and it's executed before all other tasks,
        //       we can short-circuit most cases here.
        if let Some(fetch_task) = tasks
            .iter()
            .filter(|x| x.kind == TaskKind::FetchUrlContents)
            .next()
        {
            match fetch_task.status {
                TaskStatus::Waiting => return JobStatus::Waiting,
                TaskStatus::Processing => return JobStatus::Processing,
                TaskStatus::Paused => return JobStatus::Paused,
                TaskStatus::Failed => return JobStatus::Failed,
                TaskStatus::Cancelled => return JobStatus::Cancelled,
                TaskStatus::Done => {}
            }
        }

        // HACK: FetchUrlContents task is something of a phony task.
        //       We don't care about it succeeding if everything else
        //       has failed, so it shouldn't count for 'PartiallyDone' jobs.
        let statuses: Vec<TaskStatus> = tasks
            .iter()
            .filter_map(|x| match x.kind {
                TaskKind::FetchUrlContents => None,
                TaskKind::DownloadAndConvert => Some(x.status),
            })
            .collect();
        if statuses.is_empty() {
            JobStatus::Done
        } else if statuses.contains(&TaskStatus::Processing) {
            JobStatus::Processing
        } else if statuses.contains(&TaskStatus::Waiting) {
            JobStatus::Waiting
        } else if statuses.contains(&TaskStatus::Paused) {
            JobStatus::Paused
        } else if statuses.contains(&TaskStatus::Done) {
            if statuses.contains(&TaskStatus::Cancelled) || statuses.contains(&TaskStatus::Failed) {
                JobStatus::PartiallyDone
            } else {
                JobStatus::Done
            }
        } else if statuses.contains(&TaskStatus::Cancelled) {
            JobStatus::Cancelled
        } else {
            JobStatus::Failed
        }
    }

    fn started_at_from_tasks(tasks: &[Task]) -> Option<DateTime<Utc>> {
        tasks.iter().filter_map(|x| x.started_at).min()
    }

    fn finished_at_from_tasks(status: JobStatus, tasks: &[Task]) -> Option<DateTime<Utc>> {
        match status {
            JobStatus::Waiting | JobStatus::Processing => {
                return None;
            }
            _ => tasks.iter().filter_map(|x| x.finished_at).max(),
        }
    }

    pub fn new(fetch: JobFetch, tasks: Vec<Task>) -> Self {
        let status = Self::status_from_tasks(&tasks);
        let started_at = Self::started_at_from_tasks(&tasks);
        let finished_at = Self::finished_at_from_tasks(status, &tasks);
        Job {
            tasks,
            status,
            job_id: fetch.job_id,
            thumbnail: fetch.thumbnail,
            url: fetch.url,
            format: fetch.format,
            created_at: fetch.created_at,
            started_at,
            finished_at,
            progress: Default::default(),
            prioritized: fetch.prioritized,
            title: fetch.title,
        }
    }
}
