use crate::{
    ExtensionConfigurationUpdate, ExtensionInvocation, ExtensionRefresh, ExtensionSearchQuery,
    ExtensionViewRequest,
};

pub(crate) enum ExtensionWork {
    Query(ExtensionSearchQuery),
    PrepareEntries {
        generation: u64,
        entry_ids: Vec<String>,
    },
    Invoke(ExtensionInvocation),
    ViewEvent(ExtensionViewRequest),
    Refresh(ExtensionRefresh),
    ApplyConfiguration(ExtensionConfigurationUpdate),
}
