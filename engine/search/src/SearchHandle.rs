use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

use crate::{
    MAX_QUERY_CHARS, PendingSearchQuery, SearchCommand, SearchNotifier, SearchQueueError,
    SearchSnapshot, UsageKey,
};

/// Cloneable boundary used by UI, extension workers, and the storage owner.
#[derive(Clone)]
pub struct SearchHandle {
    pub(crate) commands: SyncSender<SearchCommand>,
    pub(crate) pending_query: Arc<Mutex<Option<PendingSearchQuery>>>,
    pub(crate) latest: Arc<Mutex<Option<Arc<SearchSnapshot>>>>,
    pub(crate) next_generation: Arc<AtomicU64>,
    pub(crate) notifier: SearchNotifier,
}

impl SearchHandle {
    pub fn begin_query(&self, query: impl Into<String>) -> Result<u64, SearchQueueError> {
        self.begin_query_with_expected_extensions(query, std::iter::empty())
    }

    pub fn begin_query_with_expected_extensions(
        &self,
        query: impl Into<String>,
        expected_extensions: impl IntoIterator<Item = String>,
    ) -> Result<u64, SearchQueueError> {
        let query = query.into();
        if query.chars().count() > MAX_QUERY_CHARS {
            return Err(SearchQueueError::QueryTooLong);
        }
        let expected_extensions = expected_extensions.into_iter().collect();
        // Assign generations in the same critical section that publishes the latest query.
        let mut pending = self
            .pending_query
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let generation = self
            .next_generation
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1)
            .max(1);
        *pending = Some(PendingSearchQuery {
            generation,
            query,
            expected_extensions,
        });
        drop(pending);
        match self.commands.try_send(SearchCommand::WakeQuery) {
            Ok(()) | Err(TrySendError::Full(_)) => Ok(generation),
            Err(TrySendError::Disconnected(_)) => Err(SearchQueueError::Closed),
        }
    }

    /// Register one process lifetime before accepting any of its publications.
    pub fn register_extension(
        &self,
        extension_id: impl Into<String>,
        instance_id: u64,
    ) -> Result<crate::SearchContributor, SearchQueueError> {
        let extension_id = extension_id.into();
        let (completion, receipt) = std::sync::mpsc::sync_channel(1);
        self.send(SearchCommand::RegisterExtension {
            extension_id: extension_id.clone(),
            instance_id,
            completion,
        })?;
        receipt.recv().map_err(|_| SearchQueueError::Closed)??;
        Ok(crate::SearchContributor::new(
            extension_id,
            instance_id,
            self.commands.clone(),
        ))
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

    /// A newly admitted query revokes the previous authority before ranking finishes.
    pub fn is_current(&self, authority: crate::SearchAuthority) -> bool {
        self.next_generation.load(Ordering::Acquire) == authority.generation
            && self
                .latest
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .as_ref()
                .is_some_and(|snapshot| snapshot.authority() == authority)
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
