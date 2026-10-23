use nanika_host::{OperationError, OperationErrorCode as Code};
use tauri::Manager;

use crate::{
    ApplicationSnapshot, DesktopState, InvokeCandidateRequest, PublishQueryRequest,
    RootSearchSnapshot
};

#[tauri::command]
pub(crate) fn read_startup_diagnostic(
    window: tauri::WebviewWindow,
    session_id: u64
) -> Result<nanika_host::LifecycleFailure, OperationError> {
    authorize_launcher(&window)?;
    window
        .state::<DesktopState>()
        .startup_diagnostic(session_id)
}

#[tauri::command]
pub(crate) async fn read_configuration_menu(
    window: tauri::WebviewWindow,
    request: crate::ConfigurationMenuRequest
) -> Result<nanika_host::ConfigurationMenu, OperationError> {
    authorize_launcher(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        window.state::<DesktopState>().configuration_menu(&request)
    })
    .await
    .map_err(|error| OperationError::with_detail(Code::InvocationUnconfirmed, error.to_string()))?
}

#[tauri::command]
pub(crate) async fn invoke_configuration_menu(
    window: tauri::WebviewWindow,
    request: crate::ConfigurationMenuRequest,
    operation: crate::ConfigurationMenuOperation
) -> Result<(), crate::SettingsOperationError> {
    authorize_launcher(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let progress_owner = app.clone();
        app.state::<DesktopState>()
            .invoke_configuration_menu(&request, operation, move |update| {
                progress_owner
                    .state::<DesktopState>()
                    .publish_settings_application(update);
            })
    })
    .await
    .map_err(|error| OperationError::with_detail(Code::InvocationUnconfirmed, error.to_string()))?
}

#[tauri::command]
pub(crate) fn read_context_menu(
    window: tauri::WebviewWindow,
    request: crate::ContextMenuRequest
) -> Result<Vec<nanika_protocol::Action>, OperationError> {
    authorize_launcher(&window)?;
    window.state::<crate::DesktopState>().menu_actions(&request)
}

#[tauri::command]
pub(crate) async fn invoke_context_menu(
    window: tauri::WebviewWindow,
    request: crate::ContextMenuRequest,
    action_id: String,
    confirmed: bool
) -> Result<Option<crate::ViewEventReceipt>, OperationError> {
    authorize_launcher(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        window
            .state::<crate::DesktopState>()
            .invoke_menu_action(&request, action_id, confirmed)
    })
    .await
    .map_err(|error| OperationError::with_detail(Code::InvocationUnconfirmed, error.to_string()))?
}

#[tauri::command]
pub(crate) async fn open_session(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    updates: tauri::ipc::Channel<RootSearchSnapshot>
) -> Result<ApplicationSnapshot, OperationError> {
    authorize_launcher(&window)?;
    state.open_session(updates)
}

#[tauri::command]
pub(crate) async fn publish_query(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    request: PublishQueryRequest
) -> Result<(), OperationError> {
    authorize_launcher(&window)?;
    state.publish_query(request)
}

#[tauri::command]
pub(crate) async fn acknowledge_search(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    session_id: u64,
    revision: u64
) -> Result<(), OperationError> {
    authorize_launcher(&window)?;
    state.acknowledge_search(session_id, revision)
}

#[tauri::command]
pub(crate) async fn close_session(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    session_id: u64
) -> Result<(), OperationError> {
    authorize_launcher(&window)?;
    state.close_session(session_id);
    Ok(())
}

#[tauri::command]
pub(crate) async fn invoke_candidate(
    window: tauri::WebviewWindow,
    request: InvokeCandidateRequest
) -> Result<(), OperationError> {
    authorize_launcher(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<DesktopState>().run_invocation(
            &request,
            if request.confirmed {
                nanika_protocol::ActionInvocation::Confirmed
            } else {
                nanika_protocol::ActionInvocation::Default
            }
        )
    })
    .await
    .map_err(|error| OperationError::with_detail(Code::InvocationUnconfirmed, error.to_string()))?
}

