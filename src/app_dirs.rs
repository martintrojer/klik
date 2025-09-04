use directories::ProjectDirs;
use std::path::PathBuf;

/// Centralized application directory resolution
pub struct AppDirs;

impl AppDirs {
    pub fn db_path() -> Option<PathBuf> {
        if let Ok(home) = std::env::var("HOME") {
            let state_dir = PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("klik");
            Some(state_dir.join("stats.db"))
        } else {
            ProjectDirs::from("", "", "klik")
                .map(|proj_dirs| proj_dirs.data_local_dir().join("stats.db"))
        }
    }
}
