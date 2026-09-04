#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[path = "ApplicationSnapshot.rs"]
mod application_snapshot;
mod commands;
#[path = "DesktopState.rs"]
mod desktop_state;
#[path = "IconProtocol.rs"]
mod icon_protocol;
#[path = "IconRequest.rs"]
mod icon_request;
#[path = "InvokeCandidateRequest.rs"]
mod invoke_candidate_request;
#[path = "RootSearchSnapshot.rs"]
mod root_search_snapshot;
#[path = "SearchResult.rs"]
mod search_result;
mod tray;
mod window;

use application_snapshot::*;
use commands::*;
use desktop_state::*;
use invoke_candidate_request::*;
use root_search_snapshot::*;
use search_result::*;
use window::*;

#[cfg(target_os = "macos")]
const DEFAULT_HOTKEY: &str = "Ctrl+Space";
#[cfg(not(target_os = "macos"))]
const DEFAULT_HOTKEY: &str = "Alt+Space";

pub fn run() -> Result<(), String> {
    let paths = nanika_storage::NanikaPaths::discover()
        .ok_or_else(|| "Nanika could not resolve its data directories".to_owned())?;
    let identity = nanika_core::PROJECT_IDENTITY.bundle_id;
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
    let icon_protocol = icon_protocol::IconProtocol::spawn(paths.cache_root().to_path_buf())?;

    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol(
            "nanika-icon",
            move |context, request, responder| {
                icon_protocol.respond(context.webview_label(), request, responder);
            },
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(DesktopState::new(instance, diagnostics))
        .invoke_handler(tauri::generate_handler![
            dismiss_launcher,
            invoke_candidate,
            open_session,
            publish_query,
        ])
        .setup(move |app| {
            tray::install(app.handle())?;
            let notification_handle = app.handle().clone();
            app.state::<DesktopState>().set_notifier(Arc::new(move || {
                notification_handle
                    .state::<DesktopState>()
                    .notify_search_updates();
            }));
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
                            if !runtime.startup_diagnostics().is_empty() {
                                tracing::warn!(
                                    count = runtime.startup_diagnostics().len(),
                                    "one or more runtime capabilities are unavailable"
                                );
                            }
                            if let Err(error) = runtime_handle
                                .state::<DesktopState>()
                                .install_runtime(runtime)
                            {
                                tracing::error!(%error, "runtime initialization failed");
                            }
                        }
                        Err(error) => {
                            tracing::error!(%error, "runtime initialization failed");
                        }
                    }
                })?;
            app.global_shortcut()
                .on_shortcut(DEFAULT_HOTKEY, |app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = nanika_platform::current_hotkey_delivery_delay(shortcut.id());
                        let _ = toggle_launcher(app);
                    }
                })?;
            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("nanika-instance-bridge".to_owned())
                .spawn(move || {
                    while let Ok(event) = events.recv() {
                        if event == nanika_platform::PlatformEvent::Open {
                            let _ = show_launcher(&handle);
                        }
                    }
                })?;
            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("launcher") {
                window.set_shadow(false)?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|error| error.to_string())
}