#[tauri::command]
pub(crate) async fn view_event(
    window: tauri::WebviewWindow,
    request: crate::ViewEventRequest
) -> Result<crate::ViewEventReceipt, OperationError> {
    authorize_launcher(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<DesktopState>().run_view_event(request, None)
    })
    .await
    .map_err(|error| {
        OperationError::with_detail(Code::ViewOperationUnconfirmed, error.to_string())
    })?
}

#[tauri::command]
pub(crate) async fn read_text(
    window: tauri::WebviewWindow,
    request: crate::ReadTextRequest
) -> Result<nanika_protocol::TextChunk, nanika_host::OperationError> {
    use nanika_host::{OperationError, OperationErrorCode as Code};
    authorize_launcher(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || app.state::<DesktopState>().read_text(request))
        .await
        .map_err(|error| OperationError::with_detail(Code::TextReadFailed, error.to_string()))?
}

#[tauri::command]
pub(crate) fn dismiss_launcher(window: tauri::WebviewWindow) -> Result<(), OperationError> {
    authorize_launcher(&window)?;
    crate::window::hide_window(&window.as_ref().window())
        .map_err(|detail| OperationError::with_detail(Code::WindowCloseFailed, detail))
}

fn authorize_launcher(window: &tauri::WebviewWindow) -> Result<(), OperationError> {
    if window.label() == "launcher" && window.app_handle().get_webview_window("launcher").is_some()
    {
        Ok(())
    } else {
        Err(OperationError::new(Code::SessionUnavailable))
    }
}

#[tauri::command]
pub(crate) async fn open_settings(
    window: tauri::WebviewWindow,
    extension_id: Option<String>
) -> Result<(), OperationError> {
    authorize_launcher(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let desktop = app.state::<DesktopState>();
        let _operation = desktop
            .begin_operation()
            .map_err(|detail| OperationError::with_detail(Code::ApplicationShuttingDown, detail))?;
        if let Some(id) = &extension_id {
            desktop
                .validate_settings_destination(id)
                .map_err(|detail| OperationError::with_detail(Code::SettingsOpenFailed, detail))?;
        }
        crate::settings::show_settings(&app, extension_id)
            .map_err(|detail| OperationError::with_detail(Code::SettingsOpenFailed, detail))
    })
    .await
    .map_err(|error| OperationError::with_detail(Code::SettingsOpenFailed, error.to_string()))?
}

#[tauri::command]
pub(crate) fn settings_ready(window: tauri::WebviewWindow) -> Result<bool, OperationError> {
    authorize_settings(&window)?;
    crate::settings::ready(&window)
        .map_err(|detail| OperationError::with_detail(Code::SettingsReadyFailed, detail))?;
    Ok(crate::adapters::CUSTOM_SETTINGS_CONTROLS)
}

#[tauri::command]
pub(crate) fn settings_window_action(
    window: tauri::WebviewWindow,
    action: crate::SettingsWindowAction
) -> Result<(), OperationError> {
    authorize_settings(&window)?;
    let code = match &action {
        crate::SettingsWindowAction::Drag => Code::WindowDragFailed,
        crate::SettingsWindowAction::Minimize => Code::WindowMinimizeFailed,
        crate::SettingsWindowAction::ToggleMaximize => Code::WindowResizeFailed,
        crate::SettingsWindowAction::Close => Code::WindowCloseFailed
    };
    crate::settings::action(&window, action)
        .map_err(|detail| OperationError::with_detail(code, detail))
}

