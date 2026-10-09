use serde::Deserialize;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum SettingsWindowAction {
    Drag,
    Minimize,
    ToggleMaximize,
    Close
}
