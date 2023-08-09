use crate::auth::User;
use crate::filesystem::FilesystemDriver;
use crate::job::{Job, JobStatus};
use crate::queue_command::QueueCommand;
use crate::task::{Task, TaskStatus};
use crate::web_api::NewJobForm;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rocket::serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteArguments, SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{query, query_as, query_as_with, ConnectOptions, Pool, Sqlite};
use sqlx::{Arguments, FromRow};
use std::collections::HashMap;
use std::default::Default;
use std::str::FromStr;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct JobFetch {
    pub job_id: Uuid,
    pub thumbnail: String,
    pub url: String,
    pub format: String,
    pub created_at: DateTime<Utc>,
    pub prioritized: bool,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStats {
    pub num_total: i32,
    pub num_active: i32,
    pub num_cancelled: i32,
    pub num_waiting: i32,
    pub num_done: i32,
    pub num_failed: i32,
}

#[derive(Debug)]
pub struct ServerDatabase {
    pool: Pool<Sqlite>,
    rng: Mutex<ChaCha20Rng>,
}

pub struct PendingOperations {
    pub cleanup: Vec<Uuid>,
    pub delete: Vec<Uuid>,
    pub num_busy: u32,
}

impl PendingOperations {
    pub fn is_empty(&self) -> bool {
        self.cleanup.is_empty() && self.delete.is_empty()
    }
}

impl ServerDatabase {
    pub async fn open(fs: &FilesystemDriver) -> anyhow::Result<Self> {
        let path_string = fs.get_database_file().to_string_lossy();
        let options = SqliteConnectOptions::from_str(&path_string)?
            .create_if_missing(true)
            .disable_statement_logging();

        let pool = SqlitePoolOptions::new()
            .max_connections(3)
            .connect_with(options)
            .await?;

        let rng = Mutex::new(ChaCha20Rng::from_entropy());

        let res = Self { pool, rng };
        res.init_tables().await?;
        res.reset_state().await?;
        Ok(res)
    }

    async fn init_tables(&self) -> anyhow::Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;

        let tx = self.pool.begin().await?;

        let admin_user = self.get_user_by_name(User::admin_user_name()).await?;
        if admin_user.is_none() {
            let name = User::admin_user_name();
            let password = User::generate_api_token(&mut *self.rng.lock().await);
            self.add_user(name, &password).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn reset_state(&self) -> anyhow::Result<()> {
        let tx = self.pool.begin().await?;

        sqlx::query!(
            r#"UPDATE tasks SET status = ?1, finished_at = started_at WHERE status = ?2 OR status = ?3"#,
            TaskStatus::Failed,
            TaskStatus::Waiting,
            TaskStatus::Processing,
        )
            .execute(&self.pool)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    fn new_args() -> SqliteArguments<'static> {
        SqliteArguments::default()
    }

    pub async fn acquire_tasks(&self, max_tasks: u32) -> anyhow::Result<Vec<Task>> {
        let tx = self.pool.begin().await?;

        let mut args = Self::new_args();
        args.add(TaskStatus::Waiting);
        args.add(max_tasks);
        let tasks: Vec<Task> = query_as_with(
            r#"
        SELECT * FROM tasks
        WHERE status = ?1 AND pending_delete = false
        ORDER BY created_at
        LIMIT ?2
        "#,
            args,
        )
        .fetch_all(&self.pool)
        .await?;

        let started_at = Utc::now();

        for task in &tasks {
            sqlx::query!(
                r#"
                UPDATE tasks SET status = ?1, started_at = ?2, finished_at = null
                WHERE task_id = ?3
                "#,
                TaskStatus::Processing,
                started_at,
                task.task_id
            )
            .execute(&self.pool)
            .await?;
        }

        tx.commit().await?;
        Ok(tasks)
    }

    pub async fn get_global_task_stats(&self) -> anyhow::Result<TaskStats> {
        let mut args = Self::new_args();
        args.add(TaskStatus::Waiting);
        args.add(TaskStatus::Cancelled);
        args.add(TaskStatus::Failed);
        args.add(TaskStatus::Done);
        args.add(TaskStatus::Processing);
        let ret: (i32, i32, i32, i32, i32, i32) = query_as_with(
            r#"
            SELECT
                COUNT(*) As Total,
                SUM(CASE WHEN status=?1 THEN 1 ELSE 0 END) as numWaiting,
                SUM(CASE WHEN status=?2 THEN 1 ELSE 0 END) as numCancelled,
                SUM(CASE WHEN status=?3 THEN 1 ELSE 0 END) as numFailed,
                SUM(CASE WHEN status=?4 THEN 1 ELSE 0 END) as numDone,
                SUM(CASE WHEN status=?5 THEN 1 ELSE 0 END) as numProcessing
            FROM tasks
            "#,
            args,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(TaskStats {
            num_total: ret.0,
            num_active: ret.5,
            num_cancelled: ret.2,
            num_waiting: ret.1,
            num_done: ret.4,
            num_failed: ret.3,
        })
    }

    pub async fn get_job_task_stats(&self, job_id: Uuid) -> anyhow::Result<TaskStats> {
        let mut args = Self::new_args();
        args.add(TaskStatus::Waiting);
        args.add(TaskStatus::Cancelled);
        args.add(TaskStatus::Failed);
        args.add(TaskStatus::Done);
        args.add(TaskStatus::Processing);
        args.add(job_id);
        let ret: (i32, i32, i32, i32, i32, i32) = query_as_with(
            r#"
            SELECT
                COUNT(*) As Total,
                SUM(CASE WHEN status=?1 THEN 1 ELSE 0 END) as numWaiting,
                SUM(CASE WHEN status=?2 THEN 1 ELSE 0 END) as numCancelled,
                SUM(CASE WHEN status=?3 THEN 1 ELSE 0 END) as numFailed,
                SUM(CASE WHEN status=?4 THEN 1 ELSE 0 END) as numDone,
                SUM(CASE WHEN status=?5 THEN 1 ELSE 0 END) as numProcessing
            FROM tasks
            WHERE owner_job_id = ?6
            "#,
            args,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(TaskStats {
            num_total: ret.0,
            num_active: ret.5,
            num_cancelled: ret.2,
            num_waiting: ret.1,
            num_done: ret.4,
            num_failed: ret.3,
        })
    }

    pub async fn get_pending_operations(&self) -> anyhow::Result<PendingOperations> {
        let tasks_all: Vec<Task> = query_as(
            r#"
            SELECT * FROM tasks
            WHERE pending_delete = true OR pending_cleanup = true
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let delete = tasks_all
            .iter()
            .filter_map(|task| {
                if task.status != TaskStatus::Processing && task.pending_delete {
                    Some(task.task_id)
                } else {
                    None
                }
            })
            .collect();
        let cleanup = tasks_all
            .iter()
            .filter_map(|task| {
                if task.status != TaskStatus::Processing && task.pending_cleanup {
                    Some(task.task_id)
                } else {
                    None
                }
            })
            .collect();
        let num_busy = tasks_all
            .iter()
            .filter(|x| x.status == TaskStatus::Processing)
            .count() as u32;

        Ok(PendingOperations {
            delete,
            cleanup,
            num_busy,
        })
    }

    pub async fn confirm_deletion(&self, tasks: &[Uuid]) -> anyhow::Result<()> {
        let tx = self.pool.begin().await?;

        for &task_id in tasks {
            sqlx::query!(
                r#"
                DELETE FROM tasks
                WHERE task_id = ?1
                "#,
                task_id
            )
            .execute(&self.pool)
            .await?;
        }

        // Delete all jobs that don't have tasks
        sqlx::query!(
            r#"
            DELETE FROM jobs
            WHERE 0 = (
                SELECT Count(*) FROM tasks WHERE tasks.owner_job_id = jobs.job_id
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn confirm_cleanup(&self, tasks: &[Uuid]) -> anyhow::Result<()> {
        let tx = self.pool.begin().await?;

        for &task_id in tasks {
            sqlx::query!(
                r#"
                UPDATE tasks
                SET pending_cleanup = false
                WHERE task_id = ?1
                "#,
                task_id
            )
            .execute(&self.pool)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn create_job(&self, new_job: &NewJobForm) -> anyhow::Result<Job> {
        let tx = self.pool.begin().await?;

        // FIXME: possible id collisions
        let job_id = Uuid::new_v4();
        let created_at = Utc::now();

        let mut job = Job {
            job_id,
            status: JobStatus::Waiting,
            thumbnail: "".to_string(),
            url: new_job.url.to_string(),
            format: new_job.format.clone(),
            created_at,
            started_at: None,
            finished_at: None,
            tasks: vec![],
            progress: Default::default(),
            prioritized: false,
            title: "...".to_string(),
        };

        sqlx::query!(
            r#"
                INSERT INTO jobs
                    (job_id, thumbnail, url, format, created_at, title)
                VALUES
                    (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
            job.job_id,
            job.thumbnail,
            job.url,
            job.format,
            job.created_at,
            job.title,
        )
        .execute(&self.pool)
        .await?;

        // FIXME: possible id collisions
        let task = Task::new_fetch_url_contents(&job);

        sqlx::query!(
                r#"
                INSERT INTO tasks
                    (task_id, status, kind, thumbnail, owner_job_id, url, format, created_at, finished_at, task_index, title)
                VALUES
                    (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                task.task_id,
                task.status,
                task.kind,
                task.thumbnail,
                task.owner_job_id,
                task.url,
                task.format,
                task.created_at,
                task.finished_at,
                0,
                task.title
            )
            .execute(&self.pool)
            .await?;

        job.tasks.push(task);
        job.status = Job::status_from_tasks(&job.tasks);

        tx.commit().await?;
        Ok(job)
    }

    pub async fn modify_all_jobs(&self, command: QueueCommand) -> anyhow::Result<()> {
        let jobs = self.get_all_jobs().await?;
        for job in jobs {
            self.modify_job(job.job_id, command.clone()).await?;
        }
        Ok(())
    }

    pub async fn modify_job(&self, job_id: Uuid, command: QueueCommand) -> anyhow::Result<()> {
        let finished_at = Utc::now();
        let tx = self.pool.begin().await?;

        match command {
            QueueCommand::Pause => {
                sqlx::query!(
                    r#"
                    UPDATE tasks
                    SET status = ?2
                    WHERE owner_job_id = ?1 AND status = ?3
                    "#,
                    job_id,
                    TaskStatus::Paused,
                    TaskStatus::Waiting,
                )
                .execute(&self.pool)
                .await?;
            }
            QueueCommand::Resume => {
                sqlx::query!(
                    r#"
                    UPDATE tasks
                    SET status = ?2, is_resumed = (CASE WHEN status = ?3 THEN true ELSE false END)
                    WHERE owner_job_id = ?1 AND status = ?3
                    "#,
                    job_id,
                    TaskStatus::Waiting,
                    TaskStatus::Paused,
                )
                .execute(&self.pool)
                .await?;
            }
            QueueCommand::Cancel => {
                sqlx::query!(
                    r#"
                    UPDATE tasks
                    SET status = ?4, finished_at = ?5
                    WHERE owner_job_id = ?1 AND (status = ?2 OR status = ?3)
                    "#,
                    job_id,
                    TaskStatus::Waiting,
                    TaskStatus::Paused,
                    TaskStatus::Cancelled,
                    finished_at,
                )
                .execute(&self.pool)
                .await?;
            }
            QueueCommand::Retry => {
                sqlx::query!(
                    r#"
                    UPDATE tasks
                    SET status = ?2
                    WHERE owner_job_id = ?1 AND (status = ?3 OR status = ?4)
                    "#,
                    job_id,
                    TaskStatus::Waiting,
                    TaskStatus::Failed,
                    TaskStatus::Cancelled,
                )
                .execute(&self.pool)
                .await?;
            }
            QueueCommand::Delete => {
                sqlx::query!(
                    r#"
                    UPDATE tasks
                    SET pending_delete = true, pending_cleanup = true
                    WHERE owner_job_id = ?1
                    "#,
                    job_id,
                )
                .execute(&self.pool)
                .await?;
            }
            QueueCommand::JobUpdated {
                job_title,
                job_thumbnail,
                new_tasks,
            } => {
                sqlx::query!(
                    r#"
                    UPDATE jobs
                    SET title = ?2, thumbnail = ?3
                    WHERE job_id = ?1
                    "#,
                    job_id,
                    job_title,
                    job_thumbnail,
                )
                .execute(&self.pool)
                .await?;

                let mut args = Self::new_args();
                args.add(job_id);
                let (mut last_task_index,): (i32,) = sqlx::query_as_with(
                    r#"
                    SELECT MAX(task_index) FROM tasks
                    WHERE owner_job_id = ?1
                    "#,
                    args,
                )
                .fetch_one(&self.pool)
                .await?;

                for task in new_tasks {
                    last_task_index += 1;
                    sqlx::query!(
                        r#"
                        INSERT INTO tasks
                            (task_id, status, kind, thumbnail, owner_job_id, url, format, created_at, finished_at, task_index, title)
                        VALUES
                            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                        "#,
                        task.task_id,
                        task.status,
                        task.kind,
                        task.thumbnail,
                        task.owner_job_id,
                        task.url,
                        task.format,
                        task.created_at,
                        task.finished_at,
                        last_task_index,
                        task.title
                    )
                        .execute(&self.pool)
                        .await?;
                }
            }
            _ => {}
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn modify_task(&self, task_id: Uuid, command: QueueCommand) -> anyhow::Result<()> {
        let finished_at = Utc::now();
        let tx = self.pool.begin().await?;

        match command {
            QueueCommand::Pause => {
                sqlx::query!(
                    r#"
                    UPDATE tasks
                    SET status = ?2
                    WHERE task_id = ?1 AND status = ?3
                    "#,
                    task_id,
                    TaskStatus::Paused,
                    TaskStatus::Waiting,
                )
                .execute(&self.pool)
                .await?;
            }
            QueueCommand::Resume => {
                sqlx::query!(
                    r#"
                    UPDATE tasks
                    SET status = ?2
                    WHERE task_id = ?1 AND status = ?3
                    "#,
                    task_id,
                    TaskStatus::Waiting,
                    TaskStatus::Paused,
                )
                .execute(&self.pool)
                .await?;
            }
            QueueCommand::Cancel => {
                sqlx::query!(
                    r#"
                    UPDATE tasks
                    SET status = ?4, finished_at = ?5
                    WHERE task_id = ?1 AND (status = ?2 OR status = ?3)
                    "#,
                    task_id,
                    TaskStatus::Waiting,
                    TaskStatus::Paused,
                    TaskStatus::Cancelled,
                    finished_at,
                )
                .execute(&self.pool)
                .await?;
            }
            QueueCommand::Retry => {
                sqlx::query!(
                    r#"
                    UPDATE tasks
                    SET status = ?2
                    WHERE task_id = ?1 AND (status = ?3 OR status = ?4)
                    "#,
                    task_id,
                    TaskStatus::Waiting,
                    TaskStatus::Failed,
                    TaskStatus::Cancelled,
                )
                .execute(&self.pool)
                .await?;
            }
            QueueCommand::Delete => {
                sqlx::query!(
                    r#"
                    UPDATE tasks
                    SET pending_delete = true, pending_cleanup = true
                    WHERE task_id = ?1
                    "#,
                    task_id,
                )
                .execute(&self.pool)
                .await?;
            }
            QueueCommand::TaskStatusChange(status) => {
                let finished_at: Option<DateTime<Utc>> = match status {
                    TaskStatus::Waiting | TaskStatus::Processing | TaskStatus::Paused => None,
                    TaskStatus::Done | TaskStatus::Failed | TaskStatus::Cancelled => {
                        Some(finished_at)
                    }
                };
                // Set `pending_cleanup` flag only if we've transitioned
                // to TaskStatus::Done, otherwise leave it as is.
                sqlx::query!(
                    r#"
                    UPDATE tasks
                    SET status = ?2, finished_at = ?3, pending_cleanup = (CASE WHEN ?2 = ?4 THEN true ELSE pending_cleanup END)
                    WHERE task_id = ?1
                    "#,
                    task_id,
                    status,
                    finished_at,
                    TaskStatus::Done,
                )
                .execute(&self.pool)
                .await?;
            }
            _ => {}
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn get_job(&self, job_id: Uuid) -> anyhow::Result<Job> {
        let mut args = Self::new_args();
        args.add(job_id);
        let job: JobFetch = query_as_with(
            r#"
        SELECT * FROM jobs
        WHERE job_id = ?1
        "#,
            args,
        )
        .fetch_one(&self.pool)
        .await?;

        let mut args = Self::new_args();
        args.add(job_id);
        let tasks: Vec<Task> = query_as_with(
            r#"
            SELECT * FROM tasks
            WHERE owner_job_id = ?1
            "#,
            args,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(Job::new(job, tasks))
    }

    pub async fn get_all_jobs(&self) -> anyhow::Result<Vec<Job>> {
        let jobs: Vec<JobFetch> = query_as(r#"SELECT * FROM jobs ORDER BY created_at DESC"#)
            .fetch_all(&self.pool)
            .await?;

        let tasks: Vec<Task> = query_as(r#"SELECT * FROM tasks ORDER BY task_index"#)
            .fetch_all(&self.pool)
            .await?;

        let mut tasks_sorted = HashMap::<Uuid, Vec<Task>>::new();
        for task in tasks {
            if let Some(v) = tasks_sorted.get_mut(&task.owner_job_id) {
                v.push(task);
            } else {
                tasks_sorted.insert(task.owner_job_id, vec![task]);
            }
        }

        Ok(jobs
            .into_iter()
            .map(|x| {
                let tasks = tasks_sorted.remove(&x.job_id).unwrap_or_default();
                Job::new(x, tasks)
            })
            .collect())
    }

    pub async fn get_user_by_api_token(&self, api_token: &str) -> anyhow::Result<Option<User>> {
        let user: Option<User> = query_as(
            r#"
            SELECT * FROM users
            WHERE api_token = ?1
        "#,
        )
        .bind(api_token)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn get_user_by_name(&self, username: &str) -> anyhow::Result<Option<User>> {
        let user: Option<User> = query_as(
            r#"
            SELECT * FROM users
            WHERE name = ?1
        "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn add_user(&self, username: &str, api_token: &str) -> anyhow::Result<Uuid> {
        // FIXME: possible id collisions
        let user_id = Uuid::new_v4();
        // FIXME: use safe storage for passwords if user management becomes as actual thing
        query!(
            r#"
            INSERT INTO users
                (user_id, name, api_token)
            VALUES
                (?1, ?2, ?3)
            "#,
            user_id,
            username,
            api_token
        )
        .execute(&self.pool)
        .await?;
        Ok(user_id)
    }

    pub async fn new_session(&self, api_token: &str) -> anyhow::Result<Uuid> {
        // FIXME: possible token collisions
        let mut token_bytes = [0u8; 16];
        self.rng.lock().await.fill_bytes(&mut token_bytes);
        let session_token = Uuid::from_bytes(token_bytes);

        let tx = self.pool.begin().await?;

        let user = self
            .get_user_by_api_token(api_token)
            .await?
            .context("Invalid API token.")?;

        let user_id = user.get_user_id();

        query!(
            r#"
            INSERT INTO sessions
                (session_token, user_id)
            VALUES
                (?1, ?2)
            "#,
            session_token,
            user_id
        )
        .execute(&self.pool)
        .await?;

        tx.commit().await?;

        Ok(session_token)
    }

    pub async fn expire_all_sessions(&self) -> anyhow::Result<()> {
        query!(
            r#"
            DELETE FROM sessions
            "#
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn validate_session(
        &self,
        api_token: &str,
        session_token: Uuid,
    ) -> anyhow::Result<Option<User>> {
        let ret: Option<User> = query_as(
            r#"
                SELECT users.user_id, name, api_token FROM users
                JOIN sessions
                ON users.user_id = sessions.user_id
                WHERE api_token = ?1 AND session_token = ?2
            "#,
        )
        .bind(api_token)
        .bind(session_token)
        .fetch_optional(&self.pool)
        .await?;
        Ok(ret)
    }
}
