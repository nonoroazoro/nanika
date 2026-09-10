use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;

use clipboard_rs::ClipboardContext;

use crate::{ClipboardCommand, ClipboardDatabase, ClipboardEntry, capture};

/// Single owner for clipboard capture and SQLite writes.
pub struct ClipboardWorker {
    commands: SyncSender<ClipboardCommand>,
    last_error: Arc<Mutex<Option<String>>>,
    thread: Option<JoinHandle<()>>,
}

impl ClipboardWorker {
    pub fn spawn(
        database_path: PathBuf,
        payload_root: PathBuf,
        entries: Arc<RwLock<Vec<ClipboardEntry>>>,
    ) -> Result<Self, String> {
        let (commands, receiver) = mpsc::sync_channel(8);
        let (ready, initialized) = mpsc::sync_channel(1);
        let last_error = Arc::new(Mutex::new(None));
        let worker_error = Arc::clone(&last_error);
        let thread = std::thread::Builder::new()
            .name("nanika-clipboard-owner".to_owned())
            .spawn(move || {
                let database = match ClipboardDatabase::open(database_path) {
                    Ok(database) => database,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                let context = match ClipboardContext::new() {
                    Ok(context) => context,
                    Err(error) => {
                        let _ = ready.send(Err(error.to_string()));
                        return;
                    }
                };
                let initial = database.load();
                let Ok(initial) = initial else {
                    let _ = ready.send(initial);
                    return;
                };
                *entries.write().unwrap_or_else(|error| error.into_inner()) = initial;
                if ready.send(Ok(Vec::new())).is_err() {
                    return;
                }
                while let Ok(command) = receiver.recv() {
                    match command {
                        ClipboardCommand::Capture { response } => {
                            let result = capture(&context, &payload_root, unix_timestamp())
                                .and_then(|entry| {
                                    if let Some(entry) = entry {
                                        database.upsert(&entry)?;
                                        let loaded = database.load()?;
                                        *entries
                                            .write()
                                            .unwrap_or_else(|error| error.into_inner()) = loaded;
                                    }
                                    Ok(())
                                });
                            *worker_error
                                .lock()
                                .unwrap_or_else(|error| error.into_inner()) = result.clone().err();
                            if let Some(response) = response
                                && response.send(result.clone()).is_err()
                            {
                                eprintln!(
                                    "clipboard capture requester closed before receiving result"
                                );
                            }
                        }
                        ClipboardCommand::MarkUsed {
                            entry_id,
                            used_at,
                            response,
                        } => {
                            let result = database.mark_used(&entry_id, used_at);
                            if let Err(error) = &result {
                                *worker_error
                                    .lock()
                                    .unwrap_or_else(|error| error.into_inner()) =
                                    Some(error.clone());
                            }
                            if response.send(result).is_err() {
                                eprintln!(
                                    "clipboard history requester closed before receiving result"
                                );
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
            last_error,
            thread: Some(thread),
        })
    }

    pub(crate) fn command_sender(&self) -> SyncSender<ClipboardCommand> {
        self.commands.clone()
    }

    pub fn capture(&self) -> Result<Receiver<Result<(), String>>, String> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .send(ClipboardCommand::Capture {
                response: Some(response),
            })
            .map_err(|_| "clipboard capture owner is closed".to_owned())?;
        Ok(result)
    }

    pub fn capture_background(&self) -> Result<(), String> {
        self.commands
            .send(ClipboardCommand::Capture { response: None })
            .map_err(|_| "clipboard capture owner is closed".to_owned())
    }

    pub fn mark_used(&self, entry_id: impl Into<String>) -> Result<(), String> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .send(ClipboardCommand::MarkUsed {
                entry_id: entry_id.into(),
                used_at: unix_timestamp(),
                response,
            })
            .map_err(|_| "clipboard capture owner is closed".to_owned())?;
        result
            .recv()
            .map_err(|_| "clipboard capture owner closed without reporting the update".to_owned())?
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        if self.thread.is_none() {
            return;
        }
        if self.commands.send(ClipboardCommand::Shutdown).is_err() {
            eprintln!("clipboard worker closed before shutdown was requested");
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            eprintln!("clipboard worker panicked");
        }
    }
}

impl Drop for ClipboardWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
