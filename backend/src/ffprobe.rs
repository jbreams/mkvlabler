use serde_json::Value;
use snafu::{ResultExt, Snafu};
use std::path::Path;
use tokio::process::Command;

use crate::types::{AudioStream, SubtitleStream};

#[derive(Debug, Snafu)]
pub enum FfprobeError {
    #[snafu(display("Failed to spawn ffprobe: {source}"))]
    Spawn { source: std::io::Error },

    #[snafu(display("ffprobe returned non-zero exit status"))]
    NonZeroExit,

    #[snafu(display("Failed to parse ffprobe JSON output: {source}"))]
    ParseJson { source: serde_json::Error },
}

pub struct ProbeResult {
    pub duration: f64,
    pub duration_fmt: String,
    pub size: u64,
    pub size_fmt: String,
    pub video_streams: usize,
    pub audio_streams: Vec<AudioStream>,
    pub subtitle_streams: Vec<SubtitleStream>,
    pub title: String,
}

pub async fn probe_file(path: &Path) -> Result<ProbeResult, FfprobeError> {
    let output = Command::new("ffprobe")
        .args(["-v", "quiet", "-print_format", "json", "-show_format", "-show_streams"])
        .arg(path)
        .output()
        .await
        .context(SpawnSnafu)?;

    if !output.status.success() {
        return Err(FfprobeError::NonZeroExit);
    }

    let data: Value = serde_json::from_slice(&output.stdout).context(ParseJsonSnafu)?;
    let fmt = data["format"].as_object().cloned().unwrap_or_default();
    let empty = vec![];
    let streams = data["streams"].as_array().unwrap_or(&empty);

    let video_streams = streams.iter().filter(|s| s["codec_type"] == "video").count();

    let audio_streams = streams
        .iter()
        .filter(|s| s["codec_type"] == "audio")
        .map(|s| AudioStream {
            index: s["index"].as_i64().unwrap_or(0),
            codec: s["codec_name"].as_str().unwrap_or("").to_string(),
            language: s["tags"]["language"].as_str().unwrap_or("").to_string(),
            title: s["tags"]["title"].as_str().unwrap_or("").to_string(),
            channels: s["channels"].as_i64().unwrap_or(0),
        })
        .collect();

    let subtitle_streams = streams
        .iter()
        .filter(|s| s["codec_type"] == "subtitle")
        .map(|s| SubtitleStream {
            index: s["index"].as_i64().unwrap_or(0),
            codec: s["codec_name"].as_str().unwrap_or("").to_string(),
            language: s["tags"]["language"].as_str().unwrap_or("").to_string(),
            title: s["tags"]["title"].as_str().unwrap_or("").to_string(),
        })
        .collect();

    let duration = fmt
        .get("duration")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    let size = fmt
        .get("size")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let title = fmt
        .get("tags")
        .and_then(|t| t["title"].as_str())
        .unwrap_or("")
        .to_string();

    Ok(ProbeResult {
        duration,
        duration_fmt: fmt_duration(duration),
        size,
        size_fmt: fmt_size(size),
        video_streams,
        audio_streams,
        subtitle_streams,
        title,
    })
}

pub fn fmt_duration(seconds: f64) -> String {
    if seconds == 0.0 {
        return "0:00".to_string();
    }
    let total = seconds as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

pub fn fmt_size(bytes: u64) -> String {
    let mut val = bytes as f64;
    for unit in &["B", "KB", "MB", "GB"] {
        if val < 1024.0 {
            return format!("{val:.1} {unit}");
        }
        val /= 1024.0;
    }
    format!("{val:.1} TB")
}
