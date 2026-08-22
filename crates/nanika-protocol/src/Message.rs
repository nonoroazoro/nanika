use serde::{Deserialize, Serialize};

use crate::{
    Candidate, HostServiceRequest, HostServiceResponse, SettingUpdate, SettingsContribution,
};

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
    Refresh {
        request_id: String,
        generation: u64,
    },
    Refreshed {
        request_id: String,
        generation: u64,
    },
    GetSettings {
        request_id: String,
    },
    Settings {
        request_id: String,
        contribution: SettingsContribution,
    },
    UpdateSettings {
        request_id: String,
        updates: Vec<SettingUpdate>,
    },
    SettingsUpdated {
        request_id: String,
        contribution: SettingsContribution,
    },
    HostRequest {
        request_id: String,
        parent_request_id: String,
        generation: u64,
        request: HostServiceRequest,
    },
    HostResponse {
        request_id: String,
        parent_request_id: String,
        generation: u64,
        response: HostServiceResponse,
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
