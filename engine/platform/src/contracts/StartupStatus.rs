/// Effective login startup state reported by the operating system.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StartupStatus {
    Disabled,
    Enabled,
    RequiresApproval,
    NeedsRepair,
    NotFound,
}
