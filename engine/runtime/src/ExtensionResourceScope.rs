use serde::{Deserialize, Serialize};

/// Host-bound package identity. A content digest alone never authorizes resource access.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionResourceScope {
    pub extension_id: String,
    pub identity: String
}

impl ExtensionResourceScope {
    pub fn validate(&self) -> Result<(), String> {
        if !nanika_foundation::is_valid_extension_id(&self.extension_id)
            || self.identity.len() != 64
            || !self
                .identity
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err("Invalid extension resource scope.".into());
        }
        Ok(())
    }
}
