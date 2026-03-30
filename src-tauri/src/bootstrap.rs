use anyhow::Context;
use autoclick_diagnostics::logging;
use autoclick_domain::paths::AppPaths;
use tauri::{App, Manager};
use tracing::info;

use crate::app_state::AppState;

pub fn bootstrap_runtime(app: &mut App) -> anyhow::Result<()> {
    let base_dir = app.path().app_data_dir().context("无法解析应用数据目录")?;
    let paths = AppPaths::from_base_dir(base_dir);

    for path in paths.required_directories() {
        std::fs::create_dir_all(path)
            .with_context(|| format!("无法创建目录: {}", path.display()))?;
    }

    logging::init_logging(&paths.log_dir)?;

    if let Some(state) = app.try_state::<AppState>() {
        state.set_paths(paths.clone());
    }

    info!("应用目录初始化完成: {}", paths.data_dir.display());
    Ok(())
}
