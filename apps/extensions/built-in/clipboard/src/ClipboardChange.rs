use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Committed row changes and final ownership of affected image resources only.
#[derive(Debug, Default)]
pub struct ClipboardChange {
    pub removed: HashSet<String>,
    /// True means at least one committed row still references this path.
    pub image_ownership: HashMap<PathBuf, bool>
}
