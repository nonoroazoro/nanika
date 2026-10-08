#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeExtensionState {
    Disabled,
    Dormant,
    #[default]
    Starting,
    Ready,
    Stopping,
    Failed,
}
