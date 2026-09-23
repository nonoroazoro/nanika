use std::path::PathBuf;

/// A failed source keeps its resolved path when only inspection failed.
pub(crate) struct DiscoveryFailure {
    pub path: Option<PathBuf>,
    pub message: String,
}
