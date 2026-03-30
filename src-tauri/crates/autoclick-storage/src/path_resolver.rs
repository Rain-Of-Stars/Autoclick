use std::path::PathBuf;

use autoclick_domain::paths::AppPaths;

pub fn resolve_paths(base_dir: impl Into<PathBuf>) -> AppPaths {
    AppPaths::from_base_dir(base_dir)
}

#[cfg(test)]
mod tests {
    use super::resolve_paths;

    #[test]
    fn resolve_paths_from_any_base_dir() {
        let paths = resolve_paths("test-data/autoclick");
        assert!(paths.db_path.ends_with("data/autoclick.db"));
        assert!(paths.log_dir.ends_with("logs"));
    }
}
