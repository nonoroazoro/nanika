use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard};
use std::thread::JoinHandle;

use crate::{
    ApplicationSnapshot, DesktopRuntime, InvokeCandidateRequest, PublishQueryRequest,
    RootSearchSnapshot, SearchDelivery, SearchSession, ViewInvalidationDelivery,
};

pub(crate) struct DesktopState {
    next_session_id: AtomicU64,
    next_settings_request: AtomicU64,
    settings_operation_lock: Mutex<()>,
    settings_applications: Mutex<crate::SettingsApplications>,
    stopping: AtomicBool,
    operations: RwLock<()>,
    initializer: Mutex<Option<JoinHandle<()>>>,
    instance_bridge: Mutex<Option<JoinHandle<()>>>,
    shared: Arc<Mutex<DesktopRuntime>>,
    wakes: SyncSender<SearchDelivery>,
    dispatcher: Mutex<Option<JoinHandle<()>>>,
    view_invalidation_wakes: SyncSender<ViewInvalidationDelivery>,
    view_invalidation_dispatcher: Mutex<Option<JoinHandle<()>>>,
    instance: Mutex<Option<nanika_platform::SingleInstance>>,
    diagnostics: Mutex<Option<nanika_host::Diagnostics>>,
    hotkey_timing: Mutex<Option<nanika_platform::HotkeyTimingObserver>>,
}

impl DesktopState {
    pub(crate) fn menu_actions(
        &self,
        request: &crate::ContextMenuRequest,
    ) -> Result<Vec<nanika_protocol::Action>, String> {
        let state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let session = state
            .session
            .as_ref()
            .ok_or("The window session is not open.")?;
        session.authorize(request.session_id)?;
        if session.navigation.busy {
            return Err("An action is still running.".to_owned());
        }
        let actions = match &request.target {
            crate::MenuTarget::Search {
                request_id,
                revision,
                extension_id,
                entry_id,
            } => {
                if session.request_id != *request_id
                    || session.revision != *revision
                    || !session.navigation.stack.is_empty()
                {
                    return Err("Search changed. Reopen the menu.".to_owned());
                }
                session
                    .delivered
                    .as_ref()
                    .and_then(|snapshot| {
                        snapshot.results.iter().find(|result| {
                            result.candidate.extension_id() == extension_id
                                && result.candidate.entry_id() == entry_id
                        })
                    })
                    .ok_or("The result is no longer available.")?
                    .candidate
                    .actions()
                    .to_vec()
            }
            crate::MenuTarget::View {
                route_id,
                revision,
                item_id,
            } => {
                let route = session.navigation.authorize_route(*route_id)?;
                if route.revision != *revision {
                    return Err("The view changed. Reopen the menu.".to_owned());
                }
                match (&*route.view, item_id) {
                    (nanika_protocol::View::List { list }, Some(id)) => list
                        .sections
                        .iter()
                        .flat_map(|section| &section.items)
                        .find(|item| &item.id == id)
                        .ok_or("The item is no longer available.")?
                        .actions
                        .clone(),
                    (nanika_protocol::View::Detail { detail }, None) => detail.actions.clone(),
                    _ => return Err("The menu target is unavailable.".to_owned()),
                }
            }
        };
        nanika_protocol::validate_actions(&actions)?;
        Ok(actions)
    }

