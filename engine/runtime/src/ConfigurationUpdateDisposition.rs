#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigurationUpdateDisposition {
    LiveApplyQueued,
    SavedForNextLaunch,
}
