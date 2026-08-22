use std::sync::{Arc, Mutex};

pub(crate) type ExtensionNotifier = Arc<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>>;
