use crate::{ExtensionInvocation, ExtensionSearchQuery};

pub(crate) enum ExtensionWork {
    Query(ExtensionSearchQuery),
    Invoke(ExtensionInvocation),
}
