use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::JoinHandle;

use crate::{ApplicationEntry, EntryPriority, IconCache, RuntimeEvent};
use nanika_protocol::{IconReference, IconSource};

/// One cache writer, independent of filesystem discovery and database transactions.
pub(crate) struct IconWorker {
    _requests: Arc<(Mutex<EntryPriority>, Condvar)>,
    _thread: Option<JoinHandle<()>>
}

impl IconWorker {
    pub(crate) fn spawn(
        root: PathBuf,
        entries: Arc<RwLock<HashMap<String, ApplicationEntry>>>,
        events: SyncSender<RuntimeEvent>
    ) -> std::io::Result<Self> {
        let cache = Arc::new(IconCache::new(root));
        let reader = Arc::clone(&cache);
        Self::_spawn(
            entries,
            events,
            move |entry| reader.cached(&entry.icon_key),
            move |entry| match cache.prepare(entry) {
                Ok(icon) => IconSource::Cache(icon),
                Err(error) => {
                    eprintln!(
                        "application icon extraction failed for {}: {error}",
                        entry.target_path
                    );
                    match cache.fallback() {
                        Ok(icon) => IconSource::Cache(icon),
                        Err(error) => {
                            eprintln!("application icon cache failed: {error}");
                            // A completed failure must not expose an unavailable URL or
                            // turn viewport notifications into automatic retries.
                            IconSource::Empty
                        }
                    }
                }
            }
        )
    }

    pub(crate) fn prepare_entries(&self, generation: u64, entry_ids: Vec<String>) {
        let (requests, wake) = &*self._requests;
        requests
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .request(generation, entry_ids);
        wake.notify_one();
    }

    pub(crate) fn stop(&self) {
        let (requests, wake) = &*self._requests;
        requests
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .stop();
        wake.notify_one();
    }

    pub(crate) fn join(&mut self) -> Result<(), String> {
        if let Some(thread) = self._thread.take()
            && thread.join().is_err()
        {
            return Err("application icon worker panicked".to_owned());
        }
        Ok(())
    }

    pub(crate) fn wake_handle(&self) -> Arc<(Mutex<EntryPriority>, Condvar)> {
        Arc::clone(&self._requests)
    }

    fn _spawn(
        entries: Arc<RwLock<HashMap<String, ApplicationEntry>>>,
        events: SyncSender<RuntimeEvent>,
        cached_icon: impl Fn(&ApplicationEntry) -> Option<IconReference> + Send + 'static,
        mut prepare: impl FnMut(&ApplicationEntry) -> IconSource + Send + 'static
    ) -> std::io::Result<Self> {
        let requests = Arc::new((Mutex::new(EntryPriority::default()), Condvar::new()));
        let worker_requests = Arc::clone(&requests);
        let thread = std::thread::Builder::new()
            .name("nanika-application-icons".to_owned())
            .spawn(move || {
                loop {
                    let (requests, wake) = &*worker_requests;
                    let mut priority = requests.lock().unwrap_or_else(|error| error.into_inner());
                    while !priority.pending() && !priority.stopped() {
                        priority = wake
                            .wait(priority)
                            .unwrap_or_else(|error| error.into_inner());
                    }
                    if priority.stopped() {
                        break;
                    }
                    let ids = priority.take();
                    let retry_failed = priority.take_failed_retry();
                    drop(priority);
                    if retry_failed {
                        // Consume refresh after any active attempt has published its result.
                        // This also re-admits failed rows outside the current viewport without
                        // preparing them until a later viewport requests them.
                        _clear_failed_icons(&entries);
                    }
                    let pending = {
                        let current = entries.read().unwrap_or_else(|error| error.into_inner());
                        ids.iter()
                            .filter_map(|id| current.get(id))
                            .filter(|entry| entry._icon.is_none())
                            .cloned()
                            .collect::<Vec<_>>()
                    };
                    let mut cached = Vec::new();
                    let mut missing = Vec::new();
                    for original in pending {
                        if !requests
                            .lock()
                            .unwrap_or_else(|error| error.into_inner())
                            .contains(&original.entry_id)
                        {
                            continue;
                        }
                        if let Some(icon) = cached_icon(&original) {
                            cached.push((original, IconSource::Cache(icon)));
                        } else {
                            missing.push(original);
                        }
                    }
                    if !_publish(&entries, &events, cached) {
                        break;
                    }
                    for original in missing {
                        let priority = requests.lock().unwrap_or_else(|error| error.into_inner());
                        // A newer request may reorder still-visible rows, so resnapshot before
                        // starting another native call, including when all old rows remain visible.
                        if priority.pending() || priority.stopped() {
                            break;
                        }
                        if !priority.contains(&original.entry_id) {
                            continue;
                        }
                        drop(priority);
                        if !entries
                            .read()
                            .unwrap_or_else(|error| error.into_inner())
                            .get(&original.entry_id)
                            .is_some_and(|current| {
                                current._icon.is_none() && current.same_icon_source(&original)
                            })
                        {
                            continue;
                        }
                        let prepared = prepare(&original);
                        // Publish each native completion; the bounded catalog transport owns batching.
                        if !_publish(&entries, &events, [(original, prepared)]) {
                            return;
                        }
                    }
                }
            })?;
        Ok(Self {
            _requests: requests,
            _thread: Some(thread)
        })
    }
}

fn _clear_failed_icons(entries: &RwLock<HashMap<String, ApplicationEntry>>) {
    let mut current = entries.write().unwrap_or_else(|error| error.into_inner());
    for entry in current.values_mut() {
        let failed = match entry._icon.as_deref() {
            Some(nanika_protocol::IconSource::Empty) => true,
            Some(nanika_protocol::IconSource::Cache(reference)) => {
                reference.key() == IconCache::fallback_key()
                    && entry.icon_key != IconCache::fallback_key()
            }
            _ => false
        };
        if failed {
            entry._icon = None;
        }
    }
}

fn _publish(
    entries: &RwLock<HashMap<String, ApplicationEntry>>,
    events: &SyncSender<RuntimeEvent>,
    prepared: impl IntoIterator<Item = (ApplicationEntry, IconSource)>
) -> bool {
    let mut current = entries.write().unwrap_or_else(|error| error.into_inner());
    let mut changed = Vec::new();
    for (original, prepared) in prepared {
        if let Some(entry) = current.get_mut(&original.entry_id)
            && entry.same_icon_source(&original)
            && entry._icon.as_deref() != Some(&prepared)
        {
            // Only presentation changes: discovery owns all source and activation metadata.
            entry._icon = Some(Arc::new(prepared));
            changed.push(entry.entry_id.clone());
        }
    }
    drop(current);
    changed.is_empty()
        || events
            .send(RuntimeEvent::CatalogUpdated { entry_ids: changed })
            .is_ok()
}

#[cfg(test)]
#[path = "../tests/IconWorker.rs"]
mod tests;
