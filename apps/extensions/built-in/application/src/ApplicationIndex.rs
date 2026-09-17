use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use walkdir::WalkDir;

use crate::normalization::path_key;
use crate::platform;
use crate::{
    ApplicationConfig, ApplicationDatabase, ApplicationEntry, ApplicationError, DiscoveryState,
    IconCache, ScanReport,
};

/// Cancellable discovery and transactional indexing boundary.
pub struct ApplicationIndex {
    database: ApplicationDatabase,
    icon_cache: IconCache,
    discovery_state: DiscoveryState,
    pending_icons: Vec<ApplicationEntry>,
}

impl ApplicationIndex {
    pub fn new(database: ApplicationDatabase, icon_cache: IconCache) -> Self {
        Self {
            database,
            icon_cache,
            discovery_state: DiscoveryState::new(),
            pending_icons: Vec::new(),
        }
    }

    pub fn load(&self) -> Result<Vec<ApplicationEntry>, ApplicationError> {
        self.database.load_entries()
    }

    pub(crate) fn load_presentable(&self) -> Result<Vec<ApplicationEntry>, ApplicationError> {
        let mut entries = self.load()?;
        if let Err(error) = self.icon_cache.use_available_icons(&mut entries) {
            eprintln!("application icon cache is unavailable: {error}");
            for entry in &mut entries {
                entry.icon_key.clear();
            }
        }
        Ok(entries)
    }

    pub fn scan(
        &mut self,
        config: &ApplicationConfig,
        generation: u64,
        cancelled_through: &AtomicU64,
    ) -> Result<(ScanReport, Vec<ApplicationEntry>), ApplicationError> {
        self.database.begin_scan(generation)?;
        self.discovery_state.begin_scan();
        let standard_roots = match platform::standard_roots() {
            Ok(roots) => roots,
            Err(error) => {
                if let Err(record_error) = self.database.fail_scan(generation, &error.to_string()) {
                    eprintln!(
                        "application scan failure could not be recorded: {record_error}; original failure: {error}"
                    );
                }
                return Err(error);
            }
        };
        let mut roots = standard_roots
            .into_iter()
            .map(|path| (path, 0_usize))
            .chain(config.roots.iter().cloned().map(|path| (path, 1_usize)))
            .collect::<Vec<_>>();
        deduplicate_paths(&mut roots);

        let mut entries = HashMap::<String, ApplicationEntry>::new();
        let mut warnings = 0_usize;
        let mut complete = true;
        for (root, priority) in &roots {
            if is_cancelled(cancelled_through, generation) {
                break;
            }
            if is_excluded(root, &config.exclusions) {
                continue;
            }
            let metadata = match root.symlink_metadata() {
                Ok(metadata) => metadata,
                // A deleted source contributes no entries. A complete scan can
                // then retire its old records without recreating the directory.
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    eprintln!(
                        "application scan could not read root {}: {error}",
                        root.display()
                    );
                    warnings = warnings.saturating_add(1);
                    complete = false;
                    continue;
                }
            };
            if metadata.file_type().is_symlink() {
                eprintln!("application scan skipped symlink root: {}", root.display());
                warnings = warnings.saturating_add(1);
                complete = false;
                continue;
            }
            if metadata.is_file() || platform::is_application_bundle(root) {
                complete &= collect_entry(
                    root,
                    *priority,
                    &mut self.discovery_state,
                    &mut entries,
                    &mut warnings,
                );
                continue;
            }
            if !metadata.is_dir() {
                eprintln!("application scan root is unavailable: {}", root.display());
                warnings = warnings.saturating_add(1);
                complete = false;
                continue;
            }
            let mut walker = WalkDir::new(root).follow_links(false).into_iter();
            while let Some(result) = walker.next() {
                if is_cancelled(cancelled_through, generation) {
                    break;
                }
                let entry = match result {
                    Ok(entry) => entry,
                    Err(error) => {
                        eprintln!("application scan could not read a path: {error}");
                        warnings = warnings.saturating_add(1);
                        complete = false;
                        continue;
                    }
                };
                let path = entry.path();
                if entry.depth() > 0 && is_excluded(path, &config.exclusions) {
                    if entry.file_type().is_dir() {
                        walker.skip_current_dir();
                    }
                    continue;
                }
                if platform::is_application_bundle(path) {
                    complete &= collect_entry(
                        path,
                        *priority,
                        &mut self.discovery_state,
                        &mut entries,
                        &mut warnings,
                    );
                    walker.skip_current_dir();
                } else if entry.file_type().is_file() && platform::is_application_path(path) {
                    complete &= collect_entry(
                        path,
                        *priority,
                        &mut self.discovery_state,
                        &mut entries,
                        &mut warnings,
                    );
                }
            }
        }

