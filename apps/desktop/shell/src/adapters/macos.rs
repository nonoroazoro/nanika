pub(crate) const CUSTOM_SETTINGS_CONTROLS: bool = false;

pub(crate) fn configure_windows(config: &mut tauri::utils::config::Config) {
    for window in &mut config.app.windows {
        if window.label == "settings" {
            window.transparent = true;
            window.width = 900.0;
            window.height = 700.0;
            window.min_width = Some(900.0);
            window.min_height = Some(700.0);
            // Keep native controls over the full-height surface; the WebView owns the drag region.
            window.title_bar_style = tauri::utils::TitleBarStyle::Overlay;
            window.hidden_title = true;
            // Tao extends the titlebar container by y; y = 26 centers buttons in the 48pt drag strip.
            window.traffic_light_position =
                Some(tauri::utils::config::LogicalPosition { x: 16.0, y: 26.0 });
        }
    }
}
