use std::{sync::Arc, time::Instant};

use autoclick_capture::frame::FramePacket;
use autoclick_detect::{
    hit_policy::{HitDecision, HitPolicy, HitPolicyConfig},
    pipeline::{PipelinePolicyRequest, PipelineResult, run_pipeline_with_policy},
    preprocess::grayscale_from_frame,
    template_store::LoadedTemplate,
};
use autoclick_domain::{config::InputPolicy, types::Roi};
use autoclick_input::{
    policy::{ClickRequest, execute_click},
    post_message::ClickReport,
};
use autoclick_platform_win::window::WindowRect;
use serde::{Deserialize, Serialize};

use crate::{
    RuntimeError,
    metrics::{RuntimeMetrics, RuntimeMetricsSnapshot},
    preview_bus::{PreviewBus, PreviewBusConfig, PreviewMessage},
};

pub trait ClickExecutor: Send + Sync {
    fn execute(
        &self,
        request: &ClickRequest,
        policy: &InputPolicy,
    ) -> Result<ClickReport, RuntimeError>;
}

#[derive(Debug, Default)]
pub struct PolicyClickExecutor;

impl ClickExecutor for PolicyClickExecutor {
    fn execute(
        &self,
        request: &ClickRequest,
        policy: &InputPolicy,
    ) -> Result<ClickReport, RuntimeError> {
        execute_click(request, policy).map_err(|err| RuntimeError::Input(err.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScannerEngineConfig {
    pub roi: Roi,
    pub scales: Vec<f32>,
    pub multi_scale: bool,
    pub threshold: f32,
    pub early_exit: bool,
    pub input_policy: InputPolicy,
    pub target_hwnd: isize,
    pub window_rect: WindowRect,
    pub preview: PreviewBusConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScanIteration {
    pub pipeline: PipelineResult,
    pub decision: HitDecision,
    pub click_report: Option<ClickReport>,
    pub preview: Option<PreviewMessage>,
    pub metrics: RuntimeMetricsSnapshot,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewIteration {
    pub preview: Option<PreviewMessage>,
    pub metrics: RuntimeMetricsSnapshot,
}

pub struct ScannerEngine<E: ClickExecutor> {
    hit_policy: HitPolicy,
    metrics: RuntimeMetrics,
    preview_bus: PreviewBus,
    executor: E,
}

impl<E: ClickExecutor> ScannerEngine<E> {
    pub fn new(hit_policy_config: HitPolicyConfig, preview: PreviewBusConfig, executor: E) -> Self {
        Self {
            hit_policy: HitPolicy::new(hit_policy_config),
            metrics: RuntimeMetrics::default(),
            preview_bus: PreviewBus::new(preview),
            executor,
        }
    }

    pub fn process_frame(
        &mut self,
        frame: &FramePacket,
        templates: &[Arc<LoadedTemplate>],
        config: &ScannerEngineConfig,
        frame_stats: autoclick_capture::frame::FrameStats,
    ) -> Result<ScanIteration, RuntimeError> {
        let process_started_at = Instant::now();
        let detect_started_at = Instant::now();
        let gray =
            grayscale_from_frame(frame).map_err(|err| RuntimeError::Detect(err.to_string()))?;
        let (pipeline, decision) = run_pipeline_with_policy(
            PipelinePolicyRequest {
                frame: &gray,
                roi: &config.roi,
                templates,
                scales: &config.scales,
                multi_scale: config.multi_scale,
                threshold: config.threshold,
                early_exit: config.early_exit,
                frame_timestamp_ms: frame.timestamp_ms,
            },
            &mut self.hit_policy,
        )
        .map_err(|err| RuntimeError::Detect(err.to_string()))?;
        let detect_latency_ms = detect_started_at.elapsed().as_secs_f32() * 1000.0;

        self.metrics.record_frame(frame, frame_stats);
        self.metrics
            .record_detection(detect_latency_ms, pipeline.best_match.as_ref());

        let click_report = if let HitDecision::ShouldClick(matched) = &decision {
            let report = self.executor.execute(
                &ClickRequest {
                    hwnd: config.target_hwnd,
                    window_rect: config.window_rect.clone(),
                    matched: matched.clone(),
                },
                &config.input_policy,
            )?;
            self.metrics.record_click();
            Some(report)
        } else {
            None
        };

        let preview_started_at = Instant::now();
        let preview = self.preview_bus.publish(frame)?;
        self.metrics
            .record_preview_latency(preview_started_at.elapsed().as_secs_f32() * 1000.0);
        if let Some(preview) = &preview {
            self.metrics.record_preview(
                preview.preview.width,
                preview.preview.height,
                preview.token.clone(),
            );
        }
        self.metrics
            .record_end_to_end_latency(process_started_at.elapsed().as_secs_f32() * 1000.0);

        Ok(ScanIteration {
            pipeline,
            decision,
            click_report,
            preview,
            metrics: self.metrics.snapshot(),
        })
    }

    pub fn process_preview_frame(
        &mut self,
        frame: &FramePacket,
        frame_stats: autoclick_capture::frame::FrameStats,
    ) -> Result<PreviewIteration, RuntimeError> {
        let process_started_at = Instant::now();
        self.metrics.record_frame(frame, frame_stats);
        self.metrics.record_detection(0.0, None);

        let preview_started_at = Instant::now();
        let preview = self.preview_bus.publish(frame)?;
        self.metrics
            .record_preview_latency(preview_started_at.elapsed().as_secs_f32() * 1000.0);
        if let Some(preview) = &preview {
            self.metrics.record_preview(
                preview.preview.width,
                preview.preview.height,
                preview.token.clone(),
            );
        }
        self.metrics
            .record_end_to_end_latency(process_started_at.elapsed().as_secs_f32() * 1000.0);

        Ok(PreviewIteration {
            preview,
            metrics: self.metrics.snapshot(),
        })
    }

    pub fn metrics_snapshot(&self) -> RuntimeMetricsSnapshot {
        self.metrics.snapshot()
    }

    pub fn record_recovery(
        &mut self,
        attempts: u32,
        reason: Option<String>,
        next_retry_in_ms: Option<u64>,
    ) {
        self.metrics
            .record_recovery(attempts, reason, next_retry_in_ms);
    }

    pub fn latest_preview(&self) -> Option<PreviewMessage> {
        self.preview_bus.latest()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use autoclick_capture::frame::{FramePacket, FrameStats, PixelFormat};
    use autoclick_detect::{hit_policy::HitPolicyConfig, template_store::LoadedTemplate};
    use autoclick_domain::{
        config::{ClickMethod, InputPolicy},
        template::TemplateRef,
        types::Roi,
    };
    use autoclick_platform_win::window::WindowRect;

    use super::{ClickExecutor, ScannerEngine, ScannerEngineConfig};
    use crate::{RuntimeError, preview_bus::PreviewBusConfig};

    #[derive(Default)]
    struct FakeExecutor {
        count: Mutex<u32>,
    }

    impl ClickExecutor for FakeExecutor {
        fn execute(
            &self,
            _request: &autoclick_input::policy::ClickRequest,
            policy: &InputPolicy,
        ) -> Result<autoclick_input::post_message::ClickReport, RuntimeError> {
            *self.count.lock().expect("lock") += 1;
            Ok(autoclick_input::post_message::ClickReport {
                method: policy.method.clone(),
                dispatch_hwnd: 1,
                screen_x: 10,
                screen_y: 20,
                client_x: 5,
                client_y: 6,
                restored_from_minimized: false,
            })
        }
    }

    fn frame() -> FramePacket {
        let mut image = image::GrayImage::from_pixel(16, 16, image::Luma([0]));
        for x in 4..8 {
            for y in 6..10 {
                image.put_pixel(x, y, image::Luma([255]));
            }
        }
        FramePacket {
            frame_id: 1,
            width: 16,
            height: 16,
            pixel_format: PixelFormat::Gray8,
            timestamp_ms: 100,
            bytes: image.into_raw(),
        }
    }

    fn templates() -> Vec<Arc<LoadedTemplate>> {
        vec![Arc::new(LoadedTemplate {
            meta: TemplateRef::new("sample"),
            image: image::GrayImage::from_pixel(4, 4, image::Luma([255])),
        })]
    }

    #[test]
    fn scanner_engine_processes_frame_and_clicks() {
        let executor = FakeExecutor::default();
        let mut engine = ScannerEngine::new(
            HitPolicyConfig {
                threshold: 0.95,
                min_detections: 1,
                cooldown_ms: 0,
            },
            PreviewBusConfig {
                enabled: true,
                throttle_ms: 0,
                ..PreviewBusConfig::default()
            },
            executor,
        );
        let iteration = engine
            .process_frame(
                &frame(),
                &templates(),
                &ScannerEngineConfig {
                    roi: Roi::default(),
                    scales: vec![1.0],
                    multi_scale: false,
                    threshold: 0.95,
                    early_exit: true,
                    input_policy: InputPolicy {
                        method: ClickMethod::Message,
                        verify_window_before_click: false,
                        click_offset_x: 0.0,
                        click_offset_y: 0.0,
                    },
                    target_hwnd: 500,
                    window_rect: WindowRect {
                        left: 100,
                        top: 100,
                        right: 400,
                        bottom: 400,
                    },
                    preview: PreviewBusConfig::default(),
                },
                FrameStats {
                    published_frames: 30,
                    dropped_frames: 0,
                    last_frame_id: 1,
                },
            )
            .expect("iteration");
        assert!(iteration.click_report.is_some());
        assert!(iteration.preview.is_some());
        assert_eq!(iteration.metrics.runtime.performance.click_count, 1);
    }

    #[test]
    fn scanner_engine_without_templates_still_publishes_preview() {
        let mut engine = ScannerEngine::new(
            HitPolicyConfig {
                threshold: 0.95,
                min_detections: 1,
                cooldown_ms: 0,
            },
            PreviewBusConfig {
                enabled: true,
                throttle_ms: 0,
                ..PreviewBusConfig::default()
            },
            FakeExecutor::default(),
        );
        let iteration = engine
            .process_frame(
                &frame(),
                &[],
                &ScannerEngineConfig {
                    roi: Roi::default(),
                    scales: vec![1.0],
                    multi_scale: false,
                    threshold: 0.95,
                    early_exit: true,
                    input_policy: InputPolicy {
                        method: ClickMethod::Message,
                        verify_window_before_click: false,
                        click_offset_x: 0.0,
                        click_offset_y: 0.0,
                    },
                    target_hwnd: 500,
                    window_rect: WindowRect {
                        left: 100,
                        top: 100,
                        right: 400,
                        bottom: 400,
                    },
                    preview: PreviewBusConfig::default(),
                },
                FrameStats {
                    published_frames: 1,
                    dropped_frames: 0,
                    last_frame_id: 1,
                },
            )
            .expect("iteration");

        assert!(iteration.pipeline.best_match.is_none());
        assert!(matches!(
            iteration.decision,
            autoclick_detect::hit_policy::HitDecision::NoMatch
        ));
        assert!(iteration.click_report.is_none());
        assert!(iteration.preview.is_some());
    }

    #[test]
    fn scanner_engine_preview_only_path_publishes_first_frame() {
        let mut engine = ScannerEngine::new(
            HitPolicyConfig {
                threshold: 0.95,
                min_detections: 1,
                cooldown_ms: 0,
            },
            PreviewBusConfig {
                enabled: true,
                throttle_ms: 0,
                ..PreviewBusConfig::default()
            },
            FakeExecutor::default(),
        );

        let iteration = engine
            .process_preview_frame(
                &frame(),
                FrameStats {
                    published_frames: 1,
                    dropped_frames: 0,
                    last_frame_id: 1,
                },
            )
            .expect("preview iteration");

        assert!(iteration.preview.is_some());
        assert_eq!(iteration.metrics.runtime.capture.frame_width, 16);
        assert_eq!(iteration.metrics.runtime.performance.detect_latency_ms, 0.0);
    }
}
