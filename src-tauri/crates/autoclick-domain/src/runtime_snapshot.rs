use serde::{Deserialize, Serialize};

use crate::types::RuntimeStatus;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshot {
    pub status: RuntimeStatus,
    pub performance: PerformanceSnapshot,
    pub capture: CaptureSnapshot,
    pub recovery: RecoverySnapshot,
    pub preview: PreviewSnapshot,
    pub last_error: Option<String>,
}

impl Default for RuntimeSnapshot {
    fn default() -> Self {
        Self {
            status: RuntimeStatus::Idle,
            performance: PerformanceSnapshot::default(),
            capture: CaptureSnapshot::default(),
            recovery: RecoverySnapshot::default(),
            preview: PreviewSnapshot::default(),
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSnapshot {
    pub capture_fps: f32,
    pub frame_interval_ms: f32,
    pub detect_latency_ms: f32,
    pub preview_latency_ms: f32,
    pub end_to_end_latency_ms: f32,
    pub click_count: u64,
    pub last_score: f32,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSnapshot {
    pub frame_width: u32,
    pub frame_height: u32,
    pub drops: u64,
    pub active_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecoverySnapshot {
    pub attempts: u32,
    pub last_reason: Option<String>,
    pub next_retry_in_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSnapshot {
    pub enabled: bool,
    pub frame_token: Option<String>,
    pub width: u32,
    pub height: u32,
}

#[cfg(test)]
mod tests {
    use super::RuntimeSnapshot;
    use crate::types::RuntimeStatus;

    #[test]
    fn default_runtime_snapshot_is_idle() {
        let snapshot = RuntimeSnapshot::default();
        assert_eq!(snapshot.status, RuntimeStatus::Idle);
        assert_eq!(snapshot.performance.click_count, 0);
        assert!(!snapshot.preview.enabled);
    }
}
