pub mod coordinate;
pub mod policy;
pub mod post_message;
pub mod send_input;

#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("输入目标无效: {0}")]
    InvalidTarget(&'static str),
    #[error("坐标解析失败: {0}")]
    Coordinate(String),
    #[error("窗口不可用")]
    WindowUnavailable,
    #[error("消息发送失败: {0}")]
    Message(String),
    #[error("系统注入失败: {0}")]
    Simulate(String),
}
