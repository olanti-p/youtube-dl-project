use crate::download_manager::command;
use crate::download_manager::command::ChildWorker;
use crate::download_manager::command::WorkerError;
use crate::download_manager::WorkerControlHandle;
use crate::env::YtdlpConfig;
use crate::filesystem::FilesystemDriver;
use crate::process::read_output_to_log;
use crate::task::{Task, TaskProgress, TaskResultData};
use bytelines::AsyncByteLines;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::{ChildStderr, ChildStdout};
use tokio::sync::Mutex;

pub async fn run_task_download_and_convert(
    task: Task,
    ytdlp: Arc<YtdlpConfig>,
    fs: Arc<FilesystemDriver>,
    progress: Arc<Mutex<TaskProgress>>,
    control_handle: WorkerControlHandle,
) -> anyhow::Result<TaskResultData> {
    let src_url = &task.url;
    if task.is_resumed {
        // Remove output file as it could've been corrupted after pausing
        let _ = fs.remove_worker_output_file(&task).await;
    } else {
        // Perform cleanup as there could be leftover files from previous attempt
        let _ = fs.remove_worker_data_dir(task.task_id).await;
    }
    fs.create_worker_data_dir(task.task_id).await?;
    let _ = fs.create_worker_log_dir(task.task_id).await;
    let dst_file = fs.get_ytdlp_output_template(task.task_id);

    let (mut stdout_file, stderr_file) = fs.make_log_files(task.task_id).await?;

    let mut command = command::new_downloader_command();

    // We use `unwrap` here because any misconfiguration is most likely the user's fault
    let format = ytdlp.get_format(&task.format);
    let args = ytdlp.render_download_command(src_url, format, &dst_file);

    for arg in args {
        command.arg(arg);
    }

    let state_stdout_copy = progress.clone();

    let stdout_reader = async move |stdout: ChildStdout| -> anyhow::Result<()> {
        let stdout_reader = BufReader::new(stdout);
        let mut lines = AsyncByteLines::new(stdout_reader);
        while let Some(line_raw) = lines.next().await? {
            stdout_file.write_all(line_raw).await?;
            stdout_file.write_u8(b'\n').await?;
            let line = String::from_utf8_lossy(line_raw);
            if !line.starts_with("[dl]") {
                continue;
            }
            let parts: Vec<&str> = line.split(' ').collect();
            assert_eq!(parts.len(), 4);
            assert_eq!(parts[0], "[dl]");

            let _: Option<Duration> = if let Ok(seconds) = parts[1].parse::<f64>() {
                Some(Duration::from_secs_f64(seconds))
            } else {
                None
            };
            let estimate: Option<i32> = if let Ok(bytes) = parts[2].parse::<f64>() {
                Some(bytes as i32)
            } else {
                None
            };
            let downloaded: Option<i32> = if let Ok(bytes) = parts[3].parse::<i32>() {
                Some(bytes)
            } else {
                None
            };

            let mut state = state_stdout_copy.lock().await;
            if let Some(estimate) = estimate {
                state.bytes_estimate = estimate;
            }
            if let Some(downloaded) = downloaded {
                state.bytes_downloaded = downloaded;
            }
            if let (Some(estimate), Some(downloaded)) = (&estimate, &downloaded) {
                let progress = *downloaded as f64 * 100f64 / *estimate as f64;
                let progress = progress.round() as i32;

                state.percent = progress;
            }
        }
        Ok(())
    };

    let stderr_reader = async move |stderr: ChildStderr| -> anyhow::Result<()> {
        read_output_to_log(stderr, stderr_file).await
    };

    let exit_status =
        ChildWorker::run(&mut command, control_handle, stdout_reader, stderr_reader).await?;

    if exit_status.success() {
        fs.move_output_file(&task).await?;
        Ok(TaskResultData::DownloadAndConvert)
    } else {
        Err(anyhow!(WorkerError::BadExitCode(exit_status.code())))
    }
}
