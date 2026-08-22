use nanika_protocol::SettingUpdate;

pub(crate) enum SettingsAction {
    SaveHost,
    SaveExtension {
        extension_id: String,
        updates: Vec<SettingUpdate>,
    },
    SetStartup(bool),
    Close,
}
