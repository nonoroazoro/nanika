use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;

use nanika_config::ConfigStore;

use crate::{
    ApplicationConfig, ApplicationDatabase, ApplicationEntry, ApplicationIndex, DiscoveryCommand,
    DiscoveryServices, IconCache, RuntimeEvent,
};

/// Named owner for filesystem discovery and application SQLite writes.
pub struct DiscoveryWorker {
    commands: Sender<DiscoveryCommand>,
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
        let (commands, receiver) = mpsc::channel();
        let cancelled_through = Arc::new(AtomicU64::new(0));
        let worker_cancellation = Arc::clone(&cancelled_through);
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
    let config = match ApplicationConfig::load(services.config_store) {
        Ok(config) => config,
        Err(error) => {
            send_failure(services.events, request_id, generation, &error);
            return Some(index);
        }
    };
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
            match index.populate_icons(services.cancelled_through, generation) {
                Ok(icon_failures) => {
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
                                return None;
                            }
                        }
                        Err(error) => eprintln!("application icon cache update failed: {error}"),
                    }
                }
                Err(error) => eprintln!("application icon cache population failed: {error}"),
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
