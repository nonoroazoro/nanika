use crate::{Candidate, UsageKey};

#[derive(Debug)]
pub(crate) enum SearchCommand {
    WakeQuery,
    CatalogCommit {
        extension_id: String,
        replace: bool,
        candidates: Vec<Candidate>,
        removed: Vec<String>,
        completion: std::sync::mpsc::SyncSender<Result<(), crate::SearchQueueError>>,
    },
    RemoveExtension {
        extension_id: String,
        completion: std::sync::mpsc::SyncSender<()>,
    },
    RegisterStaticCatalog {
        extension_id: String,
        candidates: Vec<Candidate>,
    },
    ExtensionSnapshot {
        generation: u64,
        extension_id: String,
        candidates: Vec<Candidate>,
    },
    ExtensionDelta {
        generation: u64,
        extension_id: String,
        candidates: Vec<Candidate>,
        removed: Vec<String>,
    },
    ApplyPersistedExecution {
        key: UsageKey,
        executed_at: u64,
    },
    ResetPersistedUsage,
    Shutdown,
}
