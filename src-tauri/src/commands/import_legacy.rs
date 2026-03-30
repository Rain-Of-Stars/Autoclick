use std::path::PathBuf;

use autoclick_diagnostics::error_code::ErrorCode;
use autoclick_storage::import_legacy::{LegacyImportReport, dry_run_import, import_legacy};
use serde::Deserialize;
use tauri::State;

use crate::{
    app_state::AppState,
    commands::error::{CommandResult, command_error},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportLegacyRequest {
    pub legacy_root: Option<String>,
}

#[tauri::command]
pub fn dry_run_legacy_import_command(
    state: State<'_, AppState>,
    request: Option<ImportLegacyRequest>,
) -> CommandResult<LegacyImportReport> {
    let app_paths = state
        .app_paths()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    let legacy_root = resolve_legacy_root(request);
    dry_run_import(&app_paths, legacy_root)
        .map_err(|err| command_error(ErrorCode::LegacyImportFailed, err.to_string()))
}

#[tauri::command]
pub fn run_legacy_import_command(
    state: State<'_, AppState>,
    request: Option<ImportLegacyRequest>,
) -> CommandResult<LegacyImportReport> {
    let app_paths = state
        .app_paths()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    let legacy_root = resolve_legacy_root(request);
    let report = import_legacy(&app_paths, legacy_root)
        .map_err(|err| command_error(ErrorCode::LegacyImportFailed, err.to_string()))?;
    let _ = state.sync_templates_into_config();
    Ok(report)
}

fn resolve_legacy_root(request: Option<ImportLegacyRequest>) -> PathBuf {
    request
        .and_then(|value| value.legacy_root)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("tests")
                .join("fixtures")
                .join("legacy_project")
        })
}
