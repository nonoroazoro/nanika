use nanika_config::ConfigStore;
use nanika_search::{InputHistory, SearchHandle, SearchOwner};
use nanika_storage::SearchStorageWorker;

use std::sync::Arc;

use crate::{HostConfig, HostConfigService, HostServiceHandler, PendingExtension};

pub(crate) struct HostRuntime {
    pub(crate) history: InputHistory,
    pub(crate) config: Option<ConfigStore>,
    pub(crate) host_config: HostConfig,
    pub(crate) config_service: Option<HostConfigService>,
    pub(crate) startup: Option<nanika_platform::StartupService>,
    pub(crate) search_owner: Option<SearchOwner>,
    pub(crate) search: Option<SearchHandle>,
    pub(crate) storage: Option<SearchStorageWorker>,
    pub(crate) pending_extensions: Vec<PendingExtension>,
    pub(crate) host_services: Option<Arc<dyn HostServiceHandler>>,
    pub(crate) error: Option<String>,
}