#[tauri::command]
pub(crate) async fn read_settings(
    window: tauri::WebviewWindow,
    updates: Option<tauri::ipc::JavaScriptChannelId>
) -> Result<crate::SettingsSnapshot, OperationError> {
    authorize_settings(&window)?;
    // Window presentation must remain available even when reading configuration fails.
    if let Some(updates) = updates {
        window
            .state::<DesktopState>()
            .subscribe_settings(updates.channel_on(window.as_ref().clone()));
    }
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let general = app.state::<crate::host_settings::HostSettings>().snapshot();
        let mut snapshot = app
            .state::<DesktopState>()
            .read_settings(general)
            .map_err(|detail| OperationError::with_detail(Code::SettingsReadFailed, detail))?;
        snapshot.navigation = window
            .state::<crate::SettingsWindow>()
            .navigation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        snapshot.maximized = window.is_maximized().map_err(|error| {
            OperationError::with_detail(Code::SettingsReadFailed, error.to_string())
        })?;
        Ok(snapshot)
    })
    .await
    .map_err(|error| OperationError::with_detail(Code::SettingsReadFailed, error.to_string()))?
}

#[tauri::command]
pub(crate) async fn search_settings(
    window: tauri::WebviewWindow,
    query: String,
    results: tauri::ipc::JavaScriptChannelId
) -> Result<(), OperationError> {
    authorize_settings(&window)?;
    if query.chars().count() > nanika_search::MAX_QUERY_CHARS {
        return Err(OperationError::new(Code::SearchQueryTooLong));
    }
    let channel: tauri::ipc::Channel<Vec<crate::SettingsSearchEntry>> =
        results.channel_on(window.as_ref().clone());
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let entries = app.state::<DesktopState>().search_settings(&query);
        channel.send(entries).map_err(|error| {
            OperationError::with_detail(Code::SettingsSearchFailed, error.to_string())
        })
    })
    .await
    .map_err(|error| OperationError::with_detail(Code::SettingsSearchFailed, error.to_string()))?
}

#[tauri::command]
pub(crate) fn acknowledge_settings_delivery(
    window: tauri::WebviewWindow,
    delivery_id: u64
) -> Result<(), OperationError> {
    authorize_settings(&window)?;
    window
        .state::<DesktopState>()
        .acknowledge_settings_delivery(delivery_id);
    Ok(())
}

#[tauri::command]
pub(crate) async fn read_localization_resources(
    window: tauri::WebviewWindow,
    request: crate::LocalizationResourceRequest
) -> Result<tauri::ipc::Response, OperationError> {
    match &request {
        crate::LocalizationResourceRequest::Settings { .. } => authorize_settings(&window)?,
        crate::LocalizationResourceRequest::Launcher { .. } => authorize_launcher(&window)?
    }
    let permit = window.state::<DesktopState>().reserve_localization_read()?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let resources = app
            .state::<DesktopState>()
            .read_localization_resources(request)?;
        // Serialize while the bounded worker permit is held, never on the native event loop.
        let body = serde_json::to_string(&resources).map_err(|error| {
            OperationError::with_detail(Code::LocalizationReadFailed, error.to_string())
        })?;
        Ok(tauri::ipc::Response::new(body))
    })
    .await
    .map_err(|error| OperationError::with_detail(Code::LocalizationReadFailed, error.to_string()))?
}

#[tauri::command]
pub(crate) async fn invoke_settings_action(
    window: tauri::WebviewWindow,
    request: crate::SettingsActionRequest
) -> Result<(), OperationError> {
    authorize_settings(&window)?;
    if !nanika_foundation::is_valid_extension_id(&request.extension_id)
        || request.action_id.is_empty()
        || request.action_id.len() > 128
    {
        return Err(OperationError::with_detail(
            Code::ActionUnavailable,
            "Invalid settings action request."
        ));
    }
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<DesktopState>().invoke_settings_action(request)
    })
    .await
    .map_err(|error| OperationError::with_detail(Code::InvocationUnconfirmed, error.to_string()))?
}

