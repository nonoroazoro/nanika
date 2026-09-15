use nanika_protocol::ExtensionConfiguration;

/// One saved configuration snapshot waiting for a running Nanika extension.
#[derive(Debug)]
pub(crate) struct ExtensionConfigurationUpdate {
    pub(crate) request_id: String,
    pub(crate) configuration: ExtensionConfiguration,
}
