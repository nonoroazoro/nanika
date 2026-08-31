/// One extension-scoped icon cache entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct IconIdentity {
    extension_id: String,
    key: String,
}

impl IconIdentity {
    pub(crate) fn new(extension_id: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            extension_id: extension_id.into(),
            key: key.into(),
        }
    }

    pub(crate) fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub(crate) fn key(&self) -> &str {
        &self.key
    }

    pub(crate) fn texture_name(&self) -> String {
        format!("extension-icon:{}:{}", self.extension_id, self.key)
    }
}
