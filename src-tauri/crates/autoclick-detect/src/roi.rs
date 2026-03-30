use autoclick_domain::types::Roi;
use image::{GrayImage, imageops};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizedRoi {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

pub fn normalize_roi(frame_width: u32, frame_height: u32, roi: &Roi) -> NormalizedRoi {
    if roi.is_full_frame() {
        return NormalizedRoi {
            left: 0,
            top: 0,
            width: frame_width,
            height: frame_height,
        };
    }

    let left = roi.x.max(0) as u32;
    let top = roi.y.max(0) as u32;
    let right = (left + roi.width).min(frame_width);
    let bottom = (top + roi.height).min(frame_height);
    NormalizedRoi {
        left,
        top,
        width: right.saturating_sub(left),
        height: bottom.saturating_sub(top),
    }
}

pub fn crop_gray(image: &GrayImage, roi: &Roi) -> (GrayImage, NormalizedRoi) {
    let normalized = normalize_roi(image.width(), image.height(), roi);
    if normalized.left == 0
        && normalized.top == 0
        && normalized.width == image.width()
        && normalized.height == image.height()
    {
        return (image.clone(), normalized);
    }
    let cropped = imageops::crop_imm(
        image,
        normalized.left,
        normalized.top,
        normalized.width.max(1),
        normalized.height.max(1),
    )
    .to_image();
    (cropped, normalized)
}

pub fn map_point_back(normalized: NormalizedRoi, point_x: u32, point_y: u32) -> (u32, u32) {
    (normalized.left + point_x, normalized.top + point_y)
}

#[cfg(test)]
mod tests {
    use autoclick_domain::types::Roi;
    use image::GrayImage;

    use super::{crop_gray, map_point_back, normalize_roi};

    #[test]
    fn empty_roi_uses_full_frame() {
        let normalized = normalize_roi(200, 100, &Roi::default());
        assert_eq!(normalized.width, 200);
        assert_eq!(normalized.height, 100);
    }

    #[test]
    fn maps_point_back_to_full_frame() {
        let image = GrayImage::from_pixel(20, 20, image::Luma([0]));
        let roi = Roi {
            x: 5,
            y: 6,
            width: 10,
            height: 8,
        };
        let (_, normalized) = crop_gray(&image, &roi);
        assert_eq!(map_point_back(normalized, 2, 3), (7, 9));
    }

    #[test]
    fn clips_out_of_bounds_roi() {
        let normalized = normalize_roi(
            100,
            60,
            &Roi {
                x: -10,
                y: 40,
                width: 30,
                height: 40,
            },
        );
        assert_eq!(normalized.left, 0);
        assert_eq!(normalized.top, 40);
        assert_eq!(normalized.width, 30);
        assert_eq!(normalized.height, 20);
    }
}
