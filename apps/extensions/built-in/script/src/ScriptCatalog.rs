use crate::ScriptEntryData;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::{ScriptConfig, ScriptEntry};

/// Completed root snapshots; incomplete roots never replace their previous contents.
#[derive(Clone, Default)]
pub struct ScriptCatalog {
    _roots: BTreeMap<PathBuf, BTreeMap<String, ScriptEntry>>,
    _entries: BTreeMap<String, ScriptEntry>,
}

impl ScriptCatalog {
    pub fn entries(&self) -> BTreeMap<String, ScriptEntry> {
        self._entries.clone()
    }

    pub fn get(&self, id: &str) -> Option<&ScriptEntry> {
        self._entries.get(id)
    }

    pub fn scan(
        &mut self,
        config: &ScriptConfig,
        cancelled: impl Fn() -> bool,
        mut publish: impl FnMut(Vec<ScriptEntry>, Vec<String>),
    ) -> Result<(), String> {
        let roots = config
            .roots
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        let mut failures = Vec::new();
        for root in &roots {
            if cancelled() {
                return Err("script scan was cancelled".to_owned());
            }
            let updated = match _scan_root(root, &cancelled) {
                Ok(entries) => entries,
                Err(error) => {
                    failures.push(error);
                    continue;
                }
            };
            if cancelled() {
                return Err("script scan was cancelled".to_owned());
            }
            let (updated, removed) = self._replace_root(root, updated);
            if !updated.is_empty() || !removed.is_empty() {
                publish(updated, removed);
            }
        }
        if !failures.is_empty() {
            return Err(failures.join("; "));
        }
        if cancelled() {
            return Err("script scan was cancelled".to_owned());
        }
        let removed_roots = self
            ._roots
            .keys()
            .filter(|root| !roots.contains(root))
            .cloned()
            .collect::<Vec<_>>();
        for root in removed_roots {
            let (updated, removed) = self._replace_root(&root, BTreeMap::new());
            self._roots.remove(&root);
            if !updated.is_empty() || !removed.is_empty() {
                publish(updated, removed);
            }
        }
        Ok(())
    }

    fn _replace_root(
        &mut self,
        root: &Path,
        mut replacement: BTreeMap<String, ScriptEntry>,
    ) -> (Vec<ScriptEntry>, Vec<String>) {
        for (id, entry) in &mut replacement {
            if let Some(previous) = self._entries.get(id).filter(|previous| *previous == entry) {
                *entry = previous.clone();
            }
        }
        let removed = self
            ._roots
            .get(root)
            .into_iter()
            .flat_map(|entries| entries.keys())
            .filter(|id| {
                !replacement.contains_key(*id)
                    && !self
                        ._roots
                        .iter()
                        .any(|(path, entries)| path != root && entries.contains_key(*id))
            })
            .cloned()
            .collect::<Vec<_>>();
        let updated = replacement
            .values()
            .filter(|entry| self._entries.get(&entry.id) != Some(*entry))
            .cloned()
            .collect::<Vec<_>>();
        for id in &removed {
            self._entries.remove(id);
        }
        for entry in &updated {
            self._entries.insert(entry.id.clone(), entry.clone());
        }
        self._roots.insert(root.to_path_buf(), replacement);
        (updated, removed)
    }
}

fn _scan_root(
    root: &Path,
    cancelled: &impl Fn() -> bool,
) -> Result<BTreeMap<String, ScriptEntry>, String> {
    let mut entries = BTreeMap::new();
    let metadata = root
        .symlink_metadata()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(format!(
            "script root must be a regular directory: {}",
            root.display()
        ));
    }
    let mut walker = WalkDir::new(root).follow_links(false).into_iter();
    while let Some(entry) = walker.next() {
        if cancelled() {
            return Err("script scan was cancelled".to_owned());
        }
        let entry = entry.map_err(|error| error.to_string())?;
        if cfg!(target_os = "macos")
            && entry.file_type().is_dir()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
        {
            walker.skip_current_dir();
            continue;
        }
        // Do not traverse links outside the selected directory or execute link targets.
        if !entry.file_type().is_file() || !is_script(entry.path()) {
            continue;
        }
        let path = entry
            .path()
            .canonicalize()
            .map_err(|error| format!("{}: {error}", entry.path().display()))?;
        let text = path
            .to_str()
            .ok_or_else(|| format!("script path is not UTF-8: {}", path.display()))?;
        if text.len() > 4096 {
            return Err(format!(
                "script path exceeds 4096 bytes: {}",
                path.display()
            ));
        }
        let mut id = String::from("script.");
        for byte in Sha256::digest(text.as_bytes()) {
            let _ = write!(id, "{byte:02x}");
        }
        let title = path
            .file_stem()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| format!("script filename is empty: {}", path.display()))?
            .to_owned();
        entries.insert(
            id.clone(),
            ScriptEntry::new(ScriptEntryData { id, title, path }),
        );
    }
    Ok(entries)
}

fn is_script(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            ["ps1", "py", "js", "sh"]
                .iter()
                .any(|extension| value.eq_ignore_ascii_case(extension))
        })
}
