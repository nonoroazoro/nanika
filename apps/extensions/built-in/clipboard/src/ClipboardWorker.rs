use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, SyncSender};
use std::thread::JoinHandle;

use clipboard_rs::ClipboardContext;

use crate::{
    ClipboardCommand, ClipboardConfig, ClipboardContentLease, ClipboardEntry, ClipboardPayloads,
    ClipboardStore, capture
};

/// Single owner for clipboard capture and database writes.
pub struct ClipboardWorker {
    commands: SyncSender<ClipboardCommand>,
    thread: Option<JoinHandle<()>>,
}

impl ClipboardWorker {
    pub fn spawn(
        database_path: PathBuf,
        payload_root: PathBuf,
        config: ClipboardConfig,
        invalidated: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, String> {
        let context = ClipboardContext::new().map_err(|error| error.to_string())?;
        Self::_spawn(
            database_path,
            payload_root,
            config,
            invalidated,
            move |root| capture(&context, root, unix_timestamp_millis())
        )
    }

    pub(crate) fn command_sender(&self) -> SyncSender<ClipboardCommand> {
        self.commands.clone()
    }

    pub fn clear(&self, entry_ids: Arc<Vec<String>>) -> Result<(), String> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .send(ClipboardCommand::Clear {
                entry_ids,
                response
            })
            .map_err(|_| "clipboard capture owner is closed".to_owned())?;
        result
            .recv()
            .map_err(|_| "clipboard owner closed without reporting the clear result".to_owned())?
    }

