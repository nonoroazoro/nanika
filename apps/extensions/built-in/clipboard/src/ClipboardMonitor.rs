use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use clipboard_rs::{ClipboardWatcher, ClipboardWatcherContext, WatcherShutdown};

use crate::{ClipboardCommand, ClipboardWatcherHandler, ClipboardWorker};

#[derive(Default)]
pub(crate) struct ClipboardCaptureGate {
    state: ClipboardCaptureGateState,
}

// Native change delivery can race the host response and one write can expose
// intermediate revisions. Keep the gate armed until the exact completed write
// revision is observed; only a later revision is external capture work.
#[derive(Default)]
enum ClipboardCaptureGateState {
    #[default]
    Watching,
    Writing {
        latest_observed_revision: Option<u64>,
    },
    AwaitingWriteRevision {
        revision: u64,
    },
}

impl ClipboardCaptureGate {
    fn begin_write(&mut self) {
        self.state = ClipboardCaptureGateState::Writing {
            latest_observed_revision: None,
        };
    }

    pub(crate) fn observe(&mut self, revision: u64) -> bool {
        match &mut self.state {
            ClipboardCaptureGateState::Watching => true,
            ClipboardCaptureGateState::Writing {
                latest_observed_revision,
            } => {
                *latest_observed_revision = Some(revision);
                false
            }
            ClipboardCaptureGateState::AwaitingWriteRevision {
                revision: write_revision,
            } => {
                if revision == *write_revision {
                    self.state = ClipboardCaptureGateState::Watching;
                    false
                } else if nanika_platform::clipboard_revision_is_after(revision, *write_revision) {
                    self.state = ClipboardCaptureGateState::Watching;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn complete_write(&mut self, revision: u64) -> bool {
        let ClipboardCaptureGateState::Writing {
            latest_observed_revision,
        } = self.state
        else {
            self.state = ClipboardCaptureGateState::AwaitingWriteRevision { revision };
            return false;
        };
        match latest_observed_revision {
            Some(observed) if observed == revision => {
                self.state = ClipboardCaptureGateState::Watching;
                false
            }
            Some(observed) if nanika_platform::clipboard_revision_is_after(observed, revision) => {
                self.state = ClipboardCaptureGateState::Watching;
                true
            }
            _ => {
                self.state = ClipboardCaptureGateState::AwaitingWriteRevision { revision };
                false
            }
        }
    }

    fn cancel_write(&mut self) -> bool {
        let should_capture = matches!(
            self.state,
            ClipboardCaptureGateState::Writing {
                latest_observed_revision: Some(_)
            }
        );
        self.state = ClipboardCaptureGateState::Watching;
        should_capture
    }
}

/// Native change source owned by one blocking extension thread.
pub struct ClipboardMonitor {
    commands: SyncSender<ClipboardCommand>,
    capture_gate: Arc<Mutex<ClipboardCaptureGate>>,
    shutdown: Option<WatcherShutdown>,
    thread: Option<JoinHandle<()>>,
}

impl ClipboardMonitor {
    pub fn spawn(worker: &ClipboardWorker) -> Result<Self, String> {
        let commands = worker.command_sender();
        let capture_gate = Arc::new(Mutex::new(ClipboardCaptureGate::default()));
        let handler_gate = Arc::clone(&capture_gate);
        let handler_commands = commands.clone();
        let (ready, receiver) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("nanika-clipboard-events".to_owned())
            .spawn(move || {
                let result = ClipboardWatcherContext::new_with_interval(Duration::from_millis(250))
                    .map(|mut watcher| {
                        let shutdown = watcher
                            .add_handler(ClipboardWatcherHandler {
                                commands: handler_commands,
                                capture_gate: handler_gate,
                            })
                            .get_shutdown_channel();
                        let _ = ready.send(Ok(shutdown));
                        watcher.start_watch();
                    })
                    .map_err(|error| error.to_string());
                if let Err(error) = result {
                    let _ = ready.send(Err(error));
                }
            })
            .map_err(|error| error.to_string())?;
        let shutdown = receiver
            .recv()
            .map_err(|_| "clipboard watcher closed during initialization".to_owned())??;
        Ok(Self {
            commands,
            capture_gate,
            shutdown: Some(shutdown),
            thread: Some(thread),
        })
    }

    pub fn begin_internal_write(&self) {
        self.capture_gate
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .begin_write();
    }

    pub fn complete_internal_write(&self, revision: u64) -> Result<(), String> {
        let should_capture = self
            .capture_gate
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .complete_write(revision);
        self.capture_if_needed(should_capture)
    }

    pub fn cancel_internal_write(&self) -> Result<(), String> {
        let should_capture = self
            .capture_gate
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .cancel_write();
        self.capture_if_needed(should_capture)
    }

    fn capture_if_needed(&self, should_capture: bool) -> Result<(), String> {
        if !should_capture {
            return Ok(());
        }
        self.commands
            .send(ClipboardCommand::Capture)
            .map_err(|_| "clipboard capture owner is closed".to_owned())
    }

    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        if self.thread.is_none() {
            return;
        }
        self.shutdown.take();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            eprintln!("clipboard watcher thread panicked");
        }
    }
}

impl Drop for ClipboardMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::ClipboardCaptureGate;

    #[test]
    fn internal_revision_is_ignored_and_next_external_revision_is_captured() {
        let mut gate = ClipboardCaptureGate::default();
        gate.begin_write();
        assert!(!gate.complete_write(11));
        assert!(!gate.observe(11));
        assert!(gate.observe(12));
    }

    #[test]
    fn intermediate_write_revision_does_not_escape_the_gate() {
        let mut gate = ClipboardCaptureGate::default();
        gate.begin_write();
        assert!(!gate.observe(20));
        assert!(!gate.complete_write(21));
        assert!(!gate.observe(21));
        assert!(gate.observe(22));
    }

    #[test]
    fn external_revision_observed_before_the_receipt_is_captured() {
        let mut gate = ClipboardCaptureGate::default();
        gate.begin_write();
        assert!(!gate.observe(31));
        assert!(gate.complete_write(30));
    }

    #[test]
    fn failed_write_releases_an_observed_change_for_capture() {
        let mut gate = ClipboardCaptureGate::default();
        gate.begin_write();
        assert!(!gate.observe(40));
        assert!(gate.cancel_write());
        assert!(gate.observe(41));
    }
}
