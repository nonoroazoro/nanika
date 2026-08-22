use nanika_config::ConfigStore;
use nanika_search::{InputHistory, SearchHandle, SearchOwner};
use nanika_storage::SearchStorageWorker;

pub(crate) struct HostRuntime {
    pub(crate) history: InputHistory,
    pub(crate) config: Option<ConfigStore>,
    pub(crate) search_owner: Option<SearchOwner>,
    pub(crate) search: Option<SearchHandle>,
    pub(crate) storage: Option<SearchStorageWorker>,
    pub(crate) error: Option<String>,
}
