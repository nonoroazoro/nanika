pub(crate) const CUSTOM_SETTINGS_CONTROLS: bool = true;

pub(crate) fn configure_windows(config: &mut tauri::utils::config::Config) {
    for window in &mut config.app.windows {
        // All WebViews sharing a data directory must use identical scrollbar options.
        window.scroll_bar_style = tauri::utils::config::ScrollBarStyle::FluentOverlay;
        // Native shadows add a frame behind the CSS-rounded transparent surfaces.
        window.shadow = false;
        if window.label == "settings" {
            window.decorations = false;
            window.transparent = true;
        }
    }
}
