//! Logging utilities

use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_logging(log_dir: Option<PathBuf>) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tiny_claw=debug"));

    let subscriber = tracing_subscriber::registry().with(env_filter);

    if let Some(log_dir) = log_dir {
        // Ensure the log directory exists
        std::fs::create_dir_all(&log_dir)?;

        let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "tiny_claw.log");

        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        // Keep the guard alive for the lifetime of the program
        // In practice, we'd want to store this somewhere
        std::mem::forget(_guard);

        subscriber
            .with(
                fmt::layer()
                    .with_writer(non_blocking)
                    .with_ansi(false)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true),
            )
            .with(
                fmt::layer()
                    .with_writer(std::io::stderr)
                    .with_ansi(true)
                    .with_target(false),
            )
            .init();
    } else {
        subscriber
            .with(
                fmt::layer()
                    .with_writer(std::io::stderr)
                    .with_ansi(true)
                    .with_target(true)
                    .with_thread_ids(true),
            )
            .init();
    }

    Ok(())
}

/// Get default log directory
pub fn default_log_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("tiny_claw").join("logs"))
}
