use std::{fs, path::Path};

use autoclick_diagnostics::error_code::ErrorCode;
use autoclick_diagnostics::export_bundle::export_bundle;
use autoclick_domain::paths::AppPaths;
use serde::Serialize;
use tauri::State;

use crate::{
    app_state::AppState,
    commands::error::{CommandResult, command_error},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFileEntry {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsOverview {
    pub paths: AppPaths,
    pub runtime: crate::runtime_controller::RuntimeControllerSnapshot,
    pub logs: Vec<LogFileEntry>,
}

#[tauri::command]
pub fn get_diagnostics_overview(state: State<'_, AppState>) -> CommandResult<DiagnosticsOverview> {
    let paths = state
        .app_paths()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    let logs = list_files(&paths.log_dir)
        .map_err(|err| command_error(ErrorCode::DiagnosticsExportFailed, err.to_string()))?;
    Ok(DiagnosticsOverview {
        paths,
        runtime: state.runtime.snapshot(),
        logs,
    })
}

#[tauri::command]
pub fn export_diagnostics_bundle(state: State<'_, AppState>) -> CommandResult<String> {
    let paths = state
        .app_paths()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    let config = state
        .load_or_default_config()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    export_bundle(&paths, &config, &state.runtime.snapshot())
        .map(|summary| summary.archive_path.to_string_lossy().to_string())
        .map_err(|err| command_error(ErrorCode::DiagnosticsExportFailed, err.to_string()))
}

fn list_files(dir: &Path) -> anyhow::Result<Vec<LogFileEntry>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let meta = entry.metadata()?;
            entries.push(LogFileEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string(),
                size_bytes: meta.len(),
            });
        }
    }
    entries.sort_by(|left, right| right.name.cmp(&left.name));
    Ok(entries)
}
