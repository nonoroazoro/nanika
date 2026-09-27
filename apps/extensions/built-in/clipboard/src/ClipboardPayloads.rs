use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use nanika_protocol::ClipboardContent;

/// Storage-owner state for committed retention and active copy readers.
pub(crate) struct ClipboardPayloads {
    root: PathBuf,
    readers: HashMap<PathBuf, (usize, bool)>,
}

impl ClipboardPayloads {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            readers: HashMap::new(),
        }
    }

    pub(crate) fn reconcile(&self, retained: HashSet<PathBuf>) -> Result<(), String> {
        reconcile_payloads(&self.root, &retained, &self.readers)
    }

    pub(crate) fn apply(&mut self, change: crate::ClipboardChange) -> Result<(), String> {
        // Update every active reader before cleanup can fail. No complete history
        // path set remains resident after the startup reconciliation.
        for (path, retained) in &change.image_ownership {
            if let Some(reader) = self.readers.get_mut(path) {
                reader.1 = *retained;
            }
        }
        for (path, retained) in change.image_ownership {
            if !retained && !self.readers.contains_key(&path) {
                self._remove(&path)?;
            }
        }
        Ok(())
    }

    pub(crate) fn acquire(&mut self, content: &ClipboardContent) -> Result<(), String> {
        if let ClipboardContent::PngFile { path } = content {
            let path = std::path::Path::new(path);
            // Persisted paths cannot expand the storage owner's deletion authority.
            if path.parent() != Some(self.root.as_path()) || !is_generated_payload(path) {
                return Err("clipboard image is outside the owned payload namespace".into());
            }
            let reader = self.readers.entry(path.into()).or_insert((0, true));
            reader.0 += 1;
            reader.1 = true;
        }
        Ok(())
    }

    pub(crate) fn release(&mut self, path: &std::path::Path) -> Result<(), String> {
        let count = self
            .readers
            .get_mut(path)
            .expect("copy payload has a reader");
        count.0 -= 1;
        if count.0 != 0 {
            return Ok(());
        }
        let retained = count.1;
        self.readers.remove(path);
        if !retained {
            // Only the final reader reclaims a payload removed by a committed operation.
            self._remove(path)?;
        }
        Ok(())
    }

    fn _remove(&self, path: &std::path::Path) -> Result<(), String> {
        if path.parent() != Some(self.root.as_path()) || !is_generated_payload(path) {
            return Err("clipboard image is outside the owned payload namespace".into());
        }
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

fn reconcile_payloads(
    payload_root: &std::path::Path,
    retained: &HashSet<PathBuf>,
    readers: &HashMap<PathBuf, (usize, bool)>,
) -> Result<(), String> {
    let entries = match std::fs::read_dir(payload_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
            && is_generated_payload(&path)
            && !retained.contains(&path)
            && !readers.contains_key(&path)
        {
            std::fs::remove_file(&path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn is_generated_payload(path: &std::path::Path) -> bool {
    path.extension().is_some_and(|extension| extension == "png")
        && path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| {
                stem.len() == 64 && stem.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
}
