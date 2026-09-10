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
        state.session = Some(session);
        drop(state);
        self.wake();
        tracing::debug!(session_id = id, "frontend session opened");
        Ok(ApplicationSnapshot {
            session_id: id,
            locale: nanika_platform::system_locale(),
            max_query_chars: nanika_search::MAX_QUERY_CHARS,
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
        if state
            .session
            .as_ref()
            .is_some_and(|session| session.id == session_id)
        {
            state.session = None;
        }
        drop(state);
        self.wake();
    }

    pub(crate) fn invoke(
        &self,
        request: &InvokeCandidateRequest,
    ) -> Result<
        (
            std::sync::mpsc::Receiver<Result<nanika_host::ExtensionInvocationOutcome, String>>,
            String,
        ),
        String,
    > {
        let state = self
            .shared
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let session = state
            .session
            .as_ref()
            .ok_or("The window session is not open.")?;
        session.authorize(request.session_id)?;
        if session.request_id != request.request_id {
            return Err("Search changed. Select a current result.".to_owned());
        }
        let query = session.query.clone();
        let generation = session.generation;
        let runtime = Arc::clone(state.runtime.as_ref().ok_or("Nanika is still starting.")?);
        drop(state);
        let completion = runtime.invoke(
            generation,
            &request.extension_id,
            &request.entry_id,
            &request.action_id,
            &query,
        )?;
        Ok((completion, query))
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
