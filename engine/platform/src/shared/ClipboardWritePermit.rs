use crate::ClipboardWriteBudget;
use std::sync::Arc;

pub(crate) struct ClipboardWritePermit {
    pub(crate) budget: Arc<ClipboardWriteBudget>,
    pub(crate) image: bool,
}

impl Drop for ClipboardWritePermit {
    fn drop(&mut self) {
        let mut state = self.budget.state.lock().unwrap_or_else(|e| e.into_inner());
        state.0 -= 1;
        if self.image {
            state.1 = false;
        }
        self.budget.changed.notify_all();
    }
}
