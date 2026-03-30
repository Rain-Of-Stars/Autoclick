use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use autoclick_domain::types::Roi;
use image::{DynamicImage, GrayImage, Rgba, RgbaImage};
use imageproc::{drawing::draw_hollow_rect_mut, rect::Rect};
use serde::{Deserialize, Serialize};

use crate::{DetectError, r#match::MatchResult, roi::crop_gray};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DebugDumpOptions {
    pub enabled: bool,
    pub output_dir: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DebugDumpArtifacts {
    pub original_path: PathBuf,
    pub roi_path: PathBuf,
    pub overlay_path: PathBuf,
}

pub fn save_debug_dump(
    frame: &GrayImage,
    roi: &Roi,
    matched: Option<&MatchResult>,
    options: &DebugDumpOptions,
) -> Result<Option<DebugDumpArtifacts>, DetectError> {
    if !options.enabled {
        return Ok(None);
    }

    let output_dir = Path::new(&options.output_dir);
    fs::create_dir_all(output_dir).map_err(|err| DetectError::Image(err.to_string()))?;
    let prefix = format!("{}_{}", options.session_id, unix_timestamp_ms());

    let original_path = output_dir.join(format!("{prefix}_original.png"));
    let roi_path = output_dir.join(format!("{prefix}_roi.png"));
    let overlay_path = output_dir.join(format!("{prefix}_overlay.png"));

    DynamicImage::ImageLuma8(frame.clone())
        .save(&original_path)
        .map_err(|err| DetectError::Image(err.to_string()))?;

    let (roi_image, _) = crop_gray(frame, roi);
    DynamicImage::ImageLuma8(roi_image)
        .save(&roi_path)
        .map_err(|err| DetectError::Image(err.to_string()))?;

    let mut overlay = DynamicImage::ImageLuma8(frame.clone()).to_rgba8();
    if let Some(matched) = matched {
        draw_match_box(&mut overlay, matched);
    }
    DynamicImage::ImageRgba8(overlay)
        .save(&overlay_path)
        .map_err(|err| DetectError::Image(err.to_string()))?;

    Ok(Some(DebugDumpArtifacts {
        original_path,
        roi_path,
        overlay_path,
    }))
}

fn draw_match_box(image: &mut RgbaImage, matched: &MatchResult) {
    let rect = Rect::at(matched.x as i32, matched.y as i32).of_size(matched.width, matched.height);
    draw_hollow_rect_mut(image, rect, Rgba([255, 0, 0, 255]));
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use autoclick_domain::types::Roi;

    use super::{DebugDumpOptions, save_debug_dump};
    use crate::r#match::MatchResult;

    #[test]
    fn debug_dump_writes_expected_images() {
        let dir =
            std::env::temp_dir().join(format!("autoclick-debug-dump-{}", uuid::Uuid::new_v4()));
        let frame = image::GrayImage::from_pixel(20, 20, image::Luma([100]));
        let artifacts = save_debug_dump(
            &frame,
            &Roi {
                x: 2,
                y: 3,
                width: 10,
                height: 10,
            },
            Some(&MatchResult {
                template_id: "id".to_string(),
                template_name: "sample".to_string(),
                score: 0.95,
                x: 4,
                y: 5,
                width: 6,
                height: 7,
                scale: 1.0,
            }),
            &DebugDumpOptions {
                enabled: true,
                output_dir: dir.to_string_lossy().to_string(),
                session_id: "session-a".to_string(),
            },
        )
        .expect("dump")
        .expect("artifacts");
        assert!(artifacts.original_path.exists());
        assert!(artifacts.roi_path.exists());
        assert!(artifacts.overlay_path.exists());
    }
}
