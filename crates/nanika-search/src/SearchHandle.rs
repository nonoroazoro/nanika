use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

use crate::{
    Candidate, MAX_QUERY_CHARS, SearchCommand, SearchNotifier, SearchQueueError, SearchSnapshot,
    UsageKey,
};

/// Cloneable boundary used by UI, extension workers, and the storage owner.
#[derive(Clone)]
pub struct SearchHandle {
    pub(crate) commands: SyncSender<SearchCommand>,
    pub(crate) pending_query: Arc<Mutex<Option<(u64, String)>>>,
    pub(crate) latest: Arc<Mutex<Option<Arc<SearchSnapshot>>>>,
    pub(crate) next_generation: Arc<AtomicU64>,
    pub(crate) notifier: SearchNotifier,
}

impl SearchHandle {
    pub fn begin_query(&self, query: impl Into<String>) -> Result<u64, SearchQueueError> {
        let query = query.into();
        if query.chars().count() > MAX_QUERY_CHARS {
            return Err(SearchQueueError::QueryTooLong);
        }
        let generation = self
            .next_generation
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1)
            .max(1);
        *self
            .pending_query
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some((generation, query));
        match self.commands.try_send(SearchCommand::WakeQuery) {
            Ok(()) | Err(TrySendError::Full(_)) => Ok(generation),
            Err(TrySendError::Disconnected(_)) => Err(SearchQueueError::Closed),
        }
    }

    pub fn publish_extension_snapshot(
        &self,
        extension_id: impl Into<String>,
        generation: u64,
        candidates: Vec<Candidate>,
    ) -> Result<(), SearchQueueError> {
        self.send(SearchCommand::ExtensionSnapshot {
            generation,
            extension_id: extension_id.into(),
            candidates,
        })
    }

    pub fn apply_persisted_execution(
        &self,
        key: UsageKey,
        executed_at: u64,
    ) -> Result<(), SearchQueueError> {
        self.send(SearchCommand::ApplyPersistedExecution { key, executed_at })
    }

    pub fn reset_persisted_usage(&self) -> Result<(), SearchQueueError> {
        self.send(SearchCommand::ResetPersistedUsage)
    }

    pub fn latest_snapshot(&self) -> Option<Arc<SearchSnapshot>> {
        self.latest
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn set_notifier(&self, notifier: Arc<dyn Fn() + Send + Sync>) {
        *self
            .notifier
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(notifier);
    }

    fn send(&self, command: SearchCommand) -> Result<(), SearchQueueError> {
        self.commands
            .send(command)
            .map_err(|_| SearchQueueError::Closed)
    }
}
