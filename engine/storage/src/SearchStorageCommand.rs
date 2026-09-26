use nanika_search::UsageKey;
use std::sync::mpsc::SyncSender;

pub(crate) type StorageResponse = SyncSender<Result<(), String>>;

pub(crate) enum SearchStorageCommand {
    RegisterBuiltInExtension {
        extension_id: String,
        response: StorageResponse,
    },
    RecordHistory {
        history_key: String,
        display_query: String,
        used_at: u64,
        response: StorageResponse,
    },
    RecordUsage {
        extension_id: String,
        entry_id: String,
        action_id: String,
        query_context: String,
        executed_at: u64,
        response: StorageResponse,
    },
    RecordExecution {
        history_key: String,
        display_query: String,
        usage: UsageKey,
        history_used_at: u64,
        executed_at: u64,
        response: StorageResponse,
    },
    ResetUsage {
        response: StorageResponse,
    },
    Shutdown,
}
