use autoclick_domain::config::ClickMethod;
use windows::Win32::{
    Foundation::POINT,
    UI::{
        Input::KeyboardAndMouse::{
            INPUT, INPUT_0, INPUT_MOUSE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN,
            MOUSEEVENTF_LEFTUP, MOUSEINPUT, SendInput,
        },
        WindowsAndMessaging::SetCursorPos,
    },
};

use crate::{InputError, coordinate::ClickCoordinates, post_message::ClickReport};

pub trait InputInjector: Send + Sync {
    fn move_cursor(&self, screen_x: i32, screen_y: i32) -> Result<(), InputError>;
    fn left_down(&self) -> Result<(), InputError>;
    fn left_up(&self) -> Result<(), InputError>;
}

#[derive(Debug, Default)]
pub struct WindowsInputInjector;

impl InputInjector for WindowsInputInjector {
    fn move_cursor(&self, screen_x: i32, screen_y: i32) -> Result<(), InputError> {
        unsafe { SetCursorPos(screen_x, screen_y) }
            .map_err(|err| InputError::Simulate(err.to_string()))
    }

    fn left_down(&self) -> Result<(), InputError> {
        send_mouse_flags(MOUSEEVENTF_LEFTDOWN)
    }

    fn left_up(&self) -> Result<(), InputError> {
        send_mouse_flags(MOUSEEVENTF_LEFTUP)
    }
}

pub fn send_input_click(target: &ClickCoordinates) -> Result<ClickReport, InputError> {
    send_input_click_with_injector(target, &WindowsInputInjector)
}

pub fn send_input_click_with_injector(
    target: &ClickCoordinates,
    injector: &dyn InputInjector,
) -> Result<ClickReport, InputError> {
    injector.move_cursor(target.screen_x, target.screen_y)?;
    injector.left_down()?;
    injector.left_up()?;

    Ok(ClickReport {
        method: ClickMethod::Simulate,
        dispatch_hwnd: target.dispatch_hwnd,
        screen_x: target.screen_x,
        screen_y: target.screen_y,
        client_x: target.client_x,
        client_y: target.client_y,
        restored_from_minimized: target.restored_from_minimized,
    })
}

fn send_mouse_flags(flags: MOUSE_EVENT_FLAGS) -> Result<(), InputError> {
    let inputs = [INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }];

    let written = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if written != inputs.len() as u32 {
        return Err(InputError::Simulate(
            "SendInput 未写入完整输入事件".to_string(),
        ));
    }
    Ok(())
}

#[allow(dead_code)]
fn cursor_position() -> Result<POINT, InputError> {
    let mut point = POINT::default();
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point) }
        .map_err(|err| InputError::Simulate(err.to_string()))?;
    Ok(point)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::{InputInjector, send_input_click_with_injector};
    use crate::{InputError, coordinate::ClickCoordinates};

    #[derive(Default)]
    struct FakeInjector {
        calls: Mutex<Vec<String>>,
    }

    impl InputInjector for FakeInjector {
        fn move_cursor(&self, screen_x: i32, screen_y: i32) -> Result<(), InputError> {
            self.calls
                .lock()
                .expect("lock")
                .push(format!("move:{screen_x},{screen_y}"));
            Ok(())
        }

        fn left_down(&self) -> Result<(), InputError> {
            self.calls.lock().expect("lock").push("down".to_string());
            Ok(())
        }

        fn left_up(&self) -> Result<(), InputError> {
            self.calls.lock().expect("lock").push("up".to_string());
            Ok(())
        }
    }

    #[test]
    fn send_input_injects_move_and_click() {
        let injector = FakeInjector::default();
        let target = ClickCoordinates {
            window_hwnd: 1,
            dispatch_hwnd: 2,
            frame_x: 10,
            frame_y: 12,
            screen_x: -1100,
            screen_y: 212,
            client_x: 10,
            client_y: 12,
            restored_from_minimized: true,
        };
        let report = send_input_click_with_injector(&target, &injector).expect("report");
        let calls = injector.calls.lock().expect("lock");
        assert_eq!(calls.as_slice(), ["move:-1100,212", "down", "up"]);
        assert!(report.restored_from_minimized);
    }
}
