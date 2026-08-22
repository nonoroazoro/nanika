use nanika_storage::ExtensionKind;

/// One extension executable shipped by the default distribution.
pub(crate) struct BuiltinExtensionSpec {
    pub(crate) extension_id: &'static str,
    pub(crate) binary_name: &'static str,
    pub(crate) kind: ExtensionKind,
}
