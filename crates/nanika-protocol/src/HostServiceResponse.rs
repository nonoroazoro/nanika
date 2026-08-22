use serde::{Deserialize, Serialize};

/// Successful completion of a host service request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "service", rename_all = "camelCase")]
pub enum HostServiceResponse {
    Launched,
    ClipboardWritten,
}
