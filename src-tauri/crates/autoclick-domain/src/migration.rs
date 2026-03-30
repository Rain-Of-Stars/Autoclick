use serde::Deserialize;
use serde_json::Value;

use crate::config::{AppConfig, CaptureSource, ClickMethod, FinderStrategies, TargetProfile};
use crate::template::TemplateRef;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyConfig {
    pub template_path: Option<String>,
    pub template_paths: Option<Vec<String>>,
    pub threshold: Option<f32>,
    pub grayscale: Option<bool>,
    pub multi_scale: Option<bool>,
    pub scales: Option<Vec<f32>>,
    pub min_detections: Option<u32>,
    pub cooldown_s: Option<f32>,
    pub click_method: Option<String>,
    pub verify_window_before_click: Option<bool>,
    pub capture_backend: Option<String>,
    pub use_monitor: Option<bool>,
    pub monitor_index: Option<u32>,
    pub target_hwnd: Option<i64>,
    pub target_process: Option<String>,
    pub process_partial_match: Option<bool>,
    pub fps_max: Option<u32>,
    pub target_fps: Option<u32>,
    pub capture_timeout_ms: Option<u64>,
    pub include_cursor: Option<bool>,
    pub restore_minimized_noactivate: Option<bool>,
    pub restore_minimized_after_capture: Option<bool>,
    pub window_title: Option<String>,
    pub class_name: Option<String>,
    pub border_required: Option<bool>,
    pub window_border_required: Option<bool>,
    pub screen_border_required: Option<bool>,
    pub enable_logging: Option<bool>,
    pub enable_notifications: Option<bool>,
    pub auto_start_scan: Option<bool>,
    pub debug_mode: Option<bool>,
    pub save_debug_images: Option<bool>,
    pub debug_image_dir: Option<String>,
    pub enable_auto_recovery: Option<bool>,
    pub max_recovery_attempts: Option<u32>,
    pub recovery_cooldown: Option<f32>,
    pub auto_update_hwnd_by_process: Option<bool>,
    pub auto_update_hwnd_interval_ms: Option<u64>,
    pub finder_strategies: Option<FinderStrategies>,
}

pub fn migrate_legacy_value(value: &Value) -> Result<AppConfig, serde_json::Error> {
    let legacy: LegacyConfig = serde_json::from_value(value.clone())?;
    Ok(migrate_legacy_config(legacy))
}

pub fn migrate_legacy_config(legacy: LegacyConfig) -> AppConfig {
    let mut config = AppConfig::default();

    config.capture.source = if legacy.use_monitor.unwrap_or(false)
        || matches!(
            legacy.capture_backend.as_deref(),
            Some("screen" | "monitor" | "auto")
        ) {
        CaptureSource::Monitor
    } else {
        CaptureSource::Window
    };
    config.capture.monitor.id = legacy.monitor_index.unwrap_or(1).to_string();
    config.capture.target_fps = legacy.target_fps.or(legacy.fps_max).unwrap_or(30);
    config.capture.timeout_ms = legacy.capture_timeout_ms.unwrap_or(5_000);
    config.capture.include_cursor = legacy.include_cursor.unwrap_or(false);
    config.capture.restore_minimized_noactivate =
        legacy.restore_minimized_noactivate.unwrap_or(true);
    config.capture.restore_minimized_after_capture =
        legacy.restore_minimized_after_capture.unwrap_or(false);
    let legacy_border = legacy.border_required.unwrap_or(false);
    config.capture.window_border_required = legacy.window_border_required.unwrap_or(legacy_border);
    config.capture.screen_border_required = legacy.screen_border_required.unwrap_or(legacy_border);

    config.detection.threshold = legacy.threshold.unwrap_or(0.88);
    config.detection.grayscale = legacy.grayscale.unwrap_or(true);
    config.detection.multi_scale = legacy.multi_scale.unwrap_or(false);
    config.detection.scales = legacy.scales.unwrap_or_else(|| vec![1.0, 1.25, 0.8]);
    config.detection.min_detections = legacy.min_detections.unwrap_or(1);
    config.detection.cooldown_ms = (legacy.cooldown_s.unwrap_or(5.0) * 1_000.0).round() as u64;

    config.input.method = normalize_click_method(legacy.click_method.as_deref());
    config.input.verify_window_before_click = legacy.verify_window_before_click.unwrap_or(true);

    config.recovery.enable_auto_recovery = legacy.enable_auto_recovery.unwrap_or(true);
    config.recovery.max_recovery_attempts = legacy.max_recovery_attempts.unwrap_or(5);
    config.recovery.recovery_cooldown_secs = legacy.recovery_cooldown.unwrap_or(10.0);
    config.recovery.auto_update_target_by_process =
        legacy.auto_update_hwnd_by_process.unwrap_or(false);
    config.recovery.auto_update_interval_ms = legacy.auto_update_hwnd_interval_ms.unwrap_or(5_000);

    config.target = TargetProfile {
        hwnd: legacy.target_hwnd.filter(|value| *value > 0),
        process_name: legacy.target_process.filter(|value| !value.is_empty()),
        process_path: None,
        title_contains: legacy.window_title.filter(|value| !value.is_empty()),
        class_name: legacy.class_name.filter(|value| !value.is_empty()),
        allow_partial_match: legacy.process_partial_match.unwrap_or(true),
        strategies: legacy.finder_strategies.unwrap_or_default(),
    };

    config.ui.enable_logging = legacy.enable_logging.unwrap_or(false);
    config.ui.enable_notifications = legacy.enable_notifications.unwrap_or(true);
    config.ui.auto_start_scan = legacy.auto_start_scan.unwrap_or(true);
    config.ui.debug_mode = legacy.debug_mode.unwrap_or(false);
    config.ui.save_debug_images = legacy.save_debug_images.unwrap_or(false);
    config.ui.debug_image_dir = legacy
        .debug_image_dir
        .unwrap_or_else(|| "debug_images".to_string());

    for template in normalize_template_paths(legacy.template_path, legacy.template_paths) {
        let mut template_ref = TemplateRef::new(template.clone());
        template_ref.source_path = Some(template);
        config.templates.push(template_ref);
    }

    config
}

