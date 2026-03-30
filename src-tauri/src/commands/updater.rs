use autoclick_diagnostics::error_code::ErrorCode;
use tauri::AppHandle;

use crate::{
    commands::error::{CommandResult, command_error},
    updater::{UpdateCheckResult, UpdaterStatus},
};

#[tauri::command]
pub fn get_updater_status() -> CommandResult<UpdaterStatus> {
    Ok(crate::updater::status())
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> CommandResult<UpdateCheckResult> {
    crate::updater::check_for_updates(&app)
        .await
        .map_err(|err| command_error(ErrorCode::RuntimeFault, err.to_string()))
}
