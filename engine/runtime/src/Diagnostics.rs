use std::ffi::OsStr;
use std::path::Path;

use tracing_appender::non_blocking::{NonBlockingBuilder, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

use crate::BoundedLogWriter;

const MAX_LOG_BYTES: u64 = 32 * 1024 * 1024;

/// Keeps the bounded diagnostic writer alive and flushes it during shutdown.
pub struct Diagnostics {
    _worker: WorkerGuard,
}

impl Diagnostics {
    pub fn initialize(log_directory: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(log_directory).map_err(|error| error.to_string())?;
        let existing_bytes = enforce_log_budget(log_directory, MAX_LOG_BYTES)?;
        let appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("nanika")
            .filename_suffix("log")
            .max_log_files(8)
            .build(log_directory)
            .map_err(|error| error.to_string())?;
        let appender =
            BoundedLogWriter::new(appender, MAX_LOG_BYTES.saturating_sub(existing_bytes));
        let (writer, worker) = NonBlockingBuilder::default()
            .buffered_lines_limit(256)
            .lossy(true)
            .thread_name("nanika-diagnostics")
            .finish(appender);
        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_max_level(maximum_level(
                std::env::var_os("NANIKA_DIAGNOSTICS").as_deref(),
            ))
            .with_target(true)
            .with_writer(writer)
            .try_init()
            .map_err(|error| error.to_string())?;
        Ok(Self { _worker: worker })
    }
}

pub(crate) fn maximum_level(value: Option<&OsStr>) -> tracing::Level {
    if value == Some(OsStr::new("verbose")) {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    }
}

pub(crate) fn enforce_log_budget(log_directory: &Path, limit: u64) -> Result<u64, String> {
    let mut logs = Vec::new();
    let mut bytes = 0_u64;
    for entry in std::fs::read_dir(log_directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_file() && is_nanika_log_name(&entry.file_name()) {
            let length = entry.metadata().map_err(|error| error.to_string())?.len();
            bytes = bytes.saturating_add(length);
            logs.push((entry.file_name(), entry.path(), length));
        }
    }
    logs.sort_by(|left, right| left.0.cmp(&right.0));
    for (_, path, length) in logs {
        if bytes <= limit {
            break;
        }
        std::fs::remove_file(path).map_err(|error| error.to_string())?;
        bytes = bytes.saturating_sub(length);
    }
    Ok(bytes)
}

fn is_nanika_log_name(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    let Some(date) = name
        .strip_prefix("nanika.")
        .and_then(|name| name.strip_suffix(".log"))
    else {
        return false;
    };
    let bytes = date.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}
