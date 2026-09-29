#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsSearchEntry {
    pub(crate) page_id: String,
    pub(crate) target: crate::SettingsSearchTarget,
    pub(crate) title: String,
    pub(crate) page_title: String,
    #[serde(skip)]
    pub(crate) search_values: Vec<String>,
}
