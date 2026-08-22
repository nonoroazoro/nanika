use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;

use nanika_config::ConfigStore;

use crate::{
    ApplicationConfig, ApplicationDatabase, ApplicationEntry, ApplicationIndex, DiscoveryCommand,
    IconCache, RuntimeEvent,
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
                let mut index = match ApplicationDatabase::open(database_path)
                    .map(|database| ApplicationIndex::new(database, IconCache::new(icon_root)))
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
                if let Ok(loaded) = index.load() {
                    replace_entries(&entries, loaded);
                }
                run_scan(
                    &mut index,
                    &config_store,
                    &entries,
                    &events,
                    &worker_cancellation,
                    None,
                    1,
                );
                while let Ok(command) = receiver.recv() {
                    match command {
                        DiscoveryCommand::Refresh {
                            request_id,
                            generation,
                        } => run_scan(
                            &mut index,
                            &config_store,
                            &entries,
                            &events,
                            &worker_cancellation,
                            request_id,
                            generation,
                        ),
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
    index: &mut ApplicationIndex,
    config_store: &ConfigStore,
    entries: &RwLock<Vec<ApplicationEntry>>,
    events: &SyncSender<RuntimeEvent>,
    cancelled_through: &AtomicU64,
    request_id: Option<String>,
    generation: u64,
) {
    let result = ApplicationConfig::load(config_store)
        .and_then(|config| index.scan(&config, generation, cancelled_through));
    match result {
        Ok((report, discovered)) => {
            replace_entries(entries, discovered);
            let _ = events.send(RuntimeEvent::ScanFinished {
                request_id,
                response_generation: generation,
                result: Ok(report),
            });
            index.populate_icons(cancelled_through, generation);
        }
        Err(error) => {
            let _ = events.send(RuntimeEvent::ScanFinished {
                request_id,
                response_generation: generation,
                result: Err(error.to_string()),
            });
        }
    }
}

fn replace_entries(entries: &RwLock<Vec<ApplicationEntry>>, replacement: Vec<ApplicationEntry>) {
    *entries.write().unwrap_or_else(|error| error.into_inner()) = replacement;
}
