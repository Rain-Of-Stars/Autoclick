use std::{sync::Arc, time::Duration};

use autoclick_detect::{
    hit_policy::{HitPolicy, HitPolicyConfig},
    r#match::match_template_gray,
    pipeline::{PipelinePolicyRequest, run_pipeline, run_pipeline_with_policy},
    template_store::LoadedTemplate,
};
use autoclick_domain::{template::TemplateRef, types::Roi};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

fn build_frame() -> image::GrayImage {
    let mut frame = image::GrayImage::from_pixel(128, 72, image::Luma([0]));
    for x in 40..56 {
        for y in 24..40 {
            frame.put_pixel(x, y, image::Luma([255]));
        }
    }
    frame
}

fn build_templates(count: usize) -> Vec<Arc<LoadedTemplate>> {
    (0..count)
        .map(|index| {
            Arc::new(LoadedTemplate {
                meta: TemplateRef::new(format!("template-{index}")),
                image: image::GrayImage::from_pixel(16, 16, image::Luma([255])),
            })
        })
        .collect()
}

fn bench_single_template_match(c: &mut Criterion) {
    let frame = build_frame();
    let template = image::GrayImage::from_pixel(16, 16, image::Luma([255]));

    let mut group = c.benchmark_group("single_template_match");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(1));
    group.throughput(Throughput::Elements(1));
    group.bench_function("ccorr_normalized", |b| {
        b.iter(|| {
            let matched = match_template_gray(
                black_box(&frame),
                black_box(&template),
                1.0,
                "sample",
                "id-1",
            )
            .expect("match");
            black_box(matched.score);
        });
    });
    group.finish();
}

fn bench_multi_template_pipeline(c: &mut Criterion) {
    let frame = build_frame();
    let templates = build_templates(2);

    let mut group = c.benchmark_group("multi_template_pipeline");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(1));
    group.throughput(Throughput::Elements(templates.len() as u64));
    group.bench_function("pipeline_without_early_exit", |b| {
        b.iter(|| {
            let result = run_pipeline(
                black_box(&frame),
                black_box(&Roi::default()),
                black_box(&templates),
                black_box(&[1.0, 0.9]),
                true,
                0.9,
                false,
            )
            .expect("pipeline");
            black_box(result.best_match.as_ref().map(|matched| matched.score));
        });
    });
    group.finish();
}

fn bench_pipeline_with_policy(c: &mut Criterion) {
    let frame = build_frame();
    let templates = build_templates(2);

    let mut group = c.benchmark_group("pipeline_with_policy");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(1));
    group.bench_function("two_hit_policy", |b| {
        b.iter(|| {
            let mut policy = HitPolicy::new(HitPolicyConfig {
                threshold: 0.9,
                min_detections: 2,
                cooldown_ms: 0,
            });
            let (_, first) = run_pipeline_with_policy(
                PipelinePolicyRequest {
                    frame: black_box(&frame),
                    roi: black_box(&Roi::default()),
                    templates: black_box(&templates),
                    scales: black_box(&[1.0]),
                    multi_scale: false,
                    threshold: 0.9,
                    early_exit: false,
                    frame_timestamp_ms: 10,
                },
                &mut policy,
            )
            .expect("first");
            let (_, second) = run_pipeline_with_policy(
                PipelinePolicyRequest {
                    frame: black_box(&frame),
                    roi: black_box(&Roi::default()),
                    templates: black_box(&templates),
                    scales: black_box(&[1.0]),
                    multi_scale: false,
                    threshold: 0.9,
                    early_exit: false,
                    frame_timestamp_ms: 20,
                },
                &mut policy,
            )
            .expect("second");
            black_box((first, second));
        });
    });
    group.finish();
}

criterion_group!(
    detect_bench,
    bench_single_template_match,
    bench_multi_template_pipeline,
    bench_pipeline_with_policy
);
criterion_main!(detect_bench);
