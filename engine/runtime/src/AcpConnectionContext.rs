use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::SyncSender;

use crate::{AcpExtensionCommand, ExtensionCommand};
use nanika_protocol::ExtensionConfiguration;

pub(crate) struct AcpConnectionContext {
    pub(crate) extension_id: String,
    pub(crate) command: ExtensionCommand,
    pub(crate) arguments: Vec<String>,
    pub(crate) working_directory: PathBuf,
    pub(crate) configuration: ExtensionConfiguration,
    pub(crate) commands: async_channel::Receiver<AcpExtensionCommand>,
    pub(crate) shutdown: async_channel::Receiver<()>,
    pub(crate) ready: SyncSender<Result<(), String>>,
    pub(crate) ready_reported: Arc<AtomicBool>,
}
