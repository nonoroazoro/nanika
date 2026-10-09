use std::collections::{HashMap, HashSet};

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::constants::SEARCH_QUEUE_CAPACITY;
use crate::{
    CandidateCatalog, PendingSearchQuery, SearchCommand, SearchEngine, SearchHandle, UsageMap
};

/// Named owner thread for aggregation, stale-generation rejection, and ranking.
pub struct SearchOwner {
    handle: SearchHandle,
    thread: Option<JoinHandle<()>>
}

impl SearchOwner {
    pub fn spawn(mut initial_usage: UsageMap) -> std::io::Result<Self> {
        let next_generation = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let owner_generation = Arc::clone(&next_generation);
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
                let mut extension_results: HashMap<String, CandidateCatalog> = HashMap::new();
                let mut static_catalog: HashMap<String, CandidateCatalog> = HashMap::new();
                let mut expected_extensions = HashSet::new();
                let mut instances = Arc::new(HashMap::new());

                while let Ok(command) = receiver.recv() {
                    if let Some(next_query) = take_pending_query(&owner_pending_query) {
                        generation = next_query.generation;
                        query = next_query.query;
                        expected_extensions = next_query.expected_extensions;
                        extension_results.clear();
                        expected_extensions.retain(|id| !static_catalog.contains_key(id));
                        publish_current(
                            &mut engine,
                            generation,
                            &query,
                            (&extension_results, &static_catalog),
                            &initial_usage,
                            crate::SearchPublication {
                                instances: &instances,
                                pending: &expected_extensions,
                                latest: &owner_latest,
                                notifier: &owner_notifier,
                                next_generation: &owner_generation
                            }
                        );
                    }
                    match command {
                        SearchCommand::WakeQuery => {}
                        SearchCommand::RegisterExtension {
                            extension_id,
                            instance_id,
                            completion
                        } => {
                            let result = match Arc::make_mut(&mut instances).entry(extension_id) {
                                std::collections::hash_map::Entry::Vacant(entry) => {
                                    entry.insert(instance_id);
                                    Ok(())
                                }
                                std::collections::hash_map::Entry::Occupied(_) => {
                                    Err(crate::SearchQueueError::AlreadyRegistered)
                                }
                            };
                            let _ = completion.send(result);
                        }
                        SearchCommand::ExtensionQueryPending {
                            generation: started,
                            extension_id,
                            instance_id,
                            pending
                        } if started == generation
                            && instances.get(&extension_id) == Some(&instance_id) =>
                        {
                            let changed = if pending {
                                expected_extensions.insert(extension_id)
                            } else {
                                expected_extensions.remove(&extension_id)
                            };
                            if changed {
                                _publish_progress(
                                    generation,
                                    crate::SearchPublication {
                                        instances: &instances,
                                        pending: &expected_extensions,
                                        latest: &owner_latest,
                                        notifier: &owner_notifier,
                                        next_generation: &owner_generation
                                    }
                                );
                            }
                        }
                        SearchCommand::RemoveExtension {
                            extension_id,
                            instance_id,
                            completion
                        } => {
                            if instances.get(&extension_id) != Some(&instance_id) {
                                let _ = completion.send(());
                                continue;
                            }
                            Arc::make_mut(&mut instances).remove(&extension_id);
                            static_catalog.remove(&extension_id);
                            extension_results.remove(&extension_id);
                            expected_extensions.remove(&extension_id);
                            if generation != 0 {
                                publish_current(
                                    &mut engine,
                                    generation,
                                    &query,
                                    (&extension_results, &static_catalog),
                                    &initial_usage,
                                    crate::SearchPublication {
                                        instances: &instances,
                                        pending: &expected_extensions,
                                        latest: &owner_latest,
                                        notifier: &owner_notifier,
                                        next_generation: &owner_generation
                                    }
                                );
                            }
                            let _ = completion.send(());
                        }
                        SearchCommand::RegisterStaticCatalog {
                            extension_id,
                            instance_id,
                            candidates
                        } => {
                            if instances.get(&extension_id) != Some(&instance_id) {
                                continue;
                            }
                            let candidates = CandidateCatalog::new(&extension_id, candidates);
                            static_catalog.insert(extension_id.clone(), candidates);
                            expected_extensions.remove(&extension_id);
                            if generation != 0 {
                                publish_current(
                                    &mut engine,
                                    generation,
                                    &query,
                                    (&extension_results, &static_catalog),
                                    &initial_usage,
                                    crate::SearchPublication {
                                        instances: &instances,
                                        pending: &expected_extensions,
                                        latest: &owner_latest,
                                        notifier: &owner_notifier,
                                        next_generation: &owner_generation
                                    }
                                );
                            }
                        }
                        SearchCommand::CatalogCommit {
                            extension_id,
                            instance_id,
                            replace,
                            candidates,
                            removed,
                            completion
                        } => {
                            if instances.get(&extension_id) != Some(&instance_id) {
                                let _ = completion.send(Err(crate::SearchQueueError::Retired));
                                continue;
                            }
                            let changed = replace || !candidates.is_empty() || !removed.is_empty();
                            let result =
                                if let Some(catalog) = static_catalog.get_mut(&extension_id) {
                                    if replace {
                                        *catalog = CandidateCatalog::default();
                                    }
                                    catalog.update(&extension_id, candidates, removed);
                                    Ok(())
                                } else {
                                    Err(crate::SearchQueueError::Closed)
                                };
                            let applied = result.is_ok();
                            let _ = completion.send(result);
                            if applied && changed && generation != 0 {
                                publish_current(
                                    &mut engine,
                                    generation,
                                    &query,
                                    (&extension_results, &static_catalog),
                                    &initial_usage,
                                    crate::SearchPublication {
                                        instances: &instances,
                                        pending: &expected_extensions,
                                        latest: &owner_latest,
                                        notifier: &owner_notifier,
                                        next_generation: &owner_generation
                                    }
                                );
                            }
                        }
                        SearchCommand::ExtensionSnapshot {
                            complete,
                            generation: snapshot_generation,
                            extension_id,
                            instance_id,
                            candidates
                        } if snapshot_generation == generation
                            && instances.get(&extension_id) == Some(&instance_id) =>
                        {
                            if complete {
                                expected_extensions.remove(&extension_id);
                            } else {
                                expected_extensions.insert(extension_id.clone());
                            }
                            let catalog = CandidateCatalog::new(&extension_id, candidates);
                            let results_changed = extension_results.get(&extension_id).map_or_else(
                                || catalog.iter().next().is_some(),
                                |previous| previous != &catalog
                            );
                            extension_results.insert(extension_id, catalog);
                            if !results_changed {
                                _publish_progress(
                                    generation,
                                    crate::SearchPublication {
                                        instances: &instances,
                                        pending: &expected_extensions,
                                        latest: &owner_latest,
                                        notifier: &owner_notifier,
                                        next_generation: &owner_generation
                                    }
                                );
                                continue;
                            }
                            publish_current(
                                &mut engine,
                                generation,
                                &query,
                                (&extension_results, &static_catalog),
                                &initial_usage,
                                crate::SearchPublication {
                                    instances: &instances,
                                    pending: &expected_extensions,
                                    latest: &owner_latest,
                                    notifier: &owner_notifier,
                                    next_generation: &owner_generation
                                }
                            );
                        }
                        SearchCommand::ExtensionDelta {
                            complete,
                            generation: update_generation,
                            extension_id,
                            instance_id,
                            candidates,
                            removed
                        } if update_generation == generation
                            && instances.get(&extension_id) == Some(&instance_id) =>
                        {
                            // A patch cannot establish a generation's baseline or revive a removed extension.
                            let Some(catalog) = extension_results.get_mut(&extension_id) else {
                                continue;
                            };
                            let pending_changed = if complete {
                                expected_extensions.remove(&extension_id)
                            } else {
                                expected_extensions.insert(extension_id.clone())
                            };
                            if candidates.is_empty() && removed.is_empty() {
                                if pending_changed {
                                    _publish_progress(
                                        generation,
                                        crate::SearchPublication {
                                            instances: &instances,
                                            pending: &expected_extensions,
                                            latest: &owner_latest,
                                            notifier: &owner_notifier,
                                            next_generation: &owner_generation
                                        }
                                    );
                                }
                                continue;
                            }
                            catalog.update(&extension_id, candidates, removed);
                            publish_current(
                                &mut engine,
                                generation,
                                &query,
                                (&extension_results, &static_catalog),
                                &initial_usage,
                                crate::SearchPublication {
                                    instances: &instances,
                                    pending: &expected_extensions,
                                    latest: &owner_latest,
                                    notifier: &owner_notifier,
                                    next_generation: &owner_generation
                                }
                            );
                        }
                        SearchCommand::ExtensionQueryPending { .. }
                        | SearchCommand::ExtensionDelta { .. } => {}
                        SearchCommand::ExtensionSnapshot { .. } => {}
                        SearchCommand::ApplyPersistedExecution { key, executed_at } => {
                            let stat = initial_usage.entry(key).or_default();
                            stat.execution_count = stat.execution_count.saturating_add(1);
                            stat.last_executed_at = executed_at;
                            publish_current(
                                &mut engine,
                                generation,
                                &query,
                                (&extension_results, &static_catalog),
                                &initial_usage,
                                crate::SearchPublication {
                                    instances: &instances,
                                    pending: &expected_extensions,
                                    latest: &owner_latest,
                                    notifier: &owner_notifier,
                                    next_generation: &owner_generation
                                }
                            );
                        }
                        SearchCommand::ResetPersistedUsage => {
                            initial_usage.clear();
                            publish_current(
                                &mut engine,
                                generation,
                                &query,
                                (&extension_results, &static_catalog),
                                &initial_usage,
                                crate::SearchPublication {
                                    instances: &instances,
                                    pending: &expected_extensions,
                                    latest: &owner_latest,
                                    notifier: &owner_notifier,
                                    next_generation: &owner_generation
                                }
                            );
                        }
                        SearchCommand::Shutdown => break
                    }
                }
            })?;
        Ok(Self {
            handle: SearchHandle {
                commands,
                pending_query,
                latest,
                next_generation,
                notifier
            },
            thread: Some(thread)
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
    pending_query: &Mutex<Option<PendingSearchQuery>>
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
    catalogs: (
        &HashMap<String, CandidateCatalog>,
        &HashMap<String, CandidateCatalog>
    ),
    usage: &UsageMap,
    publication: crate::SearchPublication<'_>
) {
    let candidates = catalogs
        .0
        .values()
        .chain(catalogs.1.values())
        .flat_map(|entries| entries.iter());
    let Some(snapshot) = engine.rank(
        generation,
        query,
        candidates,
        usage,
        unix_timestamp(),
        || {
            publication
                .next_generation
                .load(std::sync::atomic::Ordering::Relaxed)
                > generation
        }
    ) else {
        return;
    };
    _publish_snapshot(snapshot, publication);
}

fn _publish_progress(generation: u64, publication: crate::SearchPublication<'_>) {
    if publication
        .next_generation
        .load(std::sync::atomic::Ordering::Relaxed)
        > generation
    {
        return;
    }
    let current = publication
        .latest
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    if let Some(current) = current.filter(|snapshot| snapshot.generation == generation) {
        _publish_snapshot((*current).clone(), publication);
    }
}

fn _publish_snapshot(
    mut snapshot: crate::SearchSnapshot,
    publication: crate::SearchPublication<'_>
) {
    snapshot.instances = Arc::clone(publication.instances);
    snapshot.pending_extensions = publication.pending.iter().cloned().collect();
    snapshot.pending_extensions.sort();
    let previous = publication
        .latest
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    // This owner is the sole writer. Compare outside the lock so readers never
    // wait for a full result-set comparison.
    snapshot.result_revision = match previous.as_ref() {
        Some(previous) => {
            if previous.results == snapshot.results {
                // Identical rerankings reuse the delivered payload and comparison below.
                snapshot.results = Arc::clone(&previous.results);
            }
            let same_instances = Arc::ptr_eq(&previous.instances, &snapshot.instances);
            let same_authority = previous.generation == snapshot.generation
                && ((Arc::ptr_eq(&previous.results, &snapshot.results) && same_instances)
                    || (previous.results.len() == snapshot.results.len()
                        && previous
                            .results
                            .iter()
                            .zip(snapshot.results.iter())
                            .all(|(a, b)| {
                                a.candidate.same_execution_target(&b.candidate)
                                    && (same_instances
                                        || previous.instances.get(a.candidate.extension_id())
                                            == snapshot.instances.get(b.candidate.extension_id()))
                            })));
            if same_authority {
                previous.result_revision
            } else {
                previous.result_revision + 1
            }
        }
        None => 1
    };
    *publication
        .latest
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(Arc::new(snapshot));
    let notify = publication
        .notifier
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
