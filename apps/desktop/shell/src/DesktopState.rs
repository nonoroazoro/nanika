use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::{
    ApplicationSnapshot, DesktopRuntime, InvokeCandidateRequest, PublishQueryRequest,
    RootSearchSnapshot, SearchDelivery, SearchSession,
};

pub(crate) struct DesktopState {
    next_session_id: AtomicU64,
    shared: Arc<Mutex<DesktopRuntime>>,
    view_operation_lock: Mutex<()>,
    wakes: SyncSender<SearchDelivery>,
    dispatcher: Option<JoinHandle<()>>,
    instance: Mutex<Option<nanika_platform::SingleInstance>>,
    _diagnostics: nanika_host::Diagnostics,
    _hotkey_timing: Option<nanika_platform::HotkeyTimingObserver>,
}

impl DesktopState {
    pub(crate) fn new(
        instance: nanika_platform::SingleInstance,
        diagnostics: nanika_host::Diagnostics,
    ) -> Result<Self, String> {
        let shared = Arc::new(Mutex::new(DesktopRuntime::default()));
        let (wakes, receiver) = mpsc::sync_channel(1);
        let worker_state = Arc::clone(&shared);
        let dispatcher = std::thread::Builder::new()
            .name("nanika-search-delivery".to_owned())
            .spawn(move || crate::search_delivery::run_delivery(&worker_state, receiver))
            .map_err(|error| error.to_string())?;
        Ok(Self {
            next_session_id: AtomicU64::new(1),
            shared,
            view_operation_lock: Mutex::new(()),
            wakes,
            dispatcher: Some(dispatcher),
            instance: Mutex::new(Some(instance)),
            _diagnostics: diagnostics,
            _hotkey_timing: nanika_platform::HotkeyTimingObserver::install(),
        })
    }

    pub(crate) fn open_session(
        &self,
        updates: tauri::ipc::Channel<RootSearchSnapshot>,
    ) -> Result<ApplicationSnapshot, String> {
        let id = self.next_session_id.fetch_add(1, Ordering::Relaxed);
        let mut state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut session = SearchSession::new(id, updates);
        if let Some(runtime) = &state.runtime {
            session.generation = runtime.begin_query("")?;
        }
        // Registration and the initial empty query are atomic relative to delivery.
        // The response contains metadata only; all search data uses the Channel.
        let previous = state.session.replace(session);
        let runtime = state.runtime.clone();
        drop(state);
        self.wake();
        retire_views(runtime, previous);
        tracing::debug!(session_id = id, "frontend session opened");
        Ok(ApplicationSnapshot {
            session_id: id,
            locale: nanika_platform::system_locale(),
            max_query_chars: nanika_search::MAX_QUERY_CHARS,
            resource_origin: crate::resource_protocol::origin().to_owned(),
        })
    }

    pub(crate) fn publish_query(&self, request: PublishQueryRequest) -> Result<(), String> {
        if request.request_id == 0
            || request.request_id > 9_007_199_254_740_991
            || request.query.chars().count() > nanika_search::MAX_QUERY_CHARS
        {
            return Err("Invalid search request.".to_owned());
        }
        let mut state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let DesktopRuntime {
            runtime, session, ..
        } = &mut *state;
        let session = session.as_mut().ok_or("The window session is not open.")?;
        session.authorize(request.session_id)?;
        if request.request_id <= session.request_id {
            // Async commands may reach Rust in a different order from user input.
            return Ok(());
        }
        session.request_id = request.request_id;
        session.query = request.query;
        session.delivered = None;
        session.phase = None;
        session.generation = if let Some(runtime) = runtime {
            runtime.begin_query(session.query.clone())?
        } else {
            0
        };
        drop(state);
        self.wake();
        Ok(())
    }

    pub(crate) fn refresh_search(&self, session_id: u64) -> Result<(), String> {
        let (runtime, generation) = {
            let mut state = self
                .shared
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let runtime = Arc::clone(state.runtime.as_ref().ok_or("Nanika is still starting.")?);
            let session = state
                .session
                .as_mut()
                .ok_or("The window session is not open.")?;
            session.begin_refresh(session_id)?;
            (runtime, session.generation)
        };
        self.wake();
        let result = runtime.refresh_root_search(generation);
        // A user may type or close the window while scanning. Republish only the
        // originating session's latest query, never the query captured at F5.
        let publication = (|| {
            let mut state = self
                .shared
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let Some(session) = state
                .session
                .as_mut()
                .filter(|session| session.id == session_id)
            else {
                return Ok(());
            };
            session.generation = runtime.begin_query(session.query.clone())?;
            session.delivered = None;
            session.phase = None;
            Ok::<(), String>(())
        })();
        let result = match (result, publication) {
            (Err(refresh), Err(publish)) => Err(format!("{refresh}\n{publish}")),
            (Err(error), _) | (_, Err(error)) => Err(error),
            _ => Ok(()),
        };
        if let Err(error) = &result {
            tracing::warn!(%error, "root search refresh failed");
        }
        self.finish_navigation(session_id, result.clone());
        result
    }

