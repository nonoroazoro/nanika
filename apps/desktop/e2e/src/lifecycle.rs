use std::io::{Read, Write};
use std::path::PathBuf;

use tauri::Manager;

pub(crate) fn plugin(directory: PathBuf) -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let mut ready = false;
    let mut previous_state = None;
    tauri::plugin::Builder::new("e2e-lifecycle")
        .setup(|app, _| {
            let app = app.clone();
            std::thread::Builder::new()
                .name("e2e-owner".into())
                .spawn(move || {
                    // EOF ties the isolated App to its runner, including runner crashes.
                    let mut byte = [0];
                    let result = std::io::stdin().read(&mut byte);
                    app.exit(if matches!(result, Ok(0)) { 0 } else { 1 });
                })?;
            Ok(())
        })
        .on_event(move |app, event| {
            if let tauri::RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::Focused(focused),
                ..
            } = event
            {
                eprintln!("Native focus: {label}={focused}");
            }
            if matches!(event, tauri::RunEvent::Ready) {
                ready = true;
            }
            // Window listeners run after RunEvent, and visibility changes need not emit a WindowEvent.
            if ready
                && matches!(event, tauri::RunEvent::MainEventsCleared)
                && let Err(error) = _record(app, &directory, &mut previous_state)
            {
                eprintln!("Could not record native state: {error}");
                app.exit(1);
            }
        })
        .build()
}

fn _record(
    app: &tauri::AppHandle,
    directory: &std::path::Path,
    previous_state: &mut Option<serde_json::Value>
) -> Result<(), Box<dyn std::error::Error>> {
    let mut windows = serde_json::Map::new();
    for (label, window) in app.webview_windows() {
        windows.insert(
            label,
            serde_json::json!({
                "title": window.title()?,
                "visible": window.is_visible()?,
                "focused": window.is_focused()?,
                "pickerOpen": crate::native_dialog::is_open(&window)?
            })
        );
    }
    let state = serde_json::json!({
        "pid": std::process::id(),
        "identity": app.config().identifier,
        "platform": std::env::consts::OS,
        "webviewVersion": tauri::webview_version()?,
        "windows": windows,
        "trayLabels": nanika_desktop::native_tray_labels(app)?
    });
    if previous_state.as_ref() == Some(&state) {
        return Ok(());
    }
    let temporary = directory.join("native-state.tmp");
    std::fs::write(&temporary, serde_json::to_vec(&state)?)?;
    std::fs::rename(temporary, directory.join("native-state.json"))?;
    // Preserve transitions so a final hidden state cannot erase focus-race evidence.
    let mut history = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("native-state.jsonl"))?;
    writeln!(history, "{state}")?;
    *previous_state = Some(state);
    Ok(())
}
