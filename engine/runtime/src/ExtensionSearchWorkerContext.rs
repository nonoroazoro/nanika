use std::collections::HashMap;
use std::sync::Arc;

use crate::{ExtensionNotifier, HostServiceHandler, RuntimeViewInvalidation};

pub(crate) struct ExtensionSearchWorkerContext {
    pub(crate) invocation_output: Arc<std::sync::Mutex<crate::ExtensionInvocationOutputState>>,
    pub(crate) notifier: ExtensionNotifier,
    pub(crate) host_services: Option<Arc<dyn HostServiceHandler>>,
    pub(crate) view_invalidations: Arc<std::sync::Mutex<HashMap<String, RuntimeViewInvalidation>>>,
}
