/// Persistence succeeded. Application remains a separate, concrete outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigurationSaveOutcome {
    Applied,
    SavedForNextLaunch,
    ApplyFailed(String),
}
