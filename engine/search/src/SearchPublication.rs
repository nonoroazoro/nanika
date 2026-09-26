use crate::SearchSnapshot;
use std::sync::{Arc, Mutex, atomic::AtomicU64};

pub(crate) struct SearchPublication<'a> {
    pub(crate) latest: &'a Mutex<Option<Arc<SearchSnapshot>>>,
    pub(crate) notifier: &'a Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    pub(crate) next_generation: &'a AtomicU64,
}
