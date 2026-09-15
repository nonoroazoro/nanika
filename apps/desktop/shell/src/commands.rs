use tauri::Manager;

use crate::{
    ApplicationSnapshot, DesktopState, InvokeCandidateRequest, PublishQueryRequest,
    RootSearchSnapshot,
};

#[tauri::command]
pub(crate) async fn open_session(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    updates: tauri::ipc::Channel<RootSearchSnapshot>,
) -> Result<ApplicationSnapshot, String> {
    authorize_launcher(&window)?;
    state.open_session(updates)
}

#[tauri::command]
pub(crate) async fn publish_query(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    request: PublishQueryRequest,
) -> Result<(), String> {
    authorize_launcher(&window)?;
    state.publish_query(request)
}

#[tauri::command]
pub(crate) async fn acknowledge_search(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    session_id: u64,
    revision: u64,
) -> Result<(), String> {
    authorize_launcher(&window)?;
    state.acknowledge_search(session_id, revision)
}

#[tauri::command]
pub(crate) async fn refresh_search(
    window: tauri::WebviewWindow,
    session_id: u64,
) -> Result<(), String> {
    authorize_launcher(&window)?;
    if !window.is_visible().map_err(|error| error.to_string())?
        || !window.is_focused().map_err(|error| error.to_string())?
    {
        return Err("Refresh is available only in the focused launcher.".to_owned());
    }
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<DesktopState>().refresh_search(session_id)
    })
    .await
    .map_err(|error| format!("could not wait for the refresh result: {error}"))?
}

#[tauri::command]
pub(crate) async fn close_session(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    session_id: u64,
) -> Result<(), String> {
    authorize_launcher(&window)?;
    state.close_session(session_id);
    Ok(())
}

#[tauri::command]
pub(crate) async fn invoke_candidate(
    window: tauri::WebviewWindow,
    request: InvokeCandidateRequest,
) -> Result<(), String> {
    authorize_launcher(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<DesktopState>().run_invocation(&request)
    })
    .await
    .map_err(|error| format!("could not wait for the action result: {error}"))?
}

#[tauri::command]
pub(crate) async fn view_event(
    window: tauri::WebviewWindow,
    request: crate::ViewEventRequest,
) -> Result<(), String> {
    authorize_launcher(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<DesktopState>().run_view_event(request)
    })
    .await
    .map_err(|error| format!("could not wait for the view result: {error}"))?
}

#[tauri::command]
pub(crate) fn dismiss_launcher(window: tauri::WebviewWindow) -> Result<(), String> {
    authorize_launcher(&window)?;
    window.hide().map_err(|error| error.to_string())
}

fn authorize_launcher(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == "launcher" && window.app_handle().get_webview_window("launcher").is_some()
    {
        Ok(())
    } else {
        Err("command is not available to this window".to_owned())
    }
}
