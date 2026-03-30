mod backend;

pub use backend::{
    CaptureFactory, CaptureSharedSnapshot, CaptureSharedState, RunningCapture, WgcCaptureOptions,
    WindowsCaptureFactory,
};
pub mod convert;
pub mod drop_guard;
pub mod frame;
pub mod latest_frame;
pub mod preview_encode;
pub mod recovery;
pub mod session;
pub mod single_frame;
pub mod wgc_monitor;
pub mod wgc_window;

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("捕获目标无效: {0}")]
    InvalidTarget(&'static str),
    #[error("缓冲区中没有可用帧")]
    FrameUnavailable,
    #[error("等待帧超时")]
    Timeout,
    #[error("捕获会话已在运行")]
    AlreadyRunning,
    #[error("捕获会话未启动")]
    NotRunning,
    #[error("捕获项已关闭")]
    ItemClosed,
    #[error("后端执行失败: {0}")]
    Backend(String),
    #[error("帧转换失败: {0}")]
    Convert(String),
    #[error("预览编码失败: {0}")]
    Encode(String),
}