#[tauri::command]
pub(crate) async fn save_settings(
    window: tauri::WebviewWindow,
    request: crate::SaveSettingsRequest
) -> Result<crate::SettingsApplicationUpdate, nanika_host::ConfigurationSaveError> {
    authorize_settings(&window).map_err(nanika_host::ConfigurationSaveError::rejected)?;
    crate::validate_settings_request(&request)
        .map_err(nanika_host::ConfigurationSaveError::rejected)?;
    let app = window.app_handle().clone();
    let owner = app.clone();
    let (update, receipt) = tauri::async_runtime::spawn_blocking(move || {
        let progress_owner = owner.clone();
        owner
            .state::<DesktopState>()
            .save_settings(request, move |update| {
                progress_owner
                    .state::<DesktopState>()
                    .publish_settings_application(update);
            })
    })
    .await
    .map_err(|error| {
        nanika_host::ConfigurationSaveError::unconfirmed(OperationError::with_detail(
            Code::ConfigurationUnconfirmed,
            error.to_string()
        ))
    })??;
    let completion = update.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _ = app
            .state::<DesktopState>()
            .complete_settings_save(completion, receipt);
    });
    Ok(update)
}

#[tauri::command]
pub(crate) async fn pick_settings_directory(
    window: tauri::WebviewWindow,
    extension_id: String,
    key: String
) -> Result<Option<String>, OperationError> {
    authorize_settings(&window)?;
    if !nanika_foundation::is_valid_extension_id(&extension_id) || key.len() > 128 {
        return Err(OperationError::new(Code::DirectorySettingUnavailable));
    }
    tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_dialog::DialogExt;
        let app = window.app_handle();
        let state = app.state::<crate::SettingsWindow>();
        let _picker = state
            .directory_picker
            .try_lock()
            .map_err(|_| OperationError::new(Code::OperationPending))?;
        let general = app.state::<crate::host_settings::HostSettings>().snapshot();
        let desktop = app.state::<DesktopState>();
        let settings = desktop
            .read_settings(general)
            .map_err(|detail| OperationError::with_detail(Code::SettingsReadFailed, detail))?;
        let _operation = desktop
            .begin_operation()
            .map_err(|detail| OperationError::with_detail(Code::ApplicationShuttingDown, detail))?;
        let property = settings
            .extensions
            .iter()
            .find(|extension| extension.info.id == extension_id)
            .and_then(|extension| extension.configuration.as_ref())
            .and_then(|configuration| configuration.contribution.properties.get(&key))
            .filter(|property| property.schema.is_directory_list())
            .ok_or_else(|| OperationError::new(Code::DirectorySettingUnavailable))?;
        let title = crate::host_resources::resources()
            .map_err(|detail| OperationError::with_detail(Code::DirectoryPickerFailed, detail))?
            .format(
                app.state::<crate::host_settings::HostSettings>()
                    .snapshot()
                    .localization
                    .locale,
                &nanika_protocol::message("native-add-folder", []),
                nanika_platform::target_platform().as_str(),
                128,
                false
            )
            .map_err(|detail| OperationError::with_detail(Code::DirectoryPickerFailed, detail))?;
        let Some(selected) = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title(title)
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        let path = selected.into_path().map_err(|error| {
            OperationError::with_detail(Code::DirectoryPathInvalid, error.to_string())
        })?;
        if !path.is_absolute() || !path.is_dir() {
            return Err(OperationError::new(Code::DirectoryUnavailable));
        }
        let path = path
            .into_os_string()
            .into_string()
            .map_err(|_| OperationError::new(Code::DirectoryPathInvalid))?;
        property
            .schema
            .validate_value(&serde_json::json!([path]))
            .map_err(|detail| OperationError::with_detail(Code::DirectoryPathInvalid, detail))?;
        Ok(Some(path))
    })
    .await
    .map_err(|error| OperationError::with_detail(Code::DirectoryPickerFailed, error.to_string()))?
}

