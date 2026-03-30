use autoclick_domain::user_message::UserMessage;

use crate::error_code::ErrorCode;

#[derive(Debug, thiserror::Error)]
pub enum DiagnosticsError {
    #[error("日志初始化失败: {0}")]
    LoggingInit(String),
    #[error("panic 处理初始化失败: {0}")]
    PanicHook(String),
    #[error("通用诊断错误: {0}")]
    Generic(String),
}

#[derive(Debug, Clone)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub source_chain: Vec<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, error: impl Into<anyhow::Error>) -> Self {
        let error = error.into();
        let source_chain = error.chain().map(ToString::to_string).collect::<Vec<_>>();
        Self {
            code,
            message: error.to_string(),
            source_chain,
        }
    }

    pub fn user_message(&self) -> UserMessage {
        self.code.to_user_message(self.message.clone())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        Self::new(ErrorCode::Unknown, value)
    }
}
