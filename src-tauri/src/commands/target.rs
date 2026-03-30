use std::time::Duration;

use autoclick_capture::{
    WgcCaptureOptions,
    preview_encode::{EncodedPreview, PreviewEncodeOptions, encode_preview},
    session::{CaptureSessionConfig, CaptureTarget},
    single_frame::capture_single_frame,
};
use autoclick_diagnostics::error_code::ErrorCode;
use autoclick_domain::config::TargetProfile;
use autoclick_platform_win::{
    locator::{LocatorCandidate, locate_target_window},
    monitor::{MonitorInfo, enumerate_monitors},
    window::{WindowInfo, enumerate_windows},
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
    app_state::AppState,
    capture_window::{ensure_window_capture_ready, finish_window_capture},
    commands::error::{CommandResult, command_error},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PickTargetWindowRequest {
    pub hwnd: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCaptureRequest {
    pub hwnd: Option<i64>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCaptureResult {
    pub window: Option<WindowInfo>,
    pub preview: EncodedPreview,
}

#[tauri::command]
pub fn list_target_windows() -> CommandResult<Vec<WindowInfo>> {
    enumerate_windows().map_err(|err| command_error(ErrorCode::CaptureUnavailable, err.to_string()))
}

#[tauri::command]
pub fn list_monitors() -> CommandResult<Vec<MonitorInfo>> {
    enumerate_monitors()
        .map_err(|err| command_error(ErrorCode::CaptureUnavailable, err.to_string()))
}

#[tauri::command]
pub fn locate_target(state: State<'_, AppState>) -> CommandResult<Option<LocatorCandidate>> {
    let config = state
        .load_or_default_config()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    locate_target_window(&config.target)
        .map_err(|err| command_error(ErrorCode::CaptureUnavailable, err.to_string()))
}

#[tauri::command]
pub fn pick_target_window(
    state: State<'_, AppState>,
    request: PickTargetWindowRequest,
) -> CommandResult<TargetProfile> {
    if request.hwnd <= 0 {
        return Err(command_error(ErrorCode::CaptureUnavailable, "窗口句柄无效"));
    }
    let windows = enumerate_windows()
        .map_err(|err| command_error(ErrorCode::CaptureUnavailable, err.to_string()))?;
    let selected = windows
        .into_iter()
        .find(|window| window.hwnd == request.hwnd as isize)
        .ok_or_else(|| command_error(ErrorCode::CaptureUnavailable, "未找到指定窗口"))?;

    let mut config = state
        .load_or_default_config()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    config.target.hwnd = Some(request.hwnd);
    config.target.title_contains = if selected.title.is_empty() {
        None
    } else {
        Some(selected.title)
    };
    config.target.class_name = if selected.class_name.is_empty() {
        None
    } else {
        Some(selected.class_name)
    };
    config.target.process_path = selected.exe_path.clone();
    config.target.process_name = selected
        .exe_path
        .as_ref()
        .and_then(|path| std::path::Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .map(ToString::to_string);
    state
        .save_config(&config)
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    Ok(config.target)
}

#[tauri::command]
pub fn test_target_capture(
    state: State<'_, AppState>,
    request: TargetCaptureRequest,
) -> CommandResult<TargetCaptureResult> {
    let config = state
        .load_or_default_config()
        .map_err(|err| command_error(ErrorCode::StorageUnavailable, err))?;
    let hwnd = match request.hwnd {
        Some(hwnd) if hwnd <= 0 => {
            return Err(command_error(ErrorCode::CaptureUnavailable, "窗口句柄无效"));
        }
        Some(hwnd) => hwnd,
        None => config
            .target
            .hwnd
            .ok_or_else(|| command_error(ErrorCode::CaptureUnavailable, "当前未选择目标窗口"))?,
    };
    let window = enumerate_windows()
        .map_err(|err| command_error(ErrorCode::CaptureUnavailable, err.to_string()))?
        .into_iter()
        .find(|item| item.hwnd == hwnd as isize)
        .ok_or_else(|| {
            command_error(
                ErrorCode::CaptureUnavailable,
                "测试捕获失败：当前窗口不可作为目标，请重新选择外部可见窗口",
            )
        })?;
    let restored_from_minimized = ensure_window_capture_ready(&config.capture, hwnd as isize)
        .map_err(|err| command_error(ErrorCode::CaptureUnavailable, err))?;

    let frame_result = capture_single_frame(
        CaptureSessionConfig {
            target: CaptureTarget::Window {
                hwnd: hwnd as isize,
            },
            options: WgcCaptureOptions {
                target_fps: config.capture.target_fps,
                include_cursor: config.capture.include_cursor,
                draw_border: config.capture.window_border_required,
                include_secondary_windows: false,
                remove_title_bar: false,
                dirty_region_enabled: false,
            },
        },
        Duration::from_millis(request.timeout_ms.unwrap_or(1_000).clamp(50, 5_000)),
    );
    let cleanup_result =
        finish_window_capture(&config.capture, hwnd as isize, restored_from_minimized);
    let frame = frame_result
        .map_err(|err| command_error(ErrorCode::CaptureUnavailable, err.to_string()))?;
    cleanup_result.map_err(|err| command_error(ErrorCode::CaptureUnavailable, err))?;

    let preview = encode_preview(&frame, &PreviewEncodeOptions::default())
        .map_err(|err| command_error(ErrorCode::CaptureUnavailable, err.to_string()))?;

    Ok(TargetCaptureResult {
        window: Some(window),
        preview,
    })
}
