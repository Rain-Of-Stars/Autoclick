use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{IsIconic, IsWindow, SW_MINIMIZE, SW_SHOWNOACTIVATE, ShowWindow},
};

use crate::PlatformError;

pub fn is_window_valid(hwnd: isize) -> bool {
    unsafe { IsWindow(Some(hwnd_from_isize(hwnd))).as_bool() }
}

pub fn is_window_minimized(hwnd: isize) -> Result<bool, PlatformError> {
    if hwnd == 0 {
        return Err(PlatformError::InvalidArgument("hwnd must not be zero"));
    }
    Ok(unsafe { IsIconic(hwnd_from_isize(hwnd)).as_bool() })
}

pub fn restore_window_no_activate(hwnd: isize) -> Result<(), PlatformError> {
    if hwnd == 0 {
        return Err(PlatformError::InvalidArgument("hwnd must not be zero"));
    }
    unsafe {
        let _ = ShowWindow(hwnd_from_isize(hwnd), SW_SHOWNOACTIVATE);
    }
    Ok(())
}

pub fn minimize_window(hwnd: isize) -> Result<(), PlatformError> {
    if hwnd == 0 {
        return Err(PlatformError::InvalidArgument("hwnd must not be zero"));
    }
    unsafe {
        let _ = ShowWindow(hwnd_from_isize(hwnd), SW_MINIMIZE);
    }
    Ok(())
}

fn hwnd_from_isize(hwnd: isize) -> HWND {
    HWND(hwnd as *mut _)
}

#[cfg(test)]
mod tests {
    use super::{is_window_minimized, restore_window_no_activate};

    #[test]
    fn rejects_zero_hwnd() {
        assert!(restore_window_no_activate(0).is_err());
    }

    #[test]
    fn minimized_check_rejects_zero_hwnd() {
        assert!(is_window_minimized(0).is_err());
    }
}
