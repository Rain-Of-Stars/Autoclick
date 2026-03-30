pub mod dpi;
pub mod hit_test;
pub mod locator;
pub mod monitor;
pub mod process;
pub mod window;
pub mod window_state;

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("Win32 调用失败: {0}")]
    Win32(String),
    #[error("参数无效: {0}")]
    InvalidArgument(&'static str),
}
