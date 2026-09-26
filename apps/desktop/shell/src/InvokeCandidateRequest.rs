use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InvokeCandidateRequest {
    pub(crate) session_id: u64,
    pub(crate) request_id: u64,
    pub(crate) result_revision: u64,
    pub(crate) extension_id: String,
    pub(crate) entry_id: String,
    pub(crate) action_id: String,
}
