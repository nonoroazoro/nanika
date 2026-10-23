use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering}
};

/// Reserve before scheduling so resource requests cannot create an unbounded blocking queue.
pub(crate) struct LocalizationReadPermit {
    _pending: Arc<AtomicUsize>
}

impl LocalizationReadPermit {
    pub(crate) fn acquire(pending: Arc<AtomicUsize>) -> Result<Self, nanika_host::OperationError> {
        pending
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                (count < nanika_localization::MAX_RESOURCE_LOADS).then(|| count + 1)
            })
            .map_err(|_| {
                nanika_host::OperationError::new(nanika_host::OperationErrorCode::LocalizationBusy)
            })?;
        Ok(Self { _pending: pending })
    }
}

impl Drop for LocalizationReadPermit {
    fn drop(&mut self) {
        self._pending.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
#[path = "../tests/LocalizationReadPermit.rs"]
mod tests;
