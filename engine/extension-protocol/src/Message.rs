use serde::{Deserialize, Serialize};

use crate::{
    Candidate, ExtensionConfiguration, HostServiceRequest, HostServiceResponse, NavigationEffect,
    View, ViewEvent,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Message {
    Initialize {
        request_id: String,
        protocol: String,
        configuration: ExtensionConfiguration,
    },
    Initialized {
        request_id: String,
        protocol: String,
    },
    /// Invalidate the catalog or current query according to the declared Root Search mode.
    CandidatesChanged,
    CatalogRead {
        request_id: String,
    },
    CatalogBatch {
        request_id: String,
        batch: crate::CatalogBatch,
    },
    CatalogApplied {
        transaction: u64,
    },
    /// An open host-rendered view has newer extension-owned data.
    ViewInvalidated {
        view_id: String,
    },
    /// Best-effort hint for entries that are about to be visible in the host UI.
    PrepareEntries {
        generation: u64,
        entry_ids: Vec<String>,
    },
    Query {
        request_id: String,
        generation: u64,
        query: String,
        /// Permit a patch against the last completed snapshot for this generation.
        incremental: bool,
    },
    Snapshot {
        request_id: String,
        generation: u64,
        complete: bool,
        replace: bool,
        removed: Vec<String>,
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
        effect: NavigationEffect,
    },
    ViewEvent {
        request_id: String,
        generation: u64,
        view_id: String,
        revision: u64,
        event: ViewEvent,
    },
    ViewUpdated {
        request_id: String,
        generation: u64,
        view_id: String,
        revision: u64,
        effect: NavigationEffect,
        view: Option<View>,
    },
    ViewClose {
        request_id: String,
        view_id: String,
    },
    ViewClosed {
        request_id: String,
        view_id: String,
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
    ConfigurationChanged {
        request_id: String,
        configuration: ExtensionConfiguration,
    },
    ConfigurationProgress {
        request_id: String,
        progress: crate::OperationProgress,
    },
    ConfigurationApplied {
        request_id: String,
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
    Error {
        request_id: Option<String>,
        code: String,
        message: String,
    },
}
