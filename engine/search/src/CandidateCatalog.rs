use crate::Candidate;
use std::collections::HashMap;

#[derive(Clone, Default)]
pub(crate) struct CandidateCatalog {
    _entries: HashMap<String, HashMap<String, Candidate>>,
}

impl CandidateCatalog {
    pub(crate) fn new(extension_id: &str, candidates: Vec<Candidate>) -> Self {
        let mut catalog = Self::default();
        catalog.update(extension_id, candidates, Vec::new());
        catalog
    }

    pub(crate) fn update(
        &mut self,
        extension_id: &str,
        candidates: Vec<Candidate>,
        removed: Vec<String>,
    ) {
        for id in removed {
            self._entries.remove(&id);
        }
        let mut replaced = std::collections::HashSet::new();
        for mut candidate in candidates {
            if replaced.insert(candidate.entry_id().to_owned()) {
                self._entries.remove(candidate.entry_id());
            }
            candidate.set_extension_id(extension_id);
            self._entries
                .entry(candidate.entry_id().to_owned())
                .or_default()
                .insert(candidate.action_id().to_owned(), candidate);
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Candidate> {
        self._entries.values().flat_map(|actions| actions.values())
    }
}
