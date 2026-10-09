/// Stable identities for the host-owned General settings controls.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum GeneralSettingsFieldKey {
    LauncherShortcut,
    HideOnBlur,
    Theme,
    LaunchAtLogin
}

impl GeneralSettingsFieldKey {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::LauncherShortcut => "launcherShortcut",
            Self::HideOnBlur => "hideOnBlur",
            Self::Theme => "theme",
            Self::LaunchAtLogin => "launchAtLogin"
        }
    }
}
