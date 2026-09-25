use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;

use clipboard_rs::ClipboardContext;

use crate::{ClipboardCommand, ClipboardConfig, ClipboardDatabase, ClipboardEntry, capture};

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
        entries: Arc<RwLock<Vec<ClipboardEntry>>>,
        invalidated: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, String> {
        let context = ClipboardContext::new().map_err(|error| error.to_string())?;
        Self::_spawn(
            database_path,
            payload_root,
            config,
            entries,
            invalidated,
            move |root| capture(&context, root, unix_timestamp_millis()),
        )
    }

    pub(crate) fn command_sender(&self) -> SyncSender<ClipboardCommand> {
        self.commands.clone()
    }

    pub fn clear(&self, entry_ids: Vec<String>) -> Result<(), String> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .send(ClipboardCommand::Clear {
                entry_ids,
                response,
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

    pub fn shutdown(mut self) -> Result<(), String> {
        self._stop()
    }

    fn _spawn(
        database_path: PathBuf,
        payload_root: PathBuf,
        mut config: ClipboardConfig,
        entries: Arc<RwLock<Vec<ClipboardEntry>>>,
        invalidated: Arc<dyn Fn() + Send + Sync>,
        mut capture: impl FnMut(&std::path::Path) -> Result<Option<ClipboardEntry>, String>
        + Send
        + 'static,
    ) -> Result<Self, String> {
        let (commands, receiver) = mpsc::sync_channel(8);
        let (ready, initialized) = mpsc::sync_channel(1);
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
                let initial = database
                    .apply_retention(unix_timestamp_millis(), &config)
                    .and_then(|retained| reconcile_payloads(&payload_root, &retained))
                    .and_then(|()| database.load());
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
                        ClipboardCommand::Capture => {
                            let result = capture(&payload_root)
                                .and_then(|entry| {
                                    let changed = entry.is_some();
                                    if let Some(entry) = entry {
                                        let retained = database.upsert_with_retention(
                                            &entry,
                                            unix_timestamp_millis(),
                                            &config,
                                        )?;
                                        reconcile_payloads(&payload_root, &retained)?;
                                        let loaded = database.load()?;
                                        *entries
                                            .write()
                                            .unwrap_or_else(|error| error.into_inner()) = loaded;
                                    }
                                    Ok(changed)
                                });
                            if let Err(error) = &result {
                                eprintln!("clipboard capture failed: {error}");
                            }
                            if result.as_ref().is_ok_and(|changed| *changed) {
                                invalidated();
                            }
                        }
                        ClipboardCommand::Clear { entry_ids, response } => {
                            let result = clear_entries(&database, &payload_root, &entries, &entry_ids);
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
                            let result = database
                                .apply_retention(unix_timestamp_millis(), &updated)
                                .and_then(|retained| reconcile_payloads(&payload_root, &retained))
                                .and_then(|()| database.load())
                                .map(|loaded| {
                                    config = updated;
                                    *entries.write().unwrap_or_else(|error| error.into_inner()) =
                                        loaded;
                                });
                            if response.send(result).is_err() {
                                eprintln!(
                                    "clipboard configuration requester closed before receiving result"
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
    database: &ClipboardDatabase,
    payload_root: &std::path::Path,
    entries: &RwLock<Vec<ClipboardEntry>>,
    entry_ids: &[String],
) -> Result<(), String> {
    let retained = database.clear(entry_ids)?;
    // The transaction has committed. Publish that state even if orphan cleanup later fails.
    *entries.write().unwrap_or_else(|error| error.into_inner()) = database.load()?;
    reconcile_payloads(payload_root, &retained)
}

fn reconcile_payloads(
    payload_root: &std::path::Path,
    retained: &std::collections::HashSet<PathBuf>,
) -> Result<(), String> {
    let entries = match std::fs::read_dir(payload_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
            && is_generated_payload(&path)
            && !retained.contains(&path)
        {
            std::fs::remove_file(&path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn is_generated_payload(path: &std::path::Path) -> bool {
    path.extension().is_some_and(|extension| extension == "png")
        && path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| {
                stem.len() == 64 && stem.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
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
