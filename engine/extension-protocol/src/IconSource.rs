use serde::{Deserialize, Serialize};

use crate::IconReference;

/// Declarative image identity, always scoped to the supplying extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum IconSource {
    /// Reserve the icon slot without inheriting the extension image.
    Empty,
    Package {
        path: String,
    },
    Cache(IconReference),
}

impl IconSource {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Empty => true,
            Self::Package { path } => is_valid_package_icon_path(path),
            Self::Cache(reference) => reference.is_valid(),
        }
    }
}

/// Portable package paths are URL-safe and cannot name host files or parent directories.
pub fn is_valid_package_icon_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 512
        && path.ends_with(".png")
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        })
}
