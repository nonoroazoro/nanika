pub(crate) const CUSTOM_SETTINGS_CONTROLS: bool = true;

pub(crate) fn lost_window_focus(window: &tauri::Window) -> Result<bool, String> {
    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    // WebView2 can transfer keyboard focus within the foreground window while
    // restoring its controller. Only leaving the native window dismisses it.
    #[allow(unsafe_code)]
    Ok(unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() } != hwnd)
}

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
