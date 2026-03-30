use serde::{Deserialize, Serialize};
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    UI::WindowsAndMessaging::{
        CWP_SKIPDISABLED, CWP_SKIPINVISIBLE, ChildWindowFromPointEx, GetWindowRect,
    },
};

use crate::PlatformError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HitTestResult {
    pub target_hwnd: isize,
    pub client_x: i32,
    pub client_y: i32,
}

pub fn child_window_from_point(
    hwnd: isize,
    screen_x: i32,
    screen_y: i32,
) -> Result<HitTestResult, PlatformError> {
    if hwnd == 0 {
        return Err(PlatformError::InvalidArgument("hwnd must not be zero"));
    }
    let mut point = POINT {
        x: screen_x,
        y: screen_y,
    };
    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd_from_isize(hwnd), &mut rect)
            .map_err(|err| PlatformError::Win32(err.to_string()))?;
        point.x -= rect.left;
        point.y -= rect.top;
        let child = ChildWindowFromPointEx(
            hwnd_from_isize(hwnd),
            point,
            CWP_SKIPDISABLED | CWP_SKIPINVISIBLE,
        );
        let target = if child.0.is_null() {
            hwnd
        } else {
            child.0 as isize
        };
        Ok(HitTestResult {
            target_hwnd: target,
            client_x: point.x,
            client_y: point.y,
        })
    }
}

pub fn client_coordinates(
    window_left: i32,
    window_top: i32,
    screen_x: i32,
    screen_y: i32,
) -> (i32, i32) {
    (screen_x - window_left, screen_y - window_top)
}

fn hwnd_from_isize(hwnd: isize) -> HWND {
    HWND(hwnd as *mut _)
}

#[cfg(test)]
mod tests {
    use super::client_coordinates;

    #[test]
    fn converts_negative_screen_coordinates() {
        assert_eq!(client_coordinates(-1200, 100, -1100, 160), (100, 60));
    }
}
