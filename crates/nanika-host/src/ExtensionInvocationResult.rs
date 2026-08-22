#[derive(Debug)]
pub(crate) struct ExtensionInvocationResult {
    pub(crate) extension_id: String,
    pub(crate) entry_id: String,
    pub(crate) action_id: String,
    pub(crate) query_context: String,
    pub(crate) result: Result<(), String>,
}
