use std::time::Instant;

use autoclick_capture::frame::{FramePacket, FrameStats};
use autoclick_detect::r#match::MatchResult;
use autoclick_domain::runtime_snapshot::{
    CaptureSnapshot, PreviewSnapshot, RecoverySnapshot, RuntimeSnapshot,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMetricsSnapshot {
    pub runtime: RuntimeSnapshot,
    pub recovery_count: u32,
    pub buffer_drops: u64,
    pub memory_bytes_estimate: u64,
}

#[derive(Debug)]
pub struct RuntimeMetrics {
    snapshot: RuntimeMetricsSnapshot,
    started_at: Instant,
    last_frame_timestamp_ms: Option<u64>,
}

impl Default for RuntimeMetrics {
    fn default() -> Self {
        Self {
            snapshot: RuntimeMetricsSnapshot {
                runtime: RuntimeSnapshot::default(),
                recovery_count: 0,
                buffer_drops: 0,
                memory_bytes_estimate: 0,
            },
            started_at: Instant::now(),
            last_frame_timestamp_ms: None,
        }
    }
}

impl RuntimeMetrics {
    pub fn record_frame(&mut self, frame: &FramePacket, stats: FrameStats) {
        self.snapshot.runtime.performance.frame_interval_ms =
            estimate_frame_interval_ms(self.last_frame_timestamp_ms, frame.timestamp_ms);
        self.snapshot.runtime.performance.capture_fps =
            estimate_capture_fps(self.last_frame_timestamp_ms, frame.timestamp_ms);
        self.last_frame_timestamp_ms = Some(frame.timestamp_ms);
        self.snapshot.runtime.capture = CaptureSnapshot {
            frame_width: frame.width,
            frame_height: frame.height,
            drops: stats.dropped_frames,
            active_source: Some(frame_source_name(frame)),
        };
        self.snapshot.buffer_drops = stats.dropped_frames;
        self.snapshot.memory_bytes_estimate = frame.bytes.len() as u64;
        self.snapshot.runtime.performance.uptime_secs = self.started_at.elapsed().as_secs();
    }

    pub fn record_detection(&mut self, latency_ms: f32, matched: Option<&MatchResult>) {
        self.snapshot.runtime.performance.detect_latency_ms = latency_ms;
        self.snapshot.runtime.performance.last_score =
            matched.map(|value| value.score).unwrap_or(0.0);
    }

    pub fn record_preview_latency(&mut self, latency_ms: f32) {
        self.snapshot.runtime.performance.preview_latency_ms = latency_ms;
    }

    pub fn record_end_to_end_latency(&mut self, latency_ms: f32) {
        self.snapshot.runtime.performance.end_to_end_latency_ms = latency_ms;
    }

    pub fn record_click(&mut self) {
        self.snapshot.runtime.performance.click_count += 1;
    }

    pub fn record_recovery(
        &mut self,
        attempts: u32,
        reason: Option<String>,
        next_retry_in_ms: Option<u64>,
    ) {
        self.snapshot.recovery_count += 1;
        self.snapshot.runtime.recovery = RecoverySnapshot {
            attempts,
            last_reason: reason,
            next_retry_in_ms,
        };
    }

    pub fn record_preview(&mut self, width: u32, height: u32, token: String) {
        self.snapshot.runtime.preview = PreviewSnapshot {
            enabled: true,
            frame_token: Some(token),
            width,
            height,
        };
    }

    pub fn snapshot(&self) -> RuntimeMetricsSnapshot {
        self.snapshot.clone()
    }
}

fn frame_source_name(frame: &FramePacket) -> String {
    match frame.pixel_format {
        autoclick_capture::frame::PixelFormat::Bgra8 => "bgra".to_string(),
        autoclick_capture::frame::PixelFormat::Rgba8 => "rgba".to_string(),
        autoclick_capture::frame::PixelFormat::Gray8 => "gray".to_string(),
    }
}

fn estimate_capture_fps(previous_timestamp_ms: Option<u64>, current_timestamp_ms: u64) -> f32 {
    let Some(previous_timestamp_ms) = previous_timestamp_ms else {
        return 0.0;
    };

    let delta_ms = current_timestamp_ms.saturating_sub(previous_timestamp_ms);
    if delta_ms == 0 {
        return 0.0;
    }

    1000.0 / delta_ms as f32
}

fn estimate_frame_interval_ms(
    previous_timestamp_ms: Option<u64>,
    current_timestamp_ms: u64,
) -> f32 {
    let Some(previous_timestamp_ms) = previous_timestamp_ms else {
        return 0.0;
    };

    current_timestamp_ms.saturating_sub(previous_timestamp_ms) as f32
}

#[cfg(test)]
mod tests {
    use autoclick_capture::frame::{FramePacket, FrameStats, PixelFormat};
    use autoclick_detect::r#match::MatchResult;

    use super::RuntimeMetrics;

    #[test]
    fn metrics_aggregate_capture_and_detection() {
        let mut metrics = RuntimeMetrics::default();
        metrics.record_frame(
            &FramePacket {
                frame_id: 1,
                width: 320,
                height: 200,
                pixel_format: PixelFormat::Gray8,
                timestamp_ms: 1_000,
                bytes: vec![10; 64_000],
            },
            FrameStats {
                published_frames: 30,
                dropped_frames: 2,
                last_frame_id: 1,
            },
        );
        metrics.record_frame(
            &FramePacket {
                frame_id: 2,
                width: 320,
                height: 200,
                pixel_format: PixelFormat::Gray8,
                timestamp_ms: 1_100,
                bytes: vec![10; 64_000],
            },
            FrameStats {
                published_frames: 31,
                dropped_frames: 2,
                last_frame_id: 2,
            },
        );
        metrics.record_detection(
            4.5,
            Some(&MatchResult {
                template_id: "id".to_string(),
                template_name: "sample".to_string(),
                score: 0.97,
                x: 10,
                y: 20,
                width: 8,
                height: 8,
                scale: 1.0,
            }),
        );
        metrics.record_preview_latency(1.2);
        metrics.record_end_to_end_latency(6.8);
        metrics.record_click();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.runtime.capture.frame_width, 320);
        assert_eq!(snapshot.buffer_drops, 2);
        assert_eq!(snapshot.runtime.performance.click_count, 1);
        assert_eq!(snapshot.runtime.performance.capture_fps, 10.0);
        assert_eq!(snapshot.runtime.performance.frame_interval_ms, 100.0);
        assert_eq!(snapshot.runtime.performance.preview_latency_ms, 1.2);
        assert_eq!(snapshot.runtime.performance.end_to_end_latency_ms, 6.8);
        assert_eq!(snapshot.runtime.performance.last_score, 0.97);
    }
}
