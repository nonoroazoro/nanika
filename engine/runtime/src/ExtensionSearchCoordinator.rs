use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, RwLock};

use nanika_extension_package::ExtensionContributions;
use nanika_search::SearchHandle;

use crate::{
    ExtensionInvocation, ExtensionInvocationOutcome, ExtensionInvocationOutput, ExtensionNotifier,
    ExtensionSearchWorker, ExtensionSearchWorkerContext, ExtensionViewRequest,
    ExtensionViewRequestKind, HostServiceHandler, OperationError, RuntimeViewCompletion,
    RuntimeViewInvalidation
};

/// Dynamic collection of instance-bound workers queried by one host generation.
pub struct ExtensionSearchCoordinator {
    closing: std::sync::atomic::AtomicBool,
    _registration: Arc<crate::ExtensionOperationGate>,
    workers: RwLock<Vec<Arc<ExtensionSearchWorker>>>,
    _invocation_output: Arc<Mutex<crate::ExtensionInvocationOutputState>>,
    next_invocation_id: AtomicU64,
    next_view_request_id: AtomicU64,
    next_refresh_id: AtomicU64,
    notifier: ExtensionNotifier,
    host_services: Mutex<Option<Arc<dyn HostServiceHandler>>>,
    view_invalidations: Arc<Mutex<HashMap<String, RuntimeViewInvalidation>>>
}

