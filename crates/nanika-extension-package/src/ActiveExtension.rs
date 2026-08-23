use std::path::PathBuf;

/// Validated external extension ready for host-supervised process creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveExtension {
    pub extension_id: String,
    pub program: PathBuf,
    pub permissions: Vec<String>,
}
