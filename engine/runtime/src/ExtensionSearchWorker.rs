use std::io;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;

use nanika_extension_package::ExtensionContributions;
use nanika_search::SearchHandle;

use crate::{
    DiagnosticCode, ExtensionConfigurationUpdate, ExtensionInterruption, ExtensionInvocation,
    ExtensionInvocationOutcome, ExtensionInvocationOutputState, ExtensionNotifier,
    ExtensionRefresh, ExtensionRuntime, ExtensionRuntimeInvocation, ExtensionSearchQuery,
    ExtensionSearchState, ExtensionSearchWorkerContext, ExtensionViewRequest,
    ExtensionViewRequestKind, ExtensionWork, HostDiagnostic, LifecycleFailure, OperationError,
    OperationErrorCode as Code, RuntimeViewCompletion, SupervisorError
};

/// Fixed worker that keeps extension protocol I/O off the UI thread.
pub(crate) struct ExtensionSearchWorker {
    extension_id: String,
    pub(crate) instance: Arc<crate::ExtensionInstance>,
    search: nanika_search::SearchContributor,
    static_catalog: bool,
    state: Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    last_error: Arc<Mutex<Option<HostDiagnostic>>>,
    live_configuration: bool,
    root_search: bool,
    thread: Mutex<Option<JoinHandle<()>>>
}

