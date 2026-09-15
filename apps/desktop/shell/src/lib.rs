#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[path = "ApplicationSnapshot.rs"]
mod application_snapshot;
mod commands;
#[path = "DesktopRuntime.rs"]
mod desktop_runtime;
#[path = "DesktopState.rs"]
mod desktop_state;
#[path = "ExtensionViewSnapshot.rs"]
mod extension_view_snapshot;
#[path = "IconProtocol.rs"]
mod icon_protocol;
#[path = "IconRequest.rs"]
mod icon_request;
#[path = "InvokeCandidateRequest.rs"]
mod invoke_candidate_request;
#[path = "PublishQueryRequest.rs"]
mod publish_query_request;
#[path = "ResourceProtocol.rs"]
mod resource_protocol;
#[path = "RootSearchSnapshot.rs"]
mod root_search_snapshot;
#[path = "SearchDelivery.rs"]
mod search_delivery;
#[path = "SearchPhase.rs"]
mod search_phase;
#[path = "SearchResult.rs"]
mod search_result;
#[path = "SearchSession.rs"]
mod search_session;
use extension_view_snapshot::*;
#[path = "NavigationSnapshot.rs"]
mod navigation_snapshot;
use navigation_snapshot::*;
#[path = "NavigationState.rs"]
mod navigation_state;
use navigation_state::*;
#[path = "ViewEventRequest.rs"]
mod view_event_request;
use view_event_request::*;
mod tray;
mod window;

use application_snapshot::*;
use commands::*;
use desktop_runtime::*;
use desktop_state::*;
use invoke_candidate_request::*;
use publish_query_request::*;
use root_search_snapshot::*;
use search_delivery::*;
use search_phase::*;
use search_result::*;
use search_session::*;
use window::*;

#[cfg(test)]
#[path = "../tests/unit/IconProtocol.rs"]
mod icon_protocol_tests;
#[cfg(test)]
#[path = "../tests/unit/NavigationState.rs"]
mod navigation_state_tests;
#[cfg(test)]
#[path = "../tests/unit/SearchDelivery.rs"]
mod search_delivery_tests;

#[cfg(target_os = "macos")]
const DEFAULT_HOTKEY: &str = "Ctrl+Space";
#[cfg(target_os = "windows")]
const DEFAULT_HOTKEY: &str = "Ctrl+Alt+Space";

pub fn run() -> Result<(), String> {
    let paths = nanika_storage::NanikaPaths::discover()
        .ok_or_else(|| "Nanika could not resolve its data directories".to_owned())?;
    let identity = nanika_foundation::PROJECT_IDENTITY.bundle_id;
    let instance = match nanika_platform::acquire_instance(identity, paths.app_data_root())
        .map_err(|error| error.to_string())?
    {
        nanika_platform::InstanceRole::Primary(instance) => instance,
        nanika_platform::InstanceRole::Secondary => {
            nanika_platform::signal_activate(identity, paths.app_data_root())
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
    };
    let mut instance = instance;
    let events = instance.take_events().map_err(|error| error.to_string())?;
    let diagnostics = nanika_host::Diagnostics::initialize(&paths.app_data_root().join("logs"))?;
    let icon_protocol = icon_protocol::IconProtocol::spawn(
        paths.cache_root().to_path_buf(),
        paths.payload_dir().to_path_buf(),
    )?;

    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol(
            "nanika-icon",
            move |context, request, responder| {
                icon_protocol.respond(context.webview_label(), request, responder);
            },
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(DesktopState::new(instance, diagnostics)?)
        .invoke_handler(tauri::generate_handler![
            dismiss_launcher,
            acknowledge_search,
            close_session,
            invoke_candidate,
            open_session,
            publish_query,
            view_event,
        ])
        .on_window_event(handle_window_event)
        .setup(move |app| {
            tray::install(app.handle())?;
            let runtime_handle = app.handle().clone();
            let runtime_paths = paths.clone();
            std::thread::Builder::new()
                .name("nanika-runtime-initializer".to_owned())
                .spawn(move || {
                    match nanika_host::RuntimeService::start(
                        &runtime_paths,
                        include_str!("../../../extensions/distribution.json"),
                    ) {
                        Ok(runtime) => {
                            for diagnostic in runtime.startup_diagnostics() {
                                tracing::warn!(
                                    message = diagnostic,
                                    "runtime capability unavailable"
                                );
                            }
                            if let Err(error) = runtime_handle
                                .state::<DesktopState>()
                                .install_runtime(runtime)
                            {
                                runtime_handle.state::<DesktopState>().fail_startup(error);
                            }
                        }
                        Err(error) => {
                            runtime_handle.state::<DesktopState>().fail_startup(error);
                        }
                    }
                })?;
            app.global_shortcut()
                .on_shortcut(DEFAULT_HOTKEY, |app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(delay) =
                            nanika_platform::current_hotkey_delivery_delay(shortcut.id())
                        {
                            tracing::debug!(
                                delay_ms = delay.as_millis(),
                                "global shortcut delivered"
                            );
                        }
                        if let Err(error) = toggle_launcher(app) {
                            tracing::error!(%error, "global shortcut could not toggle launcher");
                        }
                    }
                })?;
            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("nanika-instance-bridge".to_owned())
                .spawn(move || {
                    while let Ok(event) = events.recv() {
                        if event == nanika_platform::PlatformEvent::Open
                            && let Err(error) = show_launcher(&handle)
                        {
                            tracing::error!(%error, "instance activation could not show launcher");
                        }
                    }
                })?;
            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("launcher") {
                window.set_shadow(false)?;
            }
            Ok(())
        })
        .run(tauri::tauri_build_context!())
        .map_err(|error| error.to_string())
}
