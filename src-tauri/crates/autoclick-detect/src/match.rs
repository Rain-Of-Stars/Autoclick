use image::GrayImage;
use imageproc::template_matching::{MatchTemplateMethod, find_extremes, match_template};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MatchResult {
    pub template_id: String,
    pub template_name: String,
    pub score: f32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub scale: f32,
}

pub fn match_template_gray(
    image: &GrayImage,
    template: &GrayImage,
    scale: f32,
    template_name: &str,
    template_id: &str,
) -> Option<MatchResult> {
    if template.width() == 0
        || template.height() == 0
        || template.width() > image.width()
        || template.height() > image.height()
    {
        return None;
    }

    let score_image = match_template(
        image,
        template,
        MatchTemplateMethod::CrossCorrelationNormalized,
    );
    let extremes = find_extremes(&score_image);
    Some(MatchResult {
        template_id: template_id.to_string(),
        template_name: template_name.to_string(),
        score: extremes.max_value,
        x: extremes.max_value_location.0,
        y: extremes.max_value_location.1,
        width: template.width(),
        height: template.height(),
        scale,
    })
}

#[cfg(test)]
mod tests {
    use super::match_template_gray;

    #[test]
    fn match_returns_best_location() {
        let mut image = image::GrayImage::from_pixel(5, 5, image::Luma([0]));
        image.put_pixel(2, 3, image::Luma([255]));
        image.put_pixel(3, 3, image::Luma([255]));
        image.put_pixel(2, 4, image::Luma([255]));
        image.put_pixel(3, 4, image::Luma([255]));
        let template = image::GrayImage::from_pixel(2, 2, image::Luma([255]));
        let matched = match_template_gray(&image, &template, 1.0, "sample", "id-1").expect("match");
        assert_eq!((matched.x, matched.y), (2, 3));
        assert!(matched.score > 0.99);
    }

    #[test]
    fn match_rejects_template_larger_than_image() {
        let image = image::GrayImage::from_pixel(2, 2, image::Luma([0]));
        let template = image::GrayImage::from_pixel(3, 3, image::Luma([0]));
        assert!(match_template_gray(&image, &template, 1.0, "sample", "id").is_none());
    }
}
