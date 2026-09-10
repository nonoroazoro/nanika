use crate::ExtensionSearchState;
use std::sync::{Arc, Condvar, Mutex};

/// Wake admission waiters even when initialization fails or the worker panics.
pub(crate) struct ExtensionWorkerLifetime(pub(crate) Arc<(Mutex<ExtensionSearchState>, Condvar)>);

impl Drop for ExtensionWorkerLifetime {
    fn drop(&mut self) {
        let (lock, changed) = &*self.0;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        state.closed = true;
        for invocation in state.invocations.drain(..) {
            let _ = invocation.response.send(Err(
                "extension worker closed before executing the action".to_owned(),
            ));
        }
        changed.notify_all();
    }
}
