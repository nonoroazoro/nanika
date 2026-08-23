#[derive(Debug, Clone)]
pub(crate) struct ExtensionInvocation {
    pub(crate) invocation_id: u64,
    pub(crate) generation: u64,
    pub(crate) entry_id: String,
    pub(crate) action_id: String,
    pub(crate) query_context: String,
}
