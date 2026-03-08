use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use snafu::{ResultExt, Snafu};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{sync::Semaphore, task::JoinSet};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

use crate::{
    error::AppError,
    ffprobe::{self, FfprobeError},
    state::AppState,
    types::{ScanResponse, VideoFile},
};

const SCAN_EXTENSIONS: &[&str] = &["mkv", "mp4", "avi", "m2ts", "ts"];

#[derive(Debug, Snafu)]
pub enum ScanError {
    #[snafu(display("Not a directory: {path:?}"))]
    NotADirectory { path: PathBuf },

    #[snafu(display("Failed to probe {path:?}: {source}"))]
    Probe { path: PathBuf, source: FfprobeError },

    #[snafu(display("Probe task panicked: {message}"))]
    TaskPanic { message: String },
}

#[derive(Deserialize)]
pub struct ScanParams {
    dir: Option<String>,
}

pub async fn handler(
    State(state): State<AppState>,
    Query(params): Query<ScanParams>,
) -> Result<Json<ScanResponse>, AppError> {
    let dir = params.dir.unwrap_or(state.default_dir);
    let base = PathBuf::from(&dir);

    if !base.is_dir() {
        warn!("scan rejected: not a directory: {dir}");
        return Err(AppError::bad_request(format!("Not a directory: {dir}")));
    }

    info!("scanning {dir}");
    let files = scan_directory(&base)
        .await
        .map_err(|e| AppError::internal(e))?;

    let clusters = cluster_by_duration(&files);
    info!(
        "scan complete: {} files, {} clusters",
        files.len(),
        clusters.len()
    );

    Ok(Json(ScanResponse { files, clusters, directory: dir }))
}

async fn scan_directory(base: &Path) -> Result<Vec<VideoFile>, ScanError> {
    let entries: Vec<PathBuf> = WalkDir::new(base)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| SCAN_EXTENSIONS.contains(&x.to_lowercase().as_str()))
                    .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    let concurrency = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    info!(
        "probing {} files with concurrency={}",
        entries.len(),
        concurrency
    );

    let sem = Arc::new(Semaphore::new(concurrency));
    let base = Arc::<Path>::from(base);
    let mut set: JoinSet<Result<VideoFile, ScanError>> = JoinSet::new();

    for path in entries {
        let sem = sem.clone();
        let base = base.clone();
        set.spawn(async move {
            let _permit = sem.acquire_owned().await.unwrap();
            debug!("probing {}", path.display());
            let probe = ffprobe::probe_file(&path)
                .await
                .context(ProbeSnafu { path: path.clone() })?;

            let rel_path = path
                .strip_prefix(&*base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            Ok(VideoFile {
                id: path.to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string(),
                filename: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                stem: path.file_stem().unwrap_or_default().to_string_lossy().to_string(),
                parent: path.parent().unwrap_or(&base).to_string_lossy().to_string(),
                rel_path,
                duration: probe.duration,
                duration_fmt: probe.duration_fmt,
                size: probe.size,
                size_fmt: probe.size_fmt,
                video_streams: probe.video_streams,
                audio_streams: probe.audio_streams,
                subtitle_streams: probe.subtitle_streams,
                title: probe.title,
            })
        });
    }

    let mut files = Vec::with_capacity(set.len());
    while let Some(res) = set.join_next().await {
        match res {
            Ok(Ok(file)) => files.push(file),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(ScanError::TaskPanic { message: e.to_string() }),
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn cluster_by_duration(files: &[VideoFile]) -> Vec<Vec<String>> {
    const TOLERANCE_SEC: f64 = 10.0;

    let mut sorted: Vec<&VideoFile> =
        files.iter().filter(|f| f.duration > 0.0).collect();
    sorted.sort_by(|a, b| a.duration.partial_cmp(&b.duration).unwrap_or(std::cmp::Ordering::Equal));

    let mut clusters: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for file in sorted {
        if let Some(last_id) = current.last() {
            let last_dur = files
                .iter()
                .find(|f| &f.id == last_id)
                .map(|f| f.duration)
                .unwrap_or(0.0);

            if (file.duration - last_dur).abs() <= TOLERANCE_SEC {
                current.push(file.id.clone());
            } else {
                if current.len() > 1 {
                    clusters.push(current);
                }
                current = vec![file.id.clone()];
            }
        } else {
            current.push(file.id.clone());
        }
    }

    if current.len() > 1 {
        clusters.push(current);
    }

    clusters
}

