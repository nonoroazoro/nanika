use tauri::{Manager, PhysicalPosition};

const LAUNCHER_WIDTH: f32 = 760.0;
const LAUNCHER_HEIGHT: f32 = 520.0;

pub(crate) fn show_launcher(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("launcher")
        .ok_or_else(|| "launcher window is unavailable".to_owned())?;
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

pub(crate) fn toggle_launcher(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("launcher")
        .ok_or_else(|| "launcher window is unavailable".to_owned())?;
    if window.is_visible().map_err(|error| error.to_string())? {
        window.hide().map_err(|error| error.to_string())
    } else {
        show_launcher(app)
    }
}