    pub(crate) fn acknowledge_search(&self, session_id: u64, revision: u64) -> Result<(), String> {
        let mut state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let session = state
            .session
            .as_mut()
            .ok_or("The window session is not open.")?;
        session.authorize(session_id)?;
        if session.in_flight.is_some_and(|(sent, _)| sent == revision) {
            let elapsed = session
                .in_flight
                .take()
                .map(|(_, at)| at.elapsed().as_millis());
            tracing::debug!(session_id, revision, elapsed_ms = ?elapsed, "frontend received search update");
        } else {
            return Err("Unexpected search acknowledgement.".to_owned());
        }
        drop(state);
        self.wake();
        Ok(())
    }

    pub(crate) fn close_session(&self, session_id: u64) {
        let mut state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let previous = if state
            .session
            .as_ref()
            .is_some_and(|session| session.id == session_id)
        {
            state.session.take()
        } else {
            None
        };
        let runtime = state.runtime.clone();
        drop(state);
        retire_views(runtime, previous);
        self.wake();
    }

    pub(crate) fn run_invocation(&self, request: &InvokeCandidateRequest) -> Result<(), String> {
        let (runtime, query, generation) = {
            let mut state = self
                .shared
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let runtime = Arc::clone(state.runtime.as_ref().ok_or("Nanika is still starting.")?);
            let session = state
                .session
                .as_mut()
                .ok_or("The window session is not open.")?;
            session.authorize(request.session_id)?;
            if session.request_id != request.request_id || !session.navigation.stack.is_empty() {
                return Err("Search changed. Select a current result.".to_owned());
            }
            session.navigation.begin()?;
            (runtime, session.query.clone(), session.generation)
        };
        self.wake();
        let result = (|| {
            let completion = runtime.invoke(
                generation,
                &request.extension_id,
                &request.entry_id,
                &request.action_id,
                &query,
            )?;
            let outcome = completion
                .recv()
                .map_err(|_| "Extension closed without an invocation result.".to_owned())??;
            let nanika_host::ExtensionInvocationOutcome::Completed { effect, .. } = outcome else {
                return Err("The action was cancelled before it completed.".to_owned());
            };
            self.apply_navigation(
                request.session_id,
                &runtime,
                &request.extension_id,
                generation,
                effect,
                false,
            )?;
            self.record_execution(request, &query)
        })();
        self.finish_navigation(request.session_id, result.clone());
        result
    }

    pub(crate) fn run_view_event(&self, request: crate::ViewEventRequest) -> Result<(), String> {
        // Tauri may dispatch pointer and keyboard events concurrently. Serialize all
        // view operations here so route state is never rejected as already busy.
        let _operation_guard = self
            .view_operation_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let (runtime, route) = {
            let mut state = self
                .shared
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let runtime = Arc::clone(state.runtime.as_ref().ok_or("Nanika is still starting.")?);
            let session = state
                .session
                .as_mut()
                .ok_or("The window session is not open.")?;
            session.authorize(request.session_id)?;
            let route = session
                .navigation
                .authorize_route(request.route_id)?
                .clone();
            if request._revision > route.revision {
                return Err("The extension view revision is ahead of the current state.".to_owned());
            }
            if let crate::ViewOperation::Event { event } = &request.operation {
                crate::authorize_view_event(&route.view, event)?;
            }
            session.navigation.begin()?;
            (runtime, route)
        };
        self.wake();
        let closing = matches!(request.operation, crate::ViewOperation::Back);
        let result = (|| {
            let completion = if let crate::ViewOperation::Event { event } = request.operation {
                runtime.view_event(
                    &route.extension_id,
                    route.generation,
                    &route.view_id,
                    route.revision,
                    event,
                )?
            } else {
                runtime.close_view(
                    &route.extension_id,
                    route.generation,
                    &route.view_id,
                    route.revision,
                )?
            };
            let completion = completion
                .recv()
                .map_err(|_| "Extension closed without a view result.".to_owned())??;
            if let Some(view) = completion.view {
                let mut state = self
                    .shared
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                let session = state
                    .session
                    .as_mut()
                    .ok_or("The window session is not open.")?;
                session.authorize(request.session_id)?;
                session
                    .navigation
                    .authorize_view(route.route_id, route.revision)?;
                let current = session
                    .navigation
                    .stack
                    .last_mut()
                    .ok_or("No extension view is open.")?;
                current.view = view;
                current.revision = completion.revision;
            }
            self.apply_navigation(
                request.session_id,
                &runtime,
                &route.extension_id,
                route.generation,
                completion.effect,
                closing,
            )
        })();
        self.finish_navigation(request.session_id, result.clone());
        result
    }

