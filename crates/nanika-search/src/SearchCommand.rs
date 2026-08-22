use crate::{Candidate, UsageKey};

#[derive(Debug)]
pub(crate) enum SearchCommand {
    WakeQuery,
    ExtensionSnapshot {
        generation: u64,
        extension_id: String,
        candidates: Vec<Candidate>,
    },
    ApplyPersistedExecution {
        key: UsageKey,
        executed_at: u64,
    },
    ResetPersistedUsage,
    Shutdown,
}