impl ExtensionSearchCoordinator {
    pub fn new() -> Self {
        Self {
            closing: std::sync::atomic::AtomicBool::new(false),
            _registration: Arc::new(crate::ExtensionOperationGate::default()),
            workers: RwLock::new(Vec::new()),
            _invocation_output: Arc::new(Mutex::new(
                crate::ExtensionInvocationOutputState::default()
            )),
            next_invocation_id: AtomicU64::new(1),
            next_view_request_id: AtomicU64::new(1),
            next_refresh_id: AtomicU64::new(1),
            notifier: Arc::new(Mutex::new(None)),
            host_services: Mutex::new(None),
            view_invalidations: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub fn set_host_services(&self, host_services: Arc<dyn HostServiceHandler>) {
        *self
            .host_services
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(host_services);
    }

    pub fn register_source(
        &self,
        extension_id: impl Into<String>,
        source: crate::ExtensionRuntimeSource,
        search: SearchHandle,
        contributions: ExtensionContributions,
        configuration: nanika_protocol::ExtensionConfiguration
    ) -> std::io::Result<()> {
        if self.closing.load(Ordering::Acquire) {
            return Err(std::io::Error::other("Extension admission is closed."));
        }
        let extension_id = extension_id.into();
        let _registration = self
            ._registration
            .reserve(&extension_id)
            .map_err(std::io::Error::other)?;
        if source.is_deferred() && contributions.root_search.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "dynamic Root Search requires startup activation"
            ));
        }
        if self
            ._workers()
            .iter()
            .any(|worker| worker.extension_id() == extension_id)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("extension search worker already exists: {extension_id}")
            ));
        }
        let worker = Arc::new(ExtensionSearchWorker::spawn(
            extension_id,
            source,
            search,
            contributions,
            configuration,
            ExtensionSearchWorkerContext {
                invocation_output: Arc::clone(&self._invocation_output),
                notifier: Arc::clone(&self.notifier),
                host_services: self
                    .host_services
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .clone(),
                view_invalidations: Arc::clone(&self.view_invalidations)
            }
        )?);
        let mut workers = self
            .workers
            .write()
            .unwrap_or_else(|error| error.into_inner());
        if self.closing.load(Ordering::Acquire) {
            drop(workers);
            let _ = worker.retire();
            worker.request_stop();
            worker.join();
            return Err(std::io::Error::other("Extension admission is closed."));
        }
        workers.push(worker);
        Ok(())
    }

    pub(crate) fn take_view_invalidations(&self) -> Vec<RuntimeViewInvalidation> {
        std::mem::take(
            &mut *self
                .view_invalidations
                .lock()
                .unwrap_or_else(|error| error.into_inner())
        )
        .into_values()
        .collect()
    }

    pub fn query(&self, generation: u64, query: &str) {
        for worker in &self._workers() {
            worker.query(generation, query);
        }
    }

    pub(crate) fn prepare_entries(
        &self,
        generation: u64,
        visible: &[nanika_search::RankedCandidate]
    ) {
        for worker in &self._workers() {
            let entry_ids = visible
                .iter()
                .filter(|ranked| ranked.candidate.extension_id() == worker.extension_id())
                .map(|ranked| ranked.candidate.entry_id().to_owned())
                .collect();
            worker.prepare_entries(generation, entry_ids);
        }
    }

    pub(crate) fn ready_extension_ids(&self) -> Vec<String> {
        self._workers()
            .iter()
            .filter(|worker| worker.is_query_ready())
            .map(|worker| worker.extension_id().to_owned())
            .collect()
    }

    pub fn refresh(
        &self,
        extension_id: &str,
        generation: u64
    ) -> Result<Receiver<Result<(), crate::OperationError>>, OperationError> {
        let worker = self
            ._workers()
            .iter()
            .find(|worker| worker.extension_id() == extension_id)
            .cloned()
            .ok_or_else(|| OperationError::new(crate::OperationErrorCode::ExtensionUnavailable))?;
        self._refresh_worker(&worker, generation)
    }

    fn _refresh_worker(
        &self,
        worker: &ExtensionSearchWorker,
        generation: u64
    ) -> Result<Receiver<Result<(), crate::OperationError>>, OperationError> {
        let (completion, receiver) = mpsc::sync_channel(1);
        worker.refresh(crate::ExtensionRefresh {
            request_id: self.next_refresh_id.fetch_add(1, Ordering::Relaxed),
            generation,
            completion
        })?;
        Ok(receiver)
    }

    /// Refresh every dynamic Root Search contributor and retain every completion.
    /// The caller must run outside the UI thread because admission and completion wait.
    pub fn refresh_root_search(&self, generation: u64) -> Result<(), Vec<crate::RuntimeWarning>> {
        let mut pending = Vec::new();
        let mut errors = Vec::new();
        for worker in self
            ._workers()
            .iter()
            .filter(|worker| worker.contributes_root_search())
        {
            let id = worker.extension_id().to_owned();
            match self._refresh_worker(worker, generation) {
                Ok(completion) => pending.push((id, completion)),
                Err(error) => errors.push(crate::RuntimeWarning::new(Some(id), error))
            }
        }
        for (id, completion) in pending {
            let result = completion.recv().unwrap_or_else(|_| {
                Err(crate::OperationError::new(
                    crate::OperationErrorCode::RefreshUnconfirmed
                ))
            });
            if let Err(error) = result {
                errors.push(crate::RuntimeWarning::new(Some(id), error));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn first_error(&self) -> Option<crate::HostDiagnostic> {
        self._workers()
            .iter()
            .find_map(|worker| worker.last_error())
    }

    pub(crate) fn warnings(&self) -> Vec<crate::RuntimeWarning> {
        self._workers()
            .iter()
            .filter_map(|worker| {
                worker.last_error().map(|diagnostic| {
                    crate::RuntimeWarning::new(
                        Some(worker.extension_id().to_owned()),
                        diagnostic.operation_error(crate::OperationErrorCode::ExtensionUnavailable)
                    )
                })
            })
            .collect()
    }

    pub fn invoke(
        &self,
        extension_id: &str,
        instance_id: u64,
        generation: u64,
        entry_id: impl Into<String>,
        action_id: impl Into<String>,
        query_context: impl Into<String>
    ) -> Result<Receiver<Result<ExtensionInvocationOutcome, crate::OperationError>>, OperationError>
    {
        let worker = self
            ._workers()
            .iter()
            .find(|worker| {
                worker.extension_id() == extension_id && worker.instance.id == instance_id
            })
            .cloned()
            .ok_or_else(|| OperationError::new(crate::OperationErrorCode::ExtensionUnavailable))?;
        let invocation_id = self.next_invocation_id.fetch_add(1, Ordering::Relaxed);
        let (response, completion) = mpsc::sync_channel(1);
        let invocation = ExtensionInvocation {
            invocation_id,
            generation,
            entry_id: entry_id.into(),
            action_id: action_id.into(),
            query_context: query_context.into(),
            response
        };
        worker.invoke(invocation)?;
        Ok(completion)
    }

    pub fn cancel_invocation(
        &self,
        extension_id: &str,
        invocation_id: u64
    ) -> Result<(), OperationError> {
        let worker = self
            ._workers()
            .iter()
            .find(|worker| worker.extension_id() == extension_id)
            .cloned()
            .ok_or_else(|| OperationError::new(crate::OperationErrorCode::ExtensionUnavailable))?;
        worker.cancel_invocation(invocation_id);
        Ok(())
    }

    pub(crate) fn read_text(
        &self,
        extension_id: &str,
        instance_id: u64,
        generation: u64,
        view_id: String,
        read: nanika_protocol::TextRead
    ) -> Result<Receiver<Result<nanika_protocol::TextChunk, crate::OperationError>>, OperationError>
    {
        let worker = self
            ._workers()
            .iter()
            .find(|worker| {
                worker.extension_id() == extension_id && worker.instance.id == instance_id
            })
            .cloned()
            .ok_or_else(|| OperationError::new(crate::OperationErrorCode::ExtensionUnavailable))?;
        let (completion, receiver) = mpsc::channel();
        worker.read_text(crate::ExtensionTextRead {
            request_id: self.next_view_request_id.fetch_add(1, Ordering::Relaxed),
            generation,
            view_id,
            read,
            completion
        })?;
        Ok(receiver)
    }

    pub fn view_event(
        &self,
        extension_id: &str,
        instance_id: u64,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64,
        event: nanika_protocol::ViewEvent
    ) -> Result<Receiver<Result<RuntimeViewCompletion, crate::OperationError>>, OperationError>
    {
        let worker = self
            ._workers()
            .iter()
            .find(|worker| {
                worker.extension_id() == extension_id && worker.instance.id == instance_id
            })
            .cloned()
            .ok_or_else(|| OperationError::new(crate::OperationErrorCode::ExtensionUnavailable))?;
        let request_id = self.next_view_request_id.fetch_add(1, Ordering::Relaxed);
        let (completion, receiver) = mpsc::channel();
        worker.view_event(ExtensionViewRequest {
            completion,
            request_id,
            generation,
            view_id: view_id.into(),
            revision,
            kind: ExtensionViewRequestKind::Event(event)
        })?;
        Ok(receiver)
    }

    pub(crate) fn close_view(
        &self,
        extension_id: &str,
        instance_id: u64,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64
    ) -> Result<Receiver<Result<RuntimeViewCompletion, crate::OperationError>>, OperationError>
    {
        let worker = self
            ._workers()
            .iter()
            .find(|worker| {
                worker.extension_id() == extension_id && worker.instance.id == instance_id
            })
            .cloned()
            .ok_or_else(|| OperationError::new(crate::OperationErrorCode::ExtensionUnavailable))?;
        let request_id = self.next_view_request_id.fetch_add(1, Ordering::Relaxed);
        let (completion, receiver) = mpsc::channel();
        worker.view_event(ExtensionViewRequest {
            completion,
            request_id,
            generation,
            view_id: view_id.into(),
            revision,
            kind: ExtensionViewRequestKind::Close
        })?;
        Ok(receiver)
    }

    pub(crate) fn take_invocation_outputs(&self) -> Vec<ExtensionInvocationOutput> {
        self._invocation_output
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take_changed()
            .unwrap_or_default()
    }

    pub(crate) fn apply_configuration(
        &self,
        extension_id: &str,
        request_id: impl Into<String>,
        configuration: nanika_protocol::ExtensionConfiguration,
        require_live: bool,
        progress: crate::ConfigurationProgressHandler,
        completion: mpsc::SyncSender<
            Result<crate::ConfigurationApplication, crate::OperationError>
        >
    ) -> Result<bool, OperationError> {
        let Some(worker) = self
            ._workers()
            .iter()
            .find(|worker| worker.extension_id() == extension_id)
            .cloned()
        else {
            return Ok(false);
        };
        if !worker.supports_live_configuration() {
            return Ok(false);
        }
        match worker.apply_configuration(
            request_id.into(),
            configuration,
            require_live,
            progress,
            completion
        ) {
            Ok(()) => Ok(true),
            Err(error) => Err(error)
        }
    }

    pub(crate) fn set_notifier(&self, notifier: Arc<dyn Fn() + Send + Sync>) {
        *self
            .notifier
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(notifier);
    }

    pub(crate) fn notify(&self) {
        let notify = self
            .notifier
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        if let Some(notify) = notify {
            notify();
        }
    }

    pub fn is_empty(&self) -> bool {
        self._workers().is_empty()
    }

    pub fn instance_id(&self, extension_id: &str) -> Option<u64> {
        self.worker(extension_id)
            .filter(|worker| worker.instance.is_active())
            .map(|worker| worker.instance.id)
    }

    pub(crate) fn worker(&self, extension_id: &str) -> Option<Arc<ExtensionSearchWorker>> {
        self._workers()
            .into_iter()
            .find(|worker| worker.extension_id() == extension_id)
    }

    pub(crate) fn remove_stopped(&self, extension_id: &str) {
        let worker = {
            let mut workers = self
                .workers
                .write()
                .unwrap_or_else(|error| error.into_inner());
            workers
                .iter()
                .position(|worker| worker.extension_id() == extension_id)
                .map(|index| workers.remove(index))
        };
        if let Some(worker) = worker {
            worker.join();
        }
    }

    pub fn request_shutdown(&self) {
        self.closing.store(true, Ordering::Release);
        for worker in &self._workers() {
            worker.request_stop();
        }
    }

    pub fn shutdown(&self) {
        self.request_shutdown();
        for worker in &self._workers() {
            worker.join();
        }
        self.host_services
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
    }
    fn _workers(&self) -> Vec<Arc<ExtensionSearchWorker>> {
        self.workers
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
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
