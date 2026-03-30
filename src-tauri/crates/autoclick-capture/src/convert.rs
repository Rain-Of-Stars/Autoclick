use image::{DynamicImage, GrayImage, RgbaImage, imageops};

use crate::{
    CaptureError,
    frame::{FramePacket, PixelFormat},
};

pub fn frame_to_rgba_image(frame: &FramePacket) -> Result<RgbaImage, CaptureError> {
    match frame.pixel_format {
        PixelFormat::Rgba8 => RgbaImage::from_raw(frame.width, frame.height, frame.bytes.clone())
            .ok_or_else(|| CaptureError::Convert("RGBA 帧尺寸与缓冲区长度不匹配".to_string())),
        PixelFormat::Bgra8 => {
            let mut bytes = Vec::with_capacity(frame.bytes.len());
            for chunk in frame.bytes.chunks_exact(4) {
                bytes.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
            }
            RgbaImage::from_raw(frame.width, frame.height, bytes)
                .ok_or_else(|| CaptureError::Convert("BGRA 帧转换为 RGBA 失败".to_string()))
        }
        PixelFormat::Gray8 => {
            let mut bytes = Vec::with_capacity(frame.bytes.len() * 4);
            for value in &frame.bytes {
                bytes.extend_from_slice(&[*value, *value, *value, 255]);
            }
            RgbaImage::from_raw(frame.width, frame.height, bytes)
                .ok_or_else(|| CaptureError::Convert("Gray 帧转换为 RGBA 失败".to_string()))
        }
    }
}

pub fn frame_to_gray_image(frame: &FramePacket) -> Result<GrayImage, CaptureError> {
    match frame.pixel_format {
        PixelFormat::Gray8 => GrayImage::from_raw(frame.width, frame.height, frame.bytes.clone())
            .ok_or_else(|| CaptureError::Convert("Gray 帧尺寸与缓冲区长度不匹配".to_string())),
        _ => Ok(DynamicImage::ImageRgba8(frame_to_rgba_image(frame)?).to_luma8()),
    }
}

pub fn to_gray_frame(frame: &FramePacket) -> Result<FramePacket, CaptureError> {
    let image = frame_to_gray_image(frame)?;
    Ok(FramePacket {
        frame_id: frame.frame_id,
        width: image.width(),
        height: image.height(),
        pixel_format: PixelFormat::Gray8,
        timestamp_ms: frame.timestamp_ms,
        bytes: image.into_raw(),
    })
}

pub fn resize_for_preview(
    frame: &FramePacket,
    max_edge: u32,
) -> Result<DynamicImage, CaptureError> {
    let image = DynamicImage::ImageRgba8(frame_to_rgba_image(frame)?);
    if max_edge == 0 {
        return Ok(image);
    }

    let resized = imageops::thumbnail(&image, max_edge, max_edge);
    Ok(DynamicImage::ImageRgba8(resized))
}

#[cfg(test)]
mod tests {
    use crate::frame::{FramePacket, PixelFormat};

    use super::{frame_to_gray_image, frame_to_rgba_image, resize_for_preview, to_gray_frame};

    #[test]
    fn convert_bgra_frame_to_rgba_image() {
        let frame = FramePacket {
            frame_id: 1,
            width: 1,
            height: 1,
            pixel_format: PixelFormat::Bgra8,
            timestamp_ms: 1,
            bytes: vec![10, 20, 30, 255],
        };
        let image = frame_to_rgba_image(&frame).expect("rgba");
        assert_eq!(image.get_pixel(0, 0).0, [30, 20, 10, 255]);
    }

    #[test]
    fn convert_rgba_frame_to_gray_frame() {
        let frame = FramePacket {
            frame_id: 1,
            width: 1,
            height: 1,
            pixel_format: PixelFormat::Rgba8,
            timestamp_ms: 1,
            bytes: vec![255, 0, 0, 255],
        };
        let gray = to_gray_frame(&frame).expect("gray");
        assert_eq!(gray.pixel_format, PixelFormat::Gray8);
        assert_eq!(gray.bytes.len(), 1);
    }

    #[test]
    fn convert_resize_preview_thumbnail() {
        let frame = FramePacket {
            frame_id: 1,
            width: 16,
            height: 8,
            pixel_format: PixelFormat::Gray8,
            timestamp_ms: 1,
            bytes: vec![100; 128],
        };
        let preview = resize_for_preview(&frame, 4).expect("preview");
        assert!(preview.width() <= 4);
        assert!(preview.height() <= 4);
    }

    #[test]
    fn convert_gray_frame_to_gray_image() {
        let frame = FramePacket {
            frame_id: 1,
            width: 2,
            height: 2,
            pixel_format: PixelFormat::Gray8,
            timestamp_ms: 1,
            bytes: vec![1, 2, 3, 4],
        };
        let image = frame_to_gray_image(&frame).expect("gray image");
        assert_eq!(image.width(), 2);
    }
}
