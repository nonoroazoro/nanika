use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};

use nanika_extension_package::{CommandMode, ExtensionContributions};
use nanika_search::SearchHandle;

use crate::{
    ExtensionInvocation, ExtensionInvocationOutput, ExtensionInvocationResult, ExtensionNotifier,
    ExtensionRuntime, ExtensionSearchWorker, ExtensionSearchWorkerContext, ExtensionSettingsResult,
    ExtensionViewRequest, ExtensionViewRequestKind, ExtensionViewUpdate, HostServiceHandler,
    SupervisorError,
};

const INVOCATION_RESULT_CAPACITY: usize = 16;
const VIEW_UPDATE_CAPACITY: usize = 16;

/// Collection of fixed extension workers queried by one host generation.
pub struct ExtensionSearchCoordinator {
    workers: Vec<ExtensionSearchWorker>,
    results: Receiver<ExtensionInvocationResult>,
    result_sender: SyncSender<ExtensionInvocationResult>,
    view_updates: Receiver<ExtensionViewUpdate>,
    view_update_sender: SyncSender<ExtensionViewUpdate>,
    pending_invocations: AtomicUsize,
    next_invocation_id: AtomicU64,
    next_view_request_id: AtomicU64,
    notifier: ExtensionNotifier,
    host_services: Option<Arc<dyn HostServiceHandler>>,
}

impl ExtensionSearchCoordinator {
    pub fn new() -> Self {
        let (result_sender, results) = mpsc::sync_channel(INVOCATION_RESULT_CAPACITY);
        let (view_update_sender, view_updates) = mpsc::sync_channel(VIEW_UPDATE_CAPACITY);
        Self {
            workers: Vec::new(),
            results,
            result_sender,
            view_updates,
            view_update_sender,
            pending_invocations: AtomicUsize::new(0),
            next_invocation_id: AtomicU64::new(1),
            next_view_request_id: AtomicU64::new(1),
            notifier: Arc::new(Mutex::new(None)),
            host_services: None,
        }
    }

    pub fn set_host_services(&mut self, host_services: Arc<dyn HostServiceHandler>) {
        self.host_services = Some(host_services);
    }

    pub fn register(
        &mut self,
        extension_id: impl Into<String>,
        runtime: impl Into<ExtensionRuntime>,
        search: SearchHandle,
        contributions: ExtensionContributions,
    ) -> std::io::Result<()> {
        let extension_id = extension_id.into();
        let runtime = runtime.into();
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
            runtime,
            search,
            contributions,
            ExtensionSearchWorkerContext {
                invocation_results: self.result_sender.clone(),
                view_updates: self.view_update_sender.clone(),
                notifier: Arc::clone(&self.notifier),
                host_services: self.host_services.clone(),
            },
        )?);
        Ok(())
    }

    pub(crate) fn command_mode(&self, extension_id: &str, entry_id: &str) -> Option<CommandMode> {
        self.workers
            .iter()
            .find(|worker| worker.extension_id() == extension_id)
            .and_then(|worker| worker.command_mode(entry_id))
    }

    pub fn query(&self, generation: u64, query: &str) {
        for worker in &self.workers {
            worker.query(generation, query);
        }
    }

    pub fn refresh(&self, extension_id: &str, generation: u64) -> Result<(), SupervisorError> {
        self.workers
            .iter()
            .find(|worker| worker.extension_id() == extension_id)
            .ok_or_else(|| {
                SupervisorError::UnexpectedMessage(format!(
                    "extension search worker does not exist: {extension_id}"
                ))
            })?
            .refresh(generation);
        Ok(())
    }

    pub fn first_error(&self) -> Option<crate::HostDiagnostic> {
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
    ) -> Result<u64, SupervisorError> {
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
        let invocation_id = self.next_invocation_id.fetch_add(1, Ordering::Relaxed);
        let invocation = ExtensionInvocation {
            invocation_id,
            generation,
            entry_id: entry_id.into(),
            action_id: action_id.into(),
            query_context: query_context.into(),
        };
        if let Err(error) = worker.invoke(invocation) {
            self.pending_invocations.fetch_sub(1, Ordering::AcqRel);
            return Err(error);
        }
        Ok(invocation_id)
    }

    pub fn cancel_invocation(
        &self,
        extension_id: &str,
        invocation_id: u64,
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
        worker.cancel_invocation(invocation_id);
        Ok(())
    }

    pub(crate) fn view_event(
        &self,
        extension_id: &str,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64,
        event: nanika_protocol::ViewEvent,
    ) -> Result<u64, SupervisorError> {
        let worker = self
            .workers
            .iter()
            .find(|worker| worker.extension_id() == extension_id)
            .ok_or_else(|| {
                SupervisorError::UnexpectedMessage(format!(
                    "extension search worker does not exist: {extension_id}"
                ))
            })?;
        let request_id = self.next_view_request_id.fetch_add(1, Ordering::Relaxed);
        worker.view_event(ExtensionViewRequest {
            request_id,
            generation,
            view_id: view_id.into(),
            revision,
            kind: ExtensionViewRequestKind::Event(event),
        })?;
        Ok(request_id)
    }

    pub(crate) fn close_view(
        &self,
        extension_id: &str,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64,
    ) -> Result<u64, SupervisorError> {
        let worker = self
            .workers
            .iter()
            .find(|worker| worker.extension_id() == extension_id)
            .ok_or_else(|| {
                SupervisorError::UnexpectedMessage(format!(
                    "extension search worker does not exist: {extension_id}"
                ))
            })?;
        let request_id = self.next_view_request_id.fetch_add(1, Ordering::Relaxed);
        worker.view_event(ExtensionViewRequest {
            request_id,
            generation,
            view_id: view_id.into(),
            revision,
            kind: ExtensionViewRequestKind::Close,
        })?;
        Ok(request_id)
    }

    pub(crate) fn take_results(&self) -> Vec<ExtensionInvocationResult> {
        let results = self.results.try_iter().collect::<Vec<_>>();
        self.pending_invocations
            .fetch_sub(results.len(), Ordering::AcqRel);
        results
    }

    pub(crate) fn take_view_updates(&self) -> Vec<ExtensionViewUpdate> {
        self.view_updates.try_iter().collect()
    }

    pub(crate) fn take_settings(&self) -> Vec<ExtensionSettingsResult> {
        self.workers
            .iter()
            .filter_map(ExtensionSearchWorker::take_settings)
            .collect()
    }

    pub(crate) fn take_invocation_outputs(&self) -> Vec<ExtensionInvocationOutput> {
        self.workers
            .iter()
            .flat_map(ExtensionSearchWorker::take_invocation_outputs)
            .collect()
    }

    pub(crate) fn update_settings(
        &self,
        extension_id: &str,
        request_id: impl Into<String>,
        updates: Vec<nanika_protocol::SettingUpdate>,
    ) -> Result<(), SupervisorError> {
        self.workers
            .iter()
            .find(|worker| worker.extension_id() == extension_id)
            .ok_or_else(|| {
                SupervisorError::UnexpectedMessage(format!(
                    "extension search worker does not exist: {extension_id}"
                ))
            })?
            .update_settings(request_id.into(), updates)
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
