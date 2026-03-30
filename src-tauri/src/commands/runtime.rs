use autoclick_diagnostics::error_code::ErrorCode;
use autoclick_domain::{
    config::{AppConfig, CaptureSource},
    paths::AppPaths,
    user_message::UserMessage,
};
use autoclick_platform_win::locator::{LocatorCandidate, locate_target_window};
use autoclick_runtime::preview_bus::PreviewMessage;
use tauri::{AppHandle, State};

use crate::{
    app_state::AppState,
    commands::error::{CommandResult, command_error},
    runtime_controller::RuntimeControllerSnapshot,
    tray,
};

fn current_process_name() -> Option<String> {
    std::env::current_exe().ok().and_then(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
    })
}

fn targets_current_app_window(config: &AppConfig) -> bool {
    let Some(current_name) = current_process_name() else {
        return false;
    };

    config
        .target
        .process_name
        .as_deref()
        .map(|name| name.eq_ignore_ascii_case(&current_name))
        .unwrap_or(false)
        || config
            .target
            .process_path
            .as_deref()
            .map(|path| {
                path.to_ascii_lowercase()
                    .ends_with(&current_name.to_ascii_lowercase())
            })
            .unwrap_or(false)
}

fn validate_runtime_target(config: &AppConfig) -> Result<Option<LocatorCandidate>, UserMessage> {
    match config.capture.source {
        CaptureSource::Window => {
            let located = locate_target_window(&config.target)
                .map_err(|err| command_error(ErrorCode::CaptureUnavailable, err.to_string()))?
                .ok_or_else(|| {
                    command_error(
                        ErrorCode::CaptureUnavailable,
                        if targets_current_app_window(config) {
                            "当前选择的是 Autoclick 主窗口，系统已阻止自捕获。请在“目标窗口”中重新选择外部目标窗口。"
                        } else {
                            "未找到匹配的目标窗口，请重新打开“目标窗口”页面选择目标"
                        }
                    )
                })?;
            Ok(Some(located))
        }
        CaptureSource::Monitor => Ok(None),
    }
}

pub(crate) fn load_runtime_launch_context(
    state: &AppState,
) -> Result<(AppPaths, AppConfig, Option<LocatorCandidate>), UserMessage> {
    let app_paths = state
        .app_paths()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    let mut config = state
        .load_or_default_config()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    if config.templates.is_empty() {
        config.templates = state
            .list_templates()
            .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    }
    let prefetched_target = validate_runtime_target(&config)?;
    Ok((app_paths, config, prefetched_target))
}

#[tauri::command]
pub fn start_runtime(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<RuntimeControllerSnapshot> {
    let (app_paths, config, prefetched_target) = load_runtime_launch_context(state.inner())?;
    let snapshot = state
        .runtime
        .start(app_paths, config, prefetched_target)
        .map_err(|err| command_error(ErrorCode::RuntimeFault, err))?;
    let _ = tray::sync_tray(&app);
    Ok(snapshot)
}

#[tauri::command]
pub fn stop_runtime(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<RuntimeControllerSnapshot> {
    let snapshot = state
        .runtime
        .stop()
        .map_err(|err| command_error(ErrorCode::RuntimeFault, err))?;
    let _ = tray::sync_tray(&app);
    Ok(snapshot)
}

#[tauri::command]
pub fn restart_runtime(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<RuntimeControllerSnapshot> {
    let (app_paths, config, prefetched_target) = load_runtime_launch_context(state.inner())?;
    let snapshot = state
        .runtime
        .restart(app_paths, config, prefetched_target)
        .map_err(|err| command_error(ErrorCode::RuntimeFault, err))?;
    let _ = tray::sync_tray(&app);
    Ok(snapshot)
}

#[tauri::command]
pub fn get_runtime_status(state: State<'_, AppState>) -> RuntimeControllerSnapshot {
    state.runtime.snapshot()
}

#[tauri::command]
pub fn get_preview_snapshot(state: State<'_, AppState>) -> Option<PreviewMessage> {
    state.runtime.snapshot().preview
}
