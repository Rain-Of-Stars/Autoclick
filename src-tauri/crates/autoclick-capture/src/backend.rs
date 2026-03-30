use std::{
    ffi::c_void,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use windows_capture::{
    capture::{CaptureControl, Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    },
    window::Window,
};

use crate::{
    CaptureError,
    frame::{FramePacket, FrameStats, PixelFormat},
    latest_frame::LatestFrameBuffer,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSharedSnapshot {
    pub is_closed: bool,
    pub last_dimensions: Option<(u32, u32)>,
    pub last_error: Option<String>,
    pub stats: FrameStats,
}

#[derive(Debug)]
pub struct CaptureSharedState {
    latest: Arc<LatestFrameBuffer>,
    next_frame_id: AtomicU64,
    closed: AtomicBool,
    last_dimensions: RwLock<Option<(u32, u32)>>,
    last_error: RwLock<Option<String>>,
}

impl CaptureSharedState {
    pub fn new(latest: Arc<LatestFrameBuffer>) -> Self {
        Self {
            latest,
            next_frame_id: AtomicU64::new(0),
            closed: AtomicBool::new(false),
            last_dimensions: RwLock::new(None),
            last_error: RwLock::new(None),
        }
    }

    pub fn latest_buffer(&self) -> Arc<LatestFrameBuffer> {
        self.latest.clone()
    }

    pub fn publish_frame(
        &self,
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        bytes: Vec<u8>,
    ) {
        let frame_id = self.next_frame_id.fetch_add(1, Ordering::Relaxed) + 1;
        *self.last_dimensions.write() = Some((width, height));
        self.latest.publish(FramePacket {
            frame_id,
            width,
            height,
            pixel_format,
            timestamp_ms: unix_timestamp_ms(),
            bytes,
        });
    }

    pub fn mark_closed(&self) {
        self.closed.store(true, Ordering::Relaxed);
        self.latest.close();
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    pub fn record_error(&self, message: impl Into<String>) {
        *self.last_error.write() = Some(message.into());
    }

    pub fn snapshot(&self) -> CaptureSharedSnapshot {
        CaptureSharedSnapshot {
            is_closed: self.is_closed(),
            last_dimensions: *self.last_dimensions.read(),
            last_error: self.last_error.read().clone(),
            stats: self.latest.snapshot_stats(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WgcCaptureOptions {
    pub target_fps: u32,
    pub include_cursor: bool,
    pub draw_border: bool,
    pub include_secondary_windows: bool,
    pub remove_title_bar: bool,
    pub dirty_region_enabled: bool,
}

impl Default for WgcCaptureOptions {
    fn default() -> Self {
        Self {
            target_fps: 30,
            include_cursor: false,
            draw_border: false,
            include_secondary_windows: false,
            remove_title_bar: false,
            dirty_region_enabled: false,
        }
    }
}

impl WgcCaptureOptions {
    pub fn minimum_update_interval(&self) -> Duration {
        Duration::from_millis((1000 / u64::from(self.target_fps.max(1))).max(1))
    }
}

pub trait RunningCapture: Send {
    fn stop(&mut self) -> Result<(), CaptureError>;
    fn is_finished(&self) -> bool;
}

pub trait CaptureFactory: Send + Sync {
    fn start_window(
        &self,
        hwnd: isize,
        options: &WgcCaptureOptions,
        shared: Arc<CaptureSharedState>,
    ) -> Result<Box<dyn RunningCapture>, CaptureError>;

    fn start_monitor(
        &self,
        monitor_handle: Option<isize>,
        options: &WgcCaptureOptions,
        shared: Arc<CaptureSharedState>,
    ) -> Result<Box<dyn RunningCapture>, CaptureError>;
}

#[derive(Debug, Default)]
pub struct WindowsCaptureFactory;

impl CaptureFactory for WindowsCaptureFactory {
    fn start_window(
        &self,
        hwnd: isize,
        options: &WgcCaptureOptions,
        shared: Arc<CaptureSharedState>,
    ) -> Result<Box<dyn RunningCapture>, CaptureError> {
        if hwnd == 0 {
            return Err(CaptureError::InvalidTarget("窗口句柄不能为空"));
        }

        let window = Window::from_raw_hwnd(hwnd as *mut c_void);
        let control = WgcFrameHandler::start_free_threaded(build_settings(
            window,
            options,
            HandlerFlags {
                shared: shared.clone(),
                remove_title_bar: options.remove_title_bar,
            },
        ))
        .map_err(|err| CaptureError::Backend(err.to_string()))?;

        Ok(Box::new(WindowsCaptureHandle::new(control, shared)))
    }

    fn start_monitor(
        &self,
        monitor_handle: Option<isize>,
        options: &WgcCaptureOptions,
        shared: Arc<CaptureSharedState>,
    ) -> Result<Box<dyn RunningCapture>, CaptureError> {
        let monitor = match monitor_handle {
            Some(handle) if handle != 0 => Monitor::from_raw_hmonitor(handle as *mut c_void),
            _ => Monitor::primary().map_err(|err| CaptureError::Backend(err.to_string()))?,
        };

        let control = WgcFrameHandler::start_free_threaded(build_settings(
            monitor,
            options,
            HandlerFlags {
                shared: shared.clone(),
                remove_title_bar: false,
            },
        ))
        .map_err(|err| CaptureError::Backend(err.to_string()))?;

        Ok(Box::new(WindowsCaptureHandle::new(control, shared)))
    }
}

fn build_settings<T>(
    item: T,
    options: &WgcCaptureOptions,
    flags: HandlerFlags,
) -> Settings<HandlerFlags, T>
where
    T: windows_capture::settings::TryIntoCaptureItemWithType,
{
    Settings::new(
        item,
        if options.include_cursor {
            CursorCaptureSettings::WithCursor
        } else {
            CursorCaptureSettings::WithoutCursor
        },
        if options.draw_border {
            DrawBorderSettings::WithBorder
        } else {
            DrawBorderSettings::WithoutBorder
        },
        if options.include_secondary_windows {
            SecondaryWindowSettings::Include
        } else {
            SecondaryWindowSettings::Exclude
        },
        MinimumUpdateIntervalSettings::Custom(options.minimum_update_interval()),
        if options.dirty_region_enabled {
            DirtyRegionSettings::ReportOnly
        } else {
            DirtyRegionSettings::Default
        },
        ColorFormat::Bgra8,
        flags,
    )
}

#[derive(Clone)]
struct HandlerFlags {
    shared: Arc<CaptureSharedState>,
    remove_title_bar: bool,
}

struct WgcFrameHandler {
    shared: Arc<CaptureSharedState>,
    remove_title_bar: bool,
}

impl GraphicsCaptureApiHandler for WgcFrameHandler {
    type Flags = HandlerFlags;
    type Error = String;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            shared: ctx.flags.shared,
            remove_title_bar: ctx.flags.remove_title_bar,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let mut buffer = if self.remove_title_bar {
            frame
                .buffer_without_title_bar()
                .map_err(|err| err.to_string())?
        } else {
            frame.buffer().map_err(|err| err.to_string())?
        };

        let width = buffer.width();
        let height = buffer.height();
        let bytes = buffer
            .as_nopadding_buffer()
            .map_err(|err| err.to_string())?
            .to_vec();

        self.shared
            .publish_frame(width, height, PixelFormat::Bgra8, bytes);
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        self.shared.mark_closed();
        Ok(())
    }
}

struct WindowsCaptureHandle {
    control: Option<CaptureControl<WgcFrameHandler, String>>,
    shared: Arc<CaptureSharedState>,
}

impl WindowsCaptureHandle {
    fn new(
        control: CaptureControl<WgcFrameHandler, String>,
        shared: Arc<CaptureSharedState>,
    ) -> Self {
        Self {
            control: Some(control),
            shared,
        }
    }

    fn stop_inner(&mut self) -> Result<(), CaptureError> {
        if let Some(control) = self.control.take() {
            control.stop().map_err(|err| {
                let message = err.to_string();
                self.shared.record_error(message.clone());
                CaptureError::Backend(message)
            })?;
        }
        self.shared.mark_closed();
        Ok(())
    }
}

impl RunningCapture for WindowsCaptureHandle {
    fn stop(&mut self) -> Result<(), CaptureError> {
        self.stop_inner()
    }

    fn is_finished(&self) -> bool {
        self.control
            .as_ref()
            .is_none_or(CaptureControl::is_finished)
    }
}

impl Drop for WindowsCaptureHandle {
    fn drop(&mut self) {
        let _ = self.stop_inner();
    }
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
