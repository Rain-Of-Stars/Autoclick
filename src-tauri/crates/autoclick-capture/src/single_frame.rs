use std::time::Duration;

use crate::{
    CaptureError,
    frame::FramePacket,
    session::{CaptureSession, CaptureSessionConfig},
};

pub fn single_frame_from_buffer(frame: Option<FramePacket>) -> Result<FramePacket, CaptureError> {
    frame.ok_or(CaptureError::FrameUnavailable)
}

pub fn capture_single_frame(
    config: CaptureSessionConfig,
    timeout: Duration,
) -> Result<FramePacket, CaptureError> {
    let mut session = CaptureSession::new();
    capture_single_frame_with_session(&mut session, config, timeout)
}

pub fn capture_single_frame_with_session(
    session: &mut CaptureSession,
    config: CaptureSessionConfig,
    timeout: Duration,
) -> Result<FramePacket, CaptureError> {
    session.start(config)?;
    let result = session.read_next_frame(0, timeout);
    let stop_result = session.stop();

    match (result, stop_result) {
        (Ok(frame), Ok(())) => Ok(frame),
        (Ok(_), Err(err)) => Err(err),
        (Err(err), _) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread, time::Duration};

    use crate::{
        CaptureError,
        backend::{CaptureFactory, CaptureSharedState, RunningCapture, WgcCaptureOptions},
        frame::PixelFormat,
        session::{CaptureSession, CaptureSessionConfig, CaptureTarget},
    };

    use super::capture_single_frame_with_session;

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
                shared.publish_frame(4, 2, PixelFormat::Gray8, vec![1; 8]);
            });
            Ok(Box::new(FakeRunner))
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
    fn single_frame_returns_first_arrived_frame() {
        let mut session = CaptureSession::with_factory(Arc::new(FakeFactory));
        let frame = capture_single_frame_with_session(
            &mut session,
            CaptureSessionConfig {
                target: CaptureTarget::Window { hwnd: 100 },
                options: WgcCaptureOptions::default(),
            },
            Duration::from_millis(200),
        )
        .expect("single frame");
        assert_eq!(frame.width, 4);
        assert_eq!(frame.height, 2);
    }
}
