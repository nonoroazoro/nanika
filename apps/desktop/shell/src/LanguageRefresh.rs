use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::thread::JoinHandle;

/// One running read and one pending wake cover notifications arriving during a read.
pub(crate) struct LanguageRefresh {
    _wakes: SyncSender<()>,
    _stopping: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
    _observer: Option<nanika_platform::LanguageChangeObserver>
}

impl LanguageRefresh {
    pub(crate) fn spawn(mut refresh: impl FnMut() + Send + 'static) -> std::io::Result<Self> {
        let (wakes, receiver) = mpsc::sync_channel(1);
        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stopping = Arc::clone(&stopping);
        let thread = std::thread::Builder::new()
            .name("nanika-language-refresh".into())
            .spawn(move || {
                while receiver.recv().is_ok() {
                    if worker_stopping.load(Ordering::Acquire) {
                        break;
                    }
                    refresh();
                }
            })?;
        Ok(Self {
            _wakes: wakes,
            _stopping: stopping,
            _thread: Some(thread),
            _observer: None
        })
    }

    /// The Windows observer must attach to the desktop event thread.
    pub(crate) fn observe_system(&mut self) -> Result<(), String> {
        self._observer = Some(nanika_platform::LanguageChangeObserver::install(
            self._wakes.clone()
        )?);
        Ok(())
    }

    pub(crate) fn wake(&self) {
        let _ = self._wakes.try_send(());
    }
}

impl Drop for LanguageRefresh {
    fn drop(&mut self) {
        self._stopping.store(true, Ordering::Release);
        self._observer.take();
        self.wake();
        if let Some(thread) = self._thread.take()
            && thread.join().is_err()
        {
            tracing::error!("language refresh worker panicked");
        }
    }
}

#[cfg(test)]
#[path = "../tests/LanguageRefresh.rs"]
mod tests;
