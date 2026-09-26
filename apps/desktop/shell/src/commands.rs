use tauri::Manager;

use crate::{
    ApplicationSnapshot, DesktopState, InvokeCandidateRequest, PublishQueryRequest,
    RootSearchSnapshot,
};

#[tauri::command]
pub(crate) fn read_context_menu(
    window: tauri::WebviewWindow,
    request: crate::ContextMenuRequest,
) -> Result<Vec<nanika_protocol::Action>, String> {
    authorize_launcher(&window)?;
    window.state::<crate::DesktopState>().menu_actions(&request)
}

#[tauri::command]
pub(crate) async fn invoke_context_menu(
    window: tauri::WebviewWindow,
    request: crate::ContextMenuRequest,
    action_id: String,
    confirmed: bool,
) -> Result<Option<crate::ViewEventReceipt>, String> {
    authorize_launcher(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        window
            .state::<crate::DesktopState>()
            .invoke_menu_action(&request, action_id, confirmed)
    })
    .await
    .map_err(|error| error.to_string())?
}

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
) -> Result<(), String> {
    authorize_launcher(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<DesktopState>().run_invocation(
            &request,
            None,
            nanika_protocol::ActionInvocation::Default,
        )
    })
    .await
    .map_err(|error| format!("could not wait for the action result: {error}"))?
}

#[tauri::command]
pub(crate) async fn view_event(
    window: tauri::WebviewWindow,
    request: crate::ViewEventRequest,
) -> Result<crate::ViewEventReceipt, String> {
    authorize_launcher(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<DesktopState>().run_view_event(request, None)
    })
    .await
    .map_err(|error| format!("could not wait for the view result: {error}"))?
}

#[tauri::command]
pub(crate) fn dismiss_launcher(window: tauri::WebviewWindow) -> Result<(), String> {
    authorize_launcher(&window)?;
    crate::window::hide_window(&window.as_ref().window())
}

fn authorize_launcher(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == "launcher" && window.app_handle().get_webview_window("launcher").is_some()
    {
        Ok(())
    } else {
        Err("command is not available to this window".to_owned())
    }
}

#[tauri::command]
pub(crate) async fn open_settings(window: tauri::WebviewWindow) -> Result<(), String> {
    authorize_launcher(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let desktop = app.state::<DesktopState>();
        let _operation = desktop.begin_operation()?;
        crate::settings::show_settings(&app)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) fn settings_ready(window: tauri::WebviewWindow) -> Result<bool, String> {
    authorize_settings(&window)?;
    crate::settings::ready(&window)?;
    Ok(crate::adapters::CUSTOM_SETTINGS_CONTROLS)
}

#[tauri::command]
pub(crate) fn settings_window_action(
    window: tauri::WebviewWindow,
    action: crate::SettingsWindowAction,
) -> Result<(), String> {
    authorize_settings(&window)?;
    crate::settings::action(&window, action)
}

#[tauri::command]
pub(crate) async fn read_settings(
    window: tauri::WebviewWindow,
    updates: Option<tauri::ipc::JavaScriptChannelId>,
) -> Result<crate::SettingsSnapshot, String> {
    authorize_settings(&window)?;
    // Window presentation must remain available even when reading configuration fails.
    if let Some(updates) = updates {
        window
            .state::<DesktopState>()
            .subscribe_settings(updates.channel_on(window.as_ref().clone()));
    }
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let general = app.state::<crate::host_settings::HostSettings>().current();
        let mut snapshot = app.state::<DesktopState>().read_settings(general)?;
        snapshot.maximized = window.is_maximized().map_err(|error| error.to_string())?;
        Ok(snapshot)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) fn acknowledge_settings_delivery(
    window: tauri::WebviewWindow,
    delivery_id: u64,
) -> Result<(), String> {
    authorize_settings(&window)?;
    window
        .state::<DesktopState>()
        .acknowledge_settings_delivery(delivery_id);
    Ok(())
}

#[tauri::command]
pub(crate) async fn save_settings(
    window: tauri::WebviewWindow,
    request: crate::SaveSettingsRequest,
) -> Result<crate::SettingsApplicationUpdate, String> {
    authorize_settings(&window)?;
    crate::validate_settings_request(&request)?;
    let app = window.app_handle().clone();
    let owner = app.clone();
    let (update, receipt) = tauri::async_runtime::spawn_blocking(move || {
        let progress_owner = owner.clone();
        owner.state::<DesktopState>().save_settings(
            request,
            move |request_id, extension_id, key, progress| {
                progress_owner
                    .state::<DesktopState>()
                    .publish_settings_application(crate::SettingsApplicationUpdate {
                        request_id,
                        extension_id: extension_id.to_owned(),
                        key: key.to_owned(),
                        result: crate::SettingsSaveResult::Running {
                            progress: Some(progress),
                        },
                    });
            },
        )
    })
    .await
    .map_err(|error| error.to_string())??;
    let mut completion = update.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = receipt.wait();
        let error = match &result {
            Ok(outcome) => outcome.error.as_ref(),
            Err(error) => Some(error),
        };
        if let Some(error) = error {
            tracing::error!(extension_id = completion.extension_id, %error, "settings operation failed");
        }
        completion.result = result.into();
        app.state::<DesktopState>()
            .publish_settings_application(completion);
    });
    Ok(update)
}

