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
) -> Result<bool, String> {
    authorize_launcher(&window)?;
    let app = window.app_handle().clone();
    // Both admission and durable execution recording can wait on bounded owners.
    // Keep those waits outside Tauri's async executor and release shared shell state.
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<DesktopState>();
        let (completion, query) = state.invoke(&request)?;
        let outcome = completion
            .recv()
            .map_err(|_| "extension closed without reporting the action result".to_owned())??;
        match outcome {
            nanika_host::ExtensionInvocationOutcome::Completed { effect, .. } => {
                state.record_execution(&request, &query)?;
                Ok(!matches!(effect, nanika_protocol::NavigationEffect::Close))
            }
            nanika_host::ExtensionInvocationOutcome::Cancelled => {
                Err("The action was cancelled before it completed.".to_owned())
            }
        }
    })
    .await
    .map_err(|error| format!("could not wait for the action result: {error}"))?
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
