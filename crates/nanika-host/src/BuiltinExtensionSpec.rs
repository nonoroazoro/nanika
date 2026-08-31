use nanika_extension_package::ExtensionProtocol;
use nanika_storage::ExtensionKind;

use crate::BuiltinCommandSpec;

/// One extension executable shipped by the default distribution.
pub(crate) struct BuiltinExtensionSpec {
    pub(crate) extension_id: &'static str,
    pub(crate) binary_name: &'static str,
    pub(crate) protocol: ExtensionProtocol,
    pub(crate) kind: ExtensionKind,
    pub(crate) permissions: &'static [&'static str],
    pub(crate) root_search: bool,
    pub(crate) commands: &'static [BuiltinCommandSpec],
}
