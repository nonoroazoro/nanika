use serde::Deserialize;

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
