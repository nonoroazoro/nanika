/// Latest visible entries and their single queued or active preparation wake.
#[derive(Default)]
pub(crate) struct EntryPriority {
    _generation: u64,
    _entry_ids: Vec<String>,
    _wake_queued: bool,
}

impl EntryPriority {
    pub(crate) fn request(&mut self, generation: u64, entry_ids: Vec<String>) -> bool {
        if generation < self._generation {
            return false;
        }
        self._generation = generation;
        self._entry_ids = entry_ids;
        if self._wake_queued {
            return false;
        }
        self._wake_queued = true;
        true
    }

    pub(crate) fn entries(&self) -> Vec<String> {
        self._entry_ids.clone()
    }

    /// The caller holds the same lock as request admission through wake publication.
    pub(crate) fn finish(&mut self, pending: impl FnOnce(&[String]) -> bool) -> bool {
        self._wake_queued = pending(&self._entry_ids);
        self._wake_queued
    }
}

#[cfg(test)]
#[path = "../tests/EntryPriority.rs"]
mod tests;
