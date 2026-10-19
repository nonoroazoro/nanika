/// Host-resolved shortcuts contain only the configuration explicitly exposed in the menu.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationMenu {
    pub revision: u64,
    pub groups: Vec<crate::ConfigurationMenuSection>
}
