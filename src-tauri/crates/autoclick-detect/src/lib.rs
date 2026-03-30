pub mod debug_dump;
pub mod hit_policy;
pub mod r#match;
pub mod pipeline;
pub mod preprocess;
pub mod roi;
pub mod template_store;

#[derive(Debug, thiserror::Error)]
pub enum DetectError {
    #[error("检测层暂未接入: {0}")]
    NotReady(&'static str),
    #[error("模板或图像处理失败: {0}")]
    Image(String),
}
