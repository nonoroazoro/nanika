#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum ConfigurationMenuOperation {
    Set {
        key: String,
        value: bool,
        revision: u64
    },
    Action {
        id: String,
        confirmed: bool
    }
}
