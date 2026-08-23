/// Failure at the external package trust or persistence boundary.
#[derive(Debug, thiserror::Error)]
pub enum ExtensionPackageError {
    #[error("package I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid ZIP package: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("invalid extension manifest: {0}")]
    Manifest(String),
    #[error("extension storage failed: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("extension configuration failed: {0}")]
    Config(String),
}
