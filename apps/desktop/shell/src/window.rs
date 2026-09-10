use tauri::{Manager, PhysicalPosition, WindowEvent};

const LAUNCHER_WIDTH: f32 = 760.0;
const LAUNCHER_HEIGHT: f32 = 520.0;

pub(crate) fn show_launcher(app: &tauri::AppHandle) -> Result<(), String> {
    schedule_visibility(app, false)
}

pub(crate) fn toggle_launcher(app: &tauri::AppHandle) -> Result<(), String> {
    schedule_visibility(app, true)
}

fn schedule_visibility(app: &tauri::AppHandle, toggle: bool) -> Result<(), String> {
    let handle = app.clone();
    // Tray, hotkey, and single-instance callbacks have different thread origins.
    // Monitor placement and native focus must execute together on the shell thread.
    app.run_on_main_thread(move || {
        if let Err(error) = update_visibility(&handle, toggle) {
            tracing::error!(%error, "launcher visibility operation failed");
        }
    })
    .map_err(|error| error.to_string())
}

fn update_visibility(app: &tauri::AppHandle, toggle: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("launcher")
        .ok_or_else(|| "launcher window is unavailable".to_owned())?;
    if toggle && window.is_visible().map_err(|error| error.to_string())? {
        return window.hide().map_err(|error| error.to_string());
    }
    let scale_factor = window.scale_factor().map_err(|error| error.to_string())? as f32;
    let position =
        nanika_platform::active_overlay_position(LAUNCHER_WIDTH, LAUNCHER_HEIGHT, scale_factor)
            .map_err(|error| error.to_string())?;
    window
        .set_position(PhysicalPosition::new(
            position.x.round() as i32,
            position.y.round() as i32,
        ))
        .map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

pub(crate) fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    if window.label() == "launcher"
        && matches!(event, WindowEvent::Focused(false))
        && let Err(error) = window.hide()
    {
        tracing::error!(%error, "launcher could not hide after losing focus");
    }
}
