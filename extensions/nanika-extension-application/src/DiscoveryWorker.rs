use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;

use nanika_config::ConfigStore;

use crate::{
    ApplicationConfig, ApplicationDatabase, ApplicationEntry, ApplicationIndex, DiscoveryCommand,
    DiscoveryServices, IconCache, RuntimeEvent,
};

const COMMAND_CAPACITY: usize = 4;

/// Named owner for filesystem discovery and application SQLite writes.
pub struct DiscoveryWorker {
    commands: SyncSender<DiscoveryCommand>,
    cancelled_through: Arc<AtomicU64>,
    thread: Option<JoinHandle<()>>,
}

impl DiscoveryWorker {
    pub fn spawn(
        database_path: PathBuf,
        icon_root: PathBuf,
        config_store: ConfigStore,
        entries: Arc<RwLock<Vec<ApplicationEntry>>>,
        events: SyncSender<RuntimeEvent>,
    ) -> std::io::Result<Self> {
        let (commands, receiver) = mpsc::sync_channel(COMMAND_CAPACITY);
        let cancelled_through = Arc::new(AtomicU64::new(0));
        let worker_cancellation = Arc::clone(&cancelled_through);
        let thread = std::thread::Builder::new()
            .name("nanika-application-discovery".to_owned())
            .spawn(move || {
                let mut index = match ApplicationDatabase::open_recovering(&database_path)
                    .map(|database| ApplicationIndex::new(database, IconCache::new(&icon_root)))
                {
                    Ok(index) => index,
                    Err(error) => {
                        let _ = events.send(RuntimeEvent::ScanFinished {
                            request_id: None,
                            response_generation: 1,
                            result: Err(error.to_string()),
                        });
                        return;
                    }
                };
                match index.load() {
                    Ok(loaded) => replace_entries(&entries, loaded),
                    Err(error) if error.is_corrupt_database() => {
                        index = match index.rebuild_database(&database_path) {
                            Ok(index) => index,
                            Err(error) => {
                                send_failure(&events, None, 1, &error);
                                return;
                            }
                        };
                    }
                    Err(_) => {}
                }
                let services = DiscoveryServices {
                    database_path: &database_path,
                    config_store: &config_store,
                    entries: &entries,
                    events: &events,
                    cancelled_through: &worker_cancellation,
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
                            index = match run_scan(index, &services, request_id, generation) {
                                Some(index) => index,
                                None => return,
                            };
                        }
                        DiscoveryCommand::Shutdown => break,
                    }
                }
            })?;
        Ok(Self {
            commands,
            cancelled_through,
            thread: Some(thread),
        })
    }

    pub fn refresh(&self, request_id: Option<String>, generation: u64) -> bool {
        self.commands
            .try_send(DiscoveryCommand::Refresh {
                request_id,
                generation,
            })
            .is_ok()
    }

    pub fn cancel(&self, generation: u64) {
        self.cancelled_through
            .fetch_max(generation, Ordering::AcqRel);
    }

    pub fn shutdown(mut self) {
        self.cancel(u64::MAX);
        let _ = self.commands.send(DiscoveryCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_scan(
    mut index: ApplicationIndex,
    services: &DiscoveryServices<'_>,
    request_id: Option<String>,
    generation: u64,
) -> Option<ApplicationIndex> {
    let config = match ApplicationConfig::load(services.config_store) {
        Ok(config) => config,
        Err(error) => {
            send_failure(services.events, request_id, generation, &error);
            return Some(index);
        }
    };
    let result = match index.scan(&config, generation, services.cancelled_through) {
        Err(error) if error.is_corrupt_database() => {
            index = match index.rebuild_database(services.database_path) {
                Ok(index) => index,
                Err(error) => {
                    send_failure(services.events, request_id, generation, &error);
                    return None;
                }
            };
            index.scan(&config, generation, services.cancelled_through)
        }
        result => result,
    };
    match result {
        Ok((mut report, discovered)) => {
            replace_entries(services.entries, discovered);
            match index.populate_icons(services.cancelled_through, generation, report.complete) {
                Ok(icon_failures) => {
                    report.warnings = report.warnings.saturating_add(icon_failures);
                    let _ = services.events.send(RuntimeEvent::ScanFinished {
                        request_id,
                        response_generation: generation,
                        result: Ok(report),
                    });
                }
                Err(error) => send_failure(services.events, request_id, generation, &error),
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
    let _ = events.send(RuntimeEvent::ScanFinished {
        request_id,
        response_generation: generation,
        result: Err(error.to_string()),
    });
}

fn replace_entries(entries: &RwLock<Vec<ApplicationEntry>>, replacement: Vec<ApplicationEntry>) {
    *entries.write().unwrap_or_else(|error| error.into_inner()) = replacement;
}
