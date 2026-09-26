use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::thread::JoinHandle;

/// Serializes launcher-open scans without duplicate scans or idle polling.
pub(crate) struct LauncherRefresh {
    _wakes: Option<SyncSender<()>>,
    _stopping: Arc<AtomicBool>,
    _busy: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
}

impl LauncherRefresh {
    pub(crate) fn spawn(mut refresh: impl FnMut() + Send + 'static) -> std::io::Result<Self> {
        let (wakes, receiver) = mpsc::sync_channel(1);
        let stopping = Arc::new(AtomicBool::new(false));
        let busy = Arc::new(AtomicBool::new(false));
        let worker_busy = Arc::clone(&busy);
        let worker_stopping = Arc::clone(&stopping);
        let thread = std::thread::Builder::new()
            .name("nanika-launcher-refresh".to_owned())
            .spawn(move || {
                while receiver.recv().is_ok() {
                    if worker_stopping.load(Ordering::Acquire) {
                        break;
                    }
                    refresh();
                    worker_busy.store(false, Ordering::Release);
                }
            })?;
        Ok(Self {
            _wakes: Some(wakes),
            _stopping: stopping,
            _busy: busy,
            _thread: Some(thread),
        })
    }

    pub(crate) fn wake(&self) {
        // An open joins the active scan; it must not schedule another full traversal.
        if self._busy.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Some(wakes) = &self._wakes {
            match wakes.try_send(()) {
                Ok(()) => {}
                Err(TrySendError::Full(())) => {}
                Err(TrySendError::Disconnected(())) => {
                    self._busy.store(false, Ordering::Release);
                    tracing::error!("launcher refresh worker is closed");
                }
            }
        }
    }
}

impl Drop for LauncherRefresh {
    fn drop(&mut self) {
        self._stopping.store(true, Ordering::Release);
        self._wakes.take();
        // The owner requests runtime shutdown first to interrupt process waits.
        if let Some(thread) = self._thread.take()
            && thread.join().is_err()
        {
            tracing::error!("launcher refresh worker panicked");
        }
    }
}

#[cfg(test)]
#[path = "../tests/LauncherRefresh.rs"]
mod tests;
