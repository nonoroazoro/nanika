use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::ApplicationEntry;

/// Committed root contributions and an identity-to-root index for local winner selection.
#[derive(Default)]
pub(crate) struct ApplicationSources {
    _roots: HashMap<String, HashMap<String, ApplicationEntry>>,
    _owners: HashMap<String, Vec<Arc<str>>>,
}

impl ApplicationSources {
    pub(crate) fn winners(&self) -> Vec<ApplicationEntry> {
        self._owners
            .iter()
            .filter_map(|(id, owners)| {
                owners
                    .iter()
                    .filter_map(|root| self._roots.get(root.as_ref())?.get(id))
                    .max_by(|left, right| {
                        left.priority
                            .cmp(&right.priority)
                            .then_with(|| right.source_key.cmp(&left.source_key))
                    })
                    .cloned()
            })
            .collect()
    }

    pub(crate) fn roots(&self) -> impl Iterator<Item = &String> {
        self._roots.keys()
    }

    pub(crate) fn entries(&self, root: &str) -> Option<&HashMap<String, ApplicationEntry>> {
        self._roots.get(root)
    }

    /// Stage winner changes without mutating the committed sources before the database commits.
    pub(crate) fn resolve<'a>(
        &'a self,
        root: &str,
        replacement: &'a HashMap<String, ApplicationEntry>,
    ) -> (Vec<&'a ApplicationEntry>, Vec<String>) {
        let affected = self
            ._roots
            .get(root)
            .into_iter()
            .flat_map(|entries| entries.keys())
            .chain(replacement.keys())
            .collect::<HashSet<_>>();
        let mut winners = Vec::with_capacity(affected.len());
        let mut removed = Vec::new();
        for id in affected {
            let winner = self
                ._owners
                .get(id)
                .into_iter()
                .flatten()
                .filter(|owner| owner.as_ref() != root)
                .filter_map(|owner| self._roots.get(owner.as_ref())?.get(id))
                .chain(replacement.get(id))
                .max_by(|left, right| {
                    left.priority
                        .cmp(&right.priority)
                        .then_with(|| right.source_key.cmp(&left.source_key))
                });
            match winner {
                Some(entry) => winners.push(entry),
                None => removed.push(id.clone()),
            }
        }
        (winners, removed)
    }

    pub(crate) fn commit(&mut self, root: String, replacement: HashMap<String, ApplicationEntry>) {
        let previous = self._roots.remove(&root);
        if let Some(previous) = &previous {
            for id in previous.keys().filter(|id| !replacement.contains_key(*id)) {
                if let Some(owners) = self._owners.get_mut(id) {
                    owners.retain(|owner| owner.as_ref() != root);
                    if owners.is_empty() {
                        self._owners.remove(id);
                    }
                }
            }
        }
        if replacement.is_empty() {
            return;
        }
        // Root paths are shared, and unchanged memberships keep their existing allocation.
        let owner: Arc<str> = Arc::from(root.as_str());
        for id in replacement.keys().filter(|id| {
            previous
                .as_ref()
                .is_none_or(|previous| !previous.contains_key(*id))
        }) {
            self._owners
                .entry(id.clone())
                .or_default()
                .push(Arc::clone(&owner));
        }
        self._roots.insert(root, replacement);
    }
}