    pub fn apply_retention(&self, config: ClipboardConfig) -> Result<(), String> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .send(ClipboardCommand::ApplyRetention { config, response })
            .map_err(|_| "clipboard capture owner is closed".to_owned())?;
        result.recv().map_err(|_| {
            "clipboard owner closed without reporting the retention result".to_owned()
        })?
    }

    pub fn close_view(&self) -> Result<(), String> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .send(ClipboardCommand::CloseView { response })
            .map_err(|_| "clipboard owner is closed".to_owned())?;
        result
            .recv()
            .map_err(|_| "clipboard owner closed without releasing the view".to_owned())
    }

    pub fn present(
        &self,
        state: crate::ClipboardViewState,
        expected_revision: Option<u64>
    ) -> Result<crate::ClipboardPresentation, String> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .send(ClipboardCommand::Present {
                state,
                expected_revision,
                response
            })
            .map_err(|_| "clipboard owner is closed".to_owned())?;
        result
            .recv()
            .map_err(|_| "clipboard owner closed without a view result".to_owned())?
    }

    pub fn content(&self, entry_id: String) -> Result<ClipboardContentLease<'_>, String> {
        // Transfer only to a waiting reader; an abandoned response cannot strand a lease.
        let (response, result) = mpsc::sync_channel(0);
        self.commands
            .send(ClipboardCommand::Content { entry_id, response })
            .map_err(|_| "clipboard owner is closed".to_owned())?;
        let content = result
            .recv()
            .map_err(|_| "clipboard owner closed without a content result".to_owned())??;
        Ok(ClipboardContentLease::new(content, &self.commands))
    }

    pub fn shutdown(mut self) -> Result<(), String> {
        self._stop()
    }

    fn _spawn(
        database_path: PathBuf,
        payload_root: PathBuf,
        mut config: ClipboardConfig,
        invalidated: Arc<dyn Fn() + Send + Sync>,
        mut capture: impl FnMut(&std::path::Path) -> Result<Option<ClipboardEntry>, String>
        + Send
        + 'static
    ) -> Result<Self, String> {
        let (commands, receiver) = mpsc::sync_channel(8);
        let (ready, initialized) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("nanika-clipboard-owner".to_owned())
            .spawn(move || {
                let mut store = match ClipboardStore::open(database_path) {
                    Ok(store) => store,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                let mut payloads = ClipboardPayloads::new(payload_root.clone());
                let initial = store.apply_retention(unix_timestamp_millis(), &config)
                    .and_then(|_| store.retained_images()).and_then(|retained| payloads.reconcile(retained));
                if initial.is_err() { let _ = ready.send(initial); return; }
                if ready.send(Ok(())).is_err() { return; }
                while let Ok(command) = receiver.recv() {
                    match command {
                        ClipboardCommand::Capture => {
                            let result = capture(&payload_root).and_then(|entry| {
                                let Some(entry) = entry else { return Ok(()); };
                                let change = store.capture(&entry, unix_timestamp_millis(), &config)?;
                                invalidated();
                                payloads.apply(change)
                            });
                            if let Err(error) = &result {
                                eprintln!("clipboard capture failed: {error}");
                            }
                        }
                        ClipboardCommand::Clear { entry_ids, response } => {
                            let result = clear_entries(&mut store, &mut payloads, &entry_ids, &*invalidated);
                            if response.send(result).is_err() {
                                eprintln!(
                                    "clipboard clear requester closed before receiving result"
                                );
                            }
                        }
                        ClipboardCommand::ApplyRetention {
                            config: updated,
                            response,
                        } => {
                            let result = store.apply_retention(unix_timestamp_millis(), &updated).map(|change| {
                                invalidated();
                                config = updated;
                                if let Err(error) = payloads.apply(change) {
                                    // Retention has committed; cleanup cannot reject effective settings.
                                    eprintln!("clipboard payload cleanup failed: {error}");
                                }
                            });
                            if response.send(result).is_err() {
                                eprintln!(
                                    "clipboard configuration requester closed before receiving result"
                                );
                            }
                        }
                        ClipboardCommand::CloseView { response } => {
                            store.close_view();
                            let _ = response.send(());
                        }
                        ClipboardCommand::Present { state, expected_revision, response } => {
                            let _ = response.send(store.present(state, expected_revision));
                        }
                        ClipboardCommand::Content { entry_id, response } => {
                            let content = store.content(&entry_id).and_then(|content| {
                                payloads.acquire(&content)?;
                                Ok(content)
                            });
                            if let Err(mpsc::SendError(Ok(nanika_protocol::ClipboardContent::PngFile { path }))) = response.send(content)
                                && let Err(error) = payloads.release(std::path::Path::new(&path))
                            {
                                eprintln!("clipboard payload release failed: {error}");
                            }
                        }
                        ClipboardCommand::ReleaseContent { path } => {
                            if let Err(error) = payloads.release(&path) {
                                eprintln!("clipboard payload release failed: {error}");
                            }
                        }
                        ClipboardCommand::Shutdown => break,
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        initialized
            .recv()
            .map_err(|_| "clipboard owner closed during initialization".to_owned())??;
        Ok(Self {
            commands,
            thread: Some(thread),
        })
    }

    fn _stop(&mut self) -> Result<(), String> {
        if self.thread.is_none() {
            return Ok(());
        }
        let sent = self.commands.send(ClipboardCommand::Shutdown).is_ok();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            return Err("clipboard worker panicked".to_owned());
        }
        if !sent {
            return Err("clipboard worker closed before shutdown was requested".to_owned());
        }
        // Capture and command errors do not describe owner cleanup.
        Ok(())
    }
}

fn clear_entries(
    store: &mut ClipboardStore,
    payloads: &mut ClipboardPayloads,
    entry_ids: &[String],
    invalidated: &dyn Fn(),
) -> Result<(), String> {
    let change = store.clear(entry_ids)?;
    // The owner has committed and updated its query cache before notification/cleanup.
    invalidated();
    payloads
        .apply(change)
        .map_err(|error| format!("Entries were removed; payload cleanup failed: {error}"))
}

impl Drop for ClipboardWorker {
    fn drop(&mut self) {
        if let Err(error) = self._stop() {
            eprintln!("{error}");
        }
    }
}

fn unix_timestamp_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
#[path = "../tests/ClipboardWorker.rs"]
mod tests;
