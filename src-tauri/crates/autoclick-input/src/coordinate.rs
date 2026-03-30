use autoclick_detect::r#match::MatchResult;
use autoclick_platform_win::{
    hit_test::{HitTestResult, child_window_from_point},
    window::WindowRect,
    window_state::{is_window_valid, restore_window_no_activate},
};
use serde::{Deserialize, Serialize};
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::IsIconic};

use crate::InputError;

pub trait CoordinateResolver: Send + Sync {
    fn ensure_ready(&self, hwnd: isize, window_rect: &WindowRect) -> Result<bool, InputError>;
    fn hit_test(
        &self,
        hwnd: isize,
        screen_x: i32,
        screen_y: i32,
    ) -> Result<HitTestResult, InputError>;
}

#[derive(Debug, Default)]
pub struct WindowsCoordinateResolver;

impl CoordinateResolver for WindowsCoordinateResolver {
    fn ensure_ready(&self, hwnd: isize, window_rect: &WindowRect) -> Result<bool, InputError> {
        if hwnd == 0 {
            return Err(InputError::InvalidTarget("窗口句柄不能为空"));
        }
        if !is_window_valid(hwnd) {
            return Err(InputError::WindowUnavailable);
        }
        if window_rect.width() <= 0 || window_rect.height() <= 0 {
            return Err(InputError::Coordinate("窗口矩形无效".to_string()));
        }

        let minimized = unsafe { IsIconic(hwnd_from_isize(hwnd)).as_bool() };
        if minimized {
            restore_window_no_activate(hwnd)
                .map_err(|err| InputError::Coordinate(err.to_string()))?;
        }
        Ok(minimized)
    }

    fn hit_test(
        &self,
        hwnd: isize,
        screen_x: i32,
        screen_y: i32,
    ) -> Result<HitTestResult, InputError> {
        child_window_from_point(hwnd, screen_x, screen_y)
            .map_err(|err| InputError::Coordinate(err.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClickCoordinates {
    pub window_hwnd: isize,
    pub dispatch_hwnd: isize,
    pub frame_x: i32,
    pub frame_y: i32,
    pub screen_x: i32,
    pub screen_y: i32,
    pub client_x: i32,
    pub client_y: i32,
    pub restored_from_minimized: bool,
}

pub fn match_center_to_frame(matched: &MatchResult, offset_x: f32, offset_y: f32) -> (i32, i32) {
    (
        matched.x as i32 + matched.width as i32 / 2 + offset_x.round() as i32,
        matched.y as i32 + matched.height as i32 / 2 + offset_y.round() as i32,
    )
}

pub fn frame_point_to_screen(window_rect: &WindowRect, frame_x: i32, frame_y: i32) -> (i32, i32) {
    (window_rect.left + frame_x, window_rect.top + frame_y)
}

pub fn resolve_click_target_from_match(
    hwnd: isize,
    window_rect: &WindowRect,
    matched: &MatchResult,
    offset_x: f32,
    offset_y: f32,
    verify_window: bool,
) -> Result<ClickCoordinates, InputError> {
    resolve_click_target_from_match_with_resolver(
        hwnd,
        window_rect,
        matched,
        offset_x,
        offset_y,
        verify_window,
        &WindowsCoordinateResolver,
    )
}

pub fn resolve_click_target_from_match_with_resolver(
    hwnd: isize,
    window_rect: &WindowRect,
    matched: &MatchResult,
    offset_x: f32,
    offset_y: f32,
    verify_window: bool,
    resolver: &dyn CoordinateResolver,
) -> Result<ClickCoordinates, InputError> {
    let restored_from_minimized = if verify_window {
        resolver.ensure_ready(hwnd, window_rect)?
    } else {
        false
    };

    let (frame_x, frame_y) = match_center_to_frame(matched, offset_x, offset_y);
    let (screen_x, screen_y) = frame_point_to_screen(window_rect, frame_x, frame_y);
    let hit = resolver.hit_test(hwnd, screen_x, screen_y)?;

    Ok(ClickCoordinates {
        window_hwnd: hwnd,
        dispatch_hwnd: hit.target_hwnd,
        frame_x,
        frame_y,
        screen_x,
        screen_y,
        client_x: hit.client_x,
        client_y: hit.client_y,
        restored_from_minimized,
    })
}

fn hwnd_from_isize(hwnd: isize) -> HWND {
    HWND(hwnd as *mut _)
}

#[cfg(test)]
mod tests {
    use autoclick_platform_win::{hit_test::HitTestResult, window::WindowRect};

    use super::{
        ClickCoordinates, CoordinateResolver, frame_point_to_screen, match_center_to_frame,
        resolve_click_target_from_match_with_resolver,
    };
    use crate::InputError;

    struct FakeResolver;

    impl CoordinateResolver for FakeResolver {
        fn ensure_ready(
            &self,
            _hwnd: isize,
            _window_rect: &WindowRect,
        ) -> Result<bool, InputError> {
            Ok(true)
        }

        fn hit_test(
            &self,
            hwnd: isize,
            screen_x: i32,
            screen_y: i32,
        ) -> Result<HitTestResult, InputError> {
            Ok(HitTestResult {
                target_hwnd: hwnd + 1,
                client_x: screen_x - 100,
                client_y: screen_y - 200,
            })
        }
    }

    fn matched() -> autoclick_detect::r#match::MatchResult {
        autoclick_detect::r#match::MatchResult {
            template_id: "id".to_string(),
            template_name: "sample".to_string(),
            score: 0.95,
            x: 20,
            y: 30,
            width: 10,
            height: 6,
            scale: 1.0,
        }
    }

    #[test]
    fn computes_center_point_with_offset() {
        assert_eq!(match_center_to_frame(&matched(), 1.4, -2.0), (26, 31));
    }

    #[test]
    fn maps_negative_screen_coordinates() {
        let rect = WindowRect {
            left: -1200,
            top: 100,
            right: -800,
            bottom: 400,
        };
        assert_eq!(frame_point_to_screen(&rect, 100, 60), (-1100, 160));
    }

    #[test]
    fn resolves_child_window_target() {
        let rect = WindowRect {
            left: 100,
            top: 200,
            right: 500,
            bottom: 600,
        };
        let resolved = resolve_click_target_from_match_with_resolver(
            500,
            &rect,
            &matched(),
            0.0,
            0.0,
            true,
            &FakeResolver,
        )
        .expect("resolve");

        assert_eq!(
            resolved,
            ClickCoordinates {
                window_hwnd: 500,
                dispatch_hwnd: 501,
                frame_x: 25,
                frame_y: 33,
                screen_x: 125,
                screen_y: 233,
                client_x: 25,
                client_y: 33,
                restored_from_minimized: true,
            }
        );
    }
}
