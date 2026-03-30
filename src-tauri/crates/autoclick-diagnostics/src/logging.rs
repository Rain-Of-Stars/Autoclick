use std::path::Path;
use std::sync::OnceLock;

use tracing::error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::error::DiagnosticsError;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static LOG_INIT: OnceLock<()> = OnceLock::new();

pub fn init_logging(log_dir: &Path) -> Result<(), DiagnosticsError> {
    if LOG_INIT.get().is_some() {
        return Ok(());
    }

    std::fs::create_dir_all(log_dir)
        .map_err(|err| DiagnosticsError::LoggingInit(err.to_string()))?;

    let file_appender = tracing_appender::rolling::daily(log_dir, "autoclick.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tauri=warn,wry=warn"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_writer(writer),
        )
        .with(
            fmt::layer()
                .with_ansi(true)
                .with_target(false)
                .with_writer(std::io::stdout),
        )
        .try_init()
        .map_err(|err| DiagnosticsError::LoggingInit(err.to_string()))?;

    install_panic_hook();
    LOG_GUARD
        .set(guard)
        .map_err(|_| DiagnosticsError::LoggingInit("无法保留日志写入守卫".to_string()))?;
    LOG_INIT
        .set(())
        .map_err(|_| DiagnosticsError::LoggingInit("日志系统重复初始化".to_string()))?;
    tracing::info!("日志系统已初始化: {}", log_dir.display());
    Ok(())
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|location| format!("{}:{}", location.file(), location.line()))
            .unwrap_or_else(|| "unknown".to_string());
        let message = if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
            (*message).to_string()
        } else if let Some(message) = panic_info.payload().downcast_ref::<String>() {
            message.clone()
        } else {
            "panic payload unavailable".to_string()
        };
        error!("panic captured at {}: {}", location, message);
    }));
}
