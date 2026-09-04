use crate::{
    ExtensionInvocation, ExtensionRefresh, ExtensionSearchQuery, ExtensionSettingsUpdate,
    ExtensionViewRequest,
};

pub(crate) enum ExtensionWork {
    Query(ExtensionSearchQuery),
    Invoke(ExtensionInvocation),
    ViewEvent(ExtensionViewRequest),
    Refresh(ExtensionRefresh),
    UpdateSettings(ExtensionSettingsUpdate),
}
