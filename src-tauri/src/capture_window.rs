use std::{thread, time::Duration};

use autoclick_domain::config::CaptureProfile;
use autoclick_platform_win::window_state::{
    is_window_minimized, is_window_valid, minimize_window, restore_window_no_activate,
};

const WINDOW_CAPTURE_RESTORE_SETTLE_MS: Duration = Duration::from_millis(120);

trait WindowCaptureOps {
    fn is_valid(&self, hwnd: isize) -> bool;
    fn is_minimized(&self, hwnd: isize) -> Result<bool, String>;
    fn restore_no_activate(&self, hwnd: isize) -> Result<(), String>;
    fn minimize(&self, hwnd: isize) -> Result<(), String>;
    fn settle_after_restore(&self);
}

struct PlatformWindowCaptureOps;

impl WindowCaptureOps for PlatformWindowCaptureOps {
    fn is_valid(&self, hwnd: isize) -> bool {
        is_window_valid(hwnd)
    }

    fn is_minimized(&self, hwnd: isize) -> Result<bool, String> {
        is_window_minimized(hwnd).map_err(|err| err.to_string())
    }

    fn restore_no_activate(&self, hwnd: isize) -> Result<(), String> {
        restore_window_no_activate(hwnd).map_err(|err| err.to_string())
    }

    fn minimize(&self, hwnd: isize) -> Result<(), String> {
        minimize_window(hwnd).map_err(|err| err.to_string())
    }

    fn settle_after_restore(&self) {
        thread::sleep(WINDOW_CAPTURE_RESTORE_SETTLE_MS);
    }
}

pub fn ensure_window_capture_ready(capture: &CaptureProfile, hwnd: isize) -> Result<bool, String> {
    ensure_window_capture_ready_with_ops(capture, hwnd, &PlatformWindowCaptureOps)
}

pub fn finish_window_capture(
    capture: &CaptureProfile,
    hwnd: isize,
    restored_from_minimized: bool,
) -> Result<(), String> {
    finish_window_capture_with_ops(
        capture,
        hwnd,
        restored_from_minimized,
        &PlatformWindowCaptureOps,
    )
}

fn ensure_window_capture_ready_with_ops(
    capture: &CaptureProfile,
    hwnd: isize,
    ops: &dyn WindowCaptureOps,
) -> Result<bool, String> {
    if hwnd == 0 {
        return Err("目标窗口句柄不能为空".to_string());
    }
    if !ops.is_valid(hwnd) {
        return Err("目标窗口句柄无效或窗口已关闭".to_string());
    }

    let minimized = ops.is_minimized(hwnd)?;
    if !minimized {
        return Ok(false);
    }
    if !capture.restore_minimized_noactivate {
        return Err("目标窗口当前已最小化，请先恢复窗口后再执行高性能窗口捕获".to_string());
    }

    ops.restore_no_activate(hwnd)?;
    ops.settle_after_restore();
    Ok(true)
}

fn finish_window_capture_with_ops(
    capture: &CaptureProfile,
    hwnd: isize,
    restored_from_minimized: bool,
    ops: &dyn WindowCaptureOps,
) -> Result<(), String> {
    if !restored_from_minimized || !capture.restore_minimized_after_capture {
        return Ok(());
    }
    if hwnd == 0 || !ops.is_valid(hwnd) {
        return Ok(());
    }
    ops.minimize(hwnd)
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use autoclick_domain::config::CaptureProfile;

    use super::{
        WindowCaptureOps, ensure_window_capture_ready_with_ops, finish_window_capture_with_ops,
    };

    #[derive(Clone)]
    struct FakeWindowCaptureOps {
        is_valid: bool,
        is_minimized: bool,
        restore_calls: Rc<Cell<u32>>,
        minimize_calls: Rc<Cell<u32>>,
        settle_calls: Rc<Cell<u32>>,
    }

    impl FakeWindowCaptureOps {
        fn new(is_valid: bool, is_minimized: bool) -> Self {
            Self {
                is_valid,
                is_minimized,
                restore_calls: Rc::new(Cell::new(0)),
                minimize_calls: Rc::new(Cell::new(0)),
                settle_calls: Rc::new(Cell::new(0)),
            }
        }
    }

    impl WindowCaptureOps for FakeWindowCaptureOps {
        fn is_valid(&self, _hwnd: isize) -> bool {
            self.is_valid
        }

        fn is_minimized(&self, _hwnd: isize) -> Result<bool, String> {
            Ok(self.is_minimized)
        }

        fn restore_no_activate(&self, _hwnd: isize) -> Result<(), String> {
            self.restore_calls.set(self.restore_calls.get() + 1);
            Ok(())
        }

        fn minimize(&self, _hwnd: isize) -> Result<(), String> {
            self.minimize_calls.set(self.minimize_calls.get() + 1);
            Ok(())
        }

        fn settle_after_restore(&self) {
            self.settle_calls.set(self.settle_calls.get() + 1);
        }
    }

    #[test]
    fn restores_minimized_window_before_capture() {
        let capture = CaptureProfile::default();
        let ops = FakeWindowCaptureOps::new(true, true);

        let restored = ensure_window_capture_ready_with_ops(&capture, 10, &ops).expect("prepare");

        assert!(restored);
        assert_eq!(ops.restore_calls.get(), 1);
        assert_eq!(ops.settle_calls.get(), 1);
    }

    #[test]
    fn rejects_minimized_window_when_restore_disabled() {
        let capture = CaptureProfile {
            restore_minimized_noactivate: false,
            ..CaptureProfile::default()
        };
        let ops = FakeWindowCaptureOps::new(true, true);

        let error =
            ensure_window_capture_ready_with_ops(&capture, 10, &ops).expect_err("should fail");

        assert!(error.contains("最小化"));
        assert_eq!(ops.restore_calls.get(), 0);
    }

    #[test]
    fn skips_restore_for_normal_window() {
        let capture = CaptureProfile::default();
        let ops = FakeWindowCaptureOps::new(true, false);

        let restored = ensure_window_capture_ready_with_ops(&capture, 10, &ops).expect("prepare");

        assert!(!restored);
        assert_eq!(ops.restore_calls.get(), 0);
        assert_eq!(ops.settle_calls.get(), 0);
    }

    #[test]
    fn minimizes_restored_window_after_capture_when_enabled() {
        let capture = CaptureProfile {
            restore_minimized_after_capture: true,
            ..CaptureProfile::default()
        };
        let ops = FakeWindowCaptureOps::new(true, false);

        finish_window_capture_with_ops(&capture, 10, true, &ops).expect("finish");

        assert_eq!(ops.minimize_calls.get(), 1);
    }
}
