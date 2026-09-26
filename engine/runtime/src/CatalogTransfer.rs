use crate::SupervisorError;
use nanika_protocol::CatalogBatch;

/// Batches are prepared outside the search owner and become visible in one commit.
#[derive(Default)]
pub(crate) struct CatalogTransfer {
    _transaction: u64,
    _index: u64,
    _replace: bool,
    _established: bool,
    _complete: bool,
    _entries: Vec<nanika_search::Candidate>,
    _removed: Vec<String>,
}

impl CatalogTransfer {
    pub(crate) fn accept(
        &mut self,
        extension_id: &str,
        batch: CatalogBatch,
    ) -> Result<bool, SupervisorError> {
        let invalid =
            || SupervisorError::UnexpectedMessage("catalog batches are out of sequence".into());
        if self._complete {
            return Err(invalid());
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
                    "candidate default action is not declared".into(),
                ));
            }
        }
        self._entries
            .extend(crate::search_candidates(extension_id, batch.entries));
        self._removed.extend(batch.removed);
        self._index += 1;
        self._complete = batch.complete;
        Ok(batch.complete)
    }

    pub(crate) fn commit(
        &mut self,
        search: &nanika_search::SearchHandle,
        extension_id: &str,
        contributions: Vec<nanika_protocol::Candidate>,
    ) -> Result<u64, SupervisorError> {
        if !self._complete {
            return Err(SupervisorError::UnexpectedMessage(
                "incomplete catalog transaction".into(),
            ));
        }
        let reserved = contributions
            .iter()
            .map(|entry| entry.entry_id.as_str())
            .collect::<std::collections::HashSet<_>>();
        self._entries
            .retain(|entry| !reserved.contains(entry.entry_id()));
        self._removed.retain(|id| !reserved.contains(id.as_str()));
        if self._replace {
            self._entries
                .extend(crate::search_candidates(extension_id, contributions));
        }
        search
            .commit_catalog(
                extension_id,
                self._replace,
                std::mem::take(&mut self._entries),
                std::mem::take(&mut self._removed),
            )
            .map_err(|error| SupervisorError::UnexpectedMessage(error.to_string()))?;
        self._index = 0;
        self._complete = false;
        self._established = true;
        Ok(self._transaction)
    }
}