    pub(crate) fn invoke_menu_action(
        &self,
        request: &crate::ContextMenuRequest,
        action_id: String,
        confirmed: bool,
    ) -> Result<Option<crate::ViewEventReceipt>, String> {
        let invocation = if confirmed {
            nanika_protocol::ActionInvocation::Confirmed
        } else {
            nanika_protocol::ActionInvocation::Explicit
        };
        let actions = self.menu_actions(request)?;
        if !actions
            .iter()
            .any(|action| action.id == action_id && action.allows_invocation(invocation))
        {
            return Err("The action is unavailable.".to_owned());
        }
        match &request.target {
            crate::MenuTarget::Search {
                request_id,
                extension_id,
                entry_id,
                revision,
            } => self
                .run_invocation(
                    &InvokeCandidateRequest {
                        session_id: request.session_id,
                        request_id: *request_id,
                        extension_id: extension_id.clone(),
                        entry_id: entry_id.clone(),
                        action_id,
                    },
                    Some(*revision),
                    invocation,
                )
                .map(|()| None),
            crate::MenuTarget::View {
                route_id,
                revision,
                item_id,
            } => self
                .run_view_event(
                    crate::ViewEventRequest {
                        session_id: request.session_id,
                        route_id: *route_id,
                        revision: *revision,
                        operation: crate::ViewOperation::Event {
                            event: nanika_protocol::ViewEvent::ActionInvoked {
                                invocation,
                                item_id: item_id.clone(),
                                action_id,
                            },
                        },
                    },
                    Some(*revision),
                )
                .map(Some),
        }
    }

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
        let (view_invalidation_wakes, view_invalidation_receiver) = mpsc::sync_channel(1);
        let invalidation_state = Arc::clone(&shared);
        let invalidation_search_wakes = wakes.clone();
        let view_invalidation_dispatcher = std::thread::Builder::new()
            .name("nanika-view-invalidation".to_owned())
            .spawn(move || {
                crate::view_invalidation_delivery::run_delivery(
                    &invalidation_state,
                    view_invalidation_receiver,
                    &invalidation_search_wakes,
                )
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            next_session_id: AtomicU64::new(1),
            next_settings_request: AtomicU64::new(1),
            settings_operation_lock: Mutex::new(()),
            settings_applications: Mutex::new(crate::SettingsApplications::default()),
            stopping: AtomicBool::new(false),
            operations: RwLock::new(()),
            initializer: Mutex::new(None),
            instance_bridge: Mutex::new(None),
            shared,
            wakes,
            dispatcher: Mutex::new(Some(dispatcher)),
            view_invalidation_wakes,
            view_invalidation_dispatcher: Mutex::new(Some(view_invalidation_dispatcher)),
            instance: Mutex::new(Some(instance)),
            diagnostics: Mutex::new(Some(diagnostics)),
            hotkey_timing: Mutex::new(nanika_platform::HotkeyTimingObserver::install()),
        })
    }

    pub(crate) fn open_session(
        &self,
        updates: tauri::ipc::Channel<RootSearchSnapshot>,
    ) -> Result<ApplicationSnapshot, String> {
        let _operation = self.begin_operation()?;
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
        let _operation = self.begin_operation()?;
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
        let _operation = self.begin_operation()?;
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
        let _ = self.finish_navigation(session_id, result.clone());
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

    pub(crate) fn run_invocation(
        &self,
        request: &InvokeCandidateRequest,
        menu_revision: Option<u64>,
        invocation: nanika_protocol::ActionInvocation,
    ) -> Result<(), String> {
        let _operation = self.begin_operation()?;
        let (runtime, query, snapshot) = {
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
            // Recheck a menu's revision under the same lock that starts the action.
            if session.request_id != request.request_id
                || menu_revision.is_some_and(|revision| revision != session.revision)
                || !session.navigation.stack.is_empty()
            {
                return Err("Search changed. Select a current result.".to_owned());
            }
            let snapshot = session
                .delivered
                .clone()
                .filter(|snapshot| snapshot.generation == session.generation)
                .ok_or("Search results are not ready.")?;
            session.navigation.begin()?;
            (runtime, session.query.clone(), snapshot)
        };
        self.wake();
        let generation = snapshot.generation;
        let result = (|| {
            let completion = runtime.invoke_recorded(
                &snapshot,
                &request.extension_id,
                &request.entry_id,
                &request.action_id,
                &query,
                invocation,
            )?;
            apply_invocation_completion(completion, |effect| {
                self.apply_navigation(
                    request.session_id,
                    &runtime,
                    &request.extension_id,
                    generation,
                    effect,
                    false,
                )
            })
        })();
        let _ = self.finish_navigation(request.session_id, result.clone());
        result
    }

    pub(crate) fn run_view_event(
        &self,
        request: crate::ViewEventRequest,
        menu_revision: Option<u64>,
    ) -> Result<crate::ViewEventReceipt, String> {
        let _operation = self.begin_operation()?;
        // A retired WebView must not hold up an unrelated session's operations.
        let operation_lock = {
            let state = self
                .shared
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let session = state
                .session
                .as_ref()
                .ok_or("The window session is not open.")?;
            session.authorize(request.session_id)?;
            Arc::clone(&session.view_operation_lock)
        };
        let _operation_guard = operation_lock
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
            // The operation lock also excludes invalidation delivery. Validate against
            // the same route that is passed to the extension, never a UI prediction.
            let route = session
                .navigation
                .authorize_input(&request, menu_revision)?
                .clone();
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
                current.view = Arc::new(view);
                current.revision = completion.revision;
            }
            self.apply_navigation(
                request.session_id,
                &runtime,
                &route.extension_id,
                route.generation,
                completion.effect,
                closing,
            )?;
            Ok(completion.revision)
        })();
        let navigation_revision = self.finish_navigation(
            request.session_id,
            result.as_ref().map(|_| ()).map_err(Clone::clone),
        );
        result.and_then(|view_revision| {
            Ok(crate::ViewEventReceipt {
                view_revision,
                navigation_revision: navigation_revision
                    .ok_or("The originating window session has closed.")?,
            })
        })
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
                return Ok(());
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
        // Only release a newly created view. A duplicate id still belongs to an
        // existing route and must never be closed as rejection cleanup.
        let cleanup =
            match &effect {
                nanika_protocol::NavigationEffect::Push {
                    view_id, revision, ..
                } if !state.session.as_ref().is_some_and(|session| {
                    session.navigation.stack.iter().any(|route| {
                        route.extension_id == extension_id && route.view_id == *view_id
                    })
                }) =>
                {
                    Some((view_id.clone(), *revision))
                }
                _ => None,
            };
        let retired = state
            .session
            .as_ref()
            .is_none_or(|session| session.id != session_id);
        let result = if let Some(session) = state
            .session
            .as_mut()
            .filter(|session| session.id == session_id)
        {
            session.navigation.apply(extension_id, generation, effect)
        } else {
            Ok(())
        };
        drop(state);
        if (retired || result.is_err())
            && let Some((view_id, revision)) = cleanup
        {
            close_runtime_view(runtime, extension_id, generation, &view_id, revision).map_err(
                |error| {
                    format!(
                        "{} Cleanup failed: {error}",
                        result
                            .as_ref()
                            .err()
                            .map_or("The originating window session has closed.", String::as_str)
                    )
                },
            )?;
        }
        result
    }

    fn finish_navigation(&self, session_id: u64, result: Result<(), String>) -> Option<u64> {
        let mut state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let revision = state
            .session
            .as_mut()
            .filter(|session| session.id == session_id)
            .map(|session| {
                session.navigation.finish(result);
                session.navigation.revision
            });
        drop(state);
        self.wake();
        revision
    }

    pub(crate) fn install_runtime(
        &self,
        runtime: nanika_host::RuntimeService,
    ) -> Result<(), String> {
        let _operation = self.begin_operation()?;
        let wakes = self.wakes.clone();
        let view_invalidation_wakes = self.view_invalidation_wakes.clone();
        runtime.set_notifier(Arc::new(move || {
            // A full wake slot already guarantees the latest state will be read.
            if matches!(
                wakes.try_send(SearchDelivery::Wake),
                Err(TrySendError::Disconnected(_))
            ) {
                tracing::error!("search delivery worker is closed");
            }
            if matches!(
                view_invalidation_wakes.try_send(ViewInvalidationDelivery::Wake),
                Err(TrySendError::Disconnected(_))
            ) {
                tracing::error!("view invalidation worker is closed");
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

    pub(crate) fn set_initializer(&self, thread: JoinHandle<()>) {
        *self
            .initializer
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(thread);
    }

    pub(crate) fn set_instance_bridge(&self, thread: JoinHandle<()>) {
        *self
            .instance_bridge
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(thread);
    }

    pub(crate) fn begin_operation(&self) -> Result<RwLockReadGuard<'_, ()>, String> {
        let guard = self
            .operations
            .read()
            .unwrap_or_else(|error| error.into_inner());
        if self.stopping.load(Ordering::Acquire) {
            return Err("Nanika is shutting down.".to_owned());
        }
        Ok(guard)
    }

    pub(crate) fn subscribe_settings(&self, updates: tauri::ipc::Channel<crate::SettingsEvent>) {
        self.settings_applications
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .updates = Some(updates);
    }

    pub(crate) fn read_settings(
        &self,
        general: nanika_config::LauncherPreferences,
    ) -> Result<crate::SettingsSnapshot, String> {
        let _operation = self.begin_operation()?;
        // Only persistence is serialized here. Discovery never holds this lock.
        let _settings = self
            .settings_operation_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let runtime = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .runtime
            .clone()
            .ok_or("Nanika is still starting. Try loading Settings again.")?;
        let mut configurations = runtime
            .extension_configurations()
            .into_iter()
            .map(|configuration| (configuration.extension_id.clone(), configuration))
            .collect::<std::collections::BTreeMap<_, _>>();
        let applications = self
            .settings_applications
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let extensions = runtime
            .extension_info()
            .iter()
            .map(|info| crate::ExtensionSettings {
                configuration: configurations.remove(&info.id),
                info: info.clone(),
                application: applications.latest.get(&info.id).cloned(),
            })
            .collect();
        Ok(crate::SettingsSnapshot {
            maximized: false,
            version: env!("CARGO_PKG_VERSION"),
            general,
            extensions,
        })
    }

    pub(crate) fn save_settings(
        &self,
        request: crate::SaveSettingsRequest,
    ) -> Result<
        (
            crate::SettingsApplicationUpdate,
            nanika_host::ConfigurationSaveReceipt,
        ),
        String,
    > {
        let _operation = self.begin_operation()?;
        let _settings = self
            .settings_operation_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        crate::validate_settings_request(&request)?;
        let runtime = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .runtime
            .clone()
            .ok_or("Nanika is still starting.")?;
        let id = self.next_settings_request.fetch_add(1, Ordering::Relaxed);
        let receipt = runtime.save_configuration(
            &request.extension_id,
            format!("settings-{id}"),
            request.values,
        )?;
        let result = match &receipt {
            nanika_host::ConfigurationSaveReceipt::Complete(outcome) => outcome.clone().into(),
            nanika_host::ConfigurationSaveReceipt::Pending(_) => crate::SettingsSaveResult {
                status: crate::SettingsSaveStatus::Applying,
                error: None,
            },
        };
        let update = crate::SettingsApplicationUpdate {
            request_id: id,
            extension_id: request.extension_id,
            result,
        };
        self.publish_settings_application(update.clone());
        Ok((update, receipt))
    }

    pub(crate) fn publish_settings_application(&self, update: crate::SettingsApplicationUpdate) {
        let mut applications = self
            .settings_applications
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if applications
            .latest
            .get(&update.extension_id)
            .is_some_and(|current| current.request_id > update.request_id)
        {
            return;
        }
        applications
            .latest
            .insert(update.extension_id.clone(), update.clone());
        if let Some(channel) = &applications.updates {
            // A closed Settings window does not cancel a saved configuration.
            // The latest result remains available to the next window session.
            if channel
                .send(crate::SettingsEvent::Application { update })
                .is_err()
            {
                applications.updates = None;
            }
        }
    }

    pub(crate) fn settings_closed(&self) {
        if let Err(error) = self.send_settings_event(crate::SettingsEvent::Closed) {
            tracing::error!(%error, "settings close notification failed");
        }
    }

    pub(crate) fn shortcut_recorded(&self) {
        if let Err(error) = self.send_settings_event(crate::SettingsEvent::ShortcutPressed) {
            tracing::error!(%error, "settings shortcut notification failed");
        }
    }

    pub(crate) fn send_settings_event(&self, event: crate::SettingsEvent) -> Result<(), String> {
        let mut applications = self
            .settings_applications
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let channel = applications
            .updates
            .as_ref()
            .ok_or("Settings channel is unavailable.")?;
        if let Err(error) = channel.send(event) {
            applications.updates = None;
            return Err(error.to_string());
        }
        Ok(())
    }

    pub(crate) fn shutdown(&self) {
        if self.stopping.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Some(thread) = self
            .initializer
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
            && thread.join().is_err()
        {
            tracing::error!("runtime initializer panicked");
        }
        let runtime = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .runtime
            .clone();
        // Wake blocked protocol receivers before waiting for their shell callers.
        if let Some(runtime) = &runtime {
            runtime.request_shutdown();
        }
        let _operations = self
            .operations
            .write()
            .unwrap_or_else(|error| error.into_inner());
        if self
            .view_invalidation_wakes
            .send(ViewInvalidationDelivery::Shutdown)
            .is_err()
        {
            tracing::error!("view invalidation worker closed before shutdown was requested");
        }
        if self.wakes.send(SearchDelivery::Shutdown).is_err() {
            tracing::error!("search delivery worker closed before shutdown was requested");
        }
        if let Some(thread) = self
            .dispatcher
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
            && thread.join().is_err()
        {
            tracing::error!("search delivery worker panicked");
        }
        if let Some(thread) = self
            .view_invalidation_dispatcher
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
            && thread.join().is_err()
        {
            tracing::error!("view invalidation worker panicked");
        }
        if let Some(runtime) = runtime {
            runtime.shutdown();
        }
        self.instance
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(thread) = self
            .instance_bridge
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
            && thread.join().is_err()
        {
            tracing::error!("instance bridge panicked");
        }
        self.hotkey_timing
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        self.diagnostics
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
    }
}

impl Drop for DesktopState {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub(crate) fn apply_invocation_completion(
    completion: nanika_host::RuntimeInvocationCompletion,
    apply: impl FnOnce(nanika_protocol::NavigationEffect) -> Result<(), String>,
) -> Result<(), String> {
    let nanika_host::ExtensionInvocationOutcome::Completed { effect, .. } = completion.outcome
    else {
        return Err("The action was cancelled before it completed.".to_owned());
    };
    // Navigation owns presentation or explicit disposal of any newly opened view.
    // Reporting the recording error first would orphan that view in the extension.
    let navigation = apply(effect);
    match (navigation, completion.recording_error) {
        (result, None) => result,
        (Ok(()), Some(error)) => Err(error),
        (Err(navigation), Some(recording)) => Err(format!("{navigation}; {recording}")),
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
