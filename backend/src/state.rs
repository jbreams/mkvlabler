use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{process::Child, sync::Mutex};

#[derive(Clone)]
pub struct AppState {
    pub default_dir: String,
    /// Maps file path -> running ffmpeg preview process.
    pub previews: Arc<Mutex<HashMap<PathBuf, Child>>>,
}

impl AppState {
    pub fn new(default_dir: String) -> Self {
        Self {
            default_dir,
            previews: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
