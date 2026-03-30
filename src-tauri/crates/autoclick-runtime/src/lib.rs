pub mod metrics;
pub mod preview_bus;
pub mod scanner_engine;
pub mod shutdown;
pub mod state_machine;
pub mod supervisor;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("状态机错误: {0}")]
    State(String),
    #[error("捕获错误: {0}")]
    Capture(String),
    #[error("检测错误: {0}")]
    Detect(String),
    #[error("输入错误: {0}")]
    Input(String),
    #[error("预览错误: {0}")]
    Preview(String),
}
