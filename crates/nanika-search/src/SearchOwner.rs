use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::constants::{
    MAX_EXTENSION_CANDIDATES, MAX_USAGE_COUNT, MAX_USAGE_ROWS, SEARCH_QUEUE_CAPACITY,
    USAGE_RETENTION_DAYS,
};
use crate::{Candidate, SearchCommand, SearchEngine, SearchHandle, SearchSnapshot, UsageMap};

/// Named owner thread for aggregation, stale-generation rejection, and ranking.
pub struct SearchOwner {
    handle: SearchHandle,
    thread: Option<JoinHandle<()>>,
}

impl SearchOwner {
    pub fn spawn(mut initial_usage: UsageMap) -> std::io::Result<Self> {
        prune_usage(&mut initial_usage, unix_timestamp());
        let (commands, receiver) = mpsc::sync_channel(SEARCH_QUEUE_CAPACITY);
        let pending_query = Arc::new(Mutex::new(None));
        let latest = Arc::new(Mutex::new(None));
        let notifier = Arc::new(Mutex::new(None));
        let owner_latest = Arc::clone(&latest);
        let owner_notifier = Arc::clone(&notifier);
        let owner_pending_query = Arc::clone(&pending_query);
        let thread = std::thread::Builder::new()
            .name("nanika-search-owner".to_owned())
            .spawn(move || {
                let mut engine = SearchEngine::new();
                let mut generation = 0;
                let mut query = String::new();
                let mut extension_results: HashMap<String, Vec<Candidate>> = HashMap::new();
                let mut expected_extensions = 0;
                let mut waiting_for_initial_snapshots = false;

                while let Ok(command) = receiver.recv() {
                    if let Some((next_generation, next_query, next_expected_extensions)) =
                        take_pending_query(&owner_pending_query)
                    {
                        generation = next_generation;
                        query = next_query;
                        expected_extensions = next_expected_extensions;
                        waiting_for_initial_snapshots = expected_extensions > 0;
                        extension_results.clear();
                        if !waiting_for_initial_snapshots {
                            publish_current(
                                &mut engine,
                                generation,
                                &query,
                                &extension_results,
                                &initial_usage,
                                &owner_latest,
                                &owner_notifier,
                            );
                        }
                    }
                    match command {
                        SearchCommand::WakeQuery => {}
                        SearchCommand::ExtensionSnapshot {
                            generation: snapshot_generation,
                            extension_id,
                            candidates,
                        } if snapshot_generation == generation => {
                            let mut unique = HashMap::with_capacity(
                                candidates.len().min(MAX_EXTENSION_CANDIDATES),
                            );
                            for mut candidate in
                                candidates.into_iter().take(MAX_EXTENSION_CANDIDATES)
                            {
                                candidate.set_extension_id(&extension_id);
                                unique.insert(
                                    (
                                        candidate.entry_id().to_owned(),
                                        candidate.action_id().to_owned(),
                                    ),
                                    candidate,
                                );
                            }
                            extension_results.insert(extension_id, unique.into_values().collect());
                            if waiting_for_initial_snapshots
                                && extension_results.len() < expected_extensions
                            {
                                continue;
                            }
                            waiting_for_initial_snapshots = false;
                            publish_current(
                                &mut engine,
                                generation,
                                &query,
                                &extension_results,
                                &initial_usage,
                                &owner_latest,
                                &owner_notifier,
                            );
                        }
                        SearchCommand::ExtensionSnapshot { .. } => {}
                        SearchCommand::ApplyPersistedExecution { key, executed_at } => {
                            let stat = initial_usage.entry(key).or_default();
                            stat.execution_count =
                                stat.execution_count.saturating_add(1).min(MAX_USAGE_COUNT);
                            stat.last_executed_at = executed_at;
                            prune_usage(&mut initial_usage, executed_at);
                            publish_current(
                                &mut engine,
                                generation,
                                &query,
                                &extension_results,
                                &initial_usage,
                                &owner_latest,
                                &owner_notifier,
                            );
                        }
                        SearchCommand::ResetPersistedUsage => {
                            initial_usage.clear();
                            publish_current(
                                &mut engine,
                                generation,
                                &query,
                                &extension_results,
                                &initial_usage,
                                &owner_latest,
                                &owner_notifier,
                            );
                        }
                        SearchCommand::Shutdown => break,
                    }
                }
            })?;
        Ok(Self {
            handle: SearchHandle {
                commands,
                pending_query,
                latest,
                next_generation: Arc::new(AtomicU64::new(0)),
                notifier,
            },
            thread: Some(thread),
        })
    }

    pub fn handle(&self) -> SearchHandle {
        self.handle.clone()
    }

    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        let _ = self.handle.commands.send(SearchCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn take_pending_query(
    pending_query: &Mutex<Option<(u64, String, usize)>>,
) -> Option<(u64, String, usize)> {
    pending_query
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take()
}

impl Drop for SearchOwner {
    fn drop(&mut self) {
        self.stop();
    }
}

fn publish_current(
    engine: &mut SearchEngine,
    generation: u64,
    query: &str,
    extension_results: &HashMap<String, Vec<Candidate>>,
    usage: &UsageMap,
    latest: &Mutex<Option<Arc<SearchSnapshot>>>,
    notifier: &Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
) {
    let candidates = extension_results
        .values()
        .flat_map(|entries| entries.iter());
    let snapshot = Arc::new(engine.rank(generation, query, candidates, usage, unix_timestamp()));
    *latest.lock().unwrap_or_else(|error| error.into_inner()) = Some(snapshot);
    let notify = notifier
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    if let Some(notify) = notify {
        notify();
    }
}

fn prune_usage(usage: &mut UsageMap, now: u64) {
    let cutoff = now.saturating_sub(USAGE_RETENTION_DAYS.saturating_mul(86_400));
    usage.retain(|_, stat| stat.last_executed_at >= cutoff);
    if usage.len() <= MAX_USAGE_ROWS {
        return;
    }
    let mut oldest = usage
        .iter()
        .map(|(key, stat)| (stat.last_executed_at, key.clone()))
        .collect::<Vec<_>>();
    oldest.sort_unstable();
    for (_, key) in oldest.into_iter().take(usage.len() - MAX_USAGE_ROWS) {
        usage.remove(&key);
    }
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
