use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;

use nanika_extension_package::ExtensionContributions;
use nanika_search::SearchHandle;

use crate::{
    DiagnosticCode, ExtensionConfigurationResult, ExtensionConfigurationUpdate,
    ExtensionInterruption, ExtensionInvocation, ExtensionInvocationOutcome,
    ExtensionInvocationOutput, ExtensionInvocationOutputState, ExtensionNotifier, ExtensionRefresh,
    ExtensionRuntime, ExtensionRuntimeInvocation, ExtensionSearchQuery, ExtensionSearchState,
    ExtensionSearchWorkerContext, ExtensionViewRequest, ExtensionViewRequestKind, ExtensionWork,
    HostDiagnostic, RuntimeViewCompletion, SupervisorError, publish_extension_snapshot,
};

/// Fixed worker that keeps extension protocol I/O off the UI thread.
pub(crate) struct ExtensionSearchWorker {
    extension_id: String,
    static_catalog: bool,
    state: Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    last_error: Arc<Mutex<Option<HostDiagnostic>>>,
    invocation_output: Arc<Mutex<ExtensionInvocationOutputState>>,
    configuration_results: Arc<Mutex<VecDeque<ExtensionConfigurationResult>>>,
    live_configuration: bool,
    root_search: bool,
    query_ready: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl ExtensionSearchWorker {
    pub(crate) fn spawn(
        extension_id: impl Into<String>,
        source: crate::ExtensionRuntimeSource,
        search: SearchHandle,
        contributions: ExtensionContributions,
        configuration: nanika_protocol::ExtensionConfiguration,
        context: ExtensionSearchWorkerContext,
    ) -> io::Result<Self> {
        let extension_id = extension_id.into();
        let worker_extension_id = extension_id.clone();
        let contributions = Arc::new(contributions);
        let worker_contributions = Arc::clone(&contributions);
        let deferred = source.is_deferred();
        let live_configuration = source.supports_live_configuration();
        let static_catalog = contributions.root_search.is_none();
        if static_catalog {
            search
                .register_static_catalog(
                    &extension_id,
                    crate::search_candidates(
                        &extension_id,
                        contribution_candidates(&contributions),
                    ),
                )
                .map_err(io::Error::other)?;
        }
        let root_search = contributions.root_search.is_some();
        let notifier = Arc::clone(&context.notifier);
        let view_invalidations = Arc::clone(&context.view_invalidations);
        let state = Arc::new((Mutex::new(ExtensionSearchState::default()), Condvar::new()));
        let worker_state = Arc::clone(&state);
        let factory_extension_id = extension_id.clone();
        let factory_state = Arc::clone(&state);
        let factory_notifier = Arc::clone(&notifier);
        let factory = move |configuration: nanika_protocol::ExtensionConfiguration| {
            let mut runtime = source
                .start()
                .map_err(|error| SupervisorError::UnexpectedMessage(error.to_string()))?;
            let extension_id = factory_extension_id;
            let state = factory_state;
            let notifier = factory_notifier;
            if let Some(host_services) = context.host_services {
                runtime.set_host_services(extension_id.clone(), host_services);
            }
            runtime.set_shutdown_signal(Arc::clone(
                &state
                    .0
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .shutdown,
            ));
            if root_search {
                let changed = Arc::clone(&state);
                runtime.set_candidate_notifier(Arc::new(move || {
                    let (lock, ready) = &*changed;
                    let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
                    // Coalesce invalidations into one current query. User actions retain priority.
                    if !state.shutdown.load(Ordering::Acquire) && state.query.is_none() {
                        state.query = state.latest_query.clone();
                        ready.notify_one();
                    }
                }));
            }
            let invalidation_extension_id = extension_id.clone();
            let invalidation_queue = Arc::clone(&view_invalidations);
            let invalidation_notifier = Arc::clone(&notifier);
            runtime.set_view_invalidation_notifier(Arc::new(move |view_id| {
                queue_view_invalidation(&invalidation_queue, &invalidation_extension_id, view_id);
                notify(&invalidation_notifier);
            }));
            runtime.initialize_with_configuration(
                format!("initialize-{extension_id}"),
                configuration,
            )?;
            Ok(runtime)
        };
        let last_error = Arc::new(Mutex::new(None));
        let worker_error = Arc::clone(&last_error);
        let invocation_output = Arc::new(Mutex::new(ExtensionInvocationOutputState::default()));
        let worker_invocation_output = Arc::clone(&invocation_output);
        let configuration_results = Arc::new(Mutex::new(VecDeque::new()));
        let worker_configuration_results = Arc::clone(&configuration_results);
        let query_ready = Arc::new(AtomicBool::new(false));
        let worker_query_ready = Arc::clone(&query_ready);
        let thread = std::thread::Builder::new()
            .name(format!("nanika-search-extension-{extension_id}"))
            .spawn(move || {
                let _lifetime = crate::ExtensionWorkerLifetime {
                    extension_id: worker_extension_id.clone(),
                    state: Arc::clone(&worker_state),
                    configuration_results: Arc::clone(&worker_configuration_results),
                    notifier: Arc::clone(&notifier),
                };
                let mut runtime = None;
                let mut factory = Some(factory);
                let mut configuration = configuration;
                let mut activation_error = None;
                if !deferred && let Err(error) = activate_runtime(&mut runtime, &mut factory, &configuration, &mut activation_error) {
                    set_error(&worker_error, Some(extension_failure(&worker_extension_id, "initialize extension", "The extension could not start.", error)));
                    notify(&notifier);
                    return;
                }
                worker_query_ready.store(true, Ordering::Release);
                notify(&notifier);
                loop {
                    let work = next_work(&worker_state);
                    let Some(work) = work else {
                        break;
                    };
                    // Visibility hints never activate an idle process. Configuration
                    // is retained for first initialization without starting it.
                    if runtime.is_none() {
                        match work {
                            ExtensionWork::PrepareEntries { .. } => continue,
                            ExtensionWork::ApplyConfiguration(update) => {
                                configuration = update.configuration.clone();
                                update.complete(&worker_extension_id, &worker_configuration_results, Ok(()));
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
                            let message = error.to_string();
                            match work {
                                ExtensionWork::Invoke(invocation) => {
                                    let mut state = worker_state.0.lock().unwrap_or_else(|error| error.into_inner());
                                    state.active_invocation_id = None;
                                    state.cancelled_invocations.remove(&invocation.invocation_id);
                                    drop(state);
                                    let _ = invocation.response.send(Err(message));
                                }
                                ExtensionWork::ViewEvent(request) => { let _ = request.completion.send(Err(message)); }
                                ExtensionWork::Refresh(refresh) => { let _ = refresh.completion.send(Err(message)); }
                                ExtensionWork::ApplyConfiguration(update) => {
                                    update.complete(&worker_extension_id, &worker_configuration_results, Err(message));
                                    worker_state.0.lock().unwrap_or_else(|error| error.into_inner()).configuration_pending = false;
                                }
                                _ => {}
                            }
                            set_error(&worker_error, Some(extension_failure(&worker_extension_id, "activate extension", "The extension could not start.", error)));
                            notify(&notifier);
                            continue;
                        }
                    };
                    let result = match work {
                        ExtensionWork::Query(query) => {
                            let generation = query.generation;
                            let result = run_query(
                                runtime,
                                &worker_extension_id,
                                query,
                                &search,
                                &worker_state,
                                &worker_contributions,
                            );
                            match result {
                                Ok(completed) => Ok(completed),
                                Err(query_error) => {
                                    // A failed query still completes this worker's barrier slot so
                                    // one unavailable extension cannot leave the launcher pending.
                                    match publish_extension_snapshot(
                                        &search,
                                        &worker_extension_id,
                                        generation,
                                        contribution_candidates(&worker_contributions),
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
                                &worker_extension_id,
                                &invocation,
                                &worker_state,
                                &worker_invocation_output,
                                &notifier,
                            );
                            let response = result
                                .as_ref()
                                .map(Clone::clone)
                                .map_err(ToString::to_string);
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
                                .send(result.as_ref().cloned().map_err(ToString::to_string))
                                .is_err()
                            {
                                tracing::error!(
                                    extension_id = worker_extension_id,
                                    "view receiver closed before completion"
                                );
                            }
                            result.map(|_| true)
                        }
                        ExtensionWork::Refresh(refresh) => {
                            let result = run_refresh(
                                runtime,
                                &worker_extension_id,
                                &refresh,
                                &worker_state,
                            );
                            let completion = match &result {
                                Ok(true) => Ok(()),
                                Ok(false) => Err("Extension refresh was cancelled.".to_owned()),
                                Err(error) => Err(error.to_string()),
                            };
                            let _ = refresh.completion.send(completion);
                            result
                        }
                        ExtensionWork::ApplyConfiguration(update) => {
                            run_configuration_update(runtime, update, &worker_extension_id,
                                &worker_state, &worker_configuration_results, &worker_error, &notifier);
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
                if let Some(runtime) = runtime.as_mut() && let Err(error) = runtime.terminate() {
                    HostDiagnostic::from_error(
                        DiagnosticCode::ExtensionUnavailable,
                        "terminate extension",
                        "An extension process could not be terminated cleanly.",
                        error,
                    )
                    .with_safe_context(&worker_extension_id)
                    .record_warning();
                }
            })?;
        Ok(Self {
            extension_id,
            static_catalog,
            state,
            last_error,
            invocation_output,
            configuration_results,
            live_configuration,
            root_search: contributions.root_search.is_some(),
            query_ready,
            thread: Mutex::new(Some(thread)),
        })
    }

    pub fn query(&self, generation: u64, query: impl Into<String>) {
        if self.static_catalog {
            return;
        }
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        let query = ExtensionSearchQuery {
            generation,
            query: query.into(),
        };
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

    pub(crate) fn refresh(&self, refresh: ExtensionRefresh) -> Result<(), SupervisorError> {
        let (lock, ready) = &*self.state;
        let mut state = wait_for_capacity(lock, ready)?;
        state.refreshes.push_back(refresh);
        ready.notify_one();
        Ok(())
    }

    pub(crate) fn contributes_root_search(&self) -> bool {
        self.root_search
    }

    pub(crate) fn invoke(&self, invocation: ExtensionInvocation) -> Result<(), SupervisorError> {
        let (lock, ready) = &*self.state;
        let mut state = wait_for_capacity(lock, ready)?;
        state.invocations.push_back(invocation);
        ready.notify_one();
        Ok(())
    }

    pub(crate) fn view_event(&self, request: ExtensionViewRequest) -> Result<(), SupervisorError> {
        let (lock, ready) = &*self.state;
        let mut state = wait_for_capacity(lock, ready)?;
        state.view_events.push_back(request);
        ready.notify_one();
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
        completion: Option<std::sync::mpsc::SyncSender<Result<(), String>>>,
    ) -> Result<(), SupervisorError> {
        let (lock, ready) = &*self.state;
        let mut state = wait_for_capacity(lock, ready)?;
        state
            .configurations
            .push_back(ExtensionConfigurationUpdate {
                request_id,
                configuration,
                completion,
            });
        ready.notify_one();
        Ok(())
    }

    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub(crate) fn is_query_ready(&self) -> bool {
        self.static_catalog
            || (self.query_ready.load(Ordering::Acquire)
                && self
                    .thread
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .as_ref()
                    .is_some_and(|thread| !thread.is_finished()))
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

    pub(crate) fn take_configurations(&self) -> Vec<ExtensionConfigurationResult> {
        self.configuration_results
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .drain(..)
            .collect()
    }

    pub(crate) fn take_invocation_outputs(&self) -> Vec<ExtensionInvocationOutput> {
        self.invocation_output
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take_changed()
            .unwrap_or_default()
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
}

pub(crate) fn queue_view_invalidation(
    pending: &Mutex<std::collections::HashMap<String, crate::RuntimeViewInvalidation>>,
    extension_id: &str,
    view_id: String,
) {
    // A worker owns at most one visible route. Replace its pending signal so
    // extension-controlled view IDs cannot grow this queue without bound.
    pending
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(
            extension_id.to_owned(),
            crate::RuntimeViewInvalidation {
                extension_id: extension_id.to_owned(),
                view_id,
            },
        );
}

impl Drop for ExtensionSearchWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(crate) fn next_work(
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
) -> Option<ExtensionWork> {
    let (lock, ready) = &**state;
    let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
    while state.query.is_none()
        && (state.refreshes.is_empty() || state.configuration_pending)
        && state.invocations.is_empty()
        && state.view_events.is_empty()
        && (state.configurations.is_empty() || state.configuration_pending)
        && state.entry_preparation.is_none()
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
    if !state.configuration_pending {
        if let Some(update) = state.configurations.pop_front() {
            state.configuration_pending = true;
            return Some(ExtensionWork::ApplyConfiguration(update));
        }
        if let Some(refresh) = state.refreshes.pop_front() {
            return Some(ExtensionWork::Refresh(refresh));
        }
    }
    state.query.take().map(ExtensionWork::Query).or_else(|| {
        state
            .entry_preparation
            .take()
            .map(|(generation, entry_ids)| ExtensionWork::PrepareEntries {
                generation,
                entry_ids,
            })
    })
}

fn run_view_event(
    runtime: &mut ExtensionRuntime,
    extension_id: &str,
    request: ExtensionViewRequest,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
) -> Result<RuntimeViewCompletion, SupervisorError> {
    runtime.ensure_running()?;
    let request_id = request.request_id;
    match request.kind {
        ExtensionViewRequestKind::Event(event) => runtime
            .view_event_cancellable(
                format!("view-{extension_id}-{request_id}"),
                request.generation,
                request.view_id,
                request.revision,
                event,
                || {
                    let (lock, _) = &**state;
                    lock.lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .shutdown
                        .load(Ordering::Acquire)
                },
            )
            .map(|(revision, effect, view)| RuntimeViewCompletion {
                revision,
                effect,
                view,
            }),
        ExtensionViewRequestKind::Close => runtime
            .close_view(
                format!("close-view-{extension_id}-{request_id}"),
                request.view_id,
            )
            .map(|()| RuntimeViewCompletion {
                revision: request.revision,
                effect: nanika_protocol::NavigationEffect::Pop,
                view: None,
            }),
    }
}

fn run_configuration_update(
    runtime: &mut ExtensionRuntime,
    update: ExtensionConfigurationUpdate,
    extension_id: &str,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    results: &Arc<Mutex<VecDeque<ExtensionConfigurationResult>>>,
    error: &Arc<Mutex<Option<HostDiagnostic>>>,
    notifier: &ExtensionNotifier,
) {
    let state = Arc::clone(state);
    let results = Arc::clone(results);
    let error = Arc::clone(error);
    let notifier = Arc::clone(notifier);
    let extension_id = extension_id.to_owned();
    runtime.start_configuration_update(
        update.request_id.clone(),
        update.configuration.clone(),
        Box::new(move |result| {
            let completion = result.as_ref().map_err(ToString::to_string).copied();
            match result {
                Ok(()) => set_error(&error, None),
                Err(cause) => set_error(
                    &error,
                    Some(extension_failure(
                        &extension_id,
                        "apply configuration",
                        "Saved settings could not be applied.",
                        cause,
                    )),
                ),
            }
            update.complete(&extension_id, &results, completion);
            state
                .0
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .configuration_pending = false;
            state.1.notify_all();
            notify(&notifier);
        }),
    );
}

fn run_refresh(
    runtime: &mut ExtensionRuntime,
    extension_id: &str,
    refresh: &ExtensionRefresh,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
) -> Result<bool, SupervisorError> {
    runtime.ensure_running()?;
    runtime.refresh_cancellable(
        format!("refresh-{extension_id}-{}", refresh.request_id),
        refresh.generation,
        || {
            let (lock, _) = &**state;
            let state = lock.lock().unwrap_or_else(|error| error.into_inner());
            state.shutdown.load(Ordering::Acquire)
        },
    )
}

fn run_query(
    runtime: &mut ExtensionRuntime,
    extension_id: &str,
    query: ExtensionSearchQuery,
    search: &SearchHandle,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    contributions: &ExtensionContributions,
) -> Result<bool, SupervisorError> {
    runtime.ensure_running()?;
    if contributions.root_search.is_none() {
        return publish_contributions(search, extension_id, query.generation, contributions);
    }
    if !contributions.commands.is_empty() || !contributions.views.is_empty() {
        publish_contributions(search, extension_id, query.generation, contributions)?;
    }
    runtime.query_incremental(
        format!("search-{extension_id}-{}", query.generation),
        query.generation,
        query.query.clone(),
        |mut entries| {
            entries.extend(contribution_candidates(contributions));
            publish_extension_snapshot(search, extension_id, query.generation, entries)
                .map_err(|error| SupervisorError::UnexpectedMessage(error.to_string()))
        },
        || {
            let (lock, _) = &**state;
            let state = lock.lock().unwrap_or_else(|error| error.into_inner());
            state.shutdown.load(Ordering::Acquire)
                || state.query.is_some()
                || (!state.configuration_pending && !state.refreshes.is_empty())
                || !state.invocations.is_empty()
                || !state.view_events.is_empty()
        },
    )
}

fn publish_contributions(
    search: &SearchHandle,
    extension_id: &str,
    generation: u64,
    contributions: &ExtensionContributions,
) -> Result<bool, SupervisorError> {
    publish_extension_snapshot(
        search,
        extension_id,
        generation,
        contribution_candidates(contributions),
    )
    .map_err(|error| SupervisorError::UnexpectedMessage(error.to_string()))?;
    Ok(true)
}

pub(crate) fn contribution_candidates(
    contributions: &ExtensionContributions,
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
                .or_else(|| Some("Command".to_owned())),
            action_id: nanika_protocol::COMMAND_EXECUTE_ACTION_ID.to_owned(),
            aliases,
            icon: None,
            contribution_icon: command.icon.map(protocol_contribution_icon),
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
            subtitle: view.category.clone().or_else(|| Some("View".to_owned())),
            action_id: nanika_protocol::VIEW_OPEN_ACTION_ID.to_owned(),
            aliases,
            icon: None,
            contribution_icon: view.icon.map(protocol_contribution_icon),
        }
    });
    commands.chain(views).collect()
}

fn protocol_contribution_icon(
    icon: nanika_extension_package::ContributionIcon,
) -> nanika_protocol::ContributionIcon {
    match icon {
        nanika_extension_package::ContributionIcon::Applications => {
            nanika_protocol::ContributionIcon::Applications
        }
        nanika_extension_package::ContributionIcon::Calculator => {
            nanika_protocol::ContributionIcon::Calculator
        }
        nanika_extension_package::ContributionIcon::Clipboard => {
            nanika_protocol::ContributionIcon::Clipboard
        }
        nanika_extension_package::ContributionIcon::Command => {
            nanika_protocol::ContributionIcon::Command
        }
        nanika_extension_package::ContributionIcon::Script => {
            nanika_protocol::ContributionIcon::Script
        }
        nanika_extension_package::ContributionIcon::Extension => {
            nanika_protocol::ContributionIcon::Extension
        }
    }
}

fn run_invocation(
    runtime: &mut ExtensionRuntime,
    extension_id: &str,
    invocation: &ExtensionInvocation,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    invocation_output: &Arc<Mutex<ExtensionInvocationOutputState>>,
    notifier: &ExtensionNotifier,
) -> Result<ExtensionInvocationOutcome, SupervisorError> {
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
                output_invocation_id,
                &output_extension_id,
                output_generation,
                &chunk,
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
            invocation.query_context.clone(),
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
        },
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
    source: SupervisorError,
) -> HostDiagnostic {
    HostDiagnostic::from_error(
        DiagnosticCode::ExtensionUnavailable,
        operation,
        format!("{user_message} Extension: {extension_id}."),
        source,
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

// Capacity is an in-memory admission bound, not a timeout or a policy that drops
// accepted work. Waiting releases the state lock so cancellation and shutdown work.
const PENDING_WORK_CAPACITY: usize = 16;

fn wait_for_capacity<'a>(
    lock: &'a Mutex<ExtensionSearchState>,
    changed: &Condvar,
) -> Result<MutexGuard<'a, ExtensionSearchState>, SupervisorError> {
    let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
    loop {
        if state.closed || state.shutdown.load(Ordering::Acquire) {
            return Err(SupervisorError::ChannelClosed);
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
    failure: &mut Option<String>,
) -> Result<&'a mut ExtensionRuntime, SupervisorError>
where
    F: FnOnce(nanika_protocol::ExtensionConfiguration) -> Result<ExtensionRuntime, SupervisorError>,
{
    if runtime.is_none() {
        if let Some(error) = failure {
            return Err(SupervisorError::UnexpectedMessage(error.clone()));
        }
        match factory.take().expect("activation is attempted only once")(configuration.clone()) {
            Ok(started) => *runtime = Some(started),
            Err(error) => {
                *failure = Some(error.to_string());
                return Err(error);
            }
        }
    }
    Ok(runtime.as_mut().expect("activated runtime"))
}
