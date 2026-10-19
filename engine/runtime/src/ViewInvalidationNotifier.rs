use std::sync::{Arc, Mutex};

pub(crate) type ViewInvalidationNotifier =
    Arc<Mutex<Option<Arc<dyn Fn(nanika_protocol::TextInvalidation) + Send + Sync>>>>;
