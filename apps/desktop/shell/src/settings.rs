use std::sync::atomic::Ordering;

use tauri::Manager;

use crate::{SettingsEvent, SettingsWindow, SettingsWindowAction};

// WebView construction stays on blocking workers; presentation stays on the shell thread.
pub(crate) fn show_settings(app: &tauri::AppHandle) -> Result<(), String> {
    prepare_settings(app)?;
    let owner = app.clone();
    app.run_on_main_thread(move || {
        owner
            .state::<SettingsWindow>()
            .requested
            .store(true, Ordering::Release);
        if let Some(window) = owner.get_webview_window("settings")
            && let Err(error) = _present(&window)
        {
            tracing::error!(%error, "settings could not open");
        }
    })
    .map_err(|error| error.to_string())
}

pub(crate) fn prepare_settings(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<SettingsWindow>();
    let _creation = state
        .creation
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if app.get_webview_window("settings").is_none() {
        let config = app
            .config()
            .app
            .windows
            .iter()
            .find(|config| config.label == "settings")
            .ok_or("Settings window configuration is missing.")?;
        tauri::WebviewWindowBuilder::from_config(app, config)
            .map_err(|error| error.to_string())?
            .build()
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(crate) fn ready(window: &tauri::WebviewWindow) -> Result<(), String> {
    window
        .state::<SettingsWindow>()
        .ready
        .store(true, Ordering::Release);
    _present(window)
}

pub(crate) fn request_close(window: &tauri::Window) -> Result<(), String> {
    window
        .state::<crate::host_settings::HostSettings>()
        .recording
        .store(false, Ordering::Release);
    crate::window::hide_window(window)?;
    let state = window.state::<SettingsWindow>();
    state.ready.store(false, Ordering::Release);
    state.requested.store(false, Ordering::Release);
    window.state::<crate::DesktopState>().settings_closed();
    Ok(())
}

pub(crate) fn action(
    window: &tauri::WebviewWindow,
    action: SettingsWindowAction,
) -> Result<(), String> {
    match action {
        SettingsWindowAction::Drag => window.start_dragging().map_err(|error| error.to_string()),
        SettingsWindowAction::Minimize => window.minimize().map_err(|error| error.to_string()),
        SettingsWindowAction::ToggleMaximize => {
            if window.is_maximized().map_err(|error| error.to_string())? {
                window.unmaximize().map_err(|error| error.to_string())
            } else {
                window.maximize().map_err(|error| error.to_string())
            }
        }
        SettingsWindowAction::Close => request_close(&window.as_ref().window()),
    }
}

pub(crate) fn resized(window: &tauri::Window) -> Result<(), String> {
    let maximized = window.is_maximized().map_err(|error| error.to_string())?;
    if window
        .state::<SettingsWindow>()
        .maximized
        .swap(maximized, Ordering::AcqRel)
        != maximized
    {
        window
            .state::<crate::DesktopState>()
            .send_settings_event(SettingsEvent::WindowState { maximized })?;
    }
    Ok(())
}

fn _present(window: &tauri::WebviewWindow) -> Result<(), String> {
    let state = window.state::<SettingsWindow>();
    if !state.ready.load(Ordering::Acquire) {
        return Ok(());
    }
    if !state.requested.load(Ordering::Acquire) {
        return crate::window::hide_window(&window.as_ref().window());
    }
    window.unminimize().map_err(|error| error.to_string())?;
    crate::window::show_window(window)?;
    window.set_focus().map_err(|error| error.to_string())
}
