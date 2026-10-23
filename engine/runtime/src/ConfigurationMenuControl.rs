#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConfigurationMenuControl {
    Toggle {
        key: String,
        title: nanika_protocol::LocalizedText,
        checked: bool
    },
    Action {
        action: nanika_protocol::Action
    }
}
