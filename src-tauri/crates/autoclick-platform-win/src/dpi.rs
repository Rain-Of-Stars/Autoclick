use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};

use crate::PlatformError;

pub fn ensure_per_monitor_v2() -> Result<(), PlatformError> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    Ok(())
}

pub fn scale_factor_from_dpi(dpi: u32) -> f32 {
    dpi as f32 / 96.0
}

pub fn logical_to_physical(value: i32, dpi: u32) -> i32 {
    ((value as f32) * scale_factor_from_dpi(dpi)).round() as i32
}

pub fn physical_to_logical(value: i32, dpi: u32) -> i32 {
    ((value as f32) / scale_factor_from_dpi(dpi)).round() as i32
}

#[cfg(test)]
mod tests {
    use super::{logical_to_physical, physical_to_logical, scale_factor_from_dpi};

    #[test]
    fn computes_scale_factor() {
        assert!((scale_factor_from_dpi(144) - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn converts_between_logical_and_physical_pixels() {
        assert_eq!(logical_to_physical(100, 144), 150);
        assert_eq!(physical_to_logical(150, 144), 100);
    }
}
