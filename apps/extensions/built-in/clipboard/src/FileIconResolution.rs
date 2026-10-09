use nanika_protocol::IconReference;

/// A settled outcome belongs to one observed source, including metadata failures.
pub(crate) struct FileIconResolution {
    pub source: Result<IconReference, std::io::ErrorKind>,
    pub icon: Option<IconReference>
}
