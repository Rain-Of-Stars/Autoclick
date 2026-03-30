use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum PixelFormat {
    #[default]
    Bgra8,
    Rgba8,
    Gray8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FramePacket {
    pub frame_id: u64,
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub timestamp_ms: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FrameStats {
    pub published_frames: u64,
    pub dropped_frames: u64,
    pub last_frame_id: u64,
}

impl FramePacket {
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }
}
