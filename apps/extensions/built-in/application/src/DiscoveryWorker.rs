use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::JoinHandle;

use crate::{
    ApplicationConfig, ApplicationDatabase, ApplicationEntry, ApplicationIndex, DiscoveryCommand,
    DiscoveryServices, EntryPriority, IconWorker, RuntimeEvent,
};

/// Named owner for filesystem discovery and application database writes.
pub struct DiscoveryWorker {
    commands: Sender<DiscoveryCommand>,
    cancelled_through: Arc<AtomicU64>,
    _icons: IconWorker,
    thread: Option<JoinHandle<()>>,
}

impl DiscoveryWorker {
    pub fn spawn(
        database_path: PathBuf,
        icon_root: PathBuf,
        config: Arc<RwLock<ApplicationConfig>>,
        entries: Arc<RwLock<std::collections::HashMap<String, ApplicationEntry>>>,
        events: SyncSender<RuntimeEvent>,
    ) -> std::io::Result<Self> {
        let (commands, receiver) = mpsc::channel();
        let cancelled_through = Arc::new(AtomicU64::new(0));
        let worker_cancellation = Arc::clone(&cancelled_through);
        let mut icons = IconWorker::spawn(icon_root, Arc::clone(&entries), events.clone())?;
        let icon_wake = icons.wake_handle();
        let thread = std::thread::Builder::new()
            .name("nanika-application-discovery".to_owned())
            .spawn(move || {
                let mut index = match ApplicationDatabase::open(&database_path)
                    .map(ApplicationIndex::new)
                {
                    Ok(index) => index,
                    Err(error) => {
                        if events
                            .send(RuntimeEvent::ScanFinished {
                            request_id: None,
                            response_generation: 1,
                            result: Err(error.to_string()),
                            })
                            .is_err()
                        {
                            eprintln!(
                                "application runtime closed before receiving database initialization failure: {error}"
                            );
                        }
                        return;
                    }
                };
                match index.load() {
                    Ok(loaded) => publish_entries(&entries, &events, &icon_wake, loaded, Vec::new()),
                    Err(error) => {
                        send_failure(&events, None, 1, &error);
                        return;
                    }
                }
                let services = DiscoveryServices {
                    config: &config,
                    entries: &entries,
                    events: &events,
                    cancelled_through: &worker_cancellation,
                    icon_wake: &icon_wake,
                };
                index = match run_scan(index, &services, None, 1) {
                    Some(index) => index,
                    None => return,
                };
                while let Ok(command) = receiver.recv() {
                    match command {
                        DiscoveryCommand::Refresh {
                            request_id,
                            generation,
                        } => {
                            index = match run_scan(
                                index,
                                &services,
                                request_id,
                                generation,
                            ) {
                                Some(index) => index,
                                None => return,
                            };
                        }
                        DiscoveryCommand::Shutdown => break,
                    }
                }
            });
        let thread = match thread {
            Ok(thread) => thread,
            Err(error) => {
                icons.stop();
                let _ = icons.join();
                return Err(error);
            }
        };
        Ok(Self {
            commands,
            cancelled_through,
            _icons: icons,
            thread: Some(thread),
        })
    }

    pub fn refresh(&self, request_id: Option<String>, generation: u64) -> Result<(), String> {
        self.commands
            .send(DiscoveryCommand::Refresh {
                request_id,
                generation,
            })
            .map_err(|_| "application discovery worker is closed".to_owned())
    }

    pub fn cancel(&self, generation: u64) {
        self.cancelled_through
            .fetch_max(generation, Ordering::AcqRel);
    }

    pub fn prepare_entries(&self, generation: u64, entry_ids: Vec<String>) {
        self._icons.prepare_entries(generation, entry_ids);
    }

