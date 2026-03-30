use autoclick_domain::config::ClickMethod;
use serde::{Deserialize, Serialize};
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{PostMessageW, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE},
};

use crate::{InputError, coordinate::ClickCoordinates};

const MK_LBUTTON_VALUE: usize = 0x0001;

pub trait MouseMessageDispatcher: Send + Sync {
    fn post(
        &self,
        hwnd: isize,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> Result<(), InputError>;
}

#[derive(Debug, Default)]
pub struct WindowsMouseMessageDispatcher;

impl MouseMessageDispatcher for WindowsMouseMessageDispatcher {
    fn post(
        &self,
        hwnd: isize,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> Result<(), InputError> {
        unsafe {
            PostMessageW(
                Some(HWND(hwnd as *mut _)),
                message,
                WPARAM(wparam),
                LPARAM(lparam),
            )
            .map_err(|err| InputError::Message(err.to_string()))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClickReport {
    pub method: ClickMethod,
    pub dispatch_hwnd: isize,
    pub screen_x: i32,
    pub screen_y: i32,
    pub client_x: i32,
    pub client_y: i32,
    pub restored_from_minimized: bool,
}

pub fn post_message_click(target: &ClickCoordinates) -> Result<ClickReport, InputError> {
    post_message_click_with_dispatcher(target, &WindowsMouseMessageDispatcher)
}

pub fn post_message_click_with_dispatcher(
    target: &ClickCoordinates,
    dispatcher: &dyn MouseMessageDispatcher,
) -> Result<ClickReport, InputError> {
    let lparam = make_lparam(target.client_x, target.client_y);
    dispatcher.post(target.dispatch_hwnd, WM_MOUSEMOVE, 0, lparam)?;
    dispatcher.post(
        target.dispatch_hwnd,
        WM_LBUTTONDOWN,
        MK_LBUTTON_VALUE,
        lparam,
    )?;
    dispatcher.post(target.dispatch_hwnd, WM_LBUTTONUP, 0, lparam)?;

    Ok(ClickReport {
        method: ClickMethod::Message,
        dispatch_hwnd: target.dispatch_hwnd,
        screen_x: target.screen_x,
        screen_y: target.screen_y,
        client_x: target.client_x,
        client_y: target.client_y,
        restored_from_minimized: target.restored_from_minimized,
    })
}

fn make_lparam(x: i32, y: i32) -> isize {
    let x_bits = (x as u16) as u32;
    let y_bits = ((y as u16) as u32) << 16;
    (x_bits | y_bits) as isize
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::{MouseMessageDispatcher, post_message_click_with_dispatcher};
    use crate::{InputError, coordinate::ClickCoordinates};

    #[derive(Default)]
    struct FakeDispatcher {
        calls: Mutex<Vec<(u32, usize, isize)>>,
    }

    impl MouseMessageDispatcher for FakeDispatcher {
        fn post(
            &self,
            _hwnd: isize,
            message: u32,
            wparam: usize,
            lparam: isize,
        ) -> Result<(), InputError> {
            self.calls
                .lock()
                .expect("lock")
                .push((message, wparam, lparam));
            Ok(())
        }
    }

    #[test]
    fn post_message_sends_expected_sequence() {
        let dispatcher = FakeDispatcher::default();
        let target = ClickCoordinates {
            window_hwnd: 1,
            dispatch_hwnd: 2,
            frame_x: 10,
            frame_y: 12,
            screen_x: 110,
            screen_y: 212,
            client_x: 10,
            client_y: 12,
            restored_from_minimized: false,
        };
        let report = post_message_click_with_dispatcher(&target, &dispatcher).expect("report");
        let calls = dispatcher.calls.lock().expect("lock");
        assert_eq!(calls.len(), 3);
        assert_eq!(report.dispatch_hwnd, 2);
    }
}
