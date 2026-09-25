use std::fs::File;

use crate::{ConfigStore, ExtensionRegistryConfig};

/// Serializes read-modify-write and explicit rollback across all registry writers.
/// The separate lock file survives atomic replacement of the JSONC document.
pub struct ExtensionRegistryTransaction {
    _lock: File,
    _store: ConfigStore,
    _original: Option<ExtensionRegistryConfig>,
    _current: ExtensionRegistryConfig,
}

impl ExtensionRegistryTransaction {
    pub fn begin(store: &ConfigStore) -> Result<Self, String> {
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(store.config_root().join("extensions.lock"))
            .map_err(|error| format!("could not open extension registry lock: {error}"))?;
        lock.lock()
            .map_err(|error| format!("could not lock extension registry: {error}"))?;
        let current = ExtensionRegistryConfig::load(store)?;
        let original = store.extensions_file().is_file().then(|| current.clone());
        Ok(Self {
            _lock: lock,
            _store: store.clone(),
            _original: original,
            _current: current,
        })
    }

    pub fn is_enabled(&self, extension_id: &str) -> bool {
        self._current.is_enabled(extension_id)
    }

    pub fn set_enabled(&mut self, extension_id: impl Into<String>, enabled: bool) {
        self._current.set_enabled(extension_id, enabled);
    }

    pub fn remove(&mut self, extension_id: &str) {
        self._current.remove(extension_id);
    }

    pub fn save(&self) -> Result<(), String> {
        self._current._save(&self._store)
    }

    /// Caller-owned package rollback remains inside the same registry transaction.
    pub fn rollback(&self) -> Result<(), String> {
        if let Some(original) = &self._original {
            original._save(&self._store)
        } else {
            match std::fs::remove_file(self._store.extensions_file()) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.to_string()),
            }
        }
    }
}
