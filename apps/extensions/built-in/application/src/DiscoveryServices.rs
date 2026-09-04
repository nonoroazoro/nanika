use std::path::Path;
use std::sync::RwLock;
use std::sync::atomic::AtomicU64;
use std::sync::mpsc::SyncSender;

use nanika_config::ConfigStore;

use crate::{ApplicationEntry, RuntimeEvent};

/// Shared services used by one application discovery owner.
pub(crate) struct DiscoveryServices<'a> {
    pub(crate) database_path: &'a Path,
    pub(crate) config_store: &'a ConfigStore,
    pub(crate) entries: &'a RwLock<Vec<ApplicationEntry>>,
    pub(crate) events: &'a SyncSender<RuntimeEvent>,
    pub(crate) cancelled_through: &'a AtomicU64,
}
