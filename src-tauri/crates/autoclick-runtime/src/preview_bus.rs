use std::time::{Duration, Instant};

use autoclick_capture::{
    frame::FramePacket,
    preview_encode::{EncodedPreview, PreviewEncodeOptions, encode_preview},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::RuntimeError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewBusConfig {
    pub enabled: bool,
    pub throttle_ms: u64,
    pub encode: PreviewEncodeOptions,
}

impl Default for PreviewBusConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            throttle_ms: 250,
            encode: PreviewEncodeOptions::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewMessage {
    pub token: String,
    pub preview: EncodedPreview,
}

#[derive(Debug)]
pub struct PreviewBus {
    config: PreviewBusConfig,
    inner: Mutex<PreviewBusState>,
}

#[derive(Debug, Default)]
struct PreviewBusState {
    last_publish_at: Option<Instant>,
    latest: Option<PreviewMessage>,
}

impl PreviewBus {
    pub fn new(config: PreviewBusConfig) -> Self {
        Self {
            config,
            inner: Mutex::new(PreviewBusState::default()),
        }
    }

    pub fn publish(&self, frame: &FramePacket) -> Result<Option<PreviewMessage>, RuntimeError> {
        if !self.config.enabled {
            return Ok(None);
        }

        let mut inner = self.inner.lock();
        if let Some(last_publish_at) = inner.last_publish_at {
            if last_publish_at.elapsed() < Duration::from_millis(self.config.throttle_ms) {
                return Ok(None);
            }
        }

        let preview = encode_preview(frame, &self.config.encode)
            .map_err(|err| RuntimeError::Preview(err.to_string()))?;
        let message = PreviewMessage {
            token: format!("preview-{}", frame.frame_id),
            preview,
        };
        inner.last_publish_at = Some(Instant::now());
        inner.latest = Some(message.clone());
        Ok(Some(message))
    }

    pub fn latest(&self) -> Option<PreviewMessage> {
        self.inner.lock().latest.clone()
    }
}

#[cfg(test)]
mod tests {
    use autoclick_capture::frame::{FramePacket, PixelFormat};

    use super::{PreviewBus, PreviewBusConfig};

    #[test]
    fn preview_bus_throttles_updates() {
        let bus = PreviewBus::new(PreviewBusConfig {
            enabled: true,
            throttle_ms: 1_000,
            ..PreviewBusConfig::default()
        });
        let frame = FramePacket {
            frame_id: 1,
            width: 32,
            height: 32,
            pixel_format: PixelFormat::Gray8,
            timestamp_ms: 1,
            bytes: vec![120; 1_024],
        };
        assert!(bus.publish(&frame).expect("first").is_some());
        assert!(bus.publish(&frame).expect("second").is_none());
    }
}