impl ExtensionSearchWorker {
    pub(crate) fn spawn(
        extension_id: impl Into<String>,
        source: crate::ExtensionRuntimeSource,
        search: SearchHandle,
        contributions: ExtensionContributions,
        configuration: nanika_protocol::ExtensionConfiguration,
        context: ExtensionSearchWorkerContext
    ) -> io::Result<Self> {
        let extension_id = extension_id.into();
        let instance = Arc::new(crate::ExtensionInstance::new());
        let worker_instance = Arc::clone(&instance);
        let factory_instance = Arc::clone(&instance);
        let worker_extension_id = extension_id.clone();
        let contributions = Arc::new(contributions);
        let worker_contributions = Arc::clone(&contributions);
        let resources = source.localization.clone();
        let process_resources = resources.clone();
        let static_candidates = Arc::new(
            crate::search_candidates(
                &extension_id,
                contribution_candidates(&contributions),
                resources.as_deref()
            )
            .map_err(io::Error::other)?
        );
        let worker_static_candidates = Arc::clone(&static_candidates);
        let search = search
            .register_extension(&extension_id, instance.id)
            .map_err(io::Error::other)?;
        let retained_search = search.clone();
        let deferred = source.is_deferred();
        let live_configuration = source.supports_live_configuration();
        let catalog_mode = contributions
            .root_search
            .is_some_and(|root| root.mode == nanika_extension_package::RootSearchMode::Catalog);
        let static_catalog = contributions.root_search.is_none() || catalog_mode;
        if static_catalog {
            retained_search
                .register_static_catalog(static_candidates.as_ref().clone())
                .map_err(io::Error::other)?;
        }
        let root_search = contributions.root_search.is_some();
        let notifier = Arc::clone(&context.notifier);
        let view_invalidations = Arc::clone(&context.view_invalidations);
        let state = Arc::new((
            Mutex::new(ExtensionSearchState {
                catalog_pending: catalog_mode,
                lifecycle: if deferred {
                    crate::RuntimeExtensionState::Dormant
                } else {
                    crate::RuntimeExtensionState::Starting
                },
                ..Default::default()
            }),
            Condvar::new()
        ));
        let worker_state = Arc::clone(&state);
        let factory_extension_id = extension_id.clone();
        let factory_state = Arc::clone(&state);
        let factory_notifier = Arc::clone(&notifier);
        let factory = move |configuration: nanika_protocol::ExtensionConfiguration| {
            factory_state
                .0
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .lifecycle = crate::RuntimeExtensionState::Starting;
            notify(&factory_notifier);
            let mut runtime = source
                .start(configuration.clone())
                .map_err(|error| LifecycleFailure::from(SupervisorError::Io(error)))?;
            runtime.set_localization(process_resources);
            let extension_id = factory_extension_id;
            let state = factory_state;
            let notifier = factory_notifier;
            let exit_state = Arc::clone(&state);
            let exit_notifier = Arc::clone(&notifier);
            runtime.observe_exit(Arc::new(move |error| {
                let mut state = exit_state
                    .0
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                // An explicit stop owns its outcome; an idle disconnect is a lifecycle failure.
                if state.closed || state.shutdown.load(Ordering::Acquire) {
                    return;
                }
                state.lifecycle = crate::RuntimeExtensionState::Failed;
                state.lifecycle_error =
                    Some(LifecycleFailure::new(Code::ExtensionExited).with_diagnostic(Some(error)));
                drop(state);
                exit_state.1.notify_all();
                notify(&exit_notifier);
            }));
            if let Some(host_services) = context.host_services {
                runtime.set_host_services(
                    extension_id.clone(),
                    Arc::new(crate::InstanceHostServices {
                        instance: Arc::clone(&factory_instance),
                        services: host_services
                    })
                );
            }
            runtime.set_shutdown_signal(Arc::clone(
                &state
                    .0
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .shutdown
            ));
            if root_search {
                let changed = Arc::clone(&state);
                runtime.set_candidate_notifier(Arc::new(move || {
                    let (lock, ready) = &*changed;
                    let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
                    // Coalesce invalidations into one current query. User actions retain priority.
                    if !state.closed
                        && !state.shutdown.load(Ordering::Acquire)
                        && state.query.is_none()
                    {
                        if catalog_mode {
                            state.catalog_pending = true;
                        } else {
                            state.query = state.latest_query.clone();
                        }
                        ready.notify_one();
                    }
                }));
            }
            let invalidation_extension_id = extension_id.clone();
            let invalidation_queue = Arc::clone(&view_invalidations);
            let invalidation_notifier = Arc::clone(&notifier);
            let invalidation_instance = Arc::clone(&factory_instance);
            runtime.set_view_invalidation_notifier(Arc::new(move |text| {
                invalidation_instance.with_active(|| {
                    queue_view_invalidation(
                        &invalidation_queue,
                        &invalidation_extension_id,
                        invalidation_instance.id,
                        text
                    );
                    notify(&invalidation_notifier);
                });
            }));
            runtime
                .initialize_with_configuration(format!("initialize-{extension_id}"), configuration)
                .map_err(|error| {
                    LifecycleFailure::from(error).with_diagnostic(runtime.failure_details())
                })?;
            let mut state = state.0.lock().unwrap_or_else(|error| error.into_inner());
            if !state.closed && state.lifecycle == crate::RuntimeExtensionState::Starting {
                state.lifecycle = crate::RuntimeExtensionState::Ready;
            }
            Ok(runtime)
        };
        let last_error = Arc::new(Mutex::new(None));
        let worker_error = Arc::clone(&last_error);
        // Accepted output belongs to the coordinator and survives worker retirement.
        let worker_invocation_output = Arc::clone(&context.invocation_output);
        let thread = std::thread::Builder::new()
            .name(format!("nanika-search-extension-{extension_id}"))
            .spawn(move || {
                let _lifetime = crate::ExtensionWorkerLifetime {
                    state: Arc::clone(&worker_state),
                    notifier: Arc::clone(&notifier),
                };
                let mut catalog_transfer = crate::CatalogTransfer::default();
                let mut catalog_request = 0_u64;
                let mut runtime = None;
                let mut factory = Some(factory);
                let mut configuration = configuration;
                let mut activation_error = None;
                if !deferred && let Err(error) = activate_runtime(&mut runtime, &mut factory, &configuration, &mut activation_error) {
                    set_lifecycle_failure(&worker_state, error.clone());
                    set_error(&worker_error, Some(extension_failure(&worker_extension_id, "initialize extension", "The extension could not start.", error)));
                    worker_state.0.lock().unwrap_or_else(|error| error.into_inner()).stop_result = Some(Ok(()));
                    notify(&notifier);
                    return;
                }
                notify(&notifier);
                loop {
                    let work = next_work(&worker_state);
                    let Some(work) = work else {
                        break;
                    };
                    // Ignore idle visibility hints; defer configuration that does not require a live process.
                    if runtime.is_none() {
                        match work {
                            ExtensionWork::PrepareEntries { .. } => continue,
                            ExtensionWork::ApplyConfiguration(update) if !update.require_live => {
                                configuration = update.configuration.clone();
                                update.complete(Ok(crate::ConfigurationApplication::Deferred));
                                worker_state.0.lock().unwrap_or_else(|error| error.into_inner()).configuration_pending = false;
                                notify(&notifier);
                                continue;
                            }
                            _ => {}
                        }
                    }
                    let runtime = match activate_runtime(&mut runtime, &mut factory, &configuration, &mut activation_error) {
                        Ok(runtime) => runtime,
                        Err(error) => {
                            match work {
                                ExtensionWork::Invoke(invocation) => {
                                    let mut state = worker_state.0.lock().unwrap_or_else(|error| error.into_inner());
                                    state.active_invocation_id = None;
                                    state.cancelled_invocations.remove(&invocation.invocation_id);
                                    drop(state);
                                    let _ = invocation.response.send(Err(error.error.clone()));
                                }
                                ExtensionWork::ViewEvent(request) => { let _ = request.completion.send(Err(error.error.clone())); }
                                ExtensionWork::TextRead(request) => { let _ = request.completion.send(Err(error.error.clone())); }
                                ExtensionWork::Refresh(refresh) => {
                                    let _ = refresh.completion.send(Err(error.error.clone()));
                                    worker_state.0.lock().unwrap_or_else(|error| error.into_inner()).refresh_pending = false;
                                }
                                ExtensionWork::ApplyConfiguration(update) => {
                                    update.complete(Err(error.error.clone()));
                                    worker_state.0.lock().unwrap_or_else(|error| error.into_inner()).configuration_pending = false;
                                }
                                _ => {}
                            }
                            set_lifecycle_failure(&worker_state, error.clone());
                            set_error(&worker_error, Some(extension_failure(&worker_extension_id, "activate extension", "The extension could not start.", error)));
                            notify(&notifier);
                            continue;
                        }
                    };
                    let result = match work {
                        ExtensionWork::Catalog => {
                            catalog_request += 1;
                            let result = runtime.read_catalog(format!("catalog-{worker_extension_id}-{catalog_request}"))
                                .and_then(|batch| catalog_transfer.accept(batch));
                            match result {
                                Ok(true) => {
                                    // The search owner serializes publication with retirement. Peer I/O
                                    // must never hold an instance or query-admission barrier.
                                    match catalog_transfer.commit(&search, &worker_static_candidates, resources.as_deref()) {
                                        Ok(Some(transaction)) => runtime.acknowledge_catalog(transaction).map(|()| true),
                                        Ok(None) => Ok(false),
                                        Err(error) => Err(error),
                                    }
                                },
                                Ok(false) => {
                                    worker_state.0.lock().unwrap_or_else(|error| error.into_inner()).catalog_pending = true;
                                    Ok(false)
                                }
                                Err(error) => Err(error),
                            }
                        }
                        ExtensionWork::Query(query) => {
                            let generation = query.generation;
                            let result = run_query(
                                runtime,
                                query,
                                &search,
                                &worker_state,
                                &worker_contributions,
                                &worker_static_candidates,
                                resources.as_deref(),
                            );
                            match result {
                                Ok(completed) => {
                                    if !completed {
                                        search.set_extension_query_pending(generation, false).map_err(|error| SupervisorError::UnexpectedMessage(error.to_string()))
                                            .map(|()| false)
                                    } else { Ok(true) }
                                },
                                Err(query_error) => {
                                    // Publish terminal query failure without leaving the contributor pending.
                                    match search.publish_extension_snapshot(
                                        generation, worker_static_candidates.as_ref().clone(), true
                                    ) {
                                        Ok(()) => Err(query_error),
                                        Err(publish_error) => {
                                            Err(SupervisorError::UnexpectedMessage(format!(
                                                "extension query failed: {query_error}; publishing its explicit failure state also failed: {publish_error}"
                                            )))
                                        }
                                    }
                                }
                            }
                        }
                        ExtensionWork::PrepareEntries {
                            generation,
                            entry_ids,
                        } => runtime
                            .prepare_entries(generation, entry_ids)
                            .map(|()| false),
                        ExtensionWork::Invoke(invocation) => {
                            let result = run_invocation(
                                runtime,
                                (&worker_extension_id, worker_instance.id),
                                &invocation,
                                &worker_state,
                                &worker_invocation_output,
                                &notifier,
                            );
                            let response = result
                                .as_ref()
                                .map(Clone::clone)
                                .map_err(crate::OperationError::from);
                            if invocation.response.send(response).is_err() {
                                tracing::warn!(
                                    extension_id = worker_extension_id,
                                    invocation_id = invocation.invocation_id,
                                    "invocation receiver closed before completion"
                                );
                            }
                            result.map(|outcome| {
                                matches!(outcome, ExtensionInvocationOutcome::Completed { .. })
                            })
                        }
                        ExtensionWork::ViewEvent(request) => {
                            let completion = request.completion.clone();
                            let result = run_view_event(
                                runtime,
                                &worker_extension_id,
                                request,
                                &worker_state,
                            );
                            if completion
                                .send(result.as_ref().cloned().map_err(crate::OperationError::from))
                                .is_err()
                            {
                                tracing::error!(
                                    extension_id = worker_extension_id,
                                    "view receiver closed before completion"
                                );
                            }
                            result.map(|_| true)
                        }
                        ExtensionWork::TextRead(request) => {
                            runtime.start_text_read(
                                format!("text-read-{worker_extension_id}-{}", request.request_id),
                                request.generation,
                                request.view_id,
                                request.read,
                                Box::new(move |result| { let _ = request.completion.send(result.map_err(crate::OperationError::from)); }),
                            );
                            Ok(false)
                        }
                        ExtensionWork::Refresh(refresh) => {
                            run_refresh(runtime, &worker_extension_id, refresh,
                                &worker_state, &worker_error, &notifier);
                            Ok(false)
                        }
                        ExtensionWork::ApplyConfiguration(update) => {
                            run_configuration_update(runtime, update, &worker_extension_id,
                                &worker_state, &worker_error, &notifier);
                            Ok(false)
                        }
                    };
                    match result {
                        Ok(true) => set_error(&worker_error, None),
                        Ok(false) => {}
                        Err(error) => set_error(
                            &worker_error,
                            Some(extension_failure(
                                &worker_extension_id,
                                "run extension work",
                                "The extension operation failed.",
                                error,
                            )),
                        ),
                    }
                    notify(&notifier);
                }
                if let Some(runtime) = runtime.as_mut() {
                    let force = worker_state.0.lock().unwrap_or_else(|error| error.into_inner()).shutdown.load(Ordering::Acquire);
                    let result = if force { runtime.terminate().map_err(SupervisorError::Io) }
                        else { runtime.shutdown() };
                    let result = result.map_err(|error| LifecycleFailure::from(error).with_diagnostic(runtime.failure_details()));
                    let failed = result.is_err();
                    let detail = runtime.failure_details();
                    let mut state = worker_state.0.lock().unwrap_or_else(|error| error.into_inner());
                    if let Err(error) = &result {
                        state.lifecycle = crate::RuntimeExtensionState::Failed;
                        state.lifecycle_error = Some(error.clone());
                    }
                    if detail.is_some() && let Some(error) = state.lifecycle_error.take() {
                        state.lifecycle_error = Some(error.with_diagnostic(detail));
                    }
                    state.stop_result = Some(result);
                    worker_state.1.notify_all();
                    // A failed graceful stop retains its process and containment until explicit app termination.
                    while failed && !state.shutdown.load(Ordering::Acquire) {
                        state = worker_state.1.wait(state).unwrap_or_else(|error| error.into_inner());
                    }
                } else {
                    worker_state.0.lock().unwrap_or_else(|error| error.into_inner()).stop_result = Some(Ok(()));
                    worker_state.1.notify_all();
                }
            }).inspect_err(|_| {
                if let Err(cleanup) = retained_search.retire() {
                    tracing::error!(%extension_id, %cleanup, "failed to withdraw catalog after worker spawn failure");
                }
            })?;
        let worker = Self {
            extension_id,
            instance,
            search: retained_search,
            static_catalog,
            state,
            last_error,
            live_configuration,
            root_search: contributions.root_search.is_some(),
            thread: Mutex::new(Some(thread))
        };
        Ok(worker)
    }

