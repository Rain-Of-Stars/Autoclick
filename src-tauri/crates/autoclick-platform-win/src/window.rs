use serde::{Deserialize, Serialize};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, RECT},
        UI::WindowsAndMessaging::{
            EnumWindows, GWL_EXSTYLE, GetClassNameW, GetWindowLongW, GetWindowRect,
            GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
            IsWindowVisible, WS_EX_TOOLWINDOW,
        },
    },
    core::BOOL,
};

use crate::{PlatformError, process};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl WindowRect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub hwnd: isize,
    pub title: String,
    pub class_name: String,
    pub pid: u32,
    pub exe_path: Option<String>,
    pub is_minimized: bool,
    pub is_visible: bool,
    pub rect: WindowRect,
}

pub fn enumerate_windows() -> Result<Vec<WindowInfo>, PlatformError> {
    let mut windows = Vec::new();
    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            LPARAM((&mut windows as *mut Vec<WindowInfo>) as isize),
        )
        .map_err(|err| PlatformError::Win32(err.to_string()))?;
    }
    Ok(windows)
}

pub fn inspect_window(hwnd: isize) -> Result<Option<WindowInfo>, PlatformError> {
    if hwnd == 0 {
        return Ok(None);
    }

    unsafe { collect_window_info(HWND(hwnd as *mut core::ffi::c_void)) }
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };
    if let Ok(Some(window)) = unsafe { collect_window_info(hwnd) } {
        windows.push(window);
    }
    true.into()
}

unsafe fn collect_window_info(hwnd: HWND) -> Result<Option<WindowInfo>, PlatformError> {
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return Ok(None);
    }

    let style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
    if style & WS_EX_TOOLWINDOW.0 != 0 {
        return Ok(None);
    }

    let title = unsafe { window_text(hwnd) };
    let class_name = unsafe { class_name(hwnd) };
    if title.trim().is_empty() && class_name.trim().is_empty() {
        return Ok(None);
    }

    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return Ok(None);
    }
    let rect = WindowRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    };
    if should_exclude_window(pid, &rect) {
        return Ok(None);
    }

    Ok(Some(WindowInfo {
        hwnd: hwnd_to_isize(hwnd),
        title,
        class_name,
        pid,
        exe_path: process::resolve_process_path(pid).ok().flatten(),
        is_minimized: unsafe { IsIconic(hwnd) }.as_bool(),
        is_visible: true,
        rect,
    }))
}

unsafe fn window_text(hwnd: HWND) -> String {
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length <= 0 {
        return String::new();
    }
    let mut buffer = vec![0u16; length as usize + 1];
    let written = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..written as usize])
}

unsafe fn class_name(hwnd: HWND) -> String {
    let mut buffer = vec![0u16; 256];
    let written = unsafe { GetClassNameW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..written as usize])
}

fn hwnd_to_isize(hwnd: HWND) -> isize {
    hwnd.0 as isize
}

fn should_exclude_window(pid: u32, rect: &WindowRect) -> bool {
    // 排除当前应用自身窗口，避免把控制台主窗体当成捕获目标。
    if pid == std::process::id() {
        return true;
    }

    rect.width() <= 0 || rect.height() <= 0
}

#[cfg(test)]
mod tests {
    use super::{WindowRect, should_exclude_window};

    #[test]
    fn computes_rect_dimensions() {
        let rect = WindowRect {
            left: 10,
            top: 20,
            right: 210,
            bottom: 120,
        };
        assert_eq!(rect.width(), 200);
        assert_eq!(rect.height(), 100);
    }

    #[test]
    fn excludes_current_process_window() {
        let rect = WindowRect {
            left: 0,
            top: 0,
            right: 100,
            bottom: 100,
        };

        assert!(should_exclude_window(std::process::id(), &rect));
        assert!(!should_exclude_window(std::process::id() + 1, &rect));
    }

    #[test]
    fn excludes_zero_sized_window() {
        let rect = WindowRect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 100,
        };

        assert!(should_exclude_window(1, &rect));
    }
}
