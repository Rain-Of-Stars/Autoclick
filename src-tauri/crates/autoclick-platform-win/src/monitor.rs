use serde::{Deserialize, Serialize};
use windows::{
    Win32::{
        Foundation::{LPARAM, RECT},
        Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW},
        UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
    },
    core::BOOL,
};

use crate::{PlatformError, dpi, window::WindowRect};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub handle: isize,
    pub name: String,
    pub is_primary: bool,
    pub rect: WindowRect,
    pub work_rect: WindowRect,
    pub dpi: u32,
    pub scale_factor: f32,
}

pub fn enumerate_monitors() -> Result<Vec<MonitorInfo>, PlatformError> {
    let mut monitors = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            Some(HDC::default()),
            None,
            Some(enum_monitor_callback),
            LPARAM((&mut monitors as *mut Vec<MonitorInfo>) as isize),
        )
        .ok()
        .map_err(|err| PlatformError::Win32(err.to_string()))?;
    }
    Ok(monitors)
}

unsafe extern "system" fn enum_monitor_callback(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitors = unsafe { &mut *(lparam.0 as *mut Vec<MonitorInfo>) };
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if !unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo as *mut _ as *mut _) }.as_bool() {
        return true.into();
    }
    let mut dpi_x = 96u32;
    let mut dpi_y = 96u32;
    let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    let name_end = info
        .szDevice
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(info.szDevice.len());
    monitors.push(MonitorInfo {
        handle: monitor.0 as isize,
        name: String::from_utf16_lossy(&info.szDevice[..name_end]),
        is_primary: info.monitorInfo.dwFlags & 1 != 0,
        rect: WindowRect {
            left: info.monitorInfo.rcMonitor.left,
            top: info.monitorInfo.rcMonitor.top,
            right: info.monitorInfo.rcMonitor.right,
            bottom: info.monitorInfo.rcMonitor.bottom,
        },
        work_rect: WindowRect {
            left: info.monitorInfo.rcWork.left,
            top: info.monitorInfo.rcWork.top,
            right: info.monitorInfo.rcWork.right,
            bottom: info.monitorInfo.rcWork.bottom,
        },
        dpi: dpi_x.max(dpi_y),
        scale_factor: dpi::scale_factor_from_dpi(dpi_x.max(dpi_y)),
    });
    true.into()
}
