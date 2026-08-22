/// Distribution origin for an extension. It does not grant runtime privilege.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionKind {
    BuiltIn,
    External,
}

impl ExtensionKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::BuiltIn => "built-in",
            Self::External => "external",
        }
    }
}
