use std::io;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use nanika_search::SearchHandle;

use crate::{
    DiagnosticCode, ExtensionInvocation, ExtensionInvocationOutcome, ExtensionInvocationOutput,
    ExtensionInvocationOutputState, ExtensionInvocationResult, ExtensionNotifier, ExtensionRefresh,
    ExtensionRuntime, ExtensionRuntimeInvocation, ExtensionSearchQuery, ExtensionSearchState,
    ExtensionSettingsResult, ExtensionSettingsUpdate, ExtensionWork, HostDiagnostic,
    HostServiceHandler, SupervisorError, publish_extension_snapshot,
};

const MAX_PENDING_INVOCATIONS: usize = 16;
const MAX_PENDING_SETTINGS: usize = 4;

/// Fixed worker that keeps extension protocol I/O off the UI thread.
pub(crate) struct ExtensionSearchWorker {
    extension_id: String,
    state: Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    last_error: Arc<Mutex<Option<HostDiagnostic>>>,
    invocation_output: Arc<Mutex<ExtensionInvocationOutputState>>,
    settings_result: Arc<Mutex<Option<ExtensionSettingsResult>>>,
    thread: Option<JoinHandle<()>>,
}

impl ExtensionSearchWorker {
    pub(crate) fn spawn(
        extension_id: impl Into<String>,
        mut runtime: ExtensionRuntime,
        search: SearchHandle,
        invocation_results: SyncSender<ExtensionInvocationResult>,
        notifier: ExtensionNotifier,
        host_services: Option<Arc<dyn HostServiceHandler>>,
    ) -> io::Result<Self> {
        let extension_id = extension_id.into();
        let worker_extension_id = extension_id.clone();
        if let Some(host_services) = host_services {
            runtime.set_host_services(extension_id.clone(), host_services);
        }
        let state = Arc::new((Mutex::new(ExtensionSearchState::default()), Condvar::new()));
        let worker_state = Arc::clone(&state);
        let last_error = Arc::new(Mutex::new(None));
        let worker_error = Arc::clone(&last_error);
        let invocation_output = Arc::new(Mutex::new(ExtensionInvocationOutputState::default()));
        let worker_invocation_output = Arc::clone(&invocation_output);
        let settings_result: Arc<Mutex<Option<ExtensionSettingsResult>>> =
            Arc::new(Mutex::new(None));
        let worker_settings_result = Arc::clone(&settings_result);
        let thread = std::thread::Builder::new()
            .name(format!("nanika-search-extension-{extension_id}"))
            .spawn(move || {
                if let Err(error) = runtime.initialize(format!("initialize-{worker_extension_id}"))
                {
                    set_error(
                        &worker_error,
                        Some(extension_failure(
                            &worker_extension_id,
                            "initialize extension",
                            "The extension could not start.",
                            error,
                        )),
                    );
                    notify(&notifier);
                    return;
                }
                let initial_settings =
                    match runtime.settings(format!("settings-{worker_extension_id}")) {
                        Ok(settings) => Ok(settings),
                        Err(error) => {
                            let diagnostic = extension_failure(
                                &worker_extension_id,
                                "load initial extension settings",
                                "The extension could not load its settings.",
                                error,
                            );
                            let user_message = diagnostic.user_message().to_owned();
                            set_error(&worker_error, Some(diagnostic));
                            Err(user_message)
                        }
                    };
                *worker_settings_result
                    .lock()
                    .unwrap_or_else(|error| error.into_inner()) = Some(ExtensionSettingsResult {
                    extension_id: worker_extension_id.clone(),
                    request_id: None,
                    result: initial_settings,
                });
                notify(&notifier);
                loop {
                    let work = next_work(&worker_state);
                    let Some(work) = work else {
                        break;
                    };
                    let result = match work {
                        ExtensionWork::Query(query) => run_query(
                            &mut runtime,
                            &worker_extension_id,
                            query,
                            &search,
                            &worker_state,
                        ),
                        ExtensionWork::Invoke(invocation) => {
                            let result = run_invocation(
                                &mut runtime,
                                &worker_extension_id,
                                &invocation,
                                &worker_state,
                                &worker_invocation_output,
                                &notifier,
                            );
                            let report = ExtensionInvocationResult {
                                invocation_id: invocation.invocation_id,
                                extension_id: worker_extension_id.clone(),
                                generation: invocation.generation,
                                entry_id: invocation.entry_id,
                                action_id: invocation.action_id,
                                query_context: invocation.query_context,
                                result: result
                                    .as_ref()
                                    .map(Clone::clone)
                                    .map_err(ToString::to_string),
                            };
                            if invocation_results.send(report).is_err() {
                                Err(SupervisorError::ChannelClosed)
                            } else {
                                result.map(|outcome| {
                                    matches!(outcome, ExtensionInvocationOutcome::Completed { .. })
                                })
                            }
                        }
                        ExtensionWork::Refresh(refresh) => {
                            run_refresh(&mut runtime, &worker_extension_id, refresh, &worker_state)
                        }
                        ExtensionWork::UpdateSettings(update) => {
                            let request_id = update.request_id.clone();
                            let result =
                                run_settings_update(&mut runtime, &worker_extension_id, update);
                            let report = ExtensionSettingsResult {
                                extension_id: worker_extension_id.clone(),
                                request_id: Some(request_id),
                                result: result
                                    .as_ref()
                                    .map(Clone::clone)
                                    .map_err(ToString::to_string),
                            };
                            *worker_settings_result
                                .lock()
                                .unwrap_or_else(|error| error.into_inner()) = Some(report);
                            result.map(|_| true)
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
                if let Err(error) = runtime.shutdown(format!("shutdown-{worker_extension_id}")) {
                    HostDiagnostic::from_error(
                        DiagnosticCode::ExtensionUnavailable,
                        "shut down extension",
                        "An extension did not shut down cleanly.",
                        error,
                    )
                    .with_safe_context(&worker_extension_id)
                    .record_warning();
                }
            })?;
        Ok(Self {
            extension_id,
            state,
            last_error,
            invocation_output,
            settings_result,
            thread: Some(thread),
        })
    }

    pub fn query(&self, generation: u64, query: impl Into<String>) {
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        state.query = Some(ExtensionSearchQuery {
            generation,
            query: query.into(),
        });
        ready.notify_one();
    }

    pub(crate) fn refresh(&self, generation: u64) {
        let (lock, ready) = &*self.state;
        lock.lock()
            .unwrap_or_else(|error| error.into_inner())
            .refresh = Some(ExtensionRefresh { generation });
        ready.notify_one();
    }

    pub(crate) fn invoke(&self, invocation: ExtensionInvocation) -> Result<(), SupervisorError> {
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        if state.invocations.len() >= MAX_PENDING_INVOCATIONS {
            return Err(SupervisorError::QueueFull);
        }
        state.invocations.push_back(invocation);
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

    pub(crate) fn update_settings(
        &self,
        request_id: String,
        updates: Vec<nanika_protocol::SettingUpdate>,
    ) -> Result<(), SupervisorError> {
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        if state.settings.len() >= MAX_PENDING_SETTINGS {
            return Err(SupervisorError::QueueFull);
        }
        state.settings.push_back(ExtensionSettingsUpdate {
            request_id,
            updates,
        });
        ready.notify_one();
        Ok(())
    }

    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub fn last_error(&self) -> Option<HostDiagnostic> {
        self.last_error
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(crate) fn take_settings(&self) -> Option<ExtensionSettingsResult> {
        self.settings_result
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
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
            .shutdown = true;
        ready.notify_one();
    }

    pub(crate) fn join(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for ExtensionSearchWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn next_work(state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>) -> Option<ExtensionWork> {
    let (lock, ready) = &**state;
    let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
    while state.query.is_none()
        && state.refresh.is_none()
        && state.invocations.is_empty()
        && state.settings.is_empty()
        && !state.shutdown
    {
        state = ready.wait(state).unwrap_or_else(|error| error.into_inner());
    }
    if state.shutdown {
        return None;
    }
    if let Some(invocation) = state.invocations.pop_front() {
        state.active_invocation_id = Some(invocation.invocation_id);
        return Some(ExtensionWork::Invoke(invocation));
    }
    state
        .settings
        .pop_front()
        .map(ExtensionWork::UpdateSettings)
        .or_else(|| state.query.take().map(ExtensionWork::Query))
        .or_else(|| state.refresh.take().map(ExtensionWork::Refresh))
}

fn run_settings_update(
    runtime: &mut ExtensionRuntime,
    extension_id: &str,
    update: ExtensionSettingsUpdate,
) -> Result<nanika_protocol::SettingsContribution, SupervisorError> {
    recover_if_exited(runtime, extension_id, 0)?;
    let result = runtime.update_settings(update.request_id, update.updates);
    if matches!(
        &result,
        Err(SupervisorError::ChannelClosed
            | SupervisorError::Protocol(_)
            | SupervisorError::Timeout(_))
    ) {
        restart_or_terminate(runtime, format!("restart-settings-{extension_id}"))?;
    }
    result
}

fn run_refresh(
    runtime: &mut ExtensionRuntime,
    extension_id: &str,
    refresh: ExtensionRefresh,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
) -> Result<bool, SupervisorError> {
    recover_if_exited(runtime, extension_id, refresh.generation)?;
    let result = runtime.refresh_cancellable(
        format!("refresh-{extension_id}-{}", refresh.generation),
        refresh.generation,
        Duration::from_secs(30),
        || {
            let (lock, _) = &**state;
            let state = lock.lock().unwrap_or_else(|error| error.into_inner());
            state.shutdown
                || state.query.is_some()
                || !state.invocations.is_empty()
                || state.refresh.is_some()
        },
    );
    if matches!(
        &result,
        Err(SupervisorError::ChannelClosed
            | SupervisorError::Protocol(_)
            | SupervisorError::Timeout(_))
    ) {
        restart_or_terminate(
            runtime,
            format!("restart-refresh-{extension_id}-{}", refresh.generation),
        )?;
    }
    result
}

fn run_query(
    runtime: &mut ExtensionRuntime,
    extension_id: &str,
    query: ExtensionSearchQuery,
    search: &SearchHandle,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
) -> Result<bool, SupervisorError> {
    recover_if_exited(runtime, extension_id, query.generation)?;
    let mut retried = false;
    loop {
        let result = runtime.query_incremental(
            format!("search-{extension_id}-{}", query.generation),
            query.generation,
            query.query.clone(),
            Duration::from_secs(2),
            |entries| {
                publish_extension_snapshot(search, extension_id, query.generation, entries)
                    .map_err(|error| SupervisorError::UnexpectedMessage(error.to_string()))
            },
            || {
                let (lock, _) = &**state;
                let state = lock.lock().unwrap_or_else(|error| error.into_inner());
                state.shutdown || state.query.is_some() || !state.invocations.is_empty()
            },
        );
        let restartable = matches!(
            &result,
            Err(SupervisorError::ChannelClosed
                | SupervisorError::Protocol(_)
                | SupervisorError::Timeout(_))
        );
        if result.is_ok() || !restartable {
            return result;
        }
        if retried {
            let _ = runtime.terminate();
            return result;
        }
        restart_or_terminate(
            runtime,
            format!("restart-{extension_id}-{}", query.generation),
        )?;
        retried = true;
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
    recover_if_exited(runtime, extension_id, invocation.generation)?;
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
    let result = runtime.invoke_cancellable(
        ExtensionRuntimeInvocation::new(
            format!("invoke-{extension_id}-{}", invocation.generation),
            invocation.generation,
            invocation.entry_id.clone(),
            invocation.action_id.clone(),
            invocation.query_context.clone(),
        ),
        publish,
        || {
            let (lock, _) = &**state;
            let state = lock.lock().unwrap_or_else(|error| error.into_inner());
            state.shutdown
                || state
                    .cancelled_invocations
                    .contains(&invocation.invocation_id)
        },
    );
    let (lock, _) = &**state;
    let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
    let shutting_down = state.shutdown;
    let user_cancelled = state
        .cancelled_invocations
        .remove(&invocation.invocation_id);
    state.active_invocation_id = None;
    drop(state);
    if matches!(&result, Err(SupervisorError::Cancelled(_))) {
        if user_cancelled && !shutting_down {
            runtime.recover_after_cancellation(format!(
                "recover-cancelled-{extension_id}-{}",
                invocation.generation
            ))?;
        }
        return Ok(ExtensionInvocationOutcome::Cancelled);
    }
    if !shutting_down
        && matches!(
            &result,
            Err(SupervisorError::ChannelClosed
                | SupervisorError::Protocol(_)
                | SupervisorError::Timeout(_))
        )
    {
        restart_or_terminate(
            runtime,
            format!("restart-invoke-{extension_id}-{}", invocation.generation),
        )?;
    }
    result.map(|has_output| ExtensionInvocationOutcome::Completed { has_output })
}

fn restart_or_terminate(
    runtime: &mut ExtensionRuntime,
    request_id: String,
) -> Result<(), SupervisorError> {
    match runtime.restart(request_id) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = runtime.terminate();
            Err(error)
        }
    }
}

fn recover_if_exited(
    runtime: &mut ExtensionRuntime,
    extension_id: &str,
    generation: u64,
) -> Result<(), SupervisorError> {
    runtime
        .recover_if_exited(format!("recover-{extension_id}-{generation}"))
        .map(|_| ())
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
