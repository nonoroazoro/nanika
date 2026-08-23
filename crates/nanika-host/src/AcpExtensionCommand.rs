use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::SyncSender;

pub(crate) enum AcpExtensionCommand {
    Prompt {
        prompt: String,
        cancelled: Arc<AtomicBool>,
        publish: Arc<dyn Fn(String) + Send + Sync>,
        response: SyncSender<Result<(), String>>,
    },
    Shutdown {
        response: SyncSender<()>,
    },
}
