use std::path::Path;

use crate::normalization::path_key;

/// Paths whose old records can be replaced after an uninterrupted scan.
pub(crate) struct ScanCoverage {
    roots: Vec<String>,
    failed: Vec<String>,
    roots_resolved: bool,
}

impl ScanCoverage {
    pub(crate) fn new(roots: impl Iterator<Item = String>, roots_resolved: bool) -> Self {
        Self {
            roots: roots.collect(),
            failed: Vec::new(),
            roots_resolved,
        }
    }

    pub(crate) fn failed(&mut self, path: &Path) {
        self.failed.push(path_key(path));
    }

    pub(crate) fn replaces(&self, source: &str) -> bool {
        // An unresolved built-in root has no trustworthy path. Only known roots
        // may be reconciled until all configured roots resolve successfully.
        (self.roots_resolved || self.roots.iter().any(|root| _contains(root, source)))
            && !self.failed.iter().any(|path| _contains(path, source))
    }
}

fn _contains(parent: &str, child: &str) -> bool {
    let parent = parent.trim_end_matches('/');
    child == parent
        || child
            .strip_prefix(parent)
            .is_some_and(|suffix| suffix.starts_with('/'))
}