    fn apply_navigation(
        &self,
        session_id: u64,
        runtime: &nanika_host::RuntimeService,
        extension_id: &str,
        generation: u64,
        effect: nanika_protocol::NavigationEffect,
        already_closed: bool,
    ) -> Result<(), String> {
        let routes = {
            let state = self
                .shared
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let session = state
                .session
                .as_ref()
                .filter(|session| session.id == session_id);
            if let Some(session) = session {
                match &effect {
                    nanika_protocol::NavigationEffect::Pop if !already_closed => session
                        .navigation
                        .stack
                        .last()
                        .cloned()
                        .into_iter()
                        .collect(),
                    _ => Vec::new(),
                }
            } else {
                drop(state);
                // A completed action can outlive its WebView. Release a newly opened
                // extension view instead of delivering it to an unrelated session.
                if let nanika_protocol::NavigationEffect::Push {
                    view_id, revision, ..
                } = effect
                {
                    close_runtime_view(runtime, extension_id, generation, &view_id, revision)?;
                }
                return Err("The originating window session has closed.".to_owned());
            }
        };
        for route in routes {
            close_runtime_view(
                runtime,
                &route.extension_id,
                route.generation,
                &route.view_id,
                route.revision,
            )?;
        }
        let mut state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let session = state
            .session
            .as_mut()
            .ok_or("The window session is not open.")?;
        session.authorize(session_id)?;
        session.navigation.apply(extension_id, generation, effect)
    }

    fn finish_navigation(&self, session_id: u64, result: Result<(), String>) {
        let mut state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(session) = state
            .session
            .as_mut()
            .filter(|session| session.id == session_id)
        {
            session.navigation.finish(result);
        }
        drop(state);
        self.wake();
    }

    pub(crate) fn record_execution(
        &self,
        request: &InvokeCandidateRequest,
        query: &str,
    ) -> Result<(), String> {
        let state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let runtime = Arc::clone(state.runtime.as_ref().ok_or("Nanika is still starting.")?);
        drop(state);
        runtime.record_execution(
            &request.extension_id,
            &request.entry_id,
            &request.action_id,
            query,
        )
    }

    pub(crate) fn install_runtime(
        &self,
        runtime: nanika_host::RuntimeService,
    ) -> Result<(), String> {
        let wakes = self.wakes.clone();
        runtime.set_notifier(Arc::new(move || {
            // A full wake slot already guarantees the latest state will be read.
            if matches!(
                wakes.try_send(SearchDelivery::Wake),
                Err(TrySendError::Disconnected(_))
            ) {
                tracing::error!("search delivery worker is closed");
            }
        }));
        let mut state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(session) = &mut state.session {
            session.generation = runtime.begin_query(session.query.clone())?;
        }
        state.runtime = Some(Arc::new(runtime));
        drop(state);
        self.wake();
        Ok(())
    }

    pub(crate) fn fail_startup(&self, error: String) {
        tracing::error!(%error, "runtime initialization failed");
        self.shared
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .startup_error = Some(error);
        self.wake();
    }

    fn wake(&self) {
        if matches!(
            self.wakes.try_send(SearchDelivery::Wake),
            Err(TrySendError::Disconnected(_))
        ) {
            tracing::error!("search delivery worker is closed");
        }
    }
}

impl Drop for DesktopState {
    fn drop(&mut self) {
        if self.wakes.send(SearchDelivery::Shutdown).is_err() {
            tracing::error!("search delivery worker closed before shutdown was requested");
        }
        if let Some(thread) = self.dispatcher.take()
            && thread.join().is_err()
        {
            tracing::error!("search delivery worker panicked");
        }
        self.instance
            .get_mut()
            .unwrap_or_else(|error| error.into_inner())
            .take();
    }
}

fn close_runtime_view(
    runtime: &nanika_host::RuntimeService,
    extension_id: &str,
    generation: u64,
    view_id: &str,
    revision: u64,
) -> Result<(), String> {
    runtime
        .close_view(extension_id, generation, view_id, revision)?
        .recv()
        .map_err(|_| "Extension closed without acknowledging view closure.".to_owned())??;
    Ok(())
}

fn retire_views(runtime: Option<Arc<nanika_host::RuntimeService>>, session: Option<SearchSession>) {
    let (Some(runtime), Some(session)) = (runtime, session) else {
        return;
    };
    if session.navigation.stack.is_empty() {
        return;
    }
    tauri::async_runtime::spawn_blocking(move || {
        for route in session.navigation.stack.into_iter().rev() {
            if let Err(error) = close_runtime_view(
                &runtime,
                &route.extension_id,
                route.generation,
                &route.view_id,
                route.revision,
            ) {
                tracing::error!(%error, view_id = route.view_id, "could not close retired extension view");
            }
        }
    });
}
