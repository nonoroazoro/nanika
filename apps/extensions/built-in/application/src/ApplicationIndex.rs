use std::collections::{HashMap, HashSet};
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
    prepared_entries: Option<HashMap<String, ApplicationEntry>>,
    pending_icons: HashSet<String>,
    sources: Option<crate::application_sources::ApplicationSources>,
}

impl ApplicationIndex {
    pub fn new(database: ApplicationDatabase, icon_cache: IconCache) -> Self {
        Self {
            database,
            icon_cache,
            discovery_state: DiscoveryState::new(),
            prepared_entries: None,
            pending_icons: HashSet::new(),
            sources: None,
        }
    }

    pub fn load(&mut self) -> Result<Vec<ApplicationEntry>, ApplicationError> {
        if self.prepared_entries.is_none() {
            let mut sources = crate::application_sources::ApplicationSources::default();
            for (root, entries) in self.database.load_sources()? {
                sources.commit(root, entries);
            }
            let entries = sources.winners();
            self.sources = Some(sources);
            self.prepared_entries = Some(
                entries
                    .into_iter()
                    .map(|entry| (entry.entry_id.clone(), entry))
                    .collect(),
            );
        }
        let mut entries = self
            .prepared_entries
            .as_ref()
            .expect("catalog loaded")
            .values()
            .cloned()
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            left.normalized_name
                .cmp(&right.normalized_name)
                .then_with(|| left.entry_id.cmp(&right.entry_id))
        });
        Ok(entries)
    }

    pub fn scan(
        &mut self,
        config: &ApplicationConfig,
        generation: u64,
        cancelled_through: &AtomicU64,
        progress: impl FnMut(nanika_protocol::OperationProgress),
        publish: impl FnMut(Vec<ApplicationEntry>, Vec<String>),
    ) -> Result<ScanReport, ApplicationError> {
        let result = self._scan_roots(config, generation, cancelled_through, progress, publish);
        // The owner cannot run icon work during a scan. Restore its queue invariant before
        // returning on success, cancellation, or failure, preserving each committed root.
        self.pending_icons.retain(|id| {
            self.prepared_entries
                .as_ref()
                .is_some_and(|entries| entries.contains_key(id))
        });
        result
    }

    /// Populate at most `limit` icons so visible results can publish first.
    pub fn populate_icon_batch(
        &mut self,
        cancelled_through: &AtomicU64,
        generation: u64,
        limit: usize,
        requested: &[String],
    ) -> (Vec<String>, Vec<ApplicationEntry>) {
        let mut failures = Vec::new();
        let ids = requested
            .iter()
            .filter(|id| self.pending_icons.contains(*id))
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let entries = self
            .prepared_entries
            .as_mut()
            .expect("pending icons belong to the prepared catalog");
        let mut updated = Vec::new();
        for id in &ids {
            if is_cancelled(cancelled_through, generation) {
                break;
            }
            self.pending_icons.remove(id);
            let entry = entries.get_mut(id).expect("pending icon entry");
            if let Err(error) = self.icon_cache.prepare(entry) {
                failures.push(format!("{}: {error}", entry.target_path));
            }
            updated.push(entry.clone());
        }
        if let Err(error) = self.icon_cache.use_available_icons(&mut updated) {
            failures.push(error.to_string());
            for entry in &mut updated {
                entry.icon_key.clear();
            }
        }
        (failures, updated)
    }

    pub(crate) fn has_pending_icons_for(&self, requested: &[String]) -> bool {
        requested.iter().any(|id| self.pending_icons.contains(id))
    }

    pub(crate) fn has_pending_icons(&self) -> bool {
        !self.pending_icons.is_empty()
    }
    fn _scan_roots(
        &mut self,
        config: &ApplicationConfig,
        generation: u64,
        cancelled_through: &AtomicU64,
        mut progress: impl FnMut(nanika_protocol::OperationProgress),
        mut publish: impl FnMut(Vec<ApplicationEntry>, Vec<String>),
    ) -> Result<ScanReport, ApplicationError> {
        progress(nanika_protocol::OperationProgress {
            label: "Finding application sources".to_owned(),
            completed: 0,
            total: None,
        });
        self.discovery_state.begin_scan();
        let exclusions = config
            .exclusions
            .iter()
            .map(|path| path_key(path).trim_end_matches('/').to_owned())
            .collect::<Vec<_>>();
        let standard_roots = platform::configured_roots(&config.enabled_builtin_roots)?;
        let roots_resolved = standard_roots
            .failures
            .iter()
            .all(|failure| failure.path.is_some());
        let mut warnings = standard_roots.failures.len();
        let mut complete = warnings == 0;
        for failure in &standard_roots.failures {
            eprintln!("application discovery source failed: {}", failure.message);
        }
        let mut roots = standard_roots
            .paths
            .into_iter()
            .map(|path| (path, 0_usize))
            .chain(config.roots.iter().cloned().map(|path| (path, 1_usize)))
            .collect::<Vec<_>>();
        deduplicate_paths(&mut roots);

        let mut coverage = crate::scan_coverage::ScanCoverage::new(
            roots.iter().map(|(path, _)| path_key(path)),
            roots_resolved,
        );
        for path in standard_roots
            .failures
            .iter()
            .filter_map(|failure| failure.path.as_deref())
        {
            coverage.failed(path);
        }
        let mut discovered = HashSet::new();
        if self.prepared_entries.is_none() {
            self.load()?;
        }
        self.pending_icons.clear();
        let total = u32::try_from(roots.len())
            .unwrap_or(u32::MAX - 1)
            .saturating_add(1);
        for (index, (root, priority)) in roots.iter().enumerate() {
            progress(nanika_protocol::OperationProgress {
                label: "Scanning application sources".to_owned(),
                completed: index as u32,
                total: Some(total),
            });
            if is_cancelled(cancelled_through, generation) {
                break;
            }
            let mut root_entries = HashMap::new();
            'root_scan: {
                if is_excluded(root, &exclusions) {
                    break 'root_scan;
                }
                let metadata = match root.symlink_metadata() {
                    Ok(metadata) => metadata,
                    // A deleted source is empty; complete scans retire its records without recreating it.
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => break 'root_scan,
                    Err(error) => {
                        coverage.failed(root);
                        eprintln!(
                            "application scan could not read root {}: {error}",
                            root.display()
                        );
                        warnings = warnings.saturating_add(1);
                        complete = false;
                        break 'root_scan;
                    }
                };
                if metadata.file_type().is_symlink() {
                    coverage.failed(root);
                    eprintln!("application scan skipped symlink root: {}", root.display());
                    warnings = warnings.saturating_add(1);
                    complete = false;
                    break 'root_scan;
                }
                if metadata.is_file() || platform::is_application_bundle(root) {
                    complete &= collect_entry(
                        root,
                        *priority,
                        &mut self.discovery_state,
                        &mut root_entries,
                        &mut warnings,
                        &mut coverage,
                    );
                    break 'root_scan;
                }
                if !metadata.is_dir() {
                    coverage.failed(root);
                    eprintln!("application scan root is unavailable: {}", root.display());
                    warnings = warnings.saturating_add(1);
                    complete = false;
                    break 'root_scan;
                }
                let mut walker = WalkDir::new(root).follow_links(false).into_iter();
                while let Some(result) = walker.next() {
                    if is_cancelled(cancelled_through, generation) {
                        break;
                    }
                    let entry = match result {
                        Ok(entry) => entry,
                        Err(error) => {
                            coverage.failed(error.path().unwrap_or(root));
                            eprintln!("application scan could not read a path: {error}");
                            warnings = warnings.saturating_add(1);
                            complete = false;
                            continue;
                        }
                    };
                    let path = entry.path();
                    // Directory enumeration already knows the type. Avoid an extra stat for
                    // every irrelevant file merely to ask whether it is an application package.
                    if !entry.file_type().is_dir()
                        && !(entry.file_type().is_file() && platform::is_application_path(path))
                    {
                        continue;
                    }
                    if entry.depth() > 0 && is_excluded(path, &exclusions) {
                        if entry.file_type().is_dir() {
                            walker.skip_current_dir();
                        }
                        continue;
                    }
                    if entry.file_type().is_dir() && platform::is_application_bundle(path) {
                        complete &= collect_entry(
                            path,
                            *priority,
                            &mut self.discovery_state,
                            &mut root_entries,
                            &mut warnings,
                            &mut coverage,
                        );
                        walker.skip_current_dir();
                    } else if entry.file_type().is_file() && platform::is_application_path(path) {
                        complete &= collect_entry(
                            path,
                            *priority,
                            &mut self.discovery_state,
                            &mut root_entries,
                            &mut warnings,
                            &mut coverage,
                        );
                    }
                }
            }
            // Cancellation discards this root's staging area, retaining earlier commits.
            if is_cancelled(cancelled_through, generation) {
                break;
            }
            for entry in root_entries.values_mut() {
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
            if is_cancelled(cancelled_through, generation) {
                break;
            }
            discovered.extend(root_entries.keys().cloned());
            let root_key = path_key(root);
            // A failed subtree keeps its previous contribution, including non-winning sources.
            if let Some(previous) = self
                .sources
                .as_ref()
                .expect("sources initialized")
                .entries(&root_key)
            {
                for entry in previous.values() {
                    if !coverage.replaces(&entry.source_key) {
                        _insert_preferred(&mut root_entries, entry.clone());
                    }
                }
            }
            self._commit_root(root_key, root_entries, &mut publish)?;
        }

        progress(nanika_protocol::OperationProgress {
            label: "Updating application index".to_owned(),
            completed: total - 1,
            total: Some(total),
        });
        let was_cancelled = is_cancelled(cancelled_through, generation);
        complete &= !was_cancelled;
        let report = ScanReport {
            generation,
            discovered: discovered.len(),
            warnings,
            complete,
            cancelled: was_cancelled,
        };
        if !was_cancelled {
            let configured = roots
                .iter()
                .map(|(path, _)| path_key(path))
                .collect::<HashSet<_>>();
            let obsolete = self
                .sources
                .as_ref()
                .expect("sources initialized")
                .roots()
                .filter(|root| !configured.contains(*root))
                .cloned()
                .collect::<Vec<_>>();
            for root in obsolete {
                let retained = self
                    .sources
                    .as_ref()
                    .expect("sources initialized")
                    .entries(&root)
                    .into_iter()
                    .flat_map(|entries| entries.iter())
                    .filter(|(_, entry)| !coverage.replaces(&entry.source_key))
                    .map(|(id, entry)| (id.clone(), entry.clone()))
                    .collect();
                self._commit_root(root, retained, &mut publish)?;
            }
        }
        Ok(report)
    }

    fn _commit_root(
        &mut self,
        root: String,
        mut replacement: HashMap<String, ApplicationEntry>,
        publish: &mut impl FnMut(Vec<ApplicationEntry>, Vec<String>),
    ) -> Result<(), ApplicationError> {
        let sources = self.sources.as_ref().expect("sources initialized");
        if let Some(previous) = sources.entries(&root) {
            for (id, entry) in &mut replacement {
                if let Some(old) = previous.get(id).filter(|old| *old == entry) {
                    *entry = old.clone();
                }
            }
        }
        let (winners, removed) = sources.resolve(&root, &replacement);
        let current = self.prepared_entries.as_mut().expect("catalog loaded");
        let committed = winners
            .iter()
            .filter(|entry| {
                current
                    .get(&entry.entry_id)
                    .is_none_or(|old| !_same_metadata(old, entry))
            })
            .map(|entry| (*entry).clone())
            .collect::<Vec<_>>();
        let previous = sources.entries(&root);
        let changed_sources = replacement
            .values()
            .filter(|entry| {
                previous
                    .and_then(|entries| entries.get(&entry.entry_id))
                    .is_none_or(|old| old != *entry)
            })
            .cloned()
            .collect::<Vec<_>>();
        let removed_sources = previous
            .into_iter()
            .flat_map(|entries| entries.keys())
            .filter(|id| !replacement.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>();
        self.database
            .commit_root(&root, &changed_sources, &removed_sources)?;
        for entry in winners {
            self.pending_icons.insert(entry.entry_id.clone());
            if let Some(previous) = current.get_mut(&entry.entry_id)
                && (previous.icon_source != entry.icon_source
                    || previous.icon_index != entry.icon_index
                    || previous.priority != entry.priority)
            {
                previous.icon_source = entry.icon_source.clone();
                previous.icon_index = entry.icon_index;
                previous.priority = entry.priority;
            }
        }
        self.sources
            .as_mut()
            .expect("sources initialized")
            .commit(root, replacement);
        if !committed.is_empty() || !removed.is_empty() {
            let visible = self._apply_committed(committed, &removed);
            publish(visible, removed);
        }
        Ok(())
    }

    fn _apply_committed(
        &mut self,
        updated: Vec<ApplicationEntry>,
        removed: &[String],
    ) -> Vec<ApplicationEntry> {
        let current = self.prepared_entries.as_mut().expect("catalog loaded");
        let mut visible = Vec::with_capacity(updated.len());
        for entry in updated {
            visible.push(entry.clone());
            current.insert(entry.entry_id.clone(), entry);
        }
        for id in removed {
            current.remove(id);
        }
        visible
    }
}

