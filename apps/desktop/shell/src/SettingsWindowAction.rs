use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum SettingsWindowAction {
    Drag,
    Minimize,
    ToggleMaximize,
    Close,
}
