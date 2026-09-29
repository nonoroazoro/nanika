#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeneralSettingsField {
    pub(crate) key: &'static str,
    pub(crate) title: &'static str,
    pub(crate) description: Option<&'static str>,
    #[serde(skip)]
    pub(crate) keywords: &'static str,
}