#[tauri::command]
pub(crate) async fn pick_settings_directory(
    window: tauri::WebviewWindow,
    extension_id: String,
    key: String,
) -> Result<Option<String>, String> {
    authorize_settings(&window)?;
    if !nanika_foundation::is_valid_extension_id(&extension_id) || key.len() > 128 {
        return Err("Invalid directory setting.".to_owned());
    }
    tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_dialog::DialogExt;
        let app = window.app_handle();
        let state = app.state::<crate::SettingsWindow>();
        let _picker = state
            .directory_picker
            .try_lock()
            .map_err(|_| "A folder picker is already open.".to_owned())?;
        let general = app.state::<crate::host_settings::HostSettings>().current();
        let desktop = app.state::<DesktopState>();
        let settings = desktop.read_settings(general)?;
        let _operation = desktop.begin_operation()?;
        let property = settings
            .extensions
            .iter()
            .find(|extension| extension.info.id == extension_id)
            .and_then(|extension| extension.configuration.as_ref())
            .and_then(|configuration| configuration.contribution.properties.get(&key))
            .filter(|property| property.schema.is_directory_list())
            .ok_or("This setting does not accept directories.")?;
        let Some(selected) = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Add folder")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        let path = selected.into_path().map_err(|error| error.to_string())?;
        if !path.is_absolute() || !path.is_dir() {
            return Err("The selected folder is unavailable.".to_owned());
        }
        let path = path
            .into_os_string()
            .into_string()
            .map_err(|_| "The selected folder path is not valid UTF-8.".to_owned())?;
        property.schema.validate_value(&serde_json::json!([path]))?;
        Ok(Some(path))
    })
    .await
    .map_err(|error| error.to_string())?
}

fn authorize_settings(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == "settings" && window.app_handle().get_webview_window("settings").is_some()
    {
        Ok(())
    } else {
        Err("command is not available to this window".to_owned())
    }
}

#[tauri::command]
pub(crate) async fn save_host_settings(
    window: tauri::WebviewWindow,
    request: crate::HostSettingsChange,
) -> Result<crate::SettingsWriteResult<nanika_config::LauncherPreferences>, String> {
    authorize_settings(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let desktop = app.state::<DesktopState>();
        let _operation = desktop.begin_operation()?;
        app.state::<crate::host_settings::HostSettings>()
            .save(&app, request)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) fn set_shortcut_recording(
    window: tauri::WebviewWindow,
    recording: bool,
) -> Result<bool, String> {
    authorize_settings(&window)?;
    let desktop = window.state::<DesktopState>();
    let _operation = desktop.begin_operation()?;
    // Focus loss after the pointer event ends recording normally.
    let recording = recording
        && window.is_visible().map_err(|error| error.to_string())?
        && window.is_focused().map_err(|error| error.to_string())?;
    window
        .state::<crate::host_settings::HostSettings>()
        .recording
        .store(recording, std::sync::atomic::Ordering::Release);
    Ok(recording)
}

#[tauri::command]
pub(crate) async fn read_startup(window: tauri::WebviewWindow) -> Result<&'static str, String> {
    authorize_settings(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let desktop = app.state::<DesktopState>();
        let _operation = desktop.begin_operation()?;
        app.state::<crate::host_settings::HostSettings>()
            .startup(None)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn set_startup(
    window: tauri::WebviewWindow,
    enabled: bool,
) -> Result<&'static str, String> {
    authorize_settings(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let desktop = app.state::<DesktopState>();
        let _operation = desktop.begin_operation()?;
        app.state::<crate::host_settings::HostSettings>()
            .startup(Some(enabled))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn set_extension_enabled(
    window: tauri::WebviewWindow,
    extension_id: String,
    enabled: bool,
) -> Result<(), String> {
    authorize_settings(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<DesktopState>()
            .set_extension_enabled(&extension_id, enabled)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn read_results(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    request: crate::ReadResultsRequest,
) -> Result<(), String> {
    authorize_launcher(&window)?;
    state.read_results(request)
}
