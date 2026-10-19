use serde::Serialize;

/// Host-stamped authority for an extension's resource revocation.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TextInvalidationNotice {
    pub(crate) extension_id: String,
    pub(crate) instance_id: u64,
    pub(crate) text: nanika_protocol::TextInvalidation
}
