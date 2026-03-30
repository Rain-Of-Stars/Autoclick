use std::sync::Arc;

use autoclick_domain::types::Roi;
use image::GrayImage;
use rayon::prelude::*;

use crate::{
    DetectError,
    hit_policy::{HitDecision, HitPolicy},
    r#match::{MatchResult, match_template_gray},
    preprocess::{resize_gray, scale_list},
    roi::crop_gray,
    template_store::LoadedTemplate,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineResult {
    pub best_match: Option<MatchResult>,
}

pub struct PipelinePolicyRequest<'a> {
    pub frame: &'a GrayImage,
    pub roi: &'a Roi,
    pub templates: &'a [Arc<LoadedTemplate>],
    pub scales: &'a [f32],
    pub multi_scale: bool,
    pub threshold: f32,
    pub early_exit: bool,
    pub frame_timestamp_ms: u64,
}

pub fn run_pipeline_with_policy(
    request: PipelinePolicyRequest<'_>,
    hit_policy: &mut HitPolicy,
) -> Result<(PipelineResult, HitDecision), DetectError> {
    let result = run_pipeline(
        request.frame,
        request.roi,
        request.templates,
        request.scales,
        request.multi_scale,
        request.threshold,
        request.early_exit,
    )?;
    let decision = hit_policy.evaluate(result.best_match.clone(), request.frame_timestamp_ms);
    Ok((result, decision))
}

pub fn run_pipeline(
    frame: &GrayImage,
    roi: &Roi,
    templates: &[Arc<LoadedTemplate>],
    scales: &[f32],
    multi_scale: bool,
    threshold: f32,
    early_exit: bool,
) -> Result<PipelineResult, DetectError> {
    let (cropped, normalized) = crop_gray(frame, roi);
    let scale_values = scale_list(scales, multi_scale);

    let mut best_match = None;
    if early_exit {
        for template in templates {
            for scale in &scale_values {
                let scaled_template = resize_gray(&template.image, *scale)?;
                let Some(mut matched) = match_template_gray(
                    &cropped,
                    &scaled_template,
                    *scale,
                    &template.meta.name,
                    &template.meta.id.to_string(),
                ) else {
                    continue;
                };

                let (x, y) = crate::roi::map_point_back(normalized, matched.x, matched.y);
                matched.x = x;
                matched.y = y;

                if matched.score >= threshold {
                    return Ok(PipelineResult {
                        best_match: Some(matched),
                    });
                }

                if best_match
                    .as_ref()
                    .is_none_or(|current: &MatchResult| matched.score > current.score)
                {
                    best_match = Some(matched);
                }
            }
        }

        return Ok(PipelineResult {
            best_match: best_match.filter(|candidate| candidate.score >= threshold),
        });
    }

    let matches = templates
        .par_iter()
        .flat_map_iter(|template| {
            scale_values.iter().filter_map(|scale| {
                let scaled_template = resize_gray(&template.image, *scale).ok()?;
                match_template_gray(
                    &cropped,
                    &scaled_template,
                    *scale,
                    &template.meta.name,
                    &template.meta.id.to_string(),
                )
            })
        })
        .collect::<Vec<_>>();

    let mut best = matches.into_iter().max_by(|left, right| {
        left.score
            .partial_cmp(&right.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if let Some(candidate) = best.as_mut() {
        let (x, y) = crate::roi::map_point_back(normalized, candidate.x, candidate.y);
        candidate.x = x;
        candidate.y = y;
        if candidate.score < threshold {
            best = None;
        } else if early_exit {
            return Ok(PipelineResult { best_match: best });
        }
    }

    Ok(PipelineResult { best_match: best })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use autoclick_domain::{template::TemplateRef, types::Roi};

    use super::{PipelinePolicyRequest, run_pipeline, run_pipeline_with_policy};
    use crate::hit_policy::{HitDecision, HitPolicy, HitPolicyConfig};
    use crate::template_store::LoadedTemplate;

    #[test]
    fn finds_best_match_in_roi() {
        let mut frame = image::GrayImage::from_pixel(10, 10, image::Luma([0]));
        for x in 4..6 {
            for y in 5..7 {
                frame.put_pixel(x, y, image::Luma([255]));
            }
        }
        let template = image::GrayImage::from_pixel(2, 2, image::Luma([255]));
        let loaded = Arc::new(LoadedTemplate {
            meta: TemplateRef::new("sample"),
            image: template,
        });
        let result = run_pipeline(
            &frame,
            &Roi {
                x: 2,
                y: 4,
                width: 6,
                height: 4,
            },
            &[loaded],
            &[1.0],
            false,
            0.8,
            true,
        )
        .expect("pipeline");
        let matched = result.best_match.expect("best match");
        assert_eq!((matched.x, matched.y), (4, 5));
    }

    #[test]
    fn early_exit_returns_threshold_hit() {
        let frame = image::GrayImage::from_pixel(6, 6, image::Luma([255]));
        let template = image::GrayImage::from_pixel(2, 2, image::Luma([255]));
        let loaded = Arc::new(LoadedTemplate {
            meta: TemplateRef::new("sample"),
            image: template,
        });
        let result = run_pipeline(
            &frame,
            &Roi::default(),
            &[loaded],
            &[1.0],
            false,
            0.99,
            true,
        )
        .expect("pipeline");
        assert!(result.best_match.is_some());
    }

    #[test]
    fn pipeline_with_policy_applies_click_decision() {
        let frame = image::GrayImage::from_pixel(4, 4, image::Luma([255]));
        let template = image::GrayImage::from_pixel(2, 2, image::Luma([255]));
        let loaded = Arc::new(LoadedTemplate {
            meta: TemplateRef::new("sample"),
            image: template,
        });
        let mut policy = HitPolicy::new(HitPolicyConfig {
            threshold: 0.9,
            min_detections: 1,
            cooldown_ms: 0,
        });
        let (_, decision) = run_pipeline_with_policy(
            PipelinePolicyRequest {
                frame: &frame,
                roi: &Roi::default(),
                templates: &[loaded],
                scales: &[1.0],
                multi_scale: false,
                threshold: 0.9,
                early_exit: true,
                frame_timestamp_ms: 100,
            },
            &mut policy,
        )
        .expect("pipeline policy");
        assert!(matches!(decision, HitDecision::ShouldClick(_)));
    }
}
