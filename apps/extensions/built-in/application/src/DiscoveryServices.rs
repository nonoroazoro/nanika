use std::sync::RwLock;
use std::sync::atomic::AtomicU64;
use std::sync::mpsc::SyncSender;

use crate::{ApplicationConfig, ApplicationEntry, RuntimeEvent};

/// Shared services used by one application discovery owner.
pub(crate) struct DiscoveryServices<'a> {
    pub(crate) config: &'a RwLock<ApplicationConfig>,
    pub(crate) entries: &'a RwLock<Vec<ApplicationEntry>>,
    pub(crate) events: &'a SyncSender<RuntimeEvent>,
    pub(crate) cancelled_through: &'a AtomicU64,
}
