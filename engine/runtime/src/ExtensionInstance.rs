use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

/// Host-owned identity and publication/admission barrier for one process lifetime.
pub(crate) struct ExtensionInstance {
    pub(crate) id: u64,
    _active: Mutex<bool>,
}

impl ExtensionInstance {
    pub(crate) fn new() -> Self {
        Self {
            id: NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed),
            _active: Mutex::new(true),
        }
    }

    pub(crate) fn with_active<T>(&self, publish: impl FnOnce() -> T) -> Option<T> {
        let active = self
            ._active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if *active { Some(publish()) } else { None }
    }

    pub(crate) fn is_active(&self) -> bool {
        *self
            ._active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    pub(crate) fn retire(
        &self,
        search: &nanika_search::SearchHandle,
        extension_id: &str,
    ) -> Result<(), String> {
        let mut active = self
            ._active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *active = false;
        // Every earlier publication is queued before withdrawal; none can follow it.
        search
            .remove_extension(extension_id)
            .map_err(|error| error.to_string())
    }
}
