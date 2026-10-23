use crate::SupervisorError;
use nanika_protocol::CatalogBatch;

/// Bounded raw staging is indexed only after a complete transaction is admitted.
/// Rejection releases staged input and leaves the published catalog untouched.
#[derive(Default)]
pub(crate) struct CatalogTransfer {
    _transaction: u64,
    _index: u64,
    _replace: bool,
    _established: bool,
    _complete: bool,
    _entries: Vec<nanika_protocol::Candidate>,
    _removed: Vec<String>,
    _budget: crate::CatalogBudget,
    _rejected: bool
}

#[cfg(test)]
#[path = "../tests/CatalogTransfer.rs"]
mod tests;

impl CatalogTransfer {
    pub(crate) fn accept(&mut self, batch: CatalogBatch) -> Result<bool, SupervisorError> {
        let invalid =
            || SupervisorError::UnexpectedMessage("catalog batches are out of sequence".into());
        if self._complete || self._rejected {
            return Err(invalid());
        }
        if let Err(error) = self._budget.admit(&batch) {
            // The peer owns an unacknowledged publication. Only a new instance
            // may establish another baseline after exhausting its staging budget.
            self._rejected = true;
            self._entries = Vec::new();
            self._removed = Vec::new();
            return Err(SupervisorError::UnexpectedMessage(error));
        }
        if self._index == 0 {
            if batch.transaction <= self._transaction
                || batch.index != 0
                || (!self._established && !batch.replace)
            {
                return Err(invalid());
            }
            self._transaction = batch.transaction;
            self._replace = batch.replace;
        } else if batch.transaction != self._transaction
            || batch.index != self._index
            || batch.replace != self._replace
        {
            return Err(invalid());
        }
        for entry in &batch.entries {
            nanika_protocol::validate_actions(&entry.actions)
                .map_err(SupervisorError::UnexpectedMessage)?;
            if !entry
                .actions
                .iter()
                .any(|action| action.id == entry.action_id)
            {
                return Err(SupervisorError::UnexpectedMessage(
                    "candidate default action is not declared".into()
                ));
            }
        }
        self._entries.extend(batch.entries);
        self._removed.extend(batch.removed);
        self._index += 1;
        self._complete = batch.complete;
        Ok(batch.complete)
    }

    pub(crate) fn commit(
        &mut self,
        search: &nanika_search::SearchContributor,
        contributions: &[nanika_search::Candidate],
        resources: Option<&nanika_localization::ResourceSet>
    ) -> Result<Option<u64>, SupervisorError> {
        if !self._complete || self._rejected {
            return Err(SupervisorError::UnexpectedMessage(
                "incomplete catalog transaction".into()
            ));
        }
        let extension_id = search.extension_id();
        let reserved = contributions
            .iter()
            .map(|entry| entry.entry_id())
            .collect::<std::collections::HashSet<_>>();
        self._entries
            .retain(|entry| !reserved.contains(entry.entry_id.as_str()));
        self._removed.retain(|id| !reserved.contains(id.as_str()));
        let mut entries = match crate::search_candidates(
            extension_id,
            std::mem::take(&mut self._entries),
            resources
        ) {
            Ok(entries) => entries,
            Err(error) => {
                self._rejected = true;
                self._removed.clear();
                return Err(SupervisorError::UnexpectedMessage(error));
            }
        };
        if self._replace {
            entries.extend(contributions.iter().cloned());
        }
        let committed =
            search.commit_catalog(self._replace, entries, std::mem::take(&mut self._removed));
        match committed {
            Err(nanika_search::SearchQueueError::Retired) => return Ok(None),
            Err(error) => return Err(SupervisorError::UnexpectedMessage(error.to_string())),
            Ok(()) => {}
        }
        self._index = 0;
        self._complete = false;
        self._established = true;
        self._budget = Default::default();
        Ok(Some(self._transaction))
    }
}
