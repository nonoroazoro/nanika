/// Effective login startup state reported by the operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupStatus {
    Disabled,
    Enabled,
    RequiresApproval,
    NeedsRepair,
    NotFound,
}
