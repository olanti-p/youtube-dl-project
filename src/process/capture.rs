use bytelines::AsyncByteLines;
use std::process::Stdio;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::task::JoinHandle;

pub async fn read_output_to_log<T: AsyncRead + Unpin>(
    output_channel: T,
    mut log_file: File,
) -> anyhow::Result<()> {
    let stderr_reader = BufReader::new(output_channel);
    let mut lines = AsyncByteLines::new(stderr_reader);
    while let Some(line) = lines.next().await? {
        log_file.write(line).await?;
        log_file.write_u8(b'\n').await?;
    }
    Ok(())
}

pub async fn read_output_to_buf_and_log<T: AsyncRead + Unpin>(
    output_channel: T,
    mut log_file: File,
) -> anyhow::Result<Vec<u8>> {
    let mut buf = Vec::<u8>::new();
    let stderr_reader = BufReader::new(output_channel);
    let mut lines = AsyncByteLines::new(stderr_reader);
    while let Some(line) = lines.next().await? {
        log_file.write(line).await?;
        log_file.write_u8(b'\n').await?;
        buf.extend_from_slice(line);
        buf.push(b'\n');
    }
    Ok(buf)
}

pub async fn read_output_to_buf<T: AsyncRead + Unpin>(
    output_channel: T,
) -> anyhow::Result<Vec<u8>> {
    let mut buf = Vec::<u8>::new();
    let stderr_reader = BufReader::new(output_channel);
    let mut lines = AsyncByteLines::new(stderr_reader);
    while let Some(line) = lines.next().await? {
        buf.extend_from_slice(line);
        buf.push(b'\n');
    }
    Ok(buf)
}

pub struct RunResult {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub async fn run_command_to_end(mut command: Command) -> anyhow::Result<RunResult> {
    let mut join_handle = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = join_handle.stdout.take().unwrap();
    let stderr = join_handle.stderr.take().unwrap();

    let stdout_logger: JoinHandle<anyhow::Result<Vec<u8>>> =
        tokio::spawn(async move { read_output_to_buf(stdout).await });
    let stderr_logger: JoinHandle<anyhow::Result<Vec<u8>>> =
        tokio::spawn(async move { read_output_to_buf(stderr).await });

    let exit_code = join_handle.wait().await?.code();

    let stdout = stdout_logger.await.unwrap().unwrap();
    let stderr = stderr_logger.await.unwrap().unwrap();

    Ok(RunResult {
        exit_code,
        stdout,
        stderr,
    })
}
