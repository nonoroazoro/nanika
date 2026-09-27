use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use nanika_search::{SearchHandle, UsageKey};

use crate::{
    HostDatabase, SearchStorageCommand, SearchStorageFailure, SearchStorageState,
    StorageQueueError, extension_id::is_valid_extension_id,
};

/// Serialized owner for search-related host database writes.
pub struct SearchStorageWorker {
    commands: SyncSender<SearchStorageCommand>,
    last_failure: Arc<Mutex<Option<SearchStorageFailure>>>,
    search: Arc<Mutex<Option<SearchHandle>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl SearchStorageWorker {
    pub fn spawn(database_path: impl Into<PathBuf>) -> Result<(Self, SearchStorageState), String> {
        let (commands, receiver) = mpsc::sync_channel(64);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let last_failure = Arc::new(Mutex::new(None));
        let owner_failure = Arc::clone(&last_failure);
        let search = Arc::new(Mutex::new(None::<SearchHandle>));
        let owner_search = Arc::clone(&search);
        let database_path = database_path.into();
        let thread = std::thread::Builder::new()
            .name("nanika-storage-owner".to_owned())
            .spawn(move || {
                let database = match HostDatabase::open(database_path) {
                    Ok(database) => database,
                    Err(error) => {
                        let _ = ready_sender.send(Err(error.to_string()));
                        return;
                    }
                };
                let state = database
                    .load_input_history()
                    .and_then(|input_history| {
                        database.load_usage().and_then(|usage| {
                            database.load_extensions_isolated().map(|extension_load| {
                                SearchStorageState {
                                    input_history,
                                    usage,
                                    extensions: extension_load.extensions,
                                    extension_errors: extension_load.errors,
                                }
                            })
                        })
                    })
                    .map_err(|error| error.to_string());
                if ready_sender.send(state).is_err() {
                    return;
                }

                let mut failure_sequence = 0_u64;
                while let Ok(command) = receiver.recv() {
                    let (operation, result, response, projection) = match command {
                        SearchStorageCommand::RegisterBuiltInExtension {
                            extension_id,
                            response,
                        } => (
                            "register built-in extension metadata",
                            database.register_builtin_extension(&extension_id),
                            response,
                            None,
                        ),
                        SearchStorageCommand::RecordHistory {
                            history_key,
                            display_query,
                            used_at,
                            response,
                        } => (
                            "record input history",
                            database.record_input_history(&history_key, &display_query, used_at),
                            response,
                            None,
                        ),
                        SearchStorageCommand::RecordUsage {
                            extension_id,
                            entry_id,
                            action_id,
                            query_context,
                            executed_at,
                            response,
                        } => (
                            "record action usage",
                            database.record_usage(
                                &extension_id,
                                &entry_id,
                                &action_id,
                                &query_context,
                                executed_at,
                            ),
                            response,
                            Some(crate::PersistedUsageChange::Execution {
                                key: UsageKey::new(
                                    &extension_id,
                                    &entry_id,
                                    &action_id,
                                    &query_context,
                                ),
                                executed_at,
                            }),
                        ),
                        SearchStorageCommand::RecordExecution {
                            history_key,
                            display_query,
                            usage,
                            history_used_at,
                            executed_at,
                            response,
                        } => (
                            "record completed action",
                            database.record_execution(
                                &history_key,
                                &display_query,
                                &usage,
                                history_used_at,
                                executed_at,
                            ),
                            response,
                            Some(crate::PersistedUsageChange::Execution {
                                key: usage,
                                executed_at,
                            }),
                        ),
                        SearchStorageCommand::ResetUsage { response } => (
                            "reset action usage",
                            database.reset_usage(),
                            response,
                            Some(crate::PersistedUsageChange::Reset),
                        ),
                        SearchStorageCommand::Shutdown => break,
                    };
                    let result = result.map_err(|error| error.to_string());
                    let committed = result.is_ok();
                    if let Err(error) = &result {
                        _record_failure(
                            &owner_failure,
                            &mut failure_sequence,
                            operation,
                            error.clone(),
                        );
                    }
                    // The database owns accepted work even when the caller does not await it.
                    // A durable receipt precedes projection backpressure and never claims ranking delivery.
                    let _ = response.send(result);
                    if committed
                        && let Some(change) = projection
                        && let Err(error) = _notify_search(&owner_search, change)
                    {
                        _record_failure(
                            &owner_failure,
                            &mut failure_sequence,
                            "publish committed storage change",
                            error,
                        );
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        let state = ready_receiver
            .recv()
            .map_err(|_| "storage owner closed during initialization".to_owned())??;
        Ok((
            Self {
                commands,
                last_failure,
                search,
                thread: Mutex::new(Some(thread)),
            },
            state,
        ))
    }

    pub fn attach_search(&self, search: SearchHandle) {
        *self
            .search
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(search);
    }

    pub fn register_builtin_extension(
        &self,
        extension_id: impl Into<String>,
    ) -> Result<crate::StorageCommit, StorageQueueError> {
        let extension_id = extension_id.into();
        if !is_valid_extension_id(&extension_id) {
            return Err(StorageQueueError::InvalidExtensionId);
        }
        let (response, result) = mpsc::sync_channel(1);
        self.send(
            SearchStorageCommand::RegisterBuiltInExtension {
                extension_id,
                response,
            },
            result,
        )
    }

    pub fn record_history(
        &self,
        history_key: impl Into<String>,
        display_query: impl Into<String>,
        used_at: u64,
    ) -> Result<crate::StorageCommit, StorageQueueError> {
        let (response, result) = mpsc::sync_channel(1);
        self.send(
            SearchStorageCommand::RecordHistory {
                history_key: history_key.into(),
                display_query: display_query.into(),
                used_at,
                response,
            },
            result,
        )
    }

    pub fn record_usage(
        &self,
        extension_id: impl Into<String>,
        entry_id: impl Into<String>,
        action_id: impl Into<String>,
        query_context: impl Into<String>,
        executed_at: u64,
    ) -> Result<crate::StorageCommit, StorageQueueError> {
        let (response, result) = mpsc::sync_channel(1);
        self.send(
            SearchStorageCommand::RecordUsage {
                extension_id: extension_id.into(),
                entry_id: entry_id.into(),
                action_id: action_id.into(),
                query_context: query_context.into(),
                executed_at,
                response,
            },
            result,
        )
    }

    pub fn reset_usage(&self) -> Result<crate::StorageCommit, StorageQueueError> {
        let (response, result) = mpsc::sync_channel(1);
        self.send(SearchStorageCommand::ResetUsage { response }, result)
    }

    pub fn record_execution(
        &self,
        history_key: impl Into<String>,
        display_query: impl Into<String>,
        usage: UsageKey,
        history_used_at: u64,
        executed_at: u64,
    ) -> Result<crate::StorageCommit, StorageQueueError> {
        let (response, result) = mpsc::sync_channel(1);
        self.send(
            SearchStorageCommand::RecordExecution {
                history_key: history_key.into(),
                display_query: display_query.into(),
                usage,
                history_used_at,
                executed_at,
                response,
            },
            result,
        )
    }

    pub fn last_failure(&self) -> Option<SearchStorageFailure> {
        self.last_failure
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn shutdown(&self) {
        self.stop();
    }

    fn send(
        &self,
        command: SearchStorageCommand,
        response: Receiver<Result<(), String>>,
    ) -> Result<crate::StorageCommit, StorageQueueError> {
        self.commands
            .send(command)
            .map_err(|_| StorageQueueError::Closed)?;
        Ok(crate::StorageCommit::new(response))
    }

    fn stop(&self) {
        let mut thread = self
            .thread
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if thread.is_none() {
            return;
        }
        if self.commands.send(SearchStorageCommand::Shutdown).is_err() {
            tracing::error!("storage owner closed before shutdown was requested");
        }
        if let Some(thread) = thread.take()
            && thread.join().is_err()
        {
            tracing::error!("storage owner thread panicked");
        }
    }
}

fn _record_failure(
    failure: &Mutex<Option<SearchStorageFailure>>,
    sequence: &mut u64,
    operation: &'static str,
    source: String,
) {
    *sequence = sequence.saturating_add(1);
    tracing::error!(operation, source, "storage operation failed");
    *failure.lock().unwrap_or_else(|error| error.into_inner()) =
        Some(SearchStorageFailure::new(*sequence, operation, source));
}

fn _notify_search(
    search: &Mutex<Option<SearchHandle>>,
    change: crate::PersistedUsageChange,
) -> Result<(), String> {
    let search = search
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let Some(search) = search else {
        return Ok(());
    };
    match change {
        crate::PersistedUsageChange::Execution { key, executed_at } => {
            search.apply_persisted_execution(key, executed_at)
        }
        crate::PersistedUsageChange::Reset => search.reset_persisted_usage(),
    }
    .map_err(|error| error.to_string())
}

impl Drop for SearchStorageWorker {
    fn drop(&mut self) {
        self.stop();
    }
}
