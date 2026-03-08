use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFile {
    pub id: String,
    pub path: String,
    pub filename: String,
    pub stem: String,
    pub parent: String,
    pub rel_path: String,
    pub duration: f64,
    pub duration_fmt: String,
    pub size: u64,
    pub size_fmt: String,
    pub video_streams: usize,
    pub audio_streams: Vec<AudioStream>,
    pub subtitle_streams: Vec<SubtitleStream>,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStream {
    pub index: i64,
    pub codec: String,
    pub language: String,
    pub title: String,
    pub channels: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleStream {
    pub index: i64,
    pub codec: String,
    pub language: String,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResponse {
    pub files: Vec<VideoFile>,
    pub clusters: Vec<Vec<String>>,
    pub directory: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenameMapping {
    pub old_path: String,
    pub new_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenameResult {
    pub old: String,
    pub new: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub skipped: bool,
}
