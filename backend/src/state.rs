use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{process::Child, sync::Mutex};

#[derive(Clone)]
pub struct AppState {
    /// Root directory all relative scan paths are resolved against.
    pub root_dir: String,
    /// Maps file path -> running ffmpeg preview process.
    pub previews: Arc<Mutex<HashMap<PathBuf, Child>>>,
}

impl AppState {
    pub fn new(root_dir: String) -> Self {
        Self {
            root_dir,
            previews: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
