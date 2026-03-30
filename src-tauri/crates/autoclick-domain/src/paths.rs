use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
    pub templates_dir: PathBuf,
    pub debug_dir: PathBuf,
    pub db_path: PathBuf,
}

impl AppPaths {
    pub fn from_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        let data_dir = base_dir.join("data");
        let cache_dir = base_dir.join("cache");
        let log_dir = base_dir.join("logs");
        let templates_dir = data_dir.join("templates");
        let debug_dir = cache_dir.join("debug");
        let db_path = data_dir.join("autoclick.db");

        Self {
            data_dir,
            cache_dir,
            log_dir,
            templates_dir,
            debug_dir,
            db_path,
        }
    }

    pub fn required_directories(&self) -> Vec<&Path> {
        vec![
            self.data_dir.as_path(),
            self.cache_dir.as_path(),
            self.log_dir.as_path(),
            self.templates_dir.as_path(),
            self.debug_dir.as_path(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::AppPaths;
    use std::path::PathBuf;

    #[test]
    fn resolves_paths_from_base_dir() {
        let base_dir = PathBuf::from("test-data/autoclick");
        let paths = AppPaths::from_base_dir(base_dir);
        assert!(paths.db_path.ends_with("data/autoclick.db"));
        assert!(paths.templates_dir.ends_with("data/templates"));
        assert!(paths.debug_dir.ends_with("cache/debug"));
    }
}
