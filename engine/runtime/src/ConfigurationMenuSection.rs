#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationMenuSection {
    pub title: String,
    pub items: Vec<crate::ConfigurationMenuControl>
}
