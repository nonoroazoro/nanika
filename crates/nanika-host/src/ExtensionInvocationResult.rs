use crate::ExtensionInvocationOutcome;

#[derive(Debug)]
pub(crate) struct ExtensionInvocationResult {
    pub(crate) invocation_id: u64,
    pub(crate) extension_id: String,
    pub(crate) generation: u64,
    pub(crate) entry_id: String,
    pub(crate) action_id: String,
    pub(crate) query_context: String,
    pub(crate) result: Result<ExtensionInvocationOutcome, String>,
}