        let was_cancelled = is_cancelled(cancelled_through, generation);
        complete &= !was_cancelled;
        let mut entries = entries.into_values().collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            left.normalized_name
                .cmp(&right.normalized_name)
                .then_with(|| left.entry_id.cmp(&right.entry_id))
        });
        for entry in &mut entries {
            if !entry.icon_key.is_empty() {
                continue;
            }
            match self
                .icon_cache
                .key_with_state(entry, &mut self.discovery_state)
            {
                Ok(key) => entry.icon_key = key,
                Err(error) => {
                    eprintln!(
                        "application icon key failed for {}: {error}",
                        entry.target_path
                    );
                    entry.icon_source = None;
                    if let Err(fallback_error) = self.icon_cache.prepare(entry) {
                        eprintln!(
                            "application fallback icon failed for {}: {fallback_error}",
                            entry.target_path
                        );
                    }
                    entry.icon_key = IconCache::fallback_key().to_owned();
                }
            }
        }
        let was_cancelled = is_cancelled(cancelled_through, generation);
        complete &= !was_cancelled;
        let report = ScanReport {
            generation,
            discovered: entries.len(),
            warnings,
            complete,
            cancelled: was_cancelled,
        };
        let error = (warnings > 0).then(|| format!("scan completed with {warnings} warnings"));
        if let Err(commit_error) = self
            .database
            .commit_scan(report, &entries, error.as_deref())
        {
            if let Err(record_error) = self
                .database
                .fail_scan(generation, &commit_error.to_string())
            {
                eprintln!(
                    "application scan commit failure could not be recorded: {record_error}; original failure: {commit_error}"
                );
            }
            return Err(commit_error);
        }
        self.pending_icons = entries;
        Ok((report, self.load_presentable()?))
    }

    /// Populates at most `limit` pending icons and reports whether more work remains.
    ///
    /// Keeping batches bounded lets the runtime publish the first visible icon set before
    /// continuing with lower-priority cache work.
    pub fn populate_icon_batch(
        &mut self,
        cancelled_through: &AtomicU64,
        generation: u64,
        limit: usize,
    ) -> Result<(Vec<String>, bool), ApplicationError> {
        let mut failures = Vec::new();
        let count = limit.min(self.pending_icons.len());
        let mut processed = 0;
        for entry in &mut self.pending_icons[..count] {
            if is_cancelled(cancelled_through, generation) {
                break;
            }
            processed += 1;
            if let Err(error) = self.icon_cache.prepare(entry) {
                failures.push(format!("{}: {error}", entry.target_path));
            }
        }
        self.pending_icons.drain(..processed);
        Ok((failures, !self.pending_icons.is_empty()))
    }

    pub(crate) fn prioritize_pending_icons(&mut self, entry_ids: &[String]) {
        let priorities = entry_ids
            .iter()
            .enumerate()
            .map(|(index, entry_id)| (entry_id.as_str(), index))
            .collect::<HashMap<_, _>>();
        self.pending_icons.sort_by_key(|entry| {
            priorities
                .get(entry.entry_id.as_str())
                .copied()
                .unwrap_or(usize::MAX)
        });
    }

    pub(crate) fn has_pending_icons(&self) -> bool {
        !self.pending_icons.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn has_pending_icon(&self, entry_ids: &[String]) -> bool {
        entry_ids.iter().any(|entry_id| {
            self.pending_icons
                .iter()
                .any(|entry| entry.entry_id == *entry_id)
        })
    }
}

