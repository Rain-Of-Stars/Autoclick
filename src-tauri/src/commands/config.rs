use autoclick_diagnostics::error_code::ErrorCode;
use autoclick_domain::{config::AppConfig, paths::AppPaths};
use tauri::State;

use crate::{
    app_state::AppState,
    commands::error::{CommandResult, command_error},
};

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> CommandResult<AppConfig> {
    state
        .load_or_default_config()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))
}

#[tauri::command]
pub fn save_config(state: State<'_, AppState>, config: AppConfig) -> CommandResult<AppConfig> {
    state
        .save_config(&config)
        .map_err(|err| command_error(ErrorCode::ConfigInvalid, err))?;
    Ok(config)
}

#[tauri::command]
pub fn get_app_paths(state: State<'_, AppState>) -> CommandResult<AppPaths> {
    state
        .app_paths()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))
}
