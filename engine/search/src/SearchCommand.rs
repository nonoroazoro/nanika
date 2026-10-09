use crate::{Candidate, UsageKey};

#[derive(Debug)]
pub(crate) enum SearchCommand {
    WakeQuery,
    RegisterExtension {
        extension_id: String,
        instance_id: u64,
        completion: std::sync::mpsc::SyncSender<Result<(), crate::SearchQueueError>>
    },
    CatalogCommit {
        extension_id: String,
        instance_id: u64,
        replace: bool,
        candidates: Vec<Candidate>,
        removed: Vec<String>,
        completion: std::sync::mpsc::SyncSender<Result<(), crate::SearchQueueError>>
    },
    RemoveExtension {
        extension_id: String,
        instance_id: u64,
        completion: std::sync::mpsc::SyncSender<()>
    },
    RegisterStaticCatalog {
        extension_id: String,
        instance_id: u64,
        candidates: Vec<Candidate>
    },
    ExtensionQueryPending {
        generation: u64,
        extension_id: String,
        instance_id: u64,
        pending: bool
    },
    ExtensionSnapshot {
        complete: bool,
        generation: u64,
        extension_id: String,
        instance_id: u64,
        candidates: Vec<Candidate>
    },
    ExtensionDelta {
        complete: bool,
        generation: u64,
        extension_id: String,
        instance_id: u64,
        candidates: Vec<Candidate>,
        removed: Vec<String>
    },
    ApplyPersistedExecution {
        key: UsageKey,
        executed_at: u64
    },
    ResetPersistedUsage,
    Shutdown
}
