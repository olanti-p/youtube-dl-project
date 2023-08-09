use crate::download_manager::worker_handle::WorkerControlHandle;
use crate::process::kill_child_process;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;
use tokio::process::{ChildStderr, ChildStdout, Command};
use tokio::task::JoinHandle;
use tracing::{info, warn};

#[derive(Debug)]
pub enum WorkerError {
    Aborted,
    Paused,
    BadExitCode(Option<i32>),
}

impl Display for WorkerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerError::Aborted => {
                write!(f, "task has been cancelled")
            }
            WorkerError::Paused => {
                write!(f, "task has been paused")
            }
            WorkerError::BadExitCode(code) => {
                if let Some(code) = code {
                    write!(f, "child exited with code: {code}")
                } else {
                    write!(f, "child exited with code: None (terminated?)")
                }
            }
        }
    }
}

impl Error for WorkerError {}

pub struct ChildWorker<FnStdOut, FnStdErr, Fut1, Fut2>
where
    FnStdOut: FnOnce(ChildStdout) -> Fut1 + Send + 'static,
    FnStdErr: FnOnce(ChildStderr) -> Fut2 + Send + 'static,
    Fut1: Future<Output = anyhow::Result<()>> + Send,
    Fut2: Future<Output = anyhow::Result<()>> + Send,
{
    _phantom: PhantomData<(FnStdOut, FnStdErr)>,
}

impl<FnStdOut, FnStdErr, Fut1, Fut2> ChildWorker<FnStdOut, FnStdErr, Fut1, Fut2>
where
    FnStdOut: FnOnce(ChildStdout) -> Fut1 + Send + 'static,
    FnStdErr: FnOnce(ChildStderr) -> Fut2 + Send + 'static,
    Fut1: Future<Output = anyhow::Result<()>> + Send,
    Fut2: Future<Output = anyhow::Result<()>> + Send,
{
    pub async fn run(
        command: &mut Command,
        control_handle: WorkerControlHandle,
        stdout_reader: FnStdOut,
        stderr_reader: FnStdErr,
    ) -> anyhow::Result<ExitStatus> {
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        info!("Spawning child with command: {command:?}");

        let mut child = command.spawn()?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_reporter: JoinHandle<anyhow::Result<()>> =
            tokio::spawn(async move { stdout_reader(stdout).await });

        let stderr_reporter: JoinHandle<anyhow::Result<()>> =
            tokio::spawn(async move { stderr_reader(stderr).await });

        loop {
            let is_stopped = control_handle.is_stopped();
            let is_paused = control_handle.is_paused();
            if is_stopped || is_paused {
                stderr_reporter.abort();
                stdout_reporter.abort();
                if let Err(e) = kill_child_process(&child).await {
                    warn!("Failed to kill child: {e}");
                }

                // We expect either Ok() or Err(JoinError::Cancelled(...))
                let _ = stdout_reporter.await;
                let _ = stderr_reporter.await;

                // We expect Ok(ExitStatus(...))
                let _ = child.wait().await?;

                let err = if is_stopped {
                    WorkerError::Aborted
                } else {
                    WorkerError::Paused
                };
                return Err(anyhow!(err));
            }

            let wait_result = child.try_wait();
            if wait_result.is_err() || (wait_result.is_ok() && wait_result.unwrap().is_some()) {
                break;
            }

            rocket::tokio::time::sleep(Duration::from_millis(250)).await;
        }

        let stdout_join_res = stdout_reporter.await;
        let stderr_join_res = stderr_reporter.await;
        let child_join_res = child.wait().await;

        // We expect Ok(Ok(())
        stdout_join_res??;
        stderr_join_res??;

        // We expect Ok(ExitStatus(...))
        Ok(child_join_res?)
    }
}

pub fn new_downloader_command() -> Command {
    Command::new("yt-dlp.exe")
}
