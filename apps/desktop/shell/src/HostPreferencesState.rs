/// The saved preferences and effective presentation state share one transaction lock.
#[derive(Clone)]
pub(crate) struct HostPreferencesState {
    pub(crate) preferences: nanika_config::LauncherPreferences,
    pub(crate) localization: nanika_localization::LocalizationSnapshot
}