fn is_cancelled(cancelled_through: &AtomicU64, generation: u64) -> bool {
    cancelled_through.load(Ordering::Acquire) >= generation
}

fn collect_entry(
    path: &Path,
    priority: usize,
    discovery_state: &mut DiscoveryState,
    entries: &mut HashMap<String, ApplicationEntry>,
    warnings: &mut usize,
) -> bool {
    match platform::read_entry(discovery_state, path, priority) {
        Ok(Some(entry)) => match entries.get(&entry.entry_id) {
            Some(existing)
                if existing.priority > entry.priority
                    || (existing.priority == entry.priority
                        && existing.source_key <= entry.source_key) => {}
            _ => {
                entries.insert(entry.entry_id.clone(), entry);
            }
        },
        Ok(None) => {}
        Err(error) => {
            eprintln!(
                "application entry could not be read at {}: {error}",
                path.display()
            );
            *warnings = warnings.saturating_add(1);
            return !matches!(error, ApplicationError::Io(_));
        }
    }
    true
}

fn deduplicate_paths(paths: &mut Vec<(PathBuf, usize)>) {
    let mut unique = HashMap::new();
    for (path, priority) in paths.drain(..) {
        let key = path_key(&path);
        match unique.get(&key) {
            Some((_, existing_priority)) if *existing_priority > priority => {}
            _ => {
                unique.insert(key, (path, priority));
            }
        }
    }
    paths.extend(unique.into_values());
    paths.sort_by_key(|(path, _)| path_key(path));
}

fn is_excluded(path: &Path, exclusions: &[PathBuf]) -> bool {
    let path = path_key(path);
    exclusions.iter().any(|excluded| {
        let excluded = path_key(excluded).trim_end_matches('/').to_owned();
        path == excluded
            || path
                .strip_prefix(&excluded)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

#[cfg(test)]
mod priority_tests {
    use super::*;

    fn entry(id: &str) -> ApplicationEntry {
        ApplicationEntry {
            entry_id: id.to_owned(),
            source_key: id.to_owned(),
            display_name: id.to_owned(),
            normalized_name: id.to_owned(),
            normalized_tokens: id.to_owned(),
            launch_kind: "macos-bundle".to_owned(),
            target_path: format!("/{id}.app"),
            working_directory: None,
            arguments_json: "{\"kind\":\"structured\",\"values\":[]}".to_owned(),
            bundle_id: None,
            icon_key: id.to_owned(),
            icon_source: None,
            icon_index: 0,
            priority: 0,
        }
    }

    #[test]
    fn host_visible_entries_move_to_the_front_in_host_order() {
        let root = std::env::temp_dir().join(format!(
            "nanika-application-priority-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut index = ApplicationIndex::new(
            ApplicationDatabase::open(root.join("index.db")).unwrap(),
            IconCache::new(root.join("icons")),
        );
        index.pending_icons = vec![entry("a"), entry("b"), entry("c"), entry("d")];
        assert!(index.has_pending_icon(&["b".to_owned()]));
        assert!(!index.has_pending_icon(&["missing".to_owned()]));
        index.prioritize_pending_icons(&["d".to_owned(), "b".to_owned()]);
        assert_eq!(
            index
                .pending_icons
                .iter()
                .map(|entry| entry.entry_id.as_str())
                .collect::<Vec<_>>(),
            ["d", "b", "a", "c"]
        );
        drop(index);
        std::fs::remove_dir_all(root).unwrap();
    }
}
