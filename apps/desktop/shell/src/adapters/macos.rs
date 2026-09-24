pub(crate) const CUSTOM_SETTINGS_CONTROLS: bool = false;

pub(crate) fn configure_windows(config: &mut tauri::utils::config::Config) {
    for window in &mut config.app.windows {
        if window.label == "settings" {
            window.transparent = true;
        }
    }
}
