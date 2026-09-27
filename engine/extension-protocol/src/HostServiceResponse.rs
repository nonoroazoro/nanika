use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "service", rename_all = "camelCase")]
pub enum HostServiceResponse {
    /// The host owns the submitted command; no OS completion result follows.
    SystemActionSubmitted,
    PathRevealed,
    Launched,
    ClipboardWritten {
        revision: u64,
    },
}
