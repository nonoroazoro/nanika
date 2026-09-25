use std::sync::{Arc, Mutex};

type ExitObserver = Arc<dyn Fn(String) + Send + Sync>;

/// One terminal transport event, retained if it precedes worker subscription.
#[derive(Default)]
pub(crate) struct ExtensionConnectionExit {
    _state: Mutex<(Option<String>, Option<ExitObserver>)>,
}

impl ExtensionConnectionExit {
    pub(crate) fn observe(&self, observer: ExitObserver) {
        let error = {
            let mut state = self
                ._state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            state.1 = Some(Arc::clone(&observer));
            state.0.clone()
        };
        if let Some(error) = error {
            observer(error);
        }
    }

    pub(crate) fn finish(&self, error: String) {
        let observer = {
            let mut state = self
                ._state
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if state.0.is_some() {
                return;
            }
            state.0 = Some(error.clone());
            state.1.clone()
        };
        if let Some(observer) = observer {
            observer(error);
        }
    }
}
