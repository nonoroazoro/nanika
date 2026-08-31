use std::sync::Arc;
use std::sync::mpsc::SyncSender;

use crate::{
    ExtensionInvocationResult, ExtensionNotifier, ExtensionViewUpdate, HostServiceHandler,
};

/// Shared channels and services supplied when an extension worker starts.
pub(crate) struct ExtensionSearchWorkerContext {
    pub(crate) invocation_results: SyncSender<ExtensionInvocationResult>,
    pub(crate) view_updates: SyncSender<ExtensionViewUpdate>,
    pub(crate) notifier: ExtensionNotifier,
    pub(crate) host_services: Option<Arc<dyn HostServiceHandler>>,
}
