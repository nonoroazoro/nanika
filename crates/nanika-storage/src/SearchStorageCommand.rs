use crate::ExtensionKind;

pub(crate) enum SearchStorageCommand {
    RegisterExtension {
        extension_id: String,
        kind: ExtensionKind,
        updated_at: u64,
    },
    RecordHistory {
        history_key: String,
        display_query: String,
        used_at: u64,
    },
    RecordUsage {
        extension_id: String,
        entry_id: String,
        action_id: String,
        query_context: String,
        executed_at: u64,
    },
    ResetUsage,
    Shutdown,
}
