use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::RootSearchSnapshot;

pub(crate) struct DesktopState {
    next_session_id: AtomicU64,
    generation: AtomicU64,
    query: Mutex<String>,
    runtime: Mutex<Option<nanika_host::RuntimeService>>,
    runtime_notifier: Mutex<Option<std::sync::Arc<dyn Fn() + Send + Sync>>>,
    search_updates: Mutex<Option<tauri::ipc::Channel<RootSearchSnapshot>>>,
    instance: Mutex<Option<nanika_platform::SingleInstance>>,
    _diagnostics: nanika_host::Diagnostics,
    _hotkey_timing: Option<nanika_platform::HotkeyTimingObserver>,
}

impl DesktopState {
    pub(crate) fn new(
        instance: nanika_platform::SingleInstance,
        diagnostics: nanika_host::Diagnostics,
    ) -> Self {
        Self {
            next_session_id: AtomicU64::new(1),
            generation: AtomicU64::new(0),
            query: Mutex::new(String::new()),
            runtime: Mutex::new(None),
            runtime_notifier: Mutex::new(None),
            search_updates: Mutex::new(None),
            instance: Mutex::new(Some(instance)),
            _diagnostics: diagnostics,
            _hotkey_timing: nanika_platform::HotkeyTimingObserver::install(),
        }
    }

    pub(crate) fn next_session_id(&self) -> u64 {
        self.next_session_id.fetch_add(1, Ordering::Relaxed)
    }

    pub(crate) fn current_search(&self) -> RootSearchSnapshot {
        let query = self
            .query
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let generation = self.generation.load(Ordering::Acquire);
        let snapshot = self
            .runtime
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let snapshot = snapshot
            .as_ref()
            .and_then(nanika_host::RuntimeService::latest_snapshot)
            .filter(|snapshot| snapshot.generation == generation);
        match snapshot {
            Some(snapshot) => RootSearchSnapshot::from_engine(&snapshot, query),
            None => RootSearchSnapshot::pending(generation, query),
        }
    }

    pub(crate) fn publish_query(&self, query: String) -> Result<RootSearchSnapshot, String> {
        *self.query.lock().unwrap_or_else(|error| error.into_inner()) = query.clone();
        let runtime = self
            .runtime
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(runtime) = runtime.as_ref() else {
            return Ok(RootSearchSnapshot::pending(0, query));
        };
        let generation = runtime.begin_query(query.clone())?;
        self.generation.store(generation, Ordering::Release);
        Ok(RootSearchSnapshot::pending(generation, query))
    }

    pub(crate) fn ensure_search(&self) -> Result<RootSearchSnapshot, String> {
        if self.generation.load(Ordering::Acquire) == 0 {
            let query = self
                .query
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
            self.publish_query(query)
        } else {
            Ok(self.current_search())
        }
    }

    pub(crate) fn invoke(
        &self,
        extension_id: &str,
        entry_id: &str,
        action_id: &str,
    ) -> Result<bool, String> {
        let query = self
            .query
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        self.runtime
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .ok_or_else(|| "Nanika is still starting".to_owned())?
            .invoke(
                self.generation.load(Ordering::Acquire),
                extension_id,
                entry_id,
                action_id,
                &query,
            )
    }

    pub(crate) fn set_notifier(&self, notifier: std::sync::Arc<dyn Fn() + Send + Sync>) {
        *self
            .runtime_notifier
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(std::sync::Arc::clone(&notifier));
        if let Some(runtime) = self
            .runtime
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
        {
            runtime.set_notifier(notifier);
        }
    }

    pub(crate) fn install_runtime(
        &self,
        runtime: nanika_host::RuntimeService,
    ) -> Result<(), String> {
        if let Some(notifier) = self
            .runtime_notifier
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
        {
            runtime.set_notifier(notifier);
        }
        *self
            .runtime
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(runtime);
        let query = self
            .query
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        self.publish_query(query)?;
        self.notify_search_updates();
        Ok(())
    }

    pub(crate) fn bind_search_updates(&self, updates: tauri::ipc::Channel<RootSearchSnapshot>) {
        *self
            .search_updates
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(updates);
    }

    pub(crate) fn notify_search_updates(&self) {
        if self.generation.load(Ordering::Acquire) == 0 {
            return;
        }
        let snapshot = self.current_search();
        if let Some(updates) = self
            .search_updates
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
        {
            let _ = updates.send(snapshot);
        }
    }
}

impl Drop for DesktopState {
    fn drop(&mut self) {
        let _ = self
            .instance
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
    }
}
