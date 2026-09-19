use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use nanika_extension_package::ExtensionContributions;
use nanika_search::SearchHandle;

use crate::{
    ExtensionConfigurationResult, ExtensionInvocation, ExtensionInvocationOutcome,
    ExtensionInvocationOutput, ExtensionNotifier, ExtensionRuntime, ExtensionSearchWorker,
    ExtensionSearchWorkerContext, ExtensionViewRequest, ExtensionViewRequestKind,
    HostServiceHandler, RuntimeViewCompletion, RuntimeViewInvalidation, SupervisorError,
};

/// Collection of fixed extension workers queried by one host generation.
pub struct ExtensionSearchCoordinator {
    workers: Vec<ExtensionSearchWorker>,
    next_invocation_id: AtomicU64,
    next_view_request_id: AtomicU64,
    next_refresh_id: AtomicU64,
    notifier: ExtensionNotifier,
    host_services: Mutex<Option<Arc<dyn HostServiceHandler>>>,
    view_invalidations: Arc<Mutex<HashMap<String, RuntimeViewInvalidation>>>,
}

impl ExtensionSearchCoordinator {
    pub fn new() -> Self {
        Self {
            workers: Vec::new(),
            next_invocation_id: AtomicU64::new(1),
            next_view_request_id: AtomicU64::new(1),
            next_refresh_id: AtomicU64::new(1),
            notifier: Arc::new(Mutex::new(None)),
            host_services: Mutex::new(None),
            view_invalidations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set_host_services(&mut self, host_services: Arc<dyn HostServiceHandler>) {
        *self
            .host_services
            .get_mut()
            .unwrap_or_else(|error| error.into_inner()) = Some(host_services);
    }

    pub fn register(
        &mut self,
        extension_id: impl Into<String>,
        runtime: impl Into<ExtensionRuntime>,
        search: SearchHandle,
        contributions: ExtensionContributions,
    ) -> std::io::Result<()> {
        self.register_with_configuration(
            extension_id,
            runtime,
            search,
            contributions,
            nanika_protocol::ExtensionConfiguration::default(),
        )
    }

    pub fn register_with_configuration(
        &mut self,
        extension_id: impl Into<String>,
        runtime: impl Into<ExtensionRuntime>,
        search: SearchHandle,
        contributions: ExtensionContributions,
        configuration: nanika_protocol::ExtensionConfiguration,
    ) -> std::io::Result<()> {
        self.register_source(
            extension_id,
            crate::ExtensionRuntimeSource::Started(Box::new(runtime.into())),
            search,
            contributions,
            configuration,
        )
    }

    pub fn register_source(
        &mut self,
        extension_id: impl Into<String>,
        source: crate::ExtensionRuntimeSource,
        search: SearchHandle,
        contributions: ExtensionContributions,
        configuration: nanika_protocol::ExtensionConfiguration,
    ) -> std::io::Result<()> {
        let extension_id = extension_id.into();
        if source.is_deferred() && contributions.root_search.is_some() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "dynamic Root Search requires startup activation",
            ));
        }
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
            source,
            search,
            contributions,
            configuration,
            ExtensionSearchWorkerContext {
                notifier: Arc::clone(&self.notifier),
                host_services: self
                    .host_services
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .clone(),
                view_invalidations: Arc::clone(&self.view_invalidations),
            },
        )?);
        Ok(())
    }

    pub(crate) fn take_view_invalidations(&self) -> Vec<RuntimeViewInvalidation> {
        std::mem::take(
            &mut *self
                .view_invalidations
                .lock()
                .unwrap_or_else(|error| error.into_inner()),
        )
        .into_values()
        .collect()
    }

    pub fn query(&self, generation: u64, query: &str) {
        for worker in &self.workers {
            worker.query(generation, query);
        }
    }

    pub(crate) fn prepare_entries(
        &self,
        generation: u64,
        visible: &[nanika_search::RankedCandidate],
    ) {
        for worker in &self.workers {
            let entry_ids = visible
                .iter()
                .filter(|ranked| ranked.candidate.extension_id() == worker.extension_id())
                .map(|ranked| ranked.candidate.entry_id().to_owned())
                .collect();
            worker.prepare_entries(generation, entry_ids);
        }
    }

    pub(crate) fn ready_extension_ids(&self) -> Vec<String> {
        self.workers
            .iter()
            .filter(|worker| worker.is_query_ready())
            .map(|worker| worker.extension_id().to_owned())
            .collect()
    }

    pub fn refresh(
        &self,
        extension_id: &str,
        generation: u64,
    ) -> Result<Receiver<Result<(), String>>, SupervisorError> {
        let worker = self
            .workers
            .iter()
            .find(|worker| worker.extension_id() == extension_id)
            .ok_or_else(|| {
                SupervisorError::UnexpectedMessage(format!(
                    "extension search worker does not exist: {extension_id}"
                ))
            })?;
        let (completion, receiver) = mpsc::sync_channel(1);
        worker.refresh(crate::ExtensionRefresh {
            request_id: self.next_refresh_id.fetch_add(1, Ordering::Relaxed),
            generation,
            completion,
        })?;
        Ok(receiver)
    }

    /// Refresh every dynamic Root Search contributor and retain every completion.
    /// The caller must run outside the UI thread because admission and completion wait.
    pub fn refresh_root_search(&self, generation: u64) -> Result<(), String> {
        let mut pending = Vec::new();
        let mut errors = Vec::new();
        for worker in self
            .workers
            .iter()
            .filter(|worker| worker.contributes_root_search())
        {
            let id = worker.extension_id();
            match self.refresh(id, generation) {
                Ok(completion) => pending.push((id, completion)),
                Err(error) => errors.push(format!("{id}: {error}")),
            }
        }
        for (id, completion) in pending {
            let result = completion
                .recv()
                .unwrap_or_else(|_| Err("Extension closed without a refresh result.".to_owned()));
            if let Err(error) = result {
                errors.push(format!("{id}: {error}"));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }

    pub fn first_error(&self) -> Option<crate::HostDiagnostic> {
        self.workers
            .iter()
            .find_map(ExtensionSearchWorker::last_error)
    }

    pub(crate) fn warnings(&self) -> Vec<String> {
        self.workers
            .iter()
            .filter_map(|worker| {
                worker.last_error().map(|diagnostic| {
                    format!("{}: {}", worker.extension_id(), diagnostic.user_message())
                })
            })
            .collect()
    }

    pub fn invoke(
        &self,
        extension_id: &str,
        generation: u64,
        entry_id: impl Into<String>,
        action_id: impl Into<String>,
        query_context: impl Into<String>,
    ) -> Result<Receiver<Result<ExtensionInvocationOutcome, String>>, SupervisorError> {
        let worker = self
            .workers
            .iter()
            .find(|worker| worker.extension_id() == extension_id)
            .ok_or_else(|| {
                SupervisorError::UnexpectedMessage(format!(
                    "extension search worker does not exist: {extension_id}"
                ))
            })?;
        let invocation_id = self.next_invocation_id.fetch_add(1, Ordering::Relaxed);
        let (response, completion) = mpsc::sync_channel(1);
        let invocation = ExtensionInvocation {
            invocation_id,
            generation,
            entry_id: entry_id.into(),
            action_id: action_id.into(),
            query_context: query_context.into(),
            response,
        };
        worker.invoke(invocation)?;
        Ok(completion)
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
    ) -> Result<Receiver<Result<RuntimeViewCompletion, String>>, SupervisorError> {
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
        let (completion, receiver) = mpsc::channel();
        worker.view_event(ExtensionViewRequest {
            completion,
            request_id,
            generation,
            view_id: view_id.into(),
            revision,
            kind: ExtensionViewRequestKind::Event(event),
        })?;
        Ok(receiver)
    }

    pub(crate) fn close_view(
        &self,
        extension_id: &str,
        generation: u64,
        view_id: impl Into<String>,
        revision: u64,
    ) -> Result<Receiver<Result<RuntimeViewCompletion, String>>, SupervisorError> {
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
        let (completion, receiver) = mpsc::channel();
        worker.view_event(ExtensionViewRequest {
            completion,
            request_id,
            generation,
            view_id: view_id.into(),
            revision,
            kind: ExtensionViewRequestKind::Close,
        })?;
        Ok(receiver)
    }

    pub(crate) fn take_configurations(&self) -> Vec<ExtensionConfigurationResult> {
        self.workers
            .iter()
            .flat_map(ExtensionSearchWorker::take_configurations)
            .collect()
    }

    pub(crate) fn take_invocation_outputs(&self) -> Vec<ExtensionInvocationOutput> {
        self.workers
            .iter()
            .flat_map(ExtensionSearchWorker::take_invocation_outputs)
            .collect()
    }

    pub(crate) fn apply_configuration(
        &self,
        extension_id: &str,
        request_id: impl Into<String>,
        configuration: nanika_protocol::ExtensionConfiguration,
        completion: Option<mpsc::SyncSender<Result<(), String>>>,
    ) -> Result<bool, SupervisorError> {
        let Some(worker) = self
            .workers
            .iter()
            .find(|worker| worker.extension_id() == extension_id)
        else {
            return Ok(false);
        };
        if !worker.supports_live_configuration() {
            return Ok(false);
        }
        match worker.apply_configuration(request_id.into(), configuration, completion) {
            Ok(()) => Ok(true),
            Err(SupervisorError::ChannelClosed) => Ok(false),
            Err(error) => Err(error),
        }
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

    pub fn request_shutdown(&self) {
        for worker in &self.workers {
            worker.request_stop();
        }
    }

    pub fn shutdown(&self) {
        self.request_shutdown();
        for worker in &self.workers {
            worker.join();
        }
        self.host_services
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
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
