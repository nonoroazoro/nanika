use std::sync::{Arc, Mutex};

pub(crate) type SearchNotifier = Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>;
