use crate::{ExtensionInvocation, ExtensionRefresh, ExtensionSearchQuery};

pub(crate) enum ExtensionWork {
    Query(ExtensionSearchQuery),
    Invoke(ExtensionInvocation),
    Refresh(ExtensionRefresh),
}
