use autoclick_capture::frame::{FramePacket, PixelFormat};
use fast_image_resize::{PixelType, ResizeAlg, ResizeOptions, Resizer, images::Image};
use image::{DynamicImage, GrayImage, ImageReader};

use crate::DetectError;

pub fn load_gray_image(path: impl AsRef<std::path::Path>) -> Result<GrayImage, DetectError> {
    let image = ImageReader::open(path)
        .map_err(|err| DetectError::Image(err.to_string()))?
        .decode()
        .map_err(|err| DetectError::Image(err.to_string()))?;
    Ok(image.to_luma8())
}

pub fn grayscale_from_frame(frame: &FramePacket) -> Result<GrayImage, DetectError> {
    match frame.pixel_format {
        PixelFormat::Gray8 => GrayImage::from_raw(frame.width, frame.height, frame.bytes.clone())
            .ok_or_else(|| DetectError::Image("无法从灰度帧构造图像".to_string())),
        PixelFormat::Rgba8 => DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(frame.width, frame.height, frame.bytes.clone())
                .ok_or_else(|| DetectError::Image("无法从RGBA帧构造图像".to_string()))?,
        )
        .to_luma8()
        .pipe(Ok),
        PixelFormat::Bgra8 => {
            let mut rgba = Vec::with_capacity(frame.bytes.len());
            for chunk in frame.bytes.chunks_exact(4) {
                rgba.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
            }
            DynamicImage::ImageRgba8(
                image::RgbaImage::from_raw(frame.width, frame.height, rgba)
                    .ok_or_else(|| DetectError::Image("无法从BGRA帧构造图像".to_string()))?,
            )
            .to_luma8()
            .pipe(Ok)
        }
    }
}

pub fn resize_gray(image: &GrayImage, scale: f32) -> Result<GrayImage, DetectError> {
    if (scale - 1.0).abs() < f32::EPSILON {
        return Ok(image.clone());
    }
    let dst_width = ((image.width() as f32) * scale).round().max(1.0) as u32;
    let dst_height = ((image.height() as f32) * scale).round().max(1.0) as u32;
    let src = Image::from_vec_u8(
        image.width(),
        image.height(),
        image.clone().into_raw(),
        PixelType::U8,
    )
    .map_err(|err| DetectError::Image(err.to_string()))?;
    let mut dst = Image::new(dst_width, dst_height, PixelType::U8);
    let mut resizer = Resizer::new();
    resizer
        .resize(
            &src,
            &mut dst,
            &ResizeOptions::new().resize_alg(ResizeAlg::Convolution(
                fast_image_resize::FilterType::Lanczos3,
            )),
        )
        .map_err(|err| DetectError::Image(err.to_string()))?;
    GrayImage::from_raw(dst_width, dst_height, dst.into_vec())
        .ok_or_else(|| DetectError::Image("缩放后的灰度图构造失败".to_string()))
}

pub fn scale_list(scales: &[f32], multi_scale: bool) -> Vec<f32> {
    if multi_scale {
        let mut result = scales.to_vec();
        result.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        result.dedup_by(|left, right| (*left - *right).abs() < f32::EPSILON);
        result
    } else {
        vec![1.0]
    }
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}

#[cfg(test)]
mod tests {
    use autoclick_capture::frame::{FramePacket, PixelFormat};
    use image::Luma;

    use super::{grayscale_from_frame, resize_gray, scale_list};

    #[test]
    fn converts_rgba_frame_to_gray() {
        let frame = FramePacket {
            frame_id: 1,
            width: 1,
            height: 1,
            pixel_format: PixelFormat::Rgba8,
            timestamp_ms: 1,
            bytes: vec![255, 0, 0, 255],
        };
        let gray = grayscale_from_frame(&frame).expect("gray");
        assert_eq!(gray.get_pixel(0, 0)[0], image::Luma([54])[0]);
    }

    #[test]
    fn resizes_gray_image() {
        let image = image::GrayImage::from_pixel(4, 4, Luma([10]));
        let resized = resize_gray(&image, 0.5).expect("resize");
        assert_eq!(resized.width(), 2);
        assert_eq!(resized.height(), 2);
    }

    #[test]
    fn returns_single_scale_when_multi_scale_disabled() {
        assert_eq!(scale_list(&[0.8, 1.0, 1.2], false), vec![1.0]);
    }
}
