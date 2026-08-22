/// Usage data applied only inside an equivalent lexical tier.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UsageStat {
    pub execution_count: u32,
    pub last_executed_at: u64,
}
