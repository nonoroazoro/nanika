use crate::ClipboardWritePermit;
use std::sync::{Arc, Condvar, Mutex};

/// Preparation, queueing and execution share the same bounded admission budget.
#[derive(Default)]
pub(crate) struct ClipboardWriteBudget {
    pub(crate) state: Mutex<(usize, bool)>,
    pub(crate) changed: Condvar,
}

impl ClipboardWriteBudget {
    pub(crate) fn acquire(
        self: &Arc<Self>,
        image: bool,
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<ClipboardWritePermit, String> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        // One PNG credit bounds encoded-image residency across preparation,
        // queued admission and native execution.
        loop {
            if cancelled() {
                return Err("clipboard preparation was interrupted before admission".into());
            }
            if state.0 < 8 && !(image && state.1) {
                break;
            }
            // This is a cancellation observation interval, never an expiry.
            state = self
                .changed
                .wait_timeout(state, std::time::Duration::from_millis(25))
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
        state.0 += 1;
        state.1 |= image;
        Ok(ClipboardWritePermit {
            budget: Arc::clone(self),
            image,
        })
    }
}
