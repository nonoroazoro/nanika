#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct GeneralSettingsSection {
    pub(crate) key: &'static str,
    pub(crate) title: &'static str,
    pub(crate) fields: Vec<crate::GeneralSettingsField>
}
