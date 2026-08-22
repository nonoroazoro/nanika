use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};

use nanika_search::SearchHandle;

use crate::{
    ExtensionInvocation, ExtensionInvocationResult, ExtensionNotifier, ExtensionProcess,
    ExtensionSearchWorker, SupervisorError,
};

const INVOCATION_RESULT_CAPACITY: usize = 16;

/// Collection of fixed extension workers queried by one host generation.
pub struct ExtensionSearchCoordinator {
    workers: Vec<ExtensionSearchWorker>,
    results: Receiver<ExtensionInvocationResult>,
    result_sender: SyncSender<ExtensionInvocationResult>,
    pending_invocations: AtomicUsize,
    notifier: ExtensionNotifier,
}

impl ExtensionSearchCoordinator {
    pub fn new() -> Self {
        let (result_sender, results) = mpsc::sync_channel(INVOCATION_RESULT_CAPACITY);
        Self {
            workers: Vec::new(),
            results,
            result_sender,
            pending_invocations: AtomicUsize::new(0),
            notifier: Arc::new(Mutex::new(None)),
        }
    }

    pub fn register(
        &mut self,
        extension_id: impl Into<String>,
        process: ExtensionProcess,
        search: SearchHandle,
    ) -> std::io::Result<()> {
        let extension_id = extension_id.into();
        if self
            .workers
            .iter()
            .any(|worker| worker.extension_id() == extension_id)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("extension search worker already exists: {extension_id}"),
            ));
        }
        self.workers.push(ExtensionSearchWorker::spawn(
            extension_id,
            process,
            search,
            self.result_sender.clone(),
            Arc::clone(&self.notifier),
        )?);
        Ok(())
    }

    pub fn query(&self, generation: u64, query: &str) {
        for worker in &self.workers {
            worker.query(generation, query);
        }
    }

    pub fn first_error(&self) -> Option<String> {
        self.workers
            .iter()
            .find_map(ExtensionSearchWorker::last_error)
    }

    pub fn invoke(
        &self,
        extension_id: &str,
        generation: u64,
        entry_id: impl Into<String>,
        action_id: impl Into<String>,
        query_context: impl Into<String>,
    ) -> Result<(), SupervisorError> {
        let worker = self
            .workers
            .iter()
            .find(|worker| worker.extension_id() == extension_id)
            .ok_or_else(|| {
                SupervisorError::UnexpectedMessage(format!(
                    "extension search worker does not exist: {extension_id}"
                ))
            })?;
        self.pending_invocations
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |pending| {
                (pending < INVOCATION_RESULT_CAPACITY).then_some(pending + 1)
            })
            .map_err(|_| SupervisorError::QueueFull)?;
        let invocation = ExtensionInvocation {
            generation,
            entry_id: entry_id.into(),
            action_id: action_id.into(),
            query_context: query_context.into(),
        };
        if let Err(error) = worker.invoke(invocation) {
            self.pending_invocations.fetch_sub(1, Ordering::AcqRel);
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn take_results(&self) -> Vec<ExtensionInvocationResult> {
        let results = self.results.try_iter().collect::<Vec<_>>();
        self.pending_invocations
            .fetch_sub(results.len(), Ordering::AcqRel);
        results
    }

    pub(crate) fn set_notifier(&self, notifier: Arc<dyn Fn() + Send + Sync>) {
        *self
            .notifier
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(notifier);
    }

    pub fn is_empty(&self) -> bool {
        self.workers.is_empty()
    }

    pub fn shutdown(&mut self) {
        for worker in &self.workers {
            worker.request_stop();
        }
        for worker in &mut self.workers {
            worker.join();
        }
        self.workers.clear();
    }
}

impl Default for ExtensionSearchCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ExtensionSearchCoordinator {
    fn drop(&mut self) {
        self.shutdown();
    }
}
