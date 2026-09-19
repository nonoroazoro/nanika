use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use tauri::Manager;

#[derive(Default)]
pub(crate) struct SettingsWindow {
    creation: Mutex<()>,
    pub(crate) directory_picker: Mutex<()>,
    pub(crate) ready: AtomicBool,
    pub(crate) requested: AtomicBool,
}

// Embedded browser creation must not run in synchronous commands or window callbacks.
// The mutex only protects creation on blocking workers, never the main thread.
pub(crate) fn show_settings(app: &tauri::AppHandle) -> Result<(), String> {
    app.state::<SettingsWindow>()
        .requested
        .store(true, Ordering::Release);
    prepare_settings(app)
}

pub(crate) fn prepare_settings(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<SettingsWindow>();
    let _creation = state
        .creation
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(window) = app.get_webview_window("settings") {
        if state.ready.load(Ordering::Acquire) && state.requested.load(Ordering::Acquire) {
            window.unminimize().map_err(|error| error.to_string())?;
            window.show().map_err(|error| error.to_string())?;
            window.set_focus().map_err(|error| error.to_string())?;
        }
    } else {
        state.ready.store(false, Ordering::Release);
        // Use the same prepared configuration as startup windows. In particular,
        // The browser runtime rejects conflicting scrollbar options for a shared data directory.
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
