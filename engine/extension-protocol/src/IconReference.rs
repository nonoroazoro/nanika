use serde::{Deserialize, Serialize};

const MAX_ICON_KEY_BYTES: usize = 128;

/// An opaque machine-local icon cache reference scoped to one extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IconReference {
    key: String,
}

impl IconReference {
    pub fn new(key: impl Into<String>) -> Result<Self, String> {
        let key = key.into();
        if key.is_empty()
            || key.len() > MAX_ICON_KEY_BYTES
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err("icon cache key is invalid".to_owned());
        }
        Ok(Self { key })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn is_valid(&self) -> bool {
        Self::new(&self.key).is_ok()
    }
}
