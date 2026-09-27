use nanika_search::UsageKey;

/// Ordered projection work retained by the storage owner after a durable commit.
pub(crate) enum PersistedUsageChange {
    Execution { key: UsageKey, executed_at: u64 },
    Reset,
}
