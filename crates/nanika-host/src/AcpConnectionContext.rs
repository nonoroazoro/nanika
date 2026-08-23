use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::SyncSender;
use std::time::Duration;

use crate::{AcpExtensionCommand, ExtensionCommand};

pub(crate) struct AcpConnectionContext {
    pub(crate) command: ExtensionCommand,
    pub(crate) arguments: Vec<String>,
    pub(crate) working_directory: PathBuf,
    pub(crate) handshake_timeout: Duration,
    pub(crate) shutdown_timeout: Duration,
    pub(crate) commands: async_channel::Receiver<AcpExtensionCommand>,
    pub(crate) shutdown: async_channel::Receiver<()>,
    pub(crate) ready: SyncSender<Result<(), String>>,
    pub(crate) ready_reported: Arc<AtomicBool>,
}
