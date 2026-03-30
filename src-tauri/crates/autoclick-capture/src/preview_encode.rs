use std::io::Cursor;

use image::{ExtendedColorType, ImageEncoder};
use serde::{Deserialize, Serialize};

use crate::{CaptureError, convert::resize_for_preview, frame::FramePacket};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PreviewFormat {
    Png,
    Jpeg,
    Webp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewEncodeOptions {
    pub max_edge: u32,
    pub format: PreviewFormat,
    pub quality: u8,
}

impl Default for PreviewEncodeOptions {
    fn default() -> Self {
        Self {
            max_edge: 640,
            format: PreviewFormat::Png,
            quality: 85,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EncodedPreview {
    pub frame_id: u64,
    pub width: u32,
    pub height: u32,
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

pub fn encode_preview(
    frame: &FramePacket,
    options: &PreviewEncodeOptions,
) -> Result<EncodedPreview, CaptureError> {
    let image = resize_for_preview(frame, options.max_edge)?;
    let width = image.width();
    let height = image.height();
    let mut cursor = Cursor::new(Vec::new());

    let mime_type = match options.format {
        PreviewFormat::Png => {
            image
                .write_to(&mut cursor, image::ImageFormat::Png)
                .map_err(|err| CaptureError::Encode(err.to_string()))?;
            "image/png"
        }
        PreviewFormat::Jpeg => {
            let rgb = image.to_rgb8();
            let encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, options.quality);
            encoder
                .write_image(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    ExtendedColorType::Rgb8,
                )
                .map_err(|err| CaptureError::Encode(err.to_string()))?;
            "image/jpeg"
        }
        PreviewFormat::Webp => {
            image
                .write_to(&mut cursor, image::ImageFormat::WebP)
                .map_err(|err| CaptureError::Encode(err.to_string()))?;
            "image/webp"
        }
    };

    Ok(EncodedPreview {
        frame_id: frame.frame_id,
        width,
        height,
        mime_type: mime_type.to_string(),
        bytes: cursor.into_inner(),
    })
}

#[cfg(test)]
mod tests {
    use crate::frame::{FramePacket, PixelFormat};

    use super::{PreviewEncodeOptions, PreviewFormat, encode_preview};

    #[test]
    fn convert_encode_preview_png() {
        let frame = FramePacket {
            frame_id: 1,
            width: 8,
            height: 8,
            pixel_format: PixelFormat::Gray8,
            timestamp_ms: 1,
            bytes: vec![200; 64],
        };
        let encoded = encode_preview(
            &frame,
            &PreviewEncodeOptions {
                max_edge: 8,
                format: PreviewFormat::Png,
                quality: 80,
            },
        )
        .expect("encode");
        assert_eq!(encoded.mime_type, "image/png");
        assert!(!encoded.bytes.is_empty());
    }
}
