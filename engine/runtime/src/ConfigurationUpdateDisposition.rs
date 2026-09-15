/// Immediate result of saving an extension configuration update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationUpdateDisposition {
    LiveApplyQueued,
    SavedForNextLaunch,
}
