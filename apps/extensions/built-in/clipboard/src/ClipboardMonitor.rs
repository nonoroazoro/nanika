use std::thread::JoinHandle;
use std::time::Duration;

use clipboard_rs::{ClipboardWatcher, ClipboardWatcherContext, WatcherShutdown};

use crate::{ClipboardWatcherHandler, ClipboardWorker};

/// Native change source owned by one blocking extension thread.
pub struct ClipboardMonitor {
    shutdown: Option<WatcherShutdown>,
    thread: Option<JoinHandle<()>>,
}

impl ClipboardMonitor {
    pub fn spawn(worker: &ClipboardWorker) -> Result<Self, String> {
        let commands = worker.command_sender();
        let suppress_next_change = worker.capture_suppression();
        let (ready, receiver) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("nanika-clipboard-events".to_owned())
            .spawn(move || {
                let result = ClipboardWatcherContext::new_with_interval(Duration::from_millis(250))
                    .map(|mut watcher| {
                        let shutdown = watcher
                            .add_handler(ClipboardWatcherHandler {
                                commands,
                                suppress_next_change,
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
            shutdown: Some(shutdown),
            thread: Some(thread),
        })
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
