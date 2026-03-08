use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Video file types (mirror backend) ─────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioStream {
    pub index: i64,
    pub codec: String,
    pub language: String,
    pub title: String,
    pub channels: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubtitleStream {
    pub index: i64,
    pub codec: String,
    pub language: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanResponse {
    pub files: Vec<VideoFile>,
    pub clusters: Vec<Vec<String>>,
    pub directory: String,
}

// ── Mapping ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mapping {
    pub new_name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: MappingKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MappingKind {
    Episode,
    Feature,
    Movie,
    Special,
}

// ── Rename ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameMapping {
    pub old_path: String,
    pub new_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameResult {
    pub old: String,
    pub new: String,
    pub success: bool,
    pub skipped: Option<bool>,
    pub error: Option<String>,
}

// ── TVmaze ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShowResult {
    pub id: u64,
    pub name: String,
    pub year: String,
    pub summary: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpisodeResult {
    pub season: u64,
    pub episode: u64,
    pub title: String,
    pub runtime: Option<u64>,
    pub airdate: String,
    pub label: String,
    pub filename_stem: String,
}

// ── TMDB ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovieResult {
    pub id: u64,
    pub title: String,
    pub year: String,
    pub overview: String,
}

// ── DVDCompare ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvdSearchResult {
    pub compid: String,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvdFeature {
    pub title: String,
    pub timecodes: Vec<String>,
}

// ── App state ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub enum SortMode {
    #[default]
    Path,
    Duration,
    Size,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ActiveTab {
    #[default]
    Results,
    Episodes,
    Features,
    Dvd,
}

pub type Mappings = HashMap<String, Mapping>;
