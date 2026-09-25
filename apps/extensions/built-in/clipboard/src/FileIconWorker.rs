use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

const MAX_PENDING_PATHS: usize = 512;

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
                    // Settle failed paths so view refreshes neither retry them nor stall scheduling.
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

    /// `None` is pending; `Some(None)` is a settled failure that must not block previews.
    pub fn resolution(
        &self,
        path: &std::path::Path,
    ) -> Option<Option<nanika_protocol::IconReference>> {
        self.state
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .resolved
            .get(path)
            .cloned()
    }

    pub fn shutdown(mut self) -> Result<(), String> {
        self._stop()
    }

    fn _stop(&mut self) -> Result<(), String> {
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };
        let (lock, ready) = &*self.state;
        lock.lock()
            .unwrap_or_else(|error| error.into_inner())
            .shutdown = true;
        ready.notify_one();
        if thread.join().is_err() {
            return Err("clipboard file icon worker panicked".to_owned());
        }
        Ok(())
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
        if let Err(error) = self._stop() {
            eprintln!("{error}");
        }
    }
}

#[cfg(test)]
#[path = "../tests/FileIconWorker.rs"]
mod tests;
