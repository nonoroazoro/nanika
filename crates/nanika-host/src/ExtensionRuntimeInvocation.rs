/// One action request submitted to a protocol-aware extension runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionRuntimeInvocation {
    pub(crate) request_id: String,
    pub(crate) generation: u64,
    pub(crate) entry_id: String,
    pub(crate) action_id: String,
    pub(crate) query_context: String,
}

impl ExtensionRuntimeInvocation {
    pub fn new(
        request_id: impl Into<String>,
        generation: u64,
        entry_id: impl Into<String>,
        action_id: impl Into<String>,
        query_context: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            generation,
            entry_id: entry_id.into(),
            action_id: action_id.into(),
            query_context: query_context.into(),
        }
    }
}
