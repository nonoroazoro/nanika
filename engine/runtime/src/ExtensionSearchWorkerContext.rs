use std::collections::HashMap;
use std::sync::Arc;

use crate::{ExtensionNotifier, HostServiceHandler, RuntimeViewInvalidation};

/// Shared channels and services supplied when an extension worker starts.
pub(crate) struct ExtensionSearchWorkerContext {
    pub(crate) notifier: ExtensionNotifier,
    pub(crate) host_services: Option<Arc<dyn HostServiceHandler>>,
    pub(crate) view_invalidations: Arc<std::sync::Mutex<HashMap<String, RuntimeViewInvalidation>>>,
}
