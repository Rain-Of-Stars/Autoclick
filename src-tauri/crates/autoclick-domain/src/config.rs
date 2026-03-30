use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

use crate::template::TemplateRef;
use crate::types::{MonitorRef, Roi, RuntimeStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub schema_version: u32,
    pub capture: CaptureProfile,
    pub detection: DetectionProfile,
    pub input: InputPolicy,
    pub recovery: RecoveryPolicy,
    pub target: TargetProfile,
    pub ui: UiPrefs,
    pub templates: Vec<TemplateRef>,
    pub runtime_status: RuntimeStatus,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            capture: CaptureProfile::default(),
            detection: DetectionProfile::default(),
            input: InputPolicy::default(),
            recovery: RecoveryPolicy::default(),
            target: TargetProfile::default(),
            ui: UiPrefs::default(),
            templates: Vec::new(),
            runtime_status: RuntimeStatus::Idle,
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if !(0.0..=1.0).contains(&self.detection.threshold) {
            return Err(ConfigValidationError::InvalidThreshold(
                self.detection.threshold,
            ));
        }
        if self.capture.target_fps == 0 {
            return Err(ConfigValidationError::InvalidTargetFps);
        }
        if self.detection.scales.is_empty() {
            return Err(ConfigValidationError::EmptyScaleList);
        }
        if self
            .detection
            .scales
            .iter()
            .any(|scale| *scale <= 0.0 || !scale.is_finite())
        {
            return Err(ConfigValidationError::InvalidScaleValue);
        }
        validate_debug_image_dir(&self.ui.debug_image_dir)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureProfile {
    pub source: CaptureSource,
    pub monitor: MonitorRef,
    pub roi: Roi,
    pub target_fps: u32,
    pub timeout_ms: u64,
    pub include_cursor: bool,
    pub restore_minimized_noactivate: bool,
    pub restore_minimized_after_capture: bool,
    pub window_border_required: bool,
    pub screen_border_required: bool,
}

impl Default for CaptureProfile {
    fn default() -> Self {
        Self {
            source: CaptureSource::Window,
            monitor: MonitorRef::default(),
            roi: Roi::default(),
            target_fps: 30,
            timeout_ms: 5_000,
            include_cursor: false,
            restore_minimized_noactivate: true,
            restore_minimized_after_capture: false,
            window_border_required: true,
            screen_border_required: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CaptureSource {
    #[default]
    Window,
    Monitor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DetectionProfile {
    pub threshold: f32,
    pub grayscale: bool,
    pub multi_scale: bool,
    pub scales: Vec<f32>,
    pub min_detections: u32,
    pub cooldown_ms: u64,
    pub early_exit: bool,
}

impl Default for DetectionProfile {
    fn default() -> Self {
        Self {
            threshold: 0.88,
            grayscale: true,
            multi_scale: false,
            scales: vec![1.0, 1.25, 0.8],
            min_detections: 1,
            cooldown_ms: 5_000,
            early_exit: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InputPolicy {
    pub method: ClickMethod,
    pub verify_window_before_click: bool,
    pub click_offset_x: f32,
    pub click_offset_y: f32,
}

impl Default for InputPolicy {
    fn default() -> Self {
        Self {
            method: ClickMethod::Message,
            verify_window_before_click: true,
            click_offset_x: 0.0,
            click_offset_y: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ClickMethod {
    #[default]
    Message,
    Simulate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryPolicy {
    pub enable_auto_recovery: bool,
    pub max_recovery_attempts: u32,
    pub recovery_cooldown_secs: f32,
    pub auto_update_target_by_process: bool,
    pub auto_update_interval_ms: u64,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self {
            enable_auto_recovery: true,
            max_recovery_attempts: 5,
            recovery_cooldown_secs: 10.0,
            auto_update_target_by_process: false,
            auto_update_interval_ms: 5_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TargetProfile {
    pub hwnd: Option<i64>,
    pub process_name: Option<String>,
    pub process_path: Option<String>,
    pub title_contains: Option<String>,
    pub class_name: Option<String>,
    pub allow_partial_match: bool,
    pub strategies: FinderStrategies,
}

impl Default for TargetProfile {
    fn default() -> Self {
        Self {
            hwnd: None,
            process_name: None,
            process_path: None,
            title_contains: None,
            class_name: None,
            allow_partial_match: true,
            strategies: FinderStrategies::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FinderStrategies {
    pub process_name: bool,
    pub process_path: bool,
    pub window_title: bool,
    pub class_name: bool,
    pub fuzzy_match: bool,
}

impl Default for FinderStrategies {
    fn default() -> Self {
        Self {
            process_name: true,
            process_path: true,
            window_title: true,
            class_name: true,
            fuzzy_match: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UiPrefs {
    pub enable_logging: bool,
    pub enable_notifications: bool,
    pub auto_start_scan: bool,
    pub debug_mode: bool,
    pub save_debug_images: bool,
    pub debug_image_dir: String,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            enable_logging: false,
            enable_notifications: true,
            auto_start_scan: true,
            debug_mode: false,
            save_debug_images: false,
            debug_image_dir: "debug_images".to_string(),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ConfigValidationError {
    #[error("检测阈值超出范围: {0}")]
    InvalidThreshold(f32),
    #[error("目标帧率必须大于 0")]
    InvalidTargetFps,
    #[error("尺度列表不能为空")]
    EmptyScaleList,
    #[error("尺度列表包含非法值")]
    InvalidScaleValue,
    #[error("调试图目录必须是相对路径且不能包含上级目录")]
    InvalidDebugImageDir,
}

fn validate_debug_image_dir(value: &str) -> Result<(), ConfigValidationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ConfigValidationError::InvalidDebugImageDir);
    }

    let path = Path::new(trimmed);
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    }) {
        return Err(ConfigValidationError::InvalidDebugImageDir);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, ConfigValidationError};

    #[test]
    fn default_config_is_valid() {
        assert_eq!(AppConfig::default().validate(), Ok(()));
    }

    #[test]
    fn invalid_threshold_is_rejected() {
        let mut config = AppConfig::default();
        config.detection.threshold = 1.2;
        assert_eq!(
            config.validate(),
            Err(ConfigValidationError::InvalidThreshold(1.2))
        );
    }

    #[test]
    fn debug_image_dir_rejects_parent_traversal() {
        let mut config = AppConfig::default();
        config.ui.debug_image_dir = "debug/../secret".to_string();

        assert_eq!(
            config.validate(),
            Err(ConfigValidationError::InvalidDebugImageDir)
        );
    }
}
