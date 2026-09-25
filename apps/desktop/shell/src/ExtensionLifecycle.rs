/// Authoritative lifecycle observation, including configuration of the current instance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtensionLifecycle {
    #[serde(flatten)]
    pub(crate) info: nanika_host::RuntimeExtensionInfo,
    pub(crate) configuration: Option<nanika_host::RuntimeExtensionConfiguration>,
}
