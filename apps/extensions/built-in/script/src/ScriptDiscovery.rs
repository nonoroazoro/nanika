use crate::{RuntimeEvent, ScriptScan};
use nanika_extension_script::{ScriptCatalog, ScriptEntry};
use std::collections::BTreeMap;
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, Sender, SyncSender},
};
use std::thread::JoinHandle;

/// Owns the event consumer and worker so EOF and failed writes always release blocked sends.
pub(crate) struct ScriptDiscovery {
    _events: Option<Receiver<RuntimeEvent>>,
    _commands: Option<Sender<ScriptScan>>,
    _cancelled: Arc<AtomicU64>,
    _thread: Option<JoinHandle<()>>,
}

impl ScriptDiscovery {
    pub(crate) fn spawn(
        events: Receiver<RuntimeEvent>,
        sender: SyncSender<RuntimeEvent>,
        entries: Arc<RwLock<BTreeMap<String, ScriptEntry>>>,
    ) -> std::io::Result<Self> {
        let (commands, requests) = mpsc::channel::<ScriptScan>();
        let cancelled = Arc::new(AtomicU64::new(0));
        let worker_cancelled = Arc::clone(&cancelled);
        let thread = std::thread::Builder::new()
            .name("nanika-script-discovery".to_owned())
            .spawn(move || {
                let mut catalog = ScriptCatalog::default();
                while let Ok(scan) = requests.recv() {
                    if worker_cancelled.load(Ordering::Acquire) == u64::MAX {
                        break;
                    }
                    let is_cancelled =
                        || worker_cancelled.load(Ordering::Acquire) >= scan.generation;
                    let touched =
                        std::cell::RefCell::new(std::collections::HashSet::<String>::new());
                    let publish = |updated: Vec<ScriptEntry>, removed: Vec<String>| {
                        let mut current =
                            entries.write().unwrap_or_else(|error| error.into_inner());
                        let mut ids = removed.clone();
                        for id in removed {
                            current.remove(&id);
                        }
                        for entry in updated {
                            ids.push(entry.id.clone());
                            current.insert(entry.id.clone(), entry);
                        }
                        drop(current);
                        touched.borrow_mut().extend(ids.iter().cloned());
                        if !ids.is_empty() {
                            let _ = sender.send(RuntimeEvent::CatalogUpdated { entry_ids: ids });
                        }
                    };
                    // A failed settings application restores only the identities it touched.
                    let result = if scan.configuration {
                        let mut staged = catalog.clone();
                        let result = staged.scan(&scan.config, is_cancelled, &publish);
                        if result.is_ok() {
                            catalog = staged;
                        } else {
                            let ids = touched.borrow().iter().cloned().collect::<Vec<_>>();
                            let updated = ids
                                .iter()
                                .filter_map(|id| catalog.get(id).cloned())
                                .collect();
                            let removed = ids
                                .into_iter()
                                .filter(|id| catalog.get(id).is_none())
                                .collect();
                            publish(updated, removed);
                        }
                        result
                    } else {
                        catalog.scan(&scan.config, is_cancelled, publish)
                    };
                    if sender
                        .send(RuntimeEvent::ScanFinished {
                            request_id: scan.request_id,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })?;
        Ok(Self {
            _events: Some(events),
            _commands: Some(commands),
            _cancelled: cancelled,
            _thread: Some(thread),
        })
    }

    pub(crate) fn scan(&self, request: ScriptScan) -> Result<(), String> {
        self._commands
            .as_ref()
            .expect("discovery is active")
            .send(request)
            .map_err(|_| "script discovery worker is closed".to_owned())
    }

    pub(crate) fn receive(&self) -> Result<RuntimeEvent, mpsc::RecvError> {
        self._events.as_ref().expect("discovery is active").recv()
    }

    pub(crate) fn cancel(&self, generation: u64) {
        self._cancelled.fetch_max(generation, Ordering::AcqRel);
    }
}

impl Drop for ScriptDiscovery {
    fn drop(&mut self) {
        self.cancel(u64::MAX);
        self._events.take();
        self._commands.take();
        if let Some(thread) = self._thread.take()
            && thread.join().is_err()
        {
            eprintln!("script discovery worker panicked");
        }
    }
}
