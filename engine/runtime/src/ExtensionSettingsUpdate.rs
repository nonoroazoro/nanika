use nanika_protocol::SettingUpdate;

/// One serialized settings mutation for an extension process.
#[derive(Debug)]
pub(crate) struct ExtensionSettingsUpdate {
    pub(crate) request_id: String,
    pub(crate) updates: Vec<SettingUpdate>,
}
