use axum::{
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::Response,
    Json,
};
use serde::Deserialize;
use snafu::Snafu;
use std::path::PathBuf;
use tokio::process::Command;
use tokio_util::io::ReaderStream;
use tracing::{debug, info};

use crate::{error::AppError, state::AppState};

#[derive(Debug, Snafu)]
pub enum PreviewError {
    #[snafu(display("File not found: {path:?}"))]
    NotFound { path: PathBuf },

    #[snafu(display("Failed to spawn ffmpeg: {source}"))]
    Spawn { source: std::io::Error },
}

#[derive(Deserialize)]
pub struct PreviewParams {
    pub path: String,
    #[serde(default = "default_start")]
    pub start: f64,
    #[serde(default = "default_duration")]
    pub duration: f64,
}

fn default_start() -> f64 {
    30.0
}
fn default_duration() -> f64 {
    12.0
}

pub async fn handler(
    State(state): State<AppState>,
    Query(params): Query<PreviewParams>,
) -> Result<Response, AppError> {
    let path = PathBuf::from(&params.path);
    if !path.exists() {
        return Err(AppError::not_found(format!("File not found: {}", params.path)));
    }

    // Kill any stale preview for this path first.
    kill_preview(&state, &path).await;

    let use_copy = detect_h264(&path).await;
    info!(
        path = %path.display(),
        start = params.start,
        duration = params.duration,
        stream_copy = use_copy,
        "starting preview"
    );

    // ffmpeg-sidecar manages finding the ffmpeg binary; here we use its
    // located path via FfmpegCommand::new() but spawn a raw tokio Child so
    // we can stream stdout directly into the Axum response body.
    let ffmpeg_path = ffmpeg_sidecar::paths::ffmpeg_path();

    let mut cmd = Command::new(&ffmpeg_path);
    cmd.args(["-hide_banner", "-ss", &params.start.to_string(), "-i"])
        .arg(&path)
        .args(["-t", &params.duration.to_string()]);

    if use_copy {
        cmd.args(["-c:v", "copy"]);
    } else {
        // scale filter ensures even dimensions required by libx264
        cmd.args([
            "-c:v", "libx264",
            "-preset", "ultrafast",
            "-crf", "26",
            "-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2",
        ]);
    }

    cmd.args([
        "-c:a", "aac",
        "-ac", "2",
        "-b:a", "192k",
        "-f", "mp4",
        "-movflags", "frag_keyframe+empty_moov+default_base_moof",
        "pipe:1",
    ])
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::inherit());

    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::internal(format!("Failed to spawn ffmpeg: {e}")))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::internal("Could not capture ffmpeg stdout"))?;

    state.previews.lock().await.insert(path, child);

    let stream = ReaderStream::new(stdout);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "video/mp4")
        .header("Cache-Control", "no-cache")
        .body(Body::from_stream(stream))
        .unwrap())
}

#[derive(Deserialize)]
pub struct StopParams {
    path: String,
}

pub async fn stop_handler(
    State(state): State<AppState>,
    Query(params): Query<StopParams>,
) -> Json<serde_json::Value> {
    kill_preview(&state, &PathBuf::from(&params.path)).await;
    Json(serde_json::json!({ "ok": true }))
}

pub async fn kill_preview(state: &AppState, path: &PathBuf) {
    let mut previews = state.previews.lock().await;
    if let Some(mut child) = previews.remove(path) {
        debug!(path = %path.display(), "killing stale preview process");
        let _ = child.kill().await;
    }
}

/// Returns true if the file's first video stream is already H.264 (stream-copy safe).
async fn detect_h264(path: &PathBuf) -> bool {
    let Ok(output) = Command::new("ffprobe")
        .args(["-v", "quiet", "-print_format", "json", "-show_streams"])
        .arg(path)
        .output()
        .await
    else {
        return false;
    };

    let Ok(data) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return false;
    };

    data["streams"]
        .as_array()
        .map(|streams| {
            streams.iter().any(|s| {
                s["codec_type"] == "video" && s["codec_name"].as_str() == Some("h264")
            })
        })
        .unwrap_or(false)
}
