use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicU64;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::constants::SEARCH_QUEUE_CAPACITY;
use crate::{
    Candidate, PendingSearchQuery, SearchCommand, SearchEngine, SearchHandle, SearchSnapshot,
    UsageMap,
};

/// Named owner thread for aggregation, stale-generation rejection, and ranking.
pub struct SearchOwner {
    handle: SearchHandle,
    thread: Option<JoinHandle<()>>,
}

impl SearchOwner {
    pub fn spawn(mut initial_usage: UsageMap) -> std::io::Result<Self> {
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
                let mut extension_results: HashMap<String, Arc<Vec<Candidate>>> = HashMap::new();
                let mut static_catalog: HashMap<String, Arc<Vec<Candidate>>> = HashMap::new();
                let mut expected_extensions = HashSet::new();

                while let Ok(command) = receiver.recv() {
                    if let Some(next_query) = take_pending_query(&owner_pending_query) {
                        generation = next_query.generation;
                        query = next_query.query;
                        expected_extensions = next_query.expected_extensions;
                        extension_results.clone_from(&static_catalog);
                        expected_extensions.retain(|id| !static_catalog.contains_key(id));
                        if expected_extensions.is_empty() {
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
                        SearchCommand::RegisterStaticCatalog {
                            extension_id,
                            mut candidates,
                        } => {
                            for candidate in &mut candidates {
                                candidate.set_extension_id(&extension_id);
                            }
                            let candidates = Arc::new(candidates);
                            static_catalog.insert(extension_id.clone(), Arc::clone(&candidates));
                            expected_extensions.remove(&extension_id);
                            extension_results.insert(extension_id, candidates);
                            if generation != 0 && expected_extensions.is_empty() {
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
                        SearchCommand::ExtensionSnapshot {
                            generation: snapshot_generation,
                            extension_id,
                            candidates,
                        } if snapshot_generation == generation => {
                            expected_extensions.remove(&extension_id);
                            let mut unique = HashMap::with_capacity(candidates.len());
                            for mut candidate in candidates {
                                candidate.set_extension_id(&extension_id);
                                unique.insert(
                                    (
                                        candidate.entry_id().to_owned(),
                                        candidate.action_id().to_owned(),
                                    ),
                                    candidate,
                                );
                            }
                            extension_results
                                .insert(extension_id, Arc::new(unique.into_values().collect()));
                            if !expected_extensions.is_empty() {
                                continue;
                            }
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
                            stat.execution_count = stat.execution_count.saturating_add(1);
                            stat.last_executed_at = executed_at;
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
        if self.thread.is_none() {
            return;
        }
        if self.handle.commands.send(SearchCommand::Shutdown).is_err() {
            tracing::error!("search owner closed before shutdown was requested");
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!("search owner thread panicked");
        }
    }
}

fn take_pending_query(
    pending_query: &Mutex<Option<PendingSearchQuery>>,
) -> Option<PendingSearchQuery> {
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
    extension_results: &HashMap<String, Arc<Vec<Candidate>>>,
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

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
