use serde::{Deserialize, Serialize};

use crate::Candidate;

/// One request or response on the extension protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Message {
    Initialize {
        request_id: String,
        protocol: String,
    },
    Initialized {
        request_id: String,
        protocol: String,
    },
    Query {
        request_id: String,
        generation: u64,
        query: String,
    },
    Snapshot {
        request_id: String,
        generation: u64,
        #[serde(default = "snapshot_complete_by_default")]
        complete: bool,
        entries: Vec<Candidate>,
    },
    Invoke {
        request_id: String,
        generation: u64,
        entry_id: String,
        action_id: String,
    },
    Result {
        request_id: String,
        generation: u64,
    },
    Cancel {
        request_id: String,
        generation: u64,
    },
    Shutdown {
        request_id: String,
    },
    ShutdownAck {
        request_id: String,
    },
    Error {
        request_id: Option<String>,
        code: String,
        message: String,
    },
}

const fn snapshot_complete_by_default() -> bool {
    true
}
