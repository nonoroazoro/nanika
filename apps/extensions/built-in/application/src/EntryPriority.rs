/// Latest viewport and a coalesced wake shared by discovery and icon preparation.
#[derive(Default)]
pub(crate) struct EntryPriority {
    _generation: u64,
    _entry_ids: Vec<String>,
    _pending: bool,
    _stopped: bool,
    _retry_failed: bool
}

impl EntryPriority {
    pub(crate) fn request(&mut self, generation: u64, entry_ids: Vec<String>) {
        // Catalog commits have their own wake. Identical viewport notifications must
        // not repeatedly interrupt native preparation or rescan the same cache misses.
        if generation >= self._generation
            && !self._stopped
            && (generation != self._generation || entry_ids != self._entry_ids)
        {
            self._generation = generation;
            self._entry_ids = entry_ids;
            self._pending = true;
        }
    }

    pub(crate) fn wake(&mut self) {
        self._pending = true;
    }

    pub(crate) fn retry_failed(&mut self) {
        if !self._stopped {
            self._retry_failed = true;
            self._pending = true;
        }
    }

    pub(crate) fn take_failed_retry(&mut self) -> bool {
        std::mem::take(&mut self._retry_failed)
    }

    pub(crate) fn take(&mut self) -> Vec<String> {
        self._pending = false;
        self._entry_ids.clone()
    }

    pub(crate) fn pending(&self) -> bool {
        self._pending
    }

    pub(crate) fn contains(&self, id: &str) -> bool {
        !self._stopped && self._entry_ids.iter().any(|entry| entry == id)
    }

    pub(crate) fn stop(&mut self) {
        self._stopped = true;
    }

    pub(crate) fn stopped(&self) -> bool {
        self._stopped
    }
}
