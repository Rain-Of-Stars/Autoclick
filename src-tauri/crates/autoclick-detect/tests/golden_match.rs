use std::sync::Arc;

use autoclick_detect::{
    hit_policy::{HitDecision, HitPolicy, HitPolicyConfig},
    pipeline::{PipelinePolicyRequest, run_pipeline, run_pipeline_with_policy},
    template_store::LoadedTemplate,
};
use autoclick_domain::{template::TemplateRef, types::Roi};

fn make_frame_with_block() -> image::GrayImage {
    let mut frame = image::GrayImage::from_pixel(12, 12, image::Luma([0]));
    for x in 5..8 {
        for y in 6..9 {
            frame.put_pixel(x, y, image::Luma([255]));
        }
    }
    frame
}

fn make_template() -> Arc<LoadedTemplate> {
    Arc::new(LoadedTemplate {
        meta: TemplateRef::new("golden-sample"),
        image: image::GrayImage::from_pixel(3, 3, image::Luma([255])),
    })
}

#[test]
fn golden_pipeline_returns_expected_coordinate() {
    let result = run_pipeline(
        &make_frame_with_block(),
        &Roi::default(),
        &[make_template()],
        &[1.0],
        false,
        0.95,
        false,
    )
    .expect("pipeline");

    let matched = result.best_match.expect("best");
    assert_eq!((matched.x, matched.y), (5, 6));
    assert!(matched.score > 0.99);
}

#[test]
fn golden_policy_requires_two_consecutive_hits() {
    let frame = make_frame_with_block();
    let template = make_template();
    let mut policy = HitPolicy::new(HitPolicyConfig {
        threshold: 0.95,
        min_detections: 2,
        cooldown_ms: 0,
    });

    let (_, first) = run_pipeline_with_policy(
        PipelinePolicyRequest {
            frame: &frame,
            roi: &Roi::default(),
            templates: std::slice::from_ref(&template),
            scales: &[1.0],
            multi_scale: false,
            threshold: 0.95,
            early_exit: false,
            frame_timestamp_ms: 10,
        },
        &mut policy,
    )
    .expect("first");
    let (_, second) = run_pipeline_with_policy(
        PipelinePolicyRequest {
            frame: &frame,
            roi: &Roi::default(),
            templates: &[template],
            scales: &[1.0],
            multi_scale: false,
            threshold: 0.95,
            early_exit: false,
            frame_timestamp_ms: 20,
        },
        &mut policy,
    )
    .expect("second");

    assert_eq!(first, HitDecision::Pending(1));
    assert!(matches!(second, HitDecision::ShouldClick(_)));
}