fn _same_metadata(left: &ApplicationEntry, right: &ApplicationEntry) -> bool {
    left.source_key == right.source_key
        && left.display_name == right.display_name
        && left.normalized_name == right.normalized_name
        && left.normalized_tokens == right.normalized_tokens
        && left.launch_kind == right.launch_kind
        && left.target_path == right.target_path
        && left.arguments_json == right.arguments_json
        && left.icon_key == right.icon_key
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
    coverage: &mut crate::scan_coverage::ScanCoverage,
) -> bool {
    match platform::read_entry(discovery_state, path, priority) {
        Ok(Some(entry)) => _insert_preferred(entries, entry),
        Ok(None) => {}
        Err(error) => {
            coverage.failed(path);
            eprintln!(
                "application entry could not be read at {}: {error}",
                path.display()
            );
            *warnings = warnings.saturating_add(1);
            return false;
        }
    }
    true
}

// Retained failed sources and freshly read sources use the same winner ordering.
fn _insert_preferred(entries: &mut HashMap<String, ApplicationEntry>, entry: ApplicationEntry) {
    match entries.get(&entry.entry_id) {
        Some(existing)
            if existing.priority > entry.priority
                || (existing.priority == entry.priority
                    && existing.source_key <= entry.source_key) => {}
        _ => {
            entries.insert(entry.entry_id.clone(), entry);
        }
    }
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

fn is_excluded(path: &Path, exclusions: &[String]) -> bool {
    if exclusions.is_empty() {
        return false;
    }
    let path = path_key(path);
    exclusions.iter().any(|excluded| {
        path == *excluded
            || path
                .strip_prefix(excluded)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

#[cfg(test)]
#[path = "../tests/ApplicationIndex.rs"]
mod tests;
