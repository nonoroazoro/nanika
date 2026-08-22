use crate::{ExtensionInvocation, ExtensionRefresh, ExtensionSearchQuery, ExtensionSettingsUpdate};

pub(crate) enum ExtensionWork {
    Query(ExtensionSearchQuery),
    Invoke(ExtensionInvocation),
    Refresh(ExtensionRefresh),
    UpdateSettings(ExtensionSettingsUpdate),
}
