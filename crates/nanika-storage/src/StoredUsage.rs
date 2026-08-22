/// One persisted contextual usage record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredUsage {
    pub extension_id: String,
    pub entry_id: String,
    pub action_id: String,
    pub query_context: String,
    pub execution_count: u32,
    pub last_executed_at: u64,
}
