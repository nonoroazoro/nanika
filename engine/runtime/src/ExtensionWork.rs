use crate::{
    ExtensionConfigurationUpdate, ExtensionInvocation, ExtensionRefresh, ExtensionSearchQuery,
    ExtensionViewRequest,
};

pub(crate) enum ExtensionWork {
    Query(ExtensionSearchQuery),
    Invoke(ExtensionInvocation),
    ViewEvent(ExtensionViewRequest),
    Refresh(ExtensionRefresh),
    ApplyConfiguration(ExtensionConfigurationUpdate),
}
