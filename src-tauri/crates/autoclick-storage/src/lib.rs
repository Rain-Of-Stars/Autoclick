pub mod import_legacy;
pub mod migrations;
pub mod path_resolver;
pub mod repo_config;
pub mod repo_run;
pub mod repo_template;
pub mod template_fs;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("数据库初始化失败: {0}")]
    DatabaseInit(String),
    #[error("数据库访问失败: {0}")]
    Database(String),
    #[error("模板文件处理失败: {0}")]
    TemplateFs(String),
    #[error("旧数据导入失败: {0}")]
    LegacyImport(String),
}
