use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use nanika_search::{SearchHandle, UsageKey};

use crate::{
    ExtensionKind, HostDatabase, SearchStorageCommand, SearchStorageFailure, SearchStorageState,
    StorageQueueError, extension_id::is_valid_extension_id, unix_timestamp,
};

/// Bounded asynchronous owner for search-related host database writes.
pub struct SearchStorageWorker {
    commands: SyncSender<SearchStorageCommand>,
    last_failure: Arc<Mutex<Option<SearchStorageFailure>>>,
    search: Arc<Mutex<Option<SearchHandle>>>,
    thread: Option<JoinHandle<()>>,
}

impl SearchStorageWorker {
    pub fn spawn(
        database_path: impl Into<PathBuf>,
        history_limit: usize,
    ) -> Result<(Self, SearchStorageState), String> {
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
                    .prune_usage(unix_timestamp())
                    .and_then(|()| database.load_input_history(history_limit))
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
                    let (operation, result) = match command {
                        SearchStorageCommand::RegisterExtension {
                            extension_id,
                            kind,
                            updated_at,
                        } => (
                            "register extension metadata",
                            database.register_extension(&extension_id, kind, updated_at),
                        ),
                        SearchStorageCommand::RecordHistory {
                            history_key,
                            display_query,
                            used_at,
                        } => (
                            "record input history",
                            database.record_input_history(
                                &history_key,
                                &display_query,
                                used_at,
                                history_limit,
                            ),
                        ),
                        SearchStorageCommand::RecordUsage {
                            extension_id,
                            entry_id,
                            action_id,
                            query_context,
                            executed_at,
                        } => (
                            "record action usage",
                            database
                                .record_usage(
                                    &extension_id,
                                    &entry_id,
                                    &action_id,
                                    &query_context,
                                    executed_at,
                                )
                                .and_then(|()| {
                                    if let Some(search) = owner_search
                                        .lock()
                                        .unwrap_or_else(|error| error.into_inner())
                                        .clone()
                                    {
                                        search
                                            .apply_persisted_execution(
                                                UsageKey::new(
                                                    &extension_id,
                                                    &entry_id,
                                                    &action_id,
                                                    &query_context,
                                                ),
                                                executed_at,
                                            )
                                            .map_err(|error| {
                                                rusqlite::Error::ToSqlConversionFailure(Box::new(
                                                    error,
                                                ))
                                            })?;
                                    }
                                    Ok(())
                                }),
                        ),
                        SearchStorageCommand::ResetUsage => (
                            "reset action usage",
                            database.reset_usage().and_then(|()| {
                                if let Some(search) = owner_search
                                    .lock()
                                    .unwrap_or_else(|error| error.into_inner())
                                    .clone()
                                {
                                    search.reset_persisted_usage().map_err(|error| {
                                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                                    })?;
                                }
                                Ok(())
                            }),
                        ),
                        SearchStorageCommand::Shutdown => break,
                    };
                    *owner_failure
                        .lock()
                        .unwrap_or_else(|error| error.into_inner()) = result.err().map(|error| {
                        failure_sequence = failure_sequence.saturating_add(1);
                        SearchStorageFailure::new(failure_sequence, operation, error.to_string())
                    });
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
                thread: Some(thread),
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

    pub fn register_extension(
        &self,
        extension_id: impl Into<String>,
        kind: ExtensionKind,
        updated_at: u64,
    ) -> Result<(), StorageQueueError> {
        let extension_id = extension_id.into();
        if !is_valid_extension_id(&extension_id) {
            return Err(StorageQueueError::InvalidExtensionId);
        }
        self.try_send(SearchStorageCommand::RegisterExtension {
            extension_id,
            kind,
            updated_at,
        })
    }

    pub fn record_history(
        &self,
        history_key: impl Into<String>,
        display_query: impl Into<String>,
        used_at: u64,
    ) -> Result<(), StorageQueueError> {
        self.try_send(SearchStorageCommand::RecordHistory {
            history_key: history_key.into(),
            display_query: display_query.into(),
            used_at,
        })
    }

    pub fn record_usage(
        &self,
        extension_id: impl Into<String>,
        entry_id: impl Into<String>,
        action_id: impl Into<String>,
        query_context: impl Into<String>,
        executed_at: u64,
    ) -> Result<(), StorageQueueError> {
        self.try_send(SearchStorageCommand::RecordUsage {
            extension_id: extension_id.into(),
            entry_id: entry_id.into(),
            action_id: action_id.into(),
            query_context: query_context.into(),
            executed_at,
        })
    }

    pub fn reset_usage(&self) -> Result<(), StorageQueueError> {
        self.try_send(SearchStorageCommand::ResetUsage)
    }

    pub fn last_failure(&self) -> Option<SearchStorageFailure> {
        self.last_failure
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn shutdown(mut self) {
        self.stop();
    }

    fn try_send(&self, command: SearchStorageCommand) -> Result<(), StorageQueueError> {
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                TrySendError::Full(_) => StorageQueueError::Full,
                TrySendError::Disconnected(_) => StorageQueueError::Closed,
            })
    }

    fn stop(&mut self) {
        let _ = self.commands.send(SearchStorageCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for SearchStorageWorker {
    fn drop(&mut self) {
        self.stop();
    }
}
