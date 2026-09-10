use std::sync::Arc;

use crate::{ExtensionNotifier, ExtensionViewUpdate, HostServiceHandler};

/// Shared channels and services supplied when an extension worker starts.
pub(crate) struct ExtensionSearchWorkerContext {
    pub(crate) view_updates: async_channel::Sender<ExtensionViewUpdate>,
    pub(crate) notifier: ExtensionNotifier,
    pub(crate) host_services: Option<Arc<dyn HostServiceHandler>>,
}
