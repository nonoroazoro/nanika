use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ContextMenuRequest {
    pub session_id: u64,
    pub target: crate::MenuTarget,
}
