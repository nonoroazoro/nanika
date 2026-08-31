use nanika_extension_package::CommandMode;

/// Static command metadata shipped with a built-in extension.
pub(crate) struct BuiltinCommandSpec {
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) description: &'static str,
    pub(crate) mode: CommandMode,
    pub(crate) keywords: &'static [&'static str],
}
