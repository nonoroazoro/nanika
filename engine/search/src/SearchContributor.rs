use crate::{Candidate, SearchCommand, SearchQueueError};
use std::sync::mpsc::SyncSender;

/// Publication capability bound to a registered extension process lifetime.
#[derive(Clone)]
pub struct SearchContributor {
    _extension_id: String,
    _instance_id: u64,
    _commands: SyncSender<SearchCommand>
}

impl SearchContributor {
    pub(crate) fn new(
        extension_id: String,
        instance_id: u64,
        commands: SyncSender<SearchCommand>
    ) -> Self {
        Self {
            _extension_id: extension_id,
            _instance_id: instance_id,
            _commands: commands
        }
    }

    pub fn extension_id(&self) -> &str {
        &self._extension_id
    }
    pub fn instance_id(&self) -> u64 {
        self._instance_id
    }

    /// Publish request admission or cancellation without removing available results.
    pub fn set_extension_query_pending(
        &self,
        generation: u64,
        pending: bool
    ) -> Result<(), SearchQueueError> {
        self._send(SearchCommand::ExtensionQueryPending {
            generation,
            extension_id: self._extension_id.clone(),
            instance_id: self._instance_id,
            pending
        })
    }

    pub fn publish_extension_snapshot(
        &self,
        generation: u64,
        candidates: Vec<Candidate>,
        complete: bool
    ) -> Result<(), SearchQueueError> {
        self._send(SearchCommand::ExtensionSnapshot {
            complete,
            generation,
            extension_id: self._extension_id.clone(),
            instance_id: self._instance_id,
            candidates
        })
    }

    pub fn publish_extension_delta(
        &self,
        generation: u64,
        candidates: Vec<Candidate>,
        removed: Vec<String>,
        complete: bool
    ) -> Result<(), SearchQueueError> {
        self._send(SearchCommand::ExtensionDelta {
            complete,
            generation,
            extension_id: self._extension_id.clone(),
            instance_id: self._instance_id,
            candidates,
            removed
        })
    }

    /// Register immutable contributions once, outside the interactive query path.
    pub fn register_static_catalog(
        &self,
        candidates: Vec<Candidate>
    ) -> Result<(), SearchQueueError> {
        self._send(SearchCommand::RegisterStaticCatalog {
            extension_id: self._extension_id.clone(),
            instance_id: self._instance_id,
            candidates
        })
    }

    /// Completion means the search owner applied the complete transaction.
    pub fn commit_catalog(
        &self,
        replace: bool,
        candidates: Vec<Candidate>,
        removed: Vec<String>
    ) -> Result<(), SearchQueueError> {
        let (completion, receipt) = std::sync::mpsc::sync_channel(1);
        self._send(SearchCommand::CatalogCommit {
            extension_id: self._extension_id.clone(),
            instance_id: self._instance_id,
            replace,
            candidates,
            removed,
            completion
        })?;
        receipt.recv().map_err(|_| SearchQueueError::Closed)?
    }

    pub fn retire(&self) -> Result<(), SearchQueueError> {
        let (completion, receipt) = std::sync::mpsc::sync_channel(1);
        self._send(SearchCommand::RemoveExtension {
            extension_id: self._extension_id.clone(),
            instance_id: self._instance_id,
            completion
        })?;
        receipt.recv().map_err(|_| SearchQueueError::Closed)
    }

    fn _send(&self, command: SearchCommand) -> Result<(), SearchQueueError> {
        self._commands
            .send(command)
            .map_err(|_| SearchQueueError::Closed)
    }
}
