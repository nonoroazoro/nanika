use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;

use crate::{
    ApplicationConfig, ApplicationDatabase, ApplicationEntry, ApplicationIndex, DiscoveryCommand,
    DiscoveryServices, IconCache, RuntimeEvent,
};

const ICON_BATCH_SIZE: usize = 10;

/// Named owner for filesystem discovery and application database writes.
pub struct DiscoveryWorker {
    commands: Sender<DiscoveryCommand>,
    cancelled_through: Arc<AtomicU64>,
    priority: Arc<Mutex<EntryPriority>>,
    thread: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct EntryPriority {
    generation: u64,
    entry_ids: Vec<String>,
    wake_queued: bool,
}

impl DiscoveryWorker {
    pub fn spawn(
        database_path: PathBuf,
        icon_root: PathBuf,
        config: Arc<RwLock<ApplicationConfig>>,
        entries: Arc<RwLock<Vec<ApplicationEntry>>>,
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
                match index.load_presentable() {
                    Ok(loaded) => replace_entries(&entries, loaded),
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
                                    .wake_queued = false;
                                continue;
                            }
                            let entry_ids = worker_priority
                                .lock()
                                .unwrap_or_else(|error| error.into_inner())
                                .entry_ids
                                .clone();
                            index.prioritize_pending_icons(&entry_ids);
                            let more_pending = match index.populate_icon_batch(
                                services.cancelled_through,
                                scan_generation,
                                ICON_BATCH_SIZE,
                            ) {
                                Ok((icon_failures, more_pending)) => {
                                    for failure in icon_failures {
                                        eprintln!("application icon extraction failed: {failure}");
                                    }
                                    match index.load_presentable() {
                                        Ok(ready) => {
                                            replace_entries(services.entries, ready);
                                            if services
                                                .events
                                                .send(RuntimeEvent::CandidatesChanged)
                                                .is_err()
                                            {
                                                return;
                                            }
                                            more_pending
                                        }
                                        Err(error) => {
                                            eprintln!(
                                                "application icon cache update failed: {error}"
                                            );
                                            false
                                        }
                                    }
                                }
                                Err(error) => {
                                    eprintln!("application icon cache population failed: {error}");
                                    false
                                }
                            };
                            let mut priority = worker_priority
                                .lock()
                                .unwrap_or_else(|error| error.into_inner());
                            priority.wake_queued = false;
                            if more_pending
                                && services.cancelled_through.load(Ordering::Acquire)
                                    < scan_generation
                            {
                                priority.wake_queued = true;
                                if worker_commands
                                    .send(DiscoveryCommand::PopulateIcons)
                                    .is_err()
                                {
                                    return;
                                }
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
        if entry_ids.is_empty() {
            return;
        }
        let mut priority = self
            .priority
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if generation >= priority.generation {
            priority.generation = generation;
            priority.entry_ids = entry_ids;
        }
        if priority.wake_queued {
            return;
        }
        priority.wake_queued = true;
        if self.commands.send(DiscoveryCommand::PopulateIcons).is_err() {
            priority.wake_queued = false;
        }
    }

    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        if self.thread.is_none() {
            return;
        }
        self.cancel(u64::MAX);
        if self.commands.send(DiscoveryCommand::Shutdown).is_err() {
            eprintln!("application discovery worker closed before shutdown was requested");
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            eprintln!("application discovery worker panicked");
        }
    }
}

impl Drop for DiscoveryWorker {
    fn drop(&mut self) {
        self.stop();
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
    let result = index.scan(&config, generation, services.cancelled_through);
    match result {
        Ok((report, discovered)) => {
            replace_entries(services.entries, discovered);
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

fn replace_entries(entries: &RwLock<Vec<ApplicationEntry>>, replacement: Vec<ApplicationEntry>) {
    *entries.write().unwrap_or_else(|error| error.into_inner()) = replacement;
}
