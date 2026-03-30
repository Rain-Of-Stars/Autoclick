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

pub struct WgcWindowCapture {
    target_hwnd: isize,
    options: WgcCaptureOptions,
    shared: Arc<CaptureSharedState>,
    runner: Option<Box<dyn RunningCapture>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowCaptureSnapshot {
    pub target_hwnd: isize,
    pub options: WgcCaptureOptions,
    pub is_finished: bool,
    pub is_closed: bool,
    pub last_dimensions: Option<(u32, u32)>,
    pub last_error: Option<String>,
    pub stats: FrameStats,
}

impl WgcWindowCapture {
    pub fn start(
        target_hwnd: isize,
        options: WgcCaptureOptions,
        latest: Arc<LatestFrameBuffer>,
    ) -> Result<Self, CaptureError> {
        Self::start_with_factory(target_hwnd, options, latest, &WindowsCaptureFactory)
    }

    pub fn start_with_factory(
        target_hwnd: isize,
        options: WgcCaptureOptions,
        latest: Arc<LatestFrameBuffer>,
        factory: &dyn CaptureFactory,
    ) -> Result<Self, CaptureError> {
        let shared = Arc::new(CaptureSharedState::new(latest));
        let runner = factory.start_window(target_hwnd, &options, shared.clone())?;
        Ok(Self {
            target_hwnd,
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

    pub fn snapshot(&self) -> WindowCaptureSnapshot {
        let shared = self.shared_snapshot();
        WindowCaptureSnapshot {
            target_hwnd: self.target_hwnd,
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

impl Drop for WgcWindowCapture {
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

    use super::WgcWindowCapture;
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
            hwnd: isize,
            _options: &WgcCaptureOptions,
            shared: Arc<CaptureSharedState>,
        ) -> Result<Box<dyn RunningCapture>, CaptureError> {
            if hwnd == 0 {
                return Err(CaptureError::InvalidTarget("窗口句柄不能为空"));
            }
            shared.publish_frame(2, 2, PixelFormat::Gray8, vec![7; 4]);
            Ok(Box::new(FakeRunner::default()))
        }

        fn start_monitor(
            &self,
            _monitor_handle: Option<isize>,
            _options: &WgcCaptureOptions,
            _shared: Arc<CaptureSharedState>,
        ) -> Result<Box<dyn RunningCapture>, CaptureError> {
            unreachable!()
        }
    }

    #[test]
    fn wgc_window_publishes_latest_frame() {
        let latest = Arc::new(LatestFrameBuffer::new());
        let capture = WgcWindowCapture::start_with_factory(
            100,
            WgcCaptureOptions::default(),
            latest,
            &FakeFactory,
        )
        .expect("window capture");
        let frame = capture.read_latest().expect("latest");
        assert_eq!(frame.width, 2);
        assert_eq!(frame.pixel_format, PixelFormat::Gray8);
    }

    #[test]
    fn wgc_window_stop_marks_finished() {
        let latest = Arc::new(LatestFrameBuffer::new());
        let mut capture = WgcWindowCapture::start_with_factory(
            100,
            WgcCaptureOptions::default(),
            latest,
            &FakeFactory,
        )
        .expect("window capture");
        capture.stop().expect("stop");
        assert!(capture.is_finished());
    }
}
