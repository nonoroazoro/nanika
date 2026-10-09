use std::sync::atomic::AtomicU64;
use std::sync::mpsc::SyncSender;
use std::sync::{Condvar, Mutex, RwLock};

use crate::{ApplicationConfig, ApplicationEntry, RuntimeEvent};

pub(crate) struct DiscoveryServices<'a> {
    pub(crate) config: &'a RwLock<ApplicationConfig>,
    pub(crate) entries: &'a RwLock<std::collections::HashMap<String, ApplicationEntry>>,
    pub(crate) events: &'a SyncSender<RuntimeEvent>,
    pub(crate) icon_wake: &'a (Mutex<crate::EntryPriority>, Condvar),
    pub(crate) cancelled_through: &'a AtomicU64
}
