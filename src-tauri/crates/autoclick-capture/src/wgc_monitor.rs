use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    CaptureError,
    backend::{
        CaptureFactory, CaptureSharedSnapshot, CaptureSharedState, RunningCapture,
        WgcCaptureOptions, WindowsCaptureFactory,
    },
    frame::{FramePacket, FrameStats},
    latest_frame::LatestFrameBuffer,
};

pub struct WgcMonitorCapture {
    monitor_handle: Option<isize>,
    options: WgcCaptureOptions,
    shared: Arc<CaptureSharedState>,
    runner: Option<Box<dyn RunningCapture>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorCaptureSnapshot {
    pub monitor_handle: Option<isize>,
    pub options: WgcCaptureOptions,
    pub is_finished: bool,
    pub is_closed: bool,
    pub last_dimensions: Option<(u32, u32)>,
    pub last_error: Option<String>,
    pub stats: FrameStats,
}

impl WgcMonitorCapture {
    pub fn start(
        monitor_handle: Option<isize>,
        options: WgcCaptureOptions,
        latest: Arc<LatestFrameBuffer>,
    ) -> Result<Self, CaptureError> {
        Self::start_with_factory(monitor_handle, options, latest, &WindowsCaptureFactory)
    }

    pub fn start_with_factory(
        monitor_handle: Option<isize>,
        options: WgcCaptureOptions,
        latest: Arc<LatestFrameBuffer>,
        factory: &dyn CaptureFactory,
    ) -> Result<Self, CaptureError> {
        let shared = Arc::new(CaptureSharedState::new(latest));
        let runner = factory.start_monitor(monitor_handle, &options, shared.clone())?;
        Ok(Self {
            monitor_handle,
            options,
            shared,
            runner: Some(runner),
        })
    }

    pub fn read_latest(&self) -> Result<FramePacket, CaptureError> {
        self.shared.latest_buffer().read_latest()
    }

    pub fn shared_snapshot(&self) -> CaptureSharedSnapshot {
        self.shared.snapshot()
    }

    pub fn snapshot(&self) -> MonitorCaptureSnapshot {
        let shared = self.shared_snapshot();
        MonitorCaptureSnapshot {
            monitor_handle: self.monitor_handle,
            options: self.options.clone(),
            is_finished: self.is_finished(),
            is_closed: shared.is_closed,
            last_dimensions: shared.last_dimensions,
            last_error: shared.last_error,
            stats: shared.stats,
        }
    }

    pub fn stop(&mut self) -> Result<(), CaptureError> {
        if let Some(runner) = self.runner.as_mut() {
            runner.stop()?;
        }
        self.runner = None;
        Ok(())
    }

    pub fn is_finished(&self) -> bool {
        self.runner
            .as_ref()
            .is_none_or(|runner| runner.is_finished())
    }
}

impl Drop for WgcMonitorCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use super::WgcMonitorCapture;
    use crate::{
        CaptureError,
        backend::{CaptureFactory, CaptureSharedState, RunningCapture, WgcCaptureOptions},
        frame::PixelFormat,
        latest_frame::LatestFrameBuffer,
    };

    #[derive(Default)]
    struct FakeRunner {
        stopped: AtomicBool,
    }

    impl RunningCapture for FakeRunner {
        fn stop(&mut self) -> Result<(), CaptureError> {
            self.stopped.store(true, Ordering::Relaxed);
            Ok(())
        }

        fn is_finished(&self) -> bool {
            self.stopped.load(Ordering::Relaxed)
        }
    }

    struct FakeFactory;

    impl CaptureFactory for FakeFactory {
        fn start_window(
            &self,
            _hwnd: isize,
            _options: &WgcCaptureOptions,
            _shared: Arc<CaptureSharedState>,
        ) -> Result<Box<dyn RunningCapture>, CaptureError> {
            unreachable!()
        }

        fn start_monitor(
            &self,
            _monitor_handle: Option<isize>,
            _options: &WgcCaptureOptions,
            shared: Arc<CaptureSharedState>,
        ) -> Result<Box<dyn RunningCapture>, CaptureError> {
            shared.publish_frame(3, 1, PixelFormat::Gray8, vec![3; 3]);
            Ok(Box::new(FakeRunner::default()))
        }
    }

    #[test]
    fn wgc_monitor_publishes_latest_frame() {
        let latest = Arc::new(LatestFrameBuffer::new());
        let capture = WgcMonitorCapture::start_with_factory(
            None,
            WgcCaptureOptions::default(),
            latest,
            &FakeFactory,
        )
        .expect("monitor capture");
        let frame = capture.read_latest().expect("latest");
        assert_eq!(frame.width, 3);
    }

    #[test]
    fn wgc_monitor_stop_marks_finished() {
        let latest = Arc::new(LatestFrameBuffer::new());
        let mut capture = WgcMonitorCapture::start_with_factory(
            None,
            WgcCaptureOptions::default(),
            latest,
            &FakeFactory,
        )
        .expect("monitor capture");
        capture.stop().expect("stop");
        assert!(capture.is_finished());
    }
}
