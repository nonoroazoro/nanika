use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

const MAX_PENDING_PATHS: usize = 512;

/// Background owner for native file-icon acquisition.
pub struct FileIconWorker {
    state: Arc<(Mutex<State>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct State {
    pending: VecDeque<PathBuf>,
    known: HashSet<PathBuf>,
    resolved: HashMap<PathBuf, Option<nanika_protocol::IconReference>>,
    ready_order: VecDeque<PathBuf>,
    shutdown: bool,
}

impl FileIconWorker {
    pub fn spawn(
        icon_root: PathBuf,
        invalidated: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, String> {
        let state = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let worker_state = Arc::clone(&state);
        let thread = std::thread::Builder::new()
            .name("nanika-clipboard-file-icons".to_owned())
            .spawn(move || {
                let mut cache = nanika_platform::FileIconCache::new(icon_root);
                loop {
                    let path = {
                        let (lock, ready) = &*worker_state;
                        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
                        while state.pending.is_empty() && !state.shutdown {
                            state = ready.wait(state).unwrap_or_else(|error| error.into_inner());
                        }
                        if state.shutdown {
                            return;
                        }
                        state.pending.pop_front()
                    };
                    let Some(path) = path else {
                        continue;
                    };
                    let reference = match cache.cached(&path) {
                        Ok(Some(reference)) => Some(reference),
                        Ok(None) => match cache.get(&path) {
                            Ok(reference) => Some(reference),
                            Err(error) => {
                                eprintln!(
                                    "clipboard file icon acquisition failed for {}: {error}",
                                    path.display()
                                );
                                None
                            }
                        },
                        Err(error) => {
                            eprintln!(
                                "clipboard file icon metadata failed for {}: {error}",
                                path.display()
                            );
                            None
                        }
                    };
                    let mut state = worker_state
                        .0
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    state.known.remove(&path);
                    publish_resolution(&mut state, path, reference);
                    drop(state);
                    // A failed path also advances progressive scheduling without being
                    // retried on every resulting view refresh.
                    invalidated();
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            state,
            thread: Some(thread),
        })
    }

    pub fn schedule(&self, paths: impl IntoIterator<Item = PathBuf>) -> usize {
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        let mut unique = HashSet::new();
        let available = MAX_PENDING_PATHS.saturating_sub(state.known.len());
        let pending = paths
            .into_iter()
            .filter(|path| {
                !state.known.contains(path)
                    && !state.resolved.contains_key(path)
                    && unique.insert(path.clone())
            })
            .take(available)
            .collect::<Vec<_>>();
        let scheduled = pending.len();
        for path in pending {
            state.known.insert(path.clone());
            state.pending.push_back(path);
        }
        ready.notify_one();
        scheduled
    }

    /// Returns only in-memory results published by the background owner.
    pub fn reference(&self, path: &std::path::Path) -> Option<nanika_protocol::IconReference> {
        self.state
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .resolved
            .get(path)
            .cloned()
            .flatten()
    }

    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        let (lock, ready) = &*self.state;
        lock.lock()
            .unwrap_or_else(|error| error.into_inner())
            .shutdown = true;
        ready.notify_one();
        if thread.join().is_err() {
            eprintln!("clipboard file icon worker panicked");
        }
    }
}

fn publish_resolution(
    state: &mut State,
    path: PathBuf,
    reference: Option<nanika_protocol::IconReference>,
) {
    if state.resolved.len() == MAX_PENDING_PATHS
        && let Some(expired) = state.ready_order.pop_front()
    {
        state.resolved.remove(&expired);
    }
    state.ready_order.push_back(path.clone());
    state.resolved.insert(path, reference);
}

impl Drop for FileIconWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduling_is_ordered_deduplicated_and_bounded_at_capacity() {
        let worker = FileIconWorker {
            state: Arc::new((Mutex::new(State::default()), Condvar::new())),
            thread: None,
        };
        assert_eq!(
            worker.schedule([PathBuf::from("selected"), PathBuf::from("selected")]),
            1
        );
        let state = worker.state.0.lock().unwrap();
        assert_eq!(
            state.pending.iter().collect::<Vec<_>>(),
            [&PathBuf::from("selected")]
        );
        drop(state);

        assert_eq!(
            worker.schedule((1..MAX_PENDING_PATHS).map(|index| PathBuf::from(index.to_string()))),
            MAX_PENDING_PATHS - 1
        );
        let before = worker.state.0.lock().unwrap().pending.len();
        assert_eq!(worker.schedule([PathBuf::from("overflow")]), 0);
        assert_eq!(worker.state.0.lock().unwrap().pending.len(), before);
    }

    #[test]
    fn published_resolutions_are_bounded_in_insertion_order() {
        let mut state = State::default();
        for index in 0..=MAX_PENDING_PATHS {
            publish_resolution(
                &mut state,
                PathBuf::from(index.to_string()),
                Some(nanika_protocol::IconReference::new(format!("{index:064x}")).unwrap()),
            );
        }
        assert_eq!(state.resolved.len(), MAX_PENDING_PATHS);
        assert!(!state.resolved.contains_key(&PathBuf::from("0")));
        assert!(
            state
                .resolved
                .contains_key(&PathBuf::from(MAX_PENDING_PATHS.to_string()))
        );
    }

    #[test]
    fn failed_resolution_is_retained_without_exposing_an_icon() {
        let path = PathBuf::from("missing");
        let mut state = State::default();

        publish_resolution(&mut state, path.clone(), None);

        assert!(state.resolved.contains_key(&path));
        assert_eq!(state.resolved[&path], None);
    }
}
