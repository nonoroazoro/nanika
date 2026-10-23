#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeneralSettingsField {
    pub(crate) key: crate::GeneralSettingsFieldKey,
    pub(crate) title: nanika_protocol::LocalizedText,
    pub(crate) description: Option<nanika_protocol::LocalizedText>,
    #[serde(skip)]
    pub(crate) keywords: &'static str
}