fn normalize_template_paths(
    template_path: Option<String>,
    template_paths: Option<Vec<String>>,
) -> Vec<String> {
    let mut values = template_paths.unwrap_or_default();
    if let Some(single) = template_path {
        if !single.is_empty() && !values.iter().any(|value| value == &single) {
            values.push(single);
        }
    }
    values.retain(|value| !value.trim().is_empty());
    values
}

fn normalize_click_method(method: Option<&str>) -> ClickMethod {
    match method.unwrap_or("message").to_ascii_lowercase().as_str() {
        "simulate" | "sendinput" => ClickMethod::Simulate,
        _ => ClickMethod::Message,
    }
}

#[cfg(test)]
mod tests {
    use super::{LegacyConfig, migrate_legacy_config};
    use crate::config::{CaptureSource, ClickMethod};

    #[test]
    fn migrates_legacy_fields_into_strong_model() {
        let config = migrate_legacy_config(LegacyConfig {
            template_path: Some("a.png".to_string()),
            template_paths: Some(vec!["b.png".to_string()]),
            threshold: Some(0.9),
            grayscale: Some(true),
            multi_scale: Some(true),
            scales: Some(vec![1.0, 1.2]),
            min_detections: Some(2),
            cooldown_s: Some(2.5),
            click_method: Some("simulate".to_string()),
            verify_window_before_click: Some(false),
            capture_backend: Some("monitor".to_string()),
            use_monitor: Some(false),
            monitor_index: Some(2),
            target_hwnd: Some(123),
            target_process: Some("Windsurf.exe".to_string()),
            process_partial_match: Some(true),
            fps_max: Some(24),
            target_fps: None,
            capture_timeout_ms: Some(3_000),
            include_cursor: Some(true),
            restore_minimized_noactivate: Some(false),
            restore_minimized_after_capture: Some(true),
            window_title: Some("Windsurf".to_string()),
            class_name: Some("Chrome_WidgetWin_1".to_string()),
            border_required: Some(true),
            window_border_required: None,
            screen_border_required: None,
            enable_logging: Some(true),
            enable_notifications: Some(false),
            auto_start_scan: Some(false),
            debug_mode: Some(true),
            save_debug_images: Some(true),
            debug_image_dir: Some("debug".to_string()),
            enable_auto_recovery: Some(true),
            max_recovery_attempts: Some(8),
            recovery_cooldown: Some(6.5),
            auto_update_hwnd_by_process: Some(true),
            auto_update_hwnd_interval_ms: Some(6_000),
            finder_strategies: Some(crate::config::FinderStrategies::default()),
        });

        assert_eq!(config.capture.source, CaptureSource::Monitor);
        assert_eq!(config.capture.target_fps, 24);
        assert_eq!(config.input.method, ClickMethod::Simulate);
        assert_eq!(config.templates.len(), 2);
        assert_eq!(config.target.hwnd, Some(123));
        assert_eq!(config.ui.debug_image_dir, "debug");
    }
}
