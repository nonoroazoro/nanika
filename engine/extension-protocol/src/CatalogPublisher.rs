use crate::{Candidate, CatalogBatch, CatalogPublication};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Extension-owned publication state. A batch target controls scheduling, never admission.
#[derive(Default)]
pub struct CatalogPublisher {
    _entries: HashMap<String, Arc<Candidate>>,
    _dirty: HashSet<String>,
    _pending: Option<CatalogPublication>,
    _revision: u64,
    _established: bool,
}

impl CatalogPublisher {
    pub fn update(
        &mut self,
        entries: impl IntoIterator<Item = Candidate>,
        removed: impl IntoIterator<Item = String>,
    ) -> bool {
        let mut changed = false;
        for id in removed {
            if self._entries.remove(&id).is_some() {
                self._dirty.insert(id);
                changed = true;
            }
        }
        for entry in entries {
            if self
                ._entries
                .get(&entry.entry_id)
                .is_none_or(|previous| **previous != entry)
            {
                self._dirty.insert(entry.entry_id.clone());
                self._entries
                    .insert(entry.entry_id.clone(), Arc::new(entry));
                changed = true;
            }
        }
        changed
    }

    pub fn read(&mut self) -> Result<CatalogBatch, String> {
        const BATCH_ENTRIES: usize = 256;
        if self._pending.is_none() {
            self._revision = self
                ._revision
                .checked_add(1)
                .ok_or("catalog revision exhausted")?;
            let dirty = std::mem::take(&mut self._dirty);
            let entries = if self._established {
                dirty
                    .iter()
                    .filter_map(|id| self._entries.get(id).cloned())
                    .collect()
            } else {
                self._entries.values().cloned().collect()
            };
            let removed = if self._established {
                dirty
                    .into_iter()
                    .filter(|id| !self._entries.contains_key(id))
                    .collect()
            } else {
                Default::default()
            };
            self._pending = Some(CatalogPublication {
                _transaction: self._revision,
                _index: 0,
                _replace: !self._established,
                _complete: false,
                _entries: entries,
                _removed: removed,
            });
        }
        let pending = self._pending.as_mut().expect("publication established");
        if pending._complete {
            return Err("catalog commit acknowledgement is required".into());
        }
        let entries = pending
            ._entries
            .drain(..pending._entries.len().min(BATCH_ENTRIES))
            .map(|entry| (*entry).clone())
            .collect::<Vec<_>>();
        let remaining = BATCH_ENTRIES - entries.len();
        let removed = pending
            ._removed
            .drain(..pending._removed.len().min(remaining))
            .collect();
        pending._complete = pending._entries.is_empty() && pending._removed.is_empty();
        let batch = CatalogBatch {
            transaction: pending._transaction,
            index: pending._index,
            replace: pending._replace,
            complete: pending._complete,
            entries,
            removed,
        };
        pending._index += 1;
        Ok(batch)
    }

    /// Release an applied publication. Changes made while it was in flight remain dirty.
    pub fn acknowledge(&mut self, transaction: u64) -> Result<bool, String> {
        if !self
            ._pending
            .as_ref()
            .is_some_and(|pending| pending._transaction == transaction && pending._complete)
        {
            return Err("catalog acknowledgement does not identify a completed publication".into());
        }
        self._pending = None;
        self._established = true;
        Ok(!self._dirty.is_empty())
    }
}
