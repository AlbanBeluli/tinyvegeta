//! Logging setup for TinyVegeta using tracing.

use anyhow::Result;
use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize logging with file appender and console output.
pub fn init() -> Result<(WorkerGuard, PathBuf)> {
    // Get the log directory
    let log_dir = get_log_dir()?;
    std::fs::create_dir_all(&log_dir)?;

    // Create file appender with rotation
    let file_appender = tracing_appender::rolling::daily(&log_dir, "tinyvegeta.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Build the subscriber
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tinyvegeta=debug"));

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    let console_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_target(true);

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(console_layer)
        .init();

    tracing::info!("TinyVegeta logging initialized");
    tracing::info!("Log directory: {}", log_dir.display());

    Ok((guard, log_dir))
}

/// Get the log directory path.
fn get_log_dir() -> Result<PathBuf> {
    let home = directories::ProjectDirs::from("com", "tinyvegeta", "tinyvegeta")
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    Ok(home.data_dir().join("logs"))
}

/// Initialize logging for tests (console only, no file).
#[cfg(test)]
pub fn init_test() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();
}
