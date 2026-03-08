use axum::Json;
use serde::Deserialize;
use snafu::Snafu;
use std::path::PathBuf;
use tracing::{info, warn};

use crate::{
    error::AppError,
    types::{RenameMapping, RenameResult},
};

#[derive(Debug, Snafu)]
pub enum RenameError {
    #[snafu(display("Target already exists: {path:?}"))]
    TargetExists { path: PathBuf },

    #[snafu(display("Failed to rename {from:?} to {to:?}: {source}"))]
    Rename { from: PathBuf, to: PathBuf, source: std::io::Error },
}

#[derive(Deserialize)]
pub struct RenameRequest {
    pub mappings: Vec<RenameMapping>,
}

pub async fn handler(
    Json(body): Json<RenameRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if body.mappings.is_empty() {
        return Err(AppError::bad_request("No mappings provided"));
    }

    let results = apply_renames(body.mappings);
    Ok(Json(serde_json::json!({ "results": results })))
}

fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .filter(|c| !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
        .collect();
    let collapsed = sanitized.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(200).collect()
}

fn apply_renames(mappings: Vec<RenameMapping>) -> Vec<RenameResult> {
    mappings
        .into_iter()
        .map(|m| {
            let old_path = PathBuf::from(&m.old_path);
            let new_stem = sanitize_filename(&m.new_name);
            let ext = old_path
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();
            let new_path = old_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join(format!("{new_stem}{ext}"));

            if new_path == old_path {
                return RenameResult {
                    old: m.old_path,
                    new: new_path.to_string_lossy().to_string(),
                    success: true,
                    skipped: true,
                    error: None,
                };
            }

            if new_path.exists() {
                return RenameResult {
                    old: m.old_path,
                    new: new_path.to_string_lossy().to_string(),
                    success: false,
                    skipped: false,
                    error: Some("Target already exists".to_string()),
                };
            }

            match std::fs::rename(&old_path, &new_path) {
                Ok(()) => {
                    info!(
                        from = %old_path.display(),
                        to = %new_path.display(),
                        "renamed"
                    );
                    RenameResult {
                        old: m.old_path,
                        new: new_path.to_string_lossy().to_string(),
                        success: true,
                        skipped: false,
                        error: None,
                    }
                }
                Err(e) => {
                    warn!(
                        from = %old_path.display(),
                        to = %new_path.display(),
                        error = %e,
                        "rename failed"
                    );
                    RenameResult {
                        old: m.old_path,
                        new: new_path.to_string_lossy().to_string(),
                        success: false,
                        skipped: false,
                        error: Some(e.to_string()),
                    }
                }
            }
        })
        .collect()
}
