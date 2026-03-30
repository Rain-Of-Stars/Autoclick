use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};

use crate::{
    CaptureError,
    frame::{FramePacket, FrameStats},
};

#[derive(Debug, Default)]
pub struct LatestFrameBuffer {
    inner: Mutex<LatestFrameState>,
    frame_arrived: Condvar,
}

#[derive(Debug, Default)]
struct LatestFrameState {
    latest: Option<FramePacket>,
    stats: FrameStats,
    closed: bool,
}

impl LatestFrameBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn publish(&self, frame: FramePacket) {
        let mut inner = self.inner.lock();
        if inner.latest.is_some() {
            inner.stats.dropped_frames += 1;
        }
        inner.closed = false;
        inner.stats.published_frames += 1;
        inner.stats.last_frame_id = frame.frame_id;
        inner.latest = Some(frame);
        self.frame_arrived.notify_all();
    }

    pub fn close(&self) {
        let mut inner = self.inner.lock();
        inner.closed = true;
        self.frame_arrived.notify_all();
    }

    pub fn read_latest(&self) -> Result<FramePacket, CaptureError> {
        self.inner
            .lock()
            .latest
            .clone()
            .ok_or(CaptureError::FrameUnavailable)
    }

    pub fn take_latest(&self) -> Result<FramePacket, CaptureError> {
        self.inner
            .lock()
            .latest
            .take()
            .ok_or(CaptureError::FrameUnavailable)
    }

    pub fn snapshot_stats(&self) -> FrameStats {
        self.inner.lock().stats
    }

    pub fn wait_for_newer_than(
        &self,
        last_frame_id: u64,
        timeout: Duration,
    ) -> Result<FramePacket, CaptureError> {
        let deadline = Instant::now() + timeout;
        let mut inner = self.inner.lock();

        loop {
            if let Some(frame) = inner.latest.as_ref() {
                if frame.frame_id > last_frame_id {
                    return Ok(frame.clone());
                }
            }

            if inner.closed {
                return Err(CaptureError::ItemClosed);
            }

            let now = Instant::now();
            if now >= deadline {
                return Err(CaptureError::Timeout);
            }

            let remaining = deadline.saturating_duration_since(now);
            let wait_result = self.frame_arrived.wait_for(&mut inner, remaining);
            if wait_result.timed_out() {
                return Err(CaptureError::Timeout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::CaptureError;
    use crate::frame::{FramePacket, PixelFormat};

    use super::LatestFrameBuffer;

    fn make_frame(frame_id: u64, fill: u8) -> FramePacket {
        FramePacket {
            frame_id,
            width: 2,
            height: 2,
            pixel_format: PixelFormat::Gray8,
            timestamp_ms: frame_id * 10,
            bytes: vec![fill; 4],
        }
    }

    #[test]
    fn keeps_only_latest_frame() {
        let buffer = LatestFrameBuffer::new();
        buffer.publish(make_frame(1, 1));
        buffer.publish(make_frame(2, 2));
        let frame = buffer.read_latest().expect("latest frame");
        assert_eq!(frame.frame_id, 2);
        assert_eq!(buffer.snapshot_stats().dropped_frames, 1);
    }

    #[test]
    fn take_latest_drains_buffer() {
        let buffer = LatestFrameBuffer::new();
        buffer.publish(make_frame(3, 9));
        let frame = buffer.take_latest().expect("take latest");
        assert_eq!(frame.frame_id, 3);
        assert!(buffer.take_latest().is_err());
    }

    #[test]
    fn waits_for_new_frame() {
        let buffer = std::sync::Arc::new(LatestFrameBuffer::new());
        let producer = buffer.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            producer.publish(make_frame(4, 7));
        });

        let frame = buffer
            .wait_for_newer_than(0, Duration::from_millis(200))
            .expect("wait frame");
        assert_eq!(frame.frame_id, 4);
    }

    #[test]
    fn returns_item_closed_when_buffer_is_closed() {
        let buffer = std::sync::Arc::new(LatestFrameBuffer::new());
        let closer = buffer.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            closer.close();
        });

        let result = buffer.wait_for_newer_than(0, Duration::from_millis(200));

        assert!(matches!(result, Err(CaptureError::ItemClosed)));
    }
}
