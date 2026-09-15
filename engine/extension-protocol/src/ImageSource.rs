use serde::{Deserialize, Serialize};

pub const MAX_PNG_ENCODED_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_PNG_DIMENSION: u32 = 8_192;
pub const MAX_PNG_PIXELS: u64 = 16_777_216;

/// An image source rendered by the host-owned frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ImageSource {
    DataUrl {
        value: String,
    },
    /// An immutable, content-addressed PNG in the owning extension's payload directory.
    Resource {
        path: String,
    },
}

pub fn is_valid_resource_path(path: &str) -> bool {
    let Some(content_hash) = path.strip_suffix(".png") else {
        return false;
    };
    is_valid_content_hash(content_hash)
}

pub fn is_valid_content_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub fn png_dimensions_within_limits(width: u32, height: u32) -> bool {
    width <= MAX_PNG_DIMENSION
        && height <= MAX_PNG_DIMENSION
        && u64::from(width).saturating_mul(u64::from(height)) <= MAX_PNG_PIXELS
}
