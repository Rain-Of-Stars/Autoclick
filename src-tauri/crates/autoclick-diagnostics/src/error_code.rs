use autoclick_domain::user_message::UserMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    ConfigInvalid,
    StorageUnavailable,
    CaptureUnavailable,
    DetectionUnavailable,
    InputRejected,
    RuntimeFault,
    LegacyImportFailed,
    DiagnosticsExportFailed,
    Unknown,
}

impl ErrorCode {
    pub fn code(self) -> &'static str {
        match self {
            Self::ConfigInvalid => "CONFIG_INVALID",
            Self::StorageUnavailable => "STORAGE_UNAVAILABLE",
            Self::CaptureUnavailable => "CAPTURE_UNAVAILABLE",
            Self::DetectionUnavailable => "DETECTION_UNAVAILABLE",
            Self::InputRejected => "INPUT_REJECTED",
            Self::RuntimeFault => "RUNTIME_FAULT",
            Self::LegacyImportFailed => "LEGACY_IMPORT_FAILED",
            Self::DiagnosticsExportFailed => "DIAGNOSTICS_EXPORT_FAILED",
            Self::Unknown => "UNKNOWN",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::ConfigInvalid => "配置无效",
            Self::StorageUnavailable => "存储不可用",
            Self::CaptureUnavailable => "捕获不可用",
            Self::DetectionUnavailable => "检测不可用",
            Self::InputRejected => "输入被拒绝",
            Self::RuntimeFault => "运行时故障",
            Self::LegacyImportFailed => "旧数据导入失败",
            Self::DiagnosticsExportFailed => "诊断导出失败",
            Self::Unknown => "未知错误",
        }
    }

    pub fn recover_hint(self) -> &'static str {
        match self {
            Self::ConfigInvalid => "检查配置表单或重新导入配置后再试。",
            Self::StorageUnavailable => "确认数据目录可写，并检查数据库文件占用情况。",
            Self::CaptureUnavailable => "检查目标窗口是否存在、是否最小化，以及捕获权限是否正常。",
            Self::DetectionUnavailable => "检查模板文件、ROI 和阈值设置是否合理。",
            Self::InputRejected => "确认目标窗口仍然有效，并检查点击策略配置。",
            Self::RuntimeFault => "查看日志诊断页的最近错误与恢复记录。",
            Self::LegacyImportFailed => "确认旧版配置与模板文件完整，并重新执行 dry-run。",
            Self::DiagnosticsExportFailed => "确认诊断目录与导出目录可写。",
            Self::Unknown => "查看日志获取更详细的底层错误信息。",
        }
    }

    pub fn to_user_message(self, detail: impl Into<String>) -> UserMessage {
        let mut message = UserMessage::new(self.code(), self.title(), detail);
        message.recover_hint = Some(self.recover_hint().to_string());
        message
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorCode;

    #[test]
    fn creates_stable_user_message() {
        let message = ErrorCode::StorageUnavailable.to_user_message("db locked");
        assert_eq!(message.code, "STORAGE_UNAVAILABLE");
        assert_eq!(message.title, "存储不可用");
        assert!(message.recover_hint.unwrap_or_default().contains("数据库"));
    }
}
