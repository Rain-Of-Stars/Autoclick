#![cfg(target_os = "windows")]

use std::time::Duration;

use autoclick_capture::{
    WgcCaptureOptions,
    session::{CaptureSessionConfig, CaptureTarget},
    single_frame::capture_single_frame,
};
use autoclick_platform_win::{monitor::enumerate_monitors, window::enumerate_windows};

#[test]
fn windows_integration_enumerates_monitors() {
    let monitors = enumerate_monitors().expect("枚举显示器失败");
    assert!(!monitors.is_empty(), "至少应检测到一个显示器");
    assert!(
        monitors.iter().any(|monitor| monitor.is_primary),
        "至少应存在一个主显示器"
    );
}

#[test]
fn windows_integration_enumerates_visible_windows() {
    let windows = enumerate_windows().expect("枚举窗口失败");
    assert!(!windows.is_empty(), "至少应检测到一个顶层窗口");
    assert!(
        windows
            .iter()
            .any(|window| window.is_visible && (window.rect.right - window.rect.left) > 0),
        "至少应存在一个可见窗口"
    );
}

#[test]
fn windows_integration_single_frame_preview_smoke() {
    let result = capture_single_frame(
        CaptureSessionConfig {
            target: CaptureTarget::Monitor { handle: None },
            options: WgcCaptureOptions::default(),
        },
        Duration::from_millis(1_500),
    );

    match result {
        Ok(frame) => {
            assert!(frame.width > 0);
            assert!(frame.height > 0);
        }
        Err(error) => {
            eprintln!("单帧预览在当前桌面环境中不可用: {error}");
        }
    }
}

#[test]
fn windows_integration_window_single_frame_preview_smoke() {
    let Some(window) = enumerate_windows()
        .expect("枚举窗口失败")
        .into_iter()
        .find(|window| {
            window.is_visible
                && !window.is_minimized
                && window.rect.width() > 120
                && window.rect.height() > 120
        })
    else {
        eprintln!("当前桌面环境中没有可用于窗口捕获的候选窗口");
        return;
    };

    let result = capture_single_frame(
        CaptureSessionConfig {
            target: CaptureTarget::Window { hwnd: window.hwnd },
            options: WgcCaptureOptions::default(),
        },
        Duration::from_millis(1_500),
    );

    match result {
        Ok(frame) => {
            assert!(frame.width > 0);
            assert!(frame.height > 0);
        }
        Err(error) => {
            eprintln!(
                "窗口单帧预览在当前桌面环境中不可用: {error}; hwnd={}; title={}",
                window.hwnd, window.title
            );
        }
    }
}
