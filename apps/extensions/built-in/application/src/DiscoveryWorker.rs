use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;

use crate::{
    ApplicationConfig, ApplicationDatabase, ApplicationEntry, ApplicationIndex, DiscoveryCommand,
    DiscoveryServices, EntryPriority, IconCache, RuntimeEvent,
};

const ICON_BATCH_SIZE: usize = 10;

/// Named owner for filesystem discovery and application database writes.
pub struct DiscoveryWorker {
    commands: Sender<DiscoveryCommand>,
    cancelled_through: Arc<AtomicU64>,
    priority: Arc<Mutex<EntryPriority>>,
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
        let priority = Arc::new(Mutex::new(EntryPriority::default()));
        let worker_priority = Arc::clone(&priority);
        let worker_commands = commands.clone();
        let thread = std::thread::Builder::new()
            .name("nanika-application-discovery".to_owned())
            .spawn(move || {
                let mut index = match ApplicationDatabase::open(&database_path)
                    .map(|database| ApplicationIndex::new(database, IconCache::new(&icon_root)))
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
                    Ok(loaded) => publish_entries(&entries, &events, loaded, Vec::new()),
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
                };
                let mut scan_generation = 1;
                index = match run_scan(index, &services, None, scan_generation) {
                    Some(index) => index,
                    None => return,
                };
                while let Ok(command) = receiver.recv() {
                    match command {
                        DiscoveryCommand::Refresh {
                            request_id,
                            generation,
                        } => {
                            scan_generation = generation;
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
                        DiscoveryCommand::PopulateIcons => {
                            if !index.has_pending_icons() {
                                worker_priority
                                    .lock()
                                    .unwrap_or_else(|error| error.into_inner())
                                    .finish(|_| false);
                                continue;
                            }
                            let entry_ids = worker_priority
                                .lock()
                                .unwrap_or_else(|error| error.into_inner())
                                .entries();
                            let (icon_failures, updated) = index.populate_icon_batch(
                                services.cancelled_through,
                                scan_generation,
                                ICON_BATCH_SIZE,
                                &entry_ids,
                            );
                            for failure in icon_failures {
                                eprintln!("application icon extraction failed: {failure}");
                            }
                            publish_entries(services.entries, services.events, updated, Vec::new());
                            let mut priority = worker_priority
                                .lock()
                                .unwrap_or_else(|error| error.into_inner());
                            // Recheck the latest viewport while holding its admission lock.
                            // A request arriving during extraction must retain its wake even
                            // when this batch produces no catalog changes.
                            if priority.finish(|requested| {
                                services.cancelled_through.load(Ordering::Acquire) < scan_generation
                                    && index.has_pending_icons_for(requested)
                            }) && worker_commands.send(DiscoveryCommand::PopulateIcons).is_err() {
                                return;
                            }
                        }
                        DiscoveryCommand::Shutdown => break,
                    }
                }
            })?;
        Ok(Self {
            commands,
            cancelled_through,
            priority,
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
        let mut priority = self
            .priority
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if priority.request(generation, entry_ids)
            && self.commands.send(DiscoveryCommand::PopulateIcons).is_err()
        {
            priority.finish(|_| false);
        }
    }

    pub fn shutdown(mut self, events: mpsc::Receiver<RuntimeEvent>) -> Result<(), String> {
        self.cancel(u64::MAX);
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
            return Err("application discovery worker panicked".to_owned());
        }
        failure.map_or(Ok(()), Err)
    }

    fn _stop(&mut self) -> Result<(), String> {
        if self.thread.is_none() {
            return Ok(());
        }
        self.cancel(u64::MAX);
        let sent = self.commands.send(DiscoveryCommand::Shutdown).is_ok();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            return Err("application discovery worker panicked".to_owned());
        }
        if !sent {
            return Err(
                "application discovery worker closed before shutdown was requested".to_owned(),
            );
        }
        Ok(())
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
        |updated, removed| publish_entries(services.entries, services.events, updated, removed),
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
    updated: Vec<ApplicationEntry>,
    removed: Vec<String>,
) {
    let mut current = entries.write().unwrap_or_else(|error| error.into_inner());
    let mut changed = removed.clone();
    for entry in updated {
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
        let _ = events.send(RuntimeEvent::CatalogUpdated { entry_ids: changed });
    }
}
