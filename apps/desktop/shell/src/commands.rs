use tauri::Manager;

use crate::{ApplicationSnapshot, DesktopState, InvokeCandidateRequest, RootSearchSnapshot};

#[tauri::command]
pub(crate) fn open_session(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    updates: tauri::ipc::Channel<RootSearchSnapshot>,
) -> Result<ApplicationSnapshot, String> {
    authorize_launcher(&window)?;
    state.bind_search_updates(updates);
    let root_search = state.ensure_search()?;
    Ok(ApplicationSnapshot {
        session_id: state.next_session_id(),
        locale: nanika_platform::system_locale(),
        root_search,
    })
}

#[tauri::command]
pub(crate) fn publish_query(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    query: String,
) -> Result<RootSearchSnapshot, String> {
    authorize_launcher(&window)?;
    if query.chars().count() > 4_096 {
        return Err("search query exceeds the supported length".to_owned());
    }
    state.publish_query(query)
}

#[tauri::command]
pub(crate) fn invoke_candidate(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    request: InvokeCandidateRequest,
) -> Result<bool, String> {
    authorize_launcher(&window)?;
    state.invoke(&request.extension_id, &request.entry_id, &request.action_id)
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
