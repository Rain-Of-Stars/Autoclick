use std::{sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{
    CaptureError,
    backend::{CaptureFactory, WgcCaptureOptions, WindowsCaptureFactory},
    frame::{FramePacket, FrameStats},
    latest_frame::LatestFrameBuffer,
    wgc_monitor::{MonitorCaptureSnapshot, WgcMonitorCapture},
    wgc_window::{WgcWindowCapture, WindowCaptureSnapshot},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureTarget {
    Window { hwnd: isize },
    Monitor { handle: Option<isize> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSessionConfig {
    pub target: CaptureTarget,
    pub options: WgcCaptureOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSessionSnapshot {
    pub is_running: bool,
    pub config: Option<CaptureSessionConfig>,
    pub stats: FrameStats,
    pub is_closed: bool,
    pub last_dimensions: Option<(u32, u32)>,
    pub last_error: Option<String>,
}

enum ActiveCapture {
    Window(WgcWindowCapture),
    Monitor(WgcMonitorCapture),
}

impl ActiveCapture {
    fn stop(&mut self) -> Result<(), CaptureError> {
        match self {
            Self::Window(capture) => capture.stop(),
            Self::Monitor(capture) => capture.stop(),
        }
    }

    fn is_finished(&self) -> bool {
        match self {
            Self::Window(capture) => capture.is_finished(),
            Self::Monitor(capture) => capture.is_finished(),
        }
    }

    fn last_shared_snapshot(&self) -> (bool, Option<(u32, u32)>, Option<String>, FrameStats) {
        match self {
            Self::Window(capture) => {
                let WindowCaptureSnapshot {
                    is_closed,
                    last_dimensions,
                    last_error,
                    stats,
                    ..
                } = capture.snapshot();
                (is_closed, last_dimensions, last_error, stats)
            }
            Self::Monitor(capture) => {
                let MonitorCaptureSnapshot {
                    is_closed,
                    last_dimensions,
                    last_error,
                    stats,
                    ..
                } = capture.snapshot();
                (is_closed, last_dimensions, last_error, stats)
            }
        }
    }
}

pub struct CaptureSession {
    factory: Arc<dyn CaptureFactory>,
    latest: Arc<LatestFrameBuffer>,
    active: Option<ActiveCapture>,
    last_config: Option<CaptureSessionConfig>,
}

impl CaptureSession {
    pub fn new() -> Self {
        Self::with_factory(Arc::new(WindowsCaptureFactory))
    }

    pub fn with_factory(factory: Arc<dyn CaptureFactory>) -> Self {
        Self {
            factory,
            latest: Arc::new(LatestFrameBuffer::new()),
            active: None,
            last_config: None,
        }
    }

    pub fn latest_buffer(&self) -> Arc<LatestFrameBuffer> {
        self.latest.clone()
    }

    pub fn start(&mut self, config: CaptureSessionConfig) -> Result<(), CaptureError> {
        self.stop()?;
        self.latest = Arc::new(LatestFrameBuffer::new());

        let capture = match config.target {
            CaptureTarget::Window { hwnd } => {
                ActiveCapture::Window(WgcWindowCapture::start_with_factory(
                    hwnd,
                    config.options.clone(),
                    self.latest.clone(),
                    self.factory.as_ref(),
                )?)
            }
            CaptureTarget::Monitor { handle } => {
                ActiveCapture::Monitor(WgcMonitorCapture::start_with_factory(
                    handle,
                    config.options.clone(),
                    self.latest.clone(),
                    self.factory.as_ref(),
                )?)
            }
        };

        self.last_config = Some(config);
        self.active = Some(capture);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), CaptureError> {
        if let Some(active) = self.active.as_mut() {
            active.stop()?;
        }
        self.active = None;
        Ok(())
    }

    pub fn reconfigure(&mut self, config: CaptureSessionConfig) -> Result<(), CaptureError> {
        self.start(config)
    }

    pub fn read_latest(&self) -> Result<FramePacket, CaptureError> {
        self.latest.read_latest()
    }

    pub fn read_next_frame(
        &self,
        after_frame_id: u64,
        timeout: Duration,
    ) -> Result<FramePacket, CaptureError> {
        self.latest.wait_for_newer_than(after_frame_id, timeout)
    }

    pub fn is_running(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|active| !active.is_finished())
    }

    pub fn snapshot(&self) -> CaptureSessionSnapshot {
        let default_stats = self.latest.snapshot_stats();
        let (is_closed, last_dimensions, last_error, stats) = self
            .active
            .as_ref()
            .map(ActiveCapture::last_shared_snapshot)
            .unwrap_or((false, None, None, default_stats));

        CaptureSessionSnapshot {
            is_running: self.is_running(),
            config: self.last_config.clone(),
            stats,
            is_closed,
            last_dimensions,
            last_error,
        }
    }
}

impl Default for CaptureSession {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread, time::Duration};

    use crate::{
        CaptureError,
        backend::{CaptureFactory, CaptureSharedState, RunningCapture, WgcCaptureOptions},
        frame::PixelFormat,
    };

    use super::{CaptureSession, CaptureSessionConfig, CaptureTarget};

    struct FakeRunner;

    impl RunningCapture for FakeRunner {
        fn stop(&mut self) -> Result<(), CaptureError> {
            Ok(())
        }

        fn is_finished(&self) -> bool {
            false
        }
    }

    struct FakeFactory;

    impl CaptureFactory for FakeFactory {
        fn start_window(
            &self,
            _hwnd: isize,
            _options: &WgcCaptureOptions,
            shared: Arc<CaptureSharedState>,
        ) -> Result<Box<dyn RunningCapture>, CaptureError> {
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(10));
                shared.publish_frame(2, 2, PixelFormat::Gray8, vec![1; 4]);
            });
            Ok(Box::new(FakeRunner))
        }

        fn start_monitor(
            &self,
            _monitor_handle: Option<isize>,
            _options: &WgcCaptureOptions,
            shared: Arc<CaptureSharedState>,
        ) -> Result<Box<dyn RunningCapture>, CaptureError> {
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(10));
                shared.publish_frame(3, 1, PixelFormat::Gray8, vec![2; 3]);
            });
            Ok(Box::new(FakeRunner))
        }
    }

    #[test]
    fn session_start_reads_window_frame() {
        let mut session = CaptureSession::with_factory(Arc::new(FakeFactory));
        session
            .start(CaptureSessionConfig {
                target: CaptureTarget::Window { hwnd: 100 },
                options: WgcCaptureOptions::default(),
            })
            .expect("start");
        let frame = session
            .read_next_frame(0, Duration::from_millis(200))
            .expect("frame");
        assert_eq!(frame.width, 2);
    }

    #[test]
    fn session_reconfigure_switches_target() {
        let mut session = CaptureSession::with_factory(Arc::new(FakeFactory));
        session
            .start(CaptureSessionConfig {
                target: CaptureTarget::Window { hwnd: 100 },
                options: WgcCaptureOptions::default(),
            })
            .expect("start");
        session
            .reconfigure(CaptureSessionConfig {
                target: CaptureTarget::Monitor { handle: None },
                options: WgcCaptureOptions::default(),
            })
            .expect("reconfigure");
        let frame = session
            .read_next_frame(0, Duration::from_millis(200))
            .expect("frame");
        assert_eq!(frame.width, 3);
    }
}