fn authorize_settings(window: &tauri::WebviewWindow) -> Result<(), OperationError> {
    if window.label() == "settings" && window.app_handle().get_webview_window("settings").is_some()
    {
        Ok(())
    } else {
        Err(OperationError::new(Code::SessionUnavailable))
    }
}

#[tauri::command]
pub(crate) async fn save_host_settings(
    window: tauri::WebviewWindow,
    request: crate::HostSettingsChange
) -> Result<
    crate::SettingsWriteResult<nanika_config::LauncherPreferences>,
    crate::HostSettingsFailure
> {
    authorize_settings(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let desktop = app.state::<DesktopState>();
        let _operation = desktop
            .begin_operation()
            .map_err(|detail| OperationError::with_detail(Code::ApplicationShuttingDown, detail))?;
        app.state::<crate::host_settings::HostSettings>()
            .save(&app, request)
    })
    .await
    .map_err(|error| OperationError::with_detail(Code::PreferencesUnconfirmed, error.to_string()))?
}

#[tauri::command]
pub(crate) fn set_shortcut_recording(
    window: tauri::WebviewWindow,
    recording: bool
) -> Result<bool, OperationError> {
    authorize_settings(&window)?;
    let desktop = window.state::<DesktopState>();
    let _operation = desktop
        .begin_operation()
        .map_err(|detail| OperationError::with_detail(Code::ApplicationShuttingDown, detail))?;
    // Focus loss after the pointer event ends recording normally.
    let recording = recording
        && window.is_visible().map_err(|error| {
            OperationError::with_detail(Code::ShortcutRecordingFailed, error.to_string())
        })?
        && window.is_focused().map_err(|error| {
            OperationError::with_detail(Code::ShortcutRecordingFailed, error.to_string())
        })?;
    window
        .state::<crate::host_settings::HostSettings>()
        .recording
        .store(recording, std::sync::atomic::Ordering::Release);
    Ok(recording)
}

#[tauri::command]
pub(crate) async fn read_startup(
    window: tauri::WebviewWindow
) -> Result<nanika_platform::StartupStatus, OperationError> {
    authorize_settings(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let desktop = app.state::<DesktopState>();
        let _operation = desktop
            .begin_operation()
            .map_err(|detail| OperationError::with_detail(Code::ApplicationShuttingDown, detail))?;
        app.state::<crate::host_settings::HostSettings>()
            .startup(None)
    })
    .await
    .map_err(|error| OperationError::with_detail(Code::StartupReadFailed, error.to_string()))?
}

#[tauri::command]
pub(crate) async fn set_startup(
    window: tauri::WebviewWindow,
    enabled: bool
) -> Result<nanika_platform::StartupStatus, OperationError> {
    authorize_settings(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let desktop = app.state::<DesktopState>();
        let _operation = desktop
            .begin_operation()
            .map_err(|detail| OperationError::with_detail(Code::ApplicationShuttingDown, detail))?;
        app.state::<crate::host_settings::HostSettings>()
            .startup(Some(enabled))
    })
    .await
    .map_err(|error| {
        OperationError::with_detail(Code::StartupWriteUnconfirmed, error.to_string())
    })?
}

#[tauri::command]
pub(crate) async fn set_extension_enabled(
    window: tauri::WebviewWindow,
    extension_id: String,
    enabled: bool
) -> Result<(), nanika_host::LifecycleFailure> {
    authorize_settings(&window)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<DesktopState>()
            .set_extension_enabled(&extension_id, enabled)
    })
    .await
    .map_err(|error| {
        nanika_host::LifecycleFailure::from(nanika_host::OperationError::with_detail(
            nanika_host::OperationErrorCode::ExtensionLifecycleUnconfirmed,
            error.to_string()
        ))
    })?
}

#[tauri::command]
pub(crate) async fn read_results(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, DesktopState>,
    request: crate::ReadResultsRequest
) -> Result<(), OperationError> {
    authorize_launcher(&window)?;
    state.read_results(request)
}