    pub fn shutdown(mut self, events: mpsc::Receiver<RuntimeEvent>) -> Result<(), String> {
        self.cancel(u64::MAX);
        self._icons.stop();
        let _ = self.commands.send(DiscoveryCommand::Shutdown);
        // Keep consuming the bounded event queue while the owner drains. Joining
        // first can deadlock on a final progress or database failure publication.
        let mut failure = None;
        while let Ok(event) = events.recv() {
            if let RuntimeEvent::ScanFinished {
                result: Err(error), ..
            } = event
            {
                failure.get_or_insert(error);
            }
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            failure.get_or_insert("application discovery worker panicked".to_owned());
        }
        if let Err(error) = self._icons.join() {
            failure.get_or_insert(error);
        }
        failure.map_or(Ok(()), Err)
    }

    fn _stop(&mut self) -> Result<(), String> {
        self.cancel(u64::MAX);
        self._icons.stop();
        let sent = self.commands.send(DiscoveryCommand::Shutdown).is_ok();
        let mut failure = None;
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                failure = Some("application discovery worker panicked".to_owned());
            } else if !sent {
                failure = Some(
                    "application discovery worker closed before shutdown was requested".to_owned(),
                );
            }
        }
        if let Err(error) = self._icons.join() {
            failure.get_or_insert(error);
        }
        failure.map_or(Ok(()), Err)
    }
}

impl Drop for DiscoveryWorker {
    fn drop(&mut self) {
        if let Err(error) = self._stop() {
            eprintln!("{error}");
        }
    }
}

fn run_scan(
    mut index: ApplicationIndex,
    services: &DiscoveryServices<'_>,
    request_id: Option<String>,
    generation: u64,
) -> Option<ApplicationIndex> {
    let config = services
        .config
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    if request_id.is_some() && services.cancelled_through.load(Ordering::Acquire) < generation {
        // Refresh explicitly re-admits failed icons. Viewport updates only change
        // priority and never retry a completed failure.
        let (priority, wake) = services.icon_wake;
        priority
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .retry_failed();
        wake.notify_one();
    }
    let result = index.scan(
        &config,
        generation,
        services.cancelled_through,
        |progress| {
            if let Some(request_id) = &request_id {
                let _ = services.events.send(RuntimeEvent::ScanProgress {
                    request_id: request_id.clone(),
                    progress,
                });
            }
        },
        |updated, removed| {
            publish_entries(
                services.entries,
                services.events,
                services.icon_wake,
                updated,
                removed,
            )
        },
    );
    match result {
        Ok(report) => {
            if services
                .events
                .send(RuntimeEvent::ScanFinished {
                    request_id,
                    response_generation: generation,
                    result: Ok(report),
                })
                .is_err()
            {
                return None;
            }
        }
        Err(error) => send_failure(services.events, request_id, generation, &error),
    }
    Some(index)
}

fn send_failure(
    events: &SyncSender<RuntimeEvent>,
    request_id: Option<String>,
    generation: u64,
    error: &crate::ApplicationError,
) {
    if events
        .send(RuntimeEvent::ScanFinished {
            request_id,
            response_generation: generation,
            result: Err(error.to_string()),
        })
        .is_err()
    {
        eprintln!("application runtime closed before receiving scan failure: {error}");
    }
}

fn publish_entries(
    entries: &RwLock<std::collections::HashMap<String, ApplicationEntry>>,
    events: &SyncSender<RuntimeEvent>,
    icon_wake: &(Mutex<EntryPriority>, Condvar),
    updated: Vec<ApplicationEntry>,
    removed: Vec<String>,
) {
    let mut current = entries.write().unwrap_or_else(|error| error.into_inner());
    let mut changed = removed.clone();
    for mut entry in updated {
        if let Some(previous) = current.get(&entry.entry_id)
            && previous.same_icon_source(&entry)
        {
            entry._icon = previous._icon.clone();
        }
        if current.get(&entry.entry_id) != Some(&entry) {
            changed.push(entry.entry_id.clone());
            current.insert(entry.entry_id.clone(), entry);
        }
    }
    for id in removed {
        current.remove(&id);
    }
    drop(current);
    if !changed.is_empty() {
        let (priority, wake) = icon_wake;
        priority
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .wake();
        wake.notify_one();
        let _ = events.send(RuntimeEvent::CatalogUpdated { entry_ids: changed });
    }
}
