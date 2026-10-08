use serde::Deserialize;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Deserialize)]
#[serde(
    tag = "key",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum HostSettingsChange {
    LauncherShortcut(String),
    Theme(nanika_config::ThemePreference),
    HideOnBlur(bool),
}
