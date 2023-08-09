use crate::download_manager::command;
use crate::download_manager::command::{ChildWorker, WorkerError};
use crate::download_manager::WorkerControlHandle;
use crate::env::YtdlpConfig;
use crate::filesystem::FilesystemDriver;
use crate::playlist::{PlaylistInfo, VideoInfo, VideoOrPlaylist};
use crate::process::{read_output_to_buf_and_log, read_output_to_log};
use crate::task::{Task, TaskProgress, TaskResultData};
use rocket::serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::process::{ChildStderr, ChildStdout};
use tokio::sync::Mutex;

#[derive(Deserialize)]
struct DeserializeSingleVideoThumbnail {
    pub url: String,
}

#[derive(Deserialize)]
struct DeserializerSingleVideo {
    pub original_url: String,
    pub title: String,
    pub thumbnails: Vec<DeserializeSingleVideoThumbnail>,
}

#[derive(Deserialize)]
struct DeserializePlaylistVideoThumbnail {
    pub url: String,
}

#[derive(Deserialize)]
struct DeserializerPlaylistVideo {
    pub url: String,
    pub title: String,
    pub thumbnails: Vec<DeserializePlaylistVideoThumbnail>,
}

#[derive(Deserialize)]
struct DeserializePlaylistThumbnail {
    pub url: String,
}

#[derive(Deserialize)]
struct DeserializerPlaylist {
    pub original_url: String,
    pub title: String,
    pub entries: Vec<DeserializerPlaylistVideo>,
    pub thumbnails: Vec<DeserializePlaylistThumbnail>,
}

#[derive(Deserialize)]
struct DeserializerCheckType {
    #[serde(rename = "_type")]
    pub check_type: String,
}

fn parse_single_video(data: Value) -> anyhow::Result<VideoInfo> {
    let video: DeserializerSingleVideo = serde_json::from_value(data)?;
    Ok(VideoInfo {
        url: video.original_url.clone(),
        title: video.title.clone(),
        thumbnail: video.thumbnails.get(0).map(|x| x.url.to_string()),
    })
}

fn parse_playlist(data: Value) -> anyhow::Result<PlaylistInfo> {
    let playlist: DeserializerPlaylist = serde_json::from_value(data)?;
    let videos: anyhow::Result<Vec<VideoInfo>> = playlist
        .entries
        .iter()
        .map(|video| -> anyhow::Result<VideoInfo> {
            Ok(VideoInfo {
                url: video.url.clone(),
                title: video.title.clone(),
                thumbnail: video.thumbnails.get(0).map(|x| x.url.to_string()),
            })
        })
        .collect();
    Ok(PlaylistInfo {
        url: playlist.original_url,
        thumbnail: playlist.thumbnails.get(0).map(|x| x.url.clone()),
        title: playlist.title,
        videos: videos?,
    })
}

fn parse_url_video_list(data: Value) -> anyhow::Result<VideoOrPlaylist> {
    let check_type: DeserializerCheckType = serde_json::from_value(data.clone())?;
    if check_type.check_type == "video" {
        Ok(VideoOrPlaylist::Video(parse_single_video(data)?))
    } else if check_type.check_type == "playlist" {
        Ok(VideoOrPlaylist::Playlist(parse_playlist(data)?))
    } else {
        Err(anyhow!("failed to parse downloader output: expected '_type' to be 'video' or 'playlist', got {:?}", check_type.check_type))
    }
}

pub async fn run_task_fetch_url_contents(
    task: Task,
    ytdlp: Arc<YtdlpConfig>,
    fs: Arc<FilesystemDriver>,
    _progress: Arc<Mutex<TaskProgress>>,
    control_handle: WorkerControlHandle,
) -> anyhow::Result<TaskResultData> {
    fs.create_worker_data_dir(task.task_id).await?;
    let _ = fs.create_worker_log_dir(task.task_id).await;

    let mut command = command::new_downloader_command();
    let args = ytdlp.render_fetch_url_command(&task.url);
    for arg in args {
        command.arg(arg);
    }

    let (stdout_file, stderr_file) = fs.make_log_files(task.task_id).await?;

    let stdout_buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let stdout_copy = stdout_buf.clone();
    let stdout_reader = async move |stdout: ChildStdout| -> anyhow::Result<()> {
        let buf = read_output_to_buf_and_log(stdout, stdout_file).await?;
        *stdout_copy.lock().await = buf;
        Ok(())
    };

    let stderr_reader = async move |stderr: ChildStderr| -> anyhow::Result<()> {
        read_output_to_log(stderr, stderr_file).await
    };

    let exit_status =
        ChildWorker::run(&mut command, control_handle, stdout_reader, stderr_reader).await?;

    if exit_status.success() {
        let buf_lock = stdout_buf.lock().await;
        let data: Value = serde_json::from_slice(&buf_lock)?;
        let contents = parse_url_video_list(data)?;
        Ok(TaskResultData::FetchUrlContents(contents))
    } else {
        Err(anyhow!(WorkerError::BadExitCode(exit_status.code())))
    }
}
