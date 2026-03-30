use autoclick_detect::r#match::MatchResult;
use autoclick_domain::config::{ClickMethod, InputPolicy};
use autoclick_platform_win::window::WindowRect;
use serde::{Deserialize, Serialize};

use crate::{
    InputError,
    coordinate::{
        CoordinateResolver, WindowsCoordinateResolver,
        resolve_click_target_from_match_with_resolver,
    },
    post_message::{
        ClickReport, MouseMessageDispatcher, WindowsMouseMessageDispatcher,
        post_message_click_with_dispatcher,
    },
    send_input::{InputInjector, WindowsInputInjector, send_input_click_with_injector},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClickRequest {
    pub hwnd: isize,
    pub window_rect: WindowRect,
    pub matched: MatchResult,
}

pub struct ClickDependencies<'a> {
    pub resolver: &'a dyn CoordinateResolver,
    pub dispatcher: &'a dyn MouseMessageDispatcher,
    pub injector: &'a dyn InputInjector,
}

pub fn execute_click(
    request: &ClickRequest,
    policy: &InputPolicy,
) -> Result<ClickReport, InputError> {
    let resolver = WindowsCoordinateResolver;
    let dispatcher = WindowsMouseMessageDispatcher;
    let injector = WindowsInputInjector;
    execute_click_with_dependencies(
        request,
        policy,
        ClickDependencies {
            resolver: &resolver,
            dispatcher: &dispatcher,
            injector: &injector,
        },
    )
}

pub fn execute_click_with_dependencies(
    request: &ClickRequest,
    policy: &InputPolicy,
    dependencies: ClickDependencies<'_>,
) -> Result<ClickReport, InputError> {
    let coordinates = resolve_click_target_from_match_with_resolver(
        request.hwnd,
        &request.window_rect,
        &request.matched,
        policy.click_offset_x,
        policy.click_offset_y,
        policy.verify_window_before_click,
        dependencies.resolver,
    )?;

    match policy.method {
        ClickMethod::Message => {
            post_message_click_with_dispatcher(&coordinates, dependencies.dispatcher)
        }
        ClickMethod::Simulate => {
            send_input_click_with_injector(&coordinates, dependencies.injector)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use autoclick_domain::config::{ClickMethod, InputPolicy};
    use autoclick_platform_win::{hit_test::HitTestResult, window::WindowRect};

    use super::{ClickDependencies, ClickRequest, execute_click_with_dependencies};
    use crate::{
        InputError, coordinate::CoordinateResolver, post_message::MouseMessageDispatcher,
        send_input::InputInjector,
    };

    struct FakeResolver;

    impl CoordinateResolver for FakeResolver {
        fn ensure_ready(
            &self,
            _hwnd: isize,
            _window_rect: &WindowRect,
        ) -> Result<bool, InputError> {
            Ok(false)
        }

        fn hit_test(
            &self,
            hwnd: isize,
            screen_x: i32,
            screen_y: i32,
        ) -> Result<HitTestResult, InputError> {
            Ok(HitTestResult {
                target_hwnd: hwnd + 1,
                client_x: screen_x - 10,
                client_y: screen_y - 20,
            })
        }
    }

    #[derive(Default)]
    struct FakeDispatcher {
        calls: Mutex<u32>,
    }

    impl MouseMessageDispatcher for FakeDispatcher {
        fn post(
            &self,
            _hwnd: isize,
            _message: u32,
            _wparam: usize,
            _lparam: isize,
        ) -> Result<(), InputError> {
            *self.calls.lock().expect("lock") += 1;
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeInjector {
        calls: Mutex<u32>,
    }

    impl InputInjector for FakeInjector {
        fn move_cursor(&self, _screen_x: i32, _screen_y: i32) -> Result<(), InputError> {
            *self.calls.lock().expect("lock") += 1;
            Ok(())
        }

        fn left_down(&self) -> Result<(), InputError> {
            *self.calls.lock().expect("lock") += 1;
            Ok(())
        }

        fn left_up(&self) -> Result<(), InputError> {
            *self.calls.lock().expect("lock") += 1;
            Ok(())
        }
    }

    fn request() -> ClickRequest {
        ClickRequest {
            hwnd: 500,
            window_rect: WindowRect {
                left: 100,
                top: 200,
                right: 500,
                bottom: 600,
            },
            matched: autoclick_detect::r#match::MatchResult {
                template_id: "id".to_string(),
                template_name: "sample".to_string(),
                score: 0.95,
                x: 20,
                y: 30,
                width: 10,
                height: 6,
                scale: 1.0,
            },
        }
    }

    #[test]
    fn policy_uses_post_message_when_selected() {
        let dispatcher = FakeDispatcher::default();
        let injector = FakeInjector::default();
        let report = execute_click_with_dependencies(
            &request(),
            &InputPolicy {
                method: ClickMethod::Message,
                verify_window_before_click: true,
                click_offset_x: 0.0,
                click_offset_y: 0.0,
            },
            ClickDependencies {
                resolver: &FakeResolver,
                dispatcher: &dispatcher,
                injector: &injector,
            },
        )
        .expect("report");
        assert_eq!(report.method, ClickMethod::Message);
        assert_eq!(*dispatcher.calls.lock().expect("lock"), 3);
        assert_eq!(*injector.calls.lock().expect("lock"), 0);
    }

    #[test]
    fn policy_uses_send_input_when_selected() {
        let dispatcher = FakeDispatcher::default();
        let injector = FakeInjector::default();
        let report = execute_click_with_dependencies(
            &request(),
            &InputPolicy {
                method: ClickMethod::Simulate,
                verify_window_before_click: false,
                click_offset_x: 0.0,
                click_offset_y: 0.0,
            },
            ClickDependencies {
                resolver: &FakeResolver,
                dispatcher: &dispatcher,
                injector: &injector,
            },
        )
        .expect("report");
        assert_eq!(report.method, ClickMethod::Simulate);
        assert_eq!(*injector.calls.lock().expect("lock"), 3);
        assert_eq!(*dispatcher.calls.lock().expect("lock"), 0);
    }
}
