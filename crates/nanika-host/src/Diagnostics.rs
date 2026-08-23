use std::path::Path;

use tracing_appender::non_blocking::{NonBlockingBuilder, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

/// Keeps the bounded diagnostic writer alive and flushes it during shutdown.
pub struct Diagnostics {
    _worker: WorkerGuard,
}

impl Diagnostics {
    pub fn initialize(log_directory: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(log_directory).map_err(|error| error.to_string())?;
        let appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("nanika")
            .filename_suffix("log")
            .max_log_files(8)
            .build(log_directory)
            .map_err(|error| error.to_string())?;
        let (writer, worker) = NonBlockingBuilder::default()
            .buffered_lines_limit(256)
            .lossy(true)
            .thread_name("nanika-diagnostics")
            .finish(appender);
        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_max_level(tracing::Level::INFO)
            .with_target(true)
            .with_writer(writer)
            .try_init()
            .map_err(|error| error.to_string())?;
        Ok(Self { _worker: worker })
    }
}
