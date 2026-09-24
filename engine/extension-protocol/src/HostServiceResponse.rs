use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "service", rename_all = "camelCase")]
pub enum HostServiceResponse {
    PathRevealed,
    Launched,
    ClipboardWritten { revision: u64 },
}