    pub(crate) fn lifecycle(&self) -> (crate::RuntimeExtensionState, Option<LifecycleFailure>) {
        let state = self
            .state
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        (state.lifecycle, state.lifecycle_error.clone())
    }

    pub(crate) fn retire(&self) -> Result<(), LifecycleFailure> {
        self._retire(None)
    }

    pub(crate) fn retire_after_failure(
        &self,
        error: &LifecycleFailure
    ) -> Result<(), LifecycleFailure> {
        self._retire(Some(error))
    }

    pub(crate) fn wait_stopped(&self) -> Result<(), LifecycleFailure> {
        let (lock, changed) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        loop {
            if let Some(result) = &state.stop_result {
                return result.clone();
            }
            if state.finished {
                return Err(state
                    .lifecycle_error
                    .clone()
                    .unwrap_or_else(|| LifecycleFailure::new(Code::ExtensionStopUnconfirmed)));
            }
            state = changed
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
    }

    pub fn query(&self, generation: u64, query: impl Into<String>) {
        if self.static_catalog {
            return;
        }
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        let query = ExtensionSearchQuery {
            generation,
            query: query.into()
        };
        if state.closed
            || state
                .latest_query
                .as_ref()
                .is_some_and(|current| current.generation > generation)
        {
            return;
        }
        state.latest_query = Some(query.clone());
        state.query = Some(query);
        ready.notify_one();
    }

    pub(crate) fn prepare_entries(&self, generation: u64, entry_ids: Vec<String>) {
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        state.entry_preparation = Some((generation, entry_ids));
        ready.notify_one();
    }

    pub(crate) fn refresh(&self, refresh: ExtensionRefresh) -> Result<(), OperationError> {
        let (lock, ready) = &*self.state;
        let mut state = wait_for_capacity(lock, ready)?;
        state.refreshes.push_back(refresh);
        ready.notify_one();
        Ok(())
    }

    pub(crate) fn contributes_root_search(&self) -> bool {
        self.root_search
    }

    pub(crate) fn invoke(&self, invocation: ExtensionInvocation) -> Result<(), OperationError> {
        let (lock, ready) = &*self.state;
        let mut state = wait_for_capacity(lock, ready)?;
        state.invocations.push_back(invocation);
        ready.notify_one();
        Ok(())
    }

    pub(crate) fn view_event(&self, request: ExtensionViewRequest) -> Result<(), OperationError> {
        let (lock, ready) = &*self.state;
        let mut state = wait_for_capacity(lock, ready)?;
        state.view_events.push_back(request);
        ready.notify_one();
        Ok(())
    }

    pub(crate) fn read_text(
        &self,
        request: crate::ExtensionTextRead
    ) -> Result<(), OperationError> {
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        if state.closed || state.shutdown.load(Ordering::Acquire) {
            return Err(OperationError::new(Code::ExtensionUnavailable));
        }
        let previous = state.text_read.replace(request);
        ready.notify_one();
        drop(state);
        if let Some(previous) = previous {
            let _ = previous.completion.send(Err(crate::OperationError::new(
                crate::OperationErrorCode::Cancelled
            )));
        }
        Ok(())
    }

    pub(crate) fn cancel_invocation(&self, invocation_id: u64) -> bool {
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        if state.active_invocation_id != Some(invocation_id)
            && !state
                .invocations
                .iter()
                .any(|invocation| invocation.invocation_id == invocation_id)
        {
            return false;
        }
        state.cancelled_invocations.insert(invocation_id);
        ready.notify_one();
        true
    }

    pub(crate) fn apply_configuration(
        &self,
        request_id: String,
        configuration: nanika_protocol::ExtensionConfiguration,
        require_live: bool,
        progress: crate::ConfigurationProgressHandler,
        completion: std::sync::mpsc::SyncSender<
            Result<crate::ConfigurationApplication, crate::OperationError>
        >
    ) -> Result<(), OperationError> {
        let (lock, ready) = &*self.state;
        let mut state = wait_for_capacity(lock, ready)?;
        state
            .configurations
            .push_back(ExtensionConfigurationUpdate {
                request_id,
                configuration,
                require_live,
                progress,
                completion
            });
        ready.notify_one();
        Ok(())
    }

    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub(crate) fn is_query_ready(&self) -> bool {
        let state = self
            .state
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        !state.closed
            && state.lifecycle != crate::RuntimeExtensionState::Failed
            && (self.static_catalog || state.lifecycle == crate::RuntimeExtensionState::Ready)
    }

    pub fn last_error(&self) -> Option<HostDiagnostic> {
        self.last_error
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(crate) fn supports_live_configuration(&self) -> bool {
        self.live_configuration
    }

    fn stop(&mut self) {
        self.request_stop();
        self.join();
    }

    pub(crate) fn request_stop(&self) {
        let (lock, ready) = &*self.state;
        lock.lock()
            .unwrap_or_else(|error| error.into_inner())
            .shutdown
            .store(true, Ordering::Release);
        ready.notify_all();
    }

    pub(crate) fn join(&self) {
        let mut thread = self
            .thread
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(thread) = thread.take()
            && thread.join().is_err()
        {
            tracing::error!(
                extension_id = self.extension_id,
                "extension worker panicked"
            );
        }
    }
    fn _retire(&self, failure: Option<&LifecycleFailure>) -> Result<(), LifecycleFailure> {
        let (lock, changed) = &*self.state;
        let (invocations, views, refreshes, text_read) = {
            let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
            state.closed = true;
            state.lifecycle = crate::RuntimeExtensionState::Stopping;
            state.query = None;
            state.entry_preparation = None;
            (
                state.invocations.drain(..).collect::<Vec<_>>(),
                state.view_events.drain(..).collect::<Vec<_>>(),
                state.refreshes.drain(..).collect::<Vec<_>>(),
                state.text_read.take()
            )
        };
        if let Some(read) = text_read {
            let _ = read.completion.send(Err(crate::OperationError::new(
                crate::OperationErrorCode::ExtensionUnavailable
            )));
        }
        changed.notify_all();
        for invocation in invocations {
            let _ = invocation.response.send(match failure {
                Some(error) => Err(error.error.clone()),
                None => Ok(ExtensionInvocationOutcome::Cancelled)
            });
        }
        for view in views {
            let _ = view.completion.send(Err(failure.map_or_else(
                || OperationError::new(Code::ExtensionUnavailable),
                |failure| failure.error.clone()
            )));
        }
        for refresh in refreshes {
            let _ = refresh.completion.send(Err(failure.map_or_else(
                || OperationError::new(Code::ExtensionUnavailable),
                |failure| failure.error.clone()
            )));
        }
        self.instance.retire();
        self.search.retire().map_err(|error| {
            OperationError::with_detail(Code::CatalogRetirementFailed, error.to_string()).into()
        })
    }
}

pub(crate) fn queue_view_invalidation(
    pending: &Mutex<std::collections::HashMap<String, crate::RuntimeViewInvalidation>>,
    extension_id: &str,
    instance_id: u64,
    text: nanika_protocol::TextInvalidation
) {
    let mut pending = pending.lock().unwrap_or_else(|error| error.into_inner());
    let entry =
        pending
            .entry(extension_id.to_owned())
            .or_insert_with(|| crate::RuntimeViewInvalidation {
                instance_id,
                extension_id: extension_id.to_owned(),
                text: nanika_protocol::TextInvalidation::default()
            });
    if entry.instance_id != instance_id {
        entry.instance_id = instance_id;
        entry.text = nanika_protocol::TextInvalidation::default();
    }
    entry.text.merge(text);
}

impl Drop for ExtensionSearchWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(crate) fn next_work(
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>
) -> Option<ExtensionWork> {
    let (lock, ready) = &**state;
    let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
    while state.query.is_none()
        && !state.catalog_pending
        && (state.refreshes.is_empty() || state.configuration_pending || state.refresh_pending)
        && state.invocations.is_empty()
        && state.view_events.is_empty()
        && (state.configurations.is_empty() || state.configuration_pending || state.refresh_pending)
        && state.entry_preparation.is_none()
        && state.text_read.is_none()
        && !state.shutdown.load(Ordering::Acquire)
        && !state.closed
    {
        state = ready.wait(state).unwrap_or_else(|error| error.into_inner());
    }
    if state.closed || state.shutdown.load(Ordering::Acquire) {
        return None;
    }
    ready.notify_all();
    if let Some(invocation) = state.invocations.pop_front() {
        state.active_invocation_id = Some(invocation.invocation_id);
        return Some(ExtensionWork::Invoke(invocation));
    }
    if let Some(event) = state.view_events.pop_front() {
        return Some(ExtensionWork::ViewEvent(event));
    }
    if !state.configuration_pending && !state.refresh_pending {
        if let Some(update) = state.configurations.pop_front() {
            state.configuration_pending = true;
            return Some(ExtensionWork::ApplyConfiguration(update));
        }
        if let Some(refresh) = state.refreshes.pop_front() {
            state.refresh_pending = true;
            return Some(ExtensionWork::Refresh(refresh));
        }
    }
    if state.catalog_pending {
        state.catalog_pending = false;
        return Some(ExtensionWork::Catalog);
    }
    state
        .query
        .take()
        .map(ExtensionWork::Query)
        .or_else(|| {
            state
                .entry_preparation
                .take()
                .map(|(generation, entry_ids)| ExtensionWork::PrepareEntries {
                    generation,
                    entry_ids
                })
        })
        .or_else(|| state.text_read.take().map(ExtensionWork::TextRead))
}

fn run_view_event(
    runtime: &mut ExtensionRuntime,
    extension_id: &str,
    request: ExtensionViewRequest,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>
) -> Result<RuntimeViewCompletion, SupervisorError> {
    runtime.ensure_running()?;
    let request_id = request.request_id;
    match request.kind {
        ExtensionViewRequestKind::Event(event) => runtime
            .view_event_interruptible(
                format!("view-{extension_id}-{request_id}"),
                request.generation,
                request.view_id,
                request.revision,
                event,
                || {
                    let (lock, _) = &**state;
                    if lock
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .shutdown
                        .load(Ordering::Acquire)
                    {
                        ExtensionInterruption::Terminate
                    } else {
                        ExtensionInterruption::None
                    }
                }
            )
            .map(|(revision, effect, view)| RuntimeViewCompletion {
                revision,
                effect,
                view
            }),
        ExtensionViewRequestKind::Close => runtime
            .close_view(
                format!("close-view-{extension_id}-{request_id}"),
                request.view_id
            )
            .map(|()| RuntimeViewCompletion {
                revision: request.revision,
                effect: nanika_protocol::NavigationEffect::Pop,
                view: None
            })
    }
}

fn run_configuration_update(
    runtime: &mut ExtensionRuntime,
    update: ExtensionConfigurationUpdate,
    extension_id: &str,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    error: &Arc<Mutex<Option<HostDiagnostic>>>,
    notifier: &ExtensionNotifier
) {
    let state = Arc::clone(state);
    let error = Arc::clone(error);
    let notifier = Arc::clone(notifier);
    let extension_id = extension_id.to_owned();
    runtime.start_configuration_update(
        update.request_id.clone(),
        update.configuration.clone(),
        Arc::clone(&update.progress),
        Box::new(move |result| {
            let completion = result
                .as_ref()
                .map_err(crate::OperationError::from)
                .map(|_| crate::ConfigurationApplication::Applied);
            match result {
                Ok(()) => set_error(&error, None),
                Err(cause) => set_error(
                    &error,
                    Some(extension_failure(
                        &extension_id,
                        "apply configuration",
                        "Settings could not be applied.",
                        cause
                    ))
                )
            }
            update.complete(completion);
            state
                .0
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .configuration_pending = false;
            state.1.notify_all();
            notify(&notifier);
        })
    );
}

fn run_refresh(
    runtime: &mut ExtensionRuntime,
    extension_id: &str,
    refresh: ExtensionRefresh,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    error: &Arc<Mutex<Option<HostDiagnostic>>>,
    notifier: &ExtensionNotifier
) {
    let state = Arc::clone(state);
    let error = Arc::clone(error);
    let notifier = Arc::clone(notifier);
    let extension_id = extension_id.to_owned();
    runtime.start_refresh(
        format!("refresh-{extension_id}-{}", refresh.request_id),
        refresh.generation,
        Box::new(move |result| {
            let completion = result
                .as_ref()
                .map(|_| ())
                .map_err(crate::OperationError::from);
            match result {
                Ok(()) => set_error(&error, None),
                Err(cause) => set_error(
                    &error,
                    Some(extension_failure(
                        &extension_id,
                        "refresh catalog",
                        "The catalog could not be refreshed.",
                        cause
                    ))
                )
            }
            state
                .0
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .refresh_pending = false;
            state.1.notify_all();
            let _ = refresh.completion.send(completion);
            notify(&notifier);
        })
    );
}

fn run_query(
    runtime: &mut ExtensionRuntime,
    query: ExtensionSearchQuery,
    search: &nanika_search::SearchContributor,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    contributions: &ExtensionContributions,
    static_candidates: &[nanika_search::Candidate],
    resources: Option<&nanika_localization::ResourceSet>
) -> Result<bool, SupervisorError> {
    runtime.ensure_running()?;
    if contributions.root_search.is_none() {
        search
            .publish_extension_snapshot(query.generation, static_candidates.to_vec(), true)
            .map_err(|error| SupervisorError::UnexpectedMessage(error.to_string()))?;
        return Ok(true);
    }
    search
        .set_extension_query_pending(query.generation, true)
        .map_err(|error| SupervisorError::UnexpectedMessage(error.to_string()))?;
    let extension_id = search.extension_id();
    let manifest_ids = static_candidates
        .iter()
        .map(|entry| entry.entry_id())
        .collect::<std::collections::HashSet<_>>();
    runtime.query_incremental(
        format!("search-{extension_id}-{}", query.generation),
        query.generation,
        query.query.clone(),
        |mut update| {
            update
                .removed
                .retain(|id| !manifest_ids.contains(id.as_str()));
            update
                .entries
                .retain(|entry| !manifest_ids.contains(entry.entry_id.as_str()));
            let mut entries = crate::search_candidates(extension_id, update.entries, resources)
                .map_err(SupervisorError::UnexpectedMessage)?;
            if update.replace {
                // Manifest search data was prepared once at instance admission, independently of queries.
                entries.extend(static_candidates.iter().cloned());
                search.publish_extension_snapshot(query.generation, entries, update.complete)
            } else {
                search.publish_extension_delta(
                    query.generation,
                    entries,
                    update.removed,
                    update.complete
                )
            }
            .map_err(|error| SupervisorError::UnexpectedMessage(error.to_string()))
        },
        || {
            let (lock, _) = &**state;
            let state = lock.lock().unwrap_or_else(|error| error.into_inner());
            state.shutdown.load(Ordering::Acquire)
                || state.closed
                || state.query.is_some()
                || (!state.configuration_pending
                    && !state.refresh_pending
                    && !state.refreshes.is_empty())
                || !state.invocations.is_empty()
                || !state.view_events.is_empty()
        }
    )
}

pub(crate) fn contribution_candidates(
    contributions: &ExtensionContributions
) -> Vec<nanika_protocol::Candidate> {
    let commands = contributions.commands.iter().map(|command| {
        let mut aliases = command.keywords.clone();
        aliases.push(command.description.clone());
        if let Some(category) = &command.category {
            aliases.push(category.clone());
        }
        nanika_protocol::Candidate {
            kind: nanika_protocol::CandidateKind::Action,
            entry_id: command.command.clone(),
            title: command.title.clone(),
            subtitle: command
                .category
                .clone()
                .or_else(|| Some("Command".into()))
                .map(nanika_protocol::CandidateSubtitle::Label),
            action_id: nanika_protocol::COMMAND_EXECUTE_ACTION_ID.to_owned(),
            actions: vec![command.action.clone()],
            aliases,
            icon: command
                .icon
                .as_ref()
                .map(|path| nanika_protocol::IconSource::Package { path: path.clone() })
        }
    });
    let views = contributions.views.iter().map(|view| {
        let mut aliases = view.keywords.clone();
        aliases.push(view.description.clone());
        if let Some(category) = &view.category {
            aliases.push(category.clone());
        }
        nanika_protocol::Candidate {
            kind: nanika_protocol::CandidateKind::View,
            entry_id: view.id.clone(),
            title: view.title.clone(),
            subtitle: view
                .category
                .clone()
                .or_else(|| Some("View".into()))
                .map(nanika_protocol::CandidateSubtitle::Label),
            action_id: nanika_protocol::VIEW_OPEN_ACTION_ID.to_owned(),
            actions: vec![nanika_protocol::Action::primary(
                nanika_protocol::VIEW_OPEN_ACTION_ID.to_owned(),
                "Open"
            )],
            aliases,
            icon: view
                .icon
                .as_ref()
                .map(|path| nanika_protocol::IconSource::Package { path: path.clone() })
        }
    });
    commands.chain(views).collect()
}

fn run_invocation(
    runtime: &mut ExtensionRuntime,
    origin: (&str, u64),
    invocation: &ExtensionInvocation,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    invocation_output: &Arc<Mutex<ExtensionInvocationOutputState>>,
    notifier: &ExtensionNotifier
) -> Result<ExtensionInvocationOutcome, SupervisorError> {
    let (extension_id, instance_id) = origin;
    {
        let mut pending = state.0.lock().unwrap_or_else(|error| error.into_inner());
        if pending
            .cancelled_invocations
            .remove(&invocation.invocation_id)
        {
            pending.active_invocation_id = None;
            return Ok(ExtensionInvocationOutcome::Cancelled);
        }
    }
    runtime.ensure_running()?;
    let output_state = Arc::clone(invocation_output);
    let output_notifier = Arc::clone(notifier);
    let output_extension_id = extension_id.to_owned();
    let output_generation = invocation.generation;
    let output_invocation_id = invocation.invocation_id;
    let publish = Arc::new(move |chunk: String| {
        let should_notify = output_state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .append(
                instance_id,
                output_invocation_id,
                &output_extension_id,
                output_generation,
                &chunk
            );
        if should_notify {
            notify(&output_notifier);
        }
    });
    let result = runtime.invoke_interruptible(
        ExtensionRuntimeInvocation::new(
            format!("invoke-{extension_id}-{}", invocation.invocation_id),
            invocation.generation,
            invocation.entry_id.clone(),
            invocation.action_id.clone(),
            invocation.query_context.clone()
        ),
        publish,
        || {
            let (lock, _) = &**state;
            let state = lock.lock().unwrap_or_else(|error| error.into_inner());
            if state.shutdown.load(Ordering::Acquire) {
                ExtensionInterruption::Terminate
            } else if state
                .cancelled_invocations
                .contains(&invocation.invocation_id)
            {
                ExtensionInterruption::Cancel
            } else {
                ExtensionInterruption::None
            }
        }
    );
    let (lock, _) = &**state;
    let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
    state
        .cancelled_invocations
        .remove(&invocation.invocation_id);
    state.active_invocation_id = None;
    drop(state);
    if matches!(&result, Err(SupervisorError::Cancelled(_))) {
        return Ok(ExtensionInvocationOutcome::Cancelled);
    }
    result.map(|(effect, has_output)| ExtensionInvocationOutcome::Completed { effect, has_output })
}

fn set_error(error: &Mutex<Option<HostDiagnostic>>, value: Option<HostDiagnostic>) {
    if let Some(diagnostic) = value.as_ref() {
        diagnostic.record_warning();
    }
    *error.lock().unwrap_or_else(|error| error.into_inner()) = value;
}

fn extension_failure(
    extension_id: &str,
    operation: &'static str,
    user_message: &'static str,
    source: impl std::error::Error + Send + Sync + 'static
) -> HostDiagnostic {
    HostDiagnostic::from_error(
        DiagnosticCode::ExtensionUnavailable,
        operation,
        format!("{user_message} Extension: {extension_id}."),
        source
    )
    .with_safe_context(extension_id)
}

fn notify(notifier: &ExtensionNotifier) {
    let notify = notifier
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    if let Some(notify) = notify {
        notify();
    }
}

// Bound admission without expiring or dropping accepted work.
// Release the lock while waiting so cancellation and shutdown can proceed.
const PENDING_WORK_CAPACITY: usize = 16;

fn wait_for_capacity<'a>(
    lock: &'a Mutex<ExtensionSearchState>,
    changed: &Condvar
) -> Result<MutexGuard<'a, ExtensionSearchState>, OperationError> {
    let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
    loop {
        if state.lifecycle == crate::RuntimeExtensionState::Failed {
            return Err(state.lifecycle_error.as_ref().map_or_else(
                || OperationError::new(Code::ExtensionUnavailable),
                |failure| failure.error.clone()
            ));
        }
        if state.closed || state.shutdown.load(Ordering::Acquire) {
            return Err(OperationError::new(Code::ExtensionUnavailable));
        }
        if state.invocations.len()
            + state.view_events.len()
            + state.configurations.len()
            + state.refreshes.len()
            < PENDING_WORK_CAPACITY
        {
            return Ok(state);
        }
        state = changed
            .wait(state)
            .unwrap_or_else(|error| error.into_inner());
    }
}

fn activate_runtime<'a, F>(
    runtime: &'a mut Option<ExtensionRuntime>,
    factory: &mut Option<F>,
    configuration: &nanika_protocol::ExtensionConfiguration,
    failure: &mut Option<LifecycleFailure>
) -> Result<&'a mut ExtensionRuntime, LifecycleFailure>
where
    F: FnOnce(
        nanika_protocol::ExtensionConfiguration
    ) -> Result<ExtensionRuntime, LifecycleFailure>
{
    if runtime.is_none() {
        if let Some(error) = failure {
            return Err(error.clone());
        }
        match factory.take().expect("activation is attempted only once")(configuration.clone()) {
            Ok(started) => *runtime = Some(started),
            Err(error) => {
                *failure = Some(error.clone());
                return Err(error);
            }
        }
    }
    Ok(runtime.as_mut().expect("activated runtime"))
}

fn set_lifecycle_failure(
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    error: LifecycleFailure
) {
    let mut state = state.0.lock().unwrap_or_else(|error| error.into_inner());
    state.lifecycle = crate::RuntimeExtensionState::Failed;
    state.lifecycle_error = Some(error);
}
