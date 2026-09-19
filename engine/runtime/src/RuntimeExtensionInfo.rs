/// Validated presentation metadata, available without activating an extension.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeExtensionInfo {
    pub id: String,
    pub name: String,
    pub icon: nanika_extension_package::ContributionIcon,
}
