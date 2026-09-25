#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

mod adapters;
#[path = "ApplicationSnapshot.rs"]
mod application_snapshot;
mod commands;
#[path = "ContextMenuRequest.rs"]
mod context_menu_request;
use context_menu_request::*;
#[path = "MenuTarget.rs"]
mod menu_target;
use menu_target::*;
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
#[path = "ViewEventReceipt.rs"]
mod view_event_receipt;
use view_event_receipt::*;
#[path = "ViewInvalidationDelivery.rs"]
mod view_invalidation_delivery;
use view_invalidation_delivery::*;
#[path = "HostSettings.rs"]
mod host_settings;
mod settings;
#[path = "SettingsWindow.rs"]
mod settings_window;
use settings_window::*;
#[path = "SettingsWindowAction.rs"]
mod settings_window_action;
use settings_window_action::*;
#[path = "SettingsSnapshot.rs"]
mod settings_snapshot;
mod tray;
mod window;
use settings_snapshot::*;

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
#[path = "../tests/IconProtocol.rs"]
mod icon_protocol_tests;
#[cfg(test)]
#[path = "../tests/NavigationState.rs"]
mod navigation_state_tests;
#[cfg(test)]
#[path = "../tests/SearchDelivery.rs"]
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
            // Development and debug QA must never activate another build.
            if cfg!(debug_assertions)
                || std::env::var("NANIKA_DEV_REQUIRE_PRIMARY").as_deref() == Ok("1")
            {
                return Err(
                    "Another Nanika instance is running; stop it before starting the development build."
                        .to_owned(),
                );
            }
            nanika_platform::signal_activate(identity, paths.app_data_root())
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
    };
    let mut instance = instance;
    let events = instance.take_events().map_err(|error| error.to_string())?;
    let diagnostics = nanika_host::Diagnostics::initialize(&paths.app_data_root().join("logs"))?;
    let host_settings = host_settings::HostSettings::open(&paths)?;
    let icon_protocol = std::sync::Arc::new(icon_protocol::IconProtocol::spawn(
        paths.cache_root().to_path_buf(),
        paths.payload_dir().to_path_buf(),
    )?);

    let mut context = tauri::tauri_build_context!();
    adapters::configure_windows(context.config_mut());

    tauri::Builder::default()
        .manage(SettingsWindow::default())
        .manage(host_settings)
        .manage(std::sync::Arc::clone(&icon_protocol))
        .register_asynchronous_uri_scheme_protocol(
            "nanika-icon",
            move |context, request, responder| {
                icon_protocol.respond(context.webview_label(), request, responder);
            },
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(DesktopState::new(instance, diagnostics)?)
        .invoke_handler(tauri::generate_handler![
            dismiss_launcher,
            read_context_menu,
            invoke_context_menu,
            acknowledge_search,
            close_session,
            invoke_candidate,
            open_session,
            publish_query,
            refresh_search,
            view_event,
            open_settings,
            read_settings,
            acknowledge_settings_progress,
            settings_ready,
            settings_window_action,
            save_settings,
            pick_settings_directory,
            save_host_settings,
            set_shortcut_recording,
            read_startup,
            set_startup,
        ])
        .on_window_event(handle_window_event)
        .setup(move |app| {
            tray::install(app.handle())?;
            let runtime_handle = app.handle().clone();
            let runtime_paths = paths.clone();
            let initializer = std::thread::Builder::new()
                .name("nanika-runtime-initializer".to_owned())
                .spawn(move || {
                    let built_in_manifests = [
                        include_str!("../../../extensions/built-in/application/manifest.jsonc"),
                        include_str!("../../../extensions/built-in/command/manifest.jsonc"),
                        include_str!("../../../extensions/built-in/script/manifest.jsonc"),
                        include_str!("../../../extensions/built-in/calculator/manifest.jsonc"),
                        include_str!("../../../extensions/built-in/clipboard/manifest.jsonc"),
                    ];
                    match nanika_host::RuntimeService::start(&runtime_paths, &built_in_manifests) {
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
                            } else if let Err(error) = settings::prepare_settings(&runtime_handle) {
                                tracing::error!(%error, "settings window preparation failed");
                            }
                        }
                        Err(error) => {
                            runtime_handle.state::<DesktopState>().fail_startup(error);
                        }
                    }
                })?;
            app.state::<DesktopState>().set_initializer(initializer);
            let preferences = app.state::<host_settings::HostSettings>().current();
            app.set_theme(host_settings::native_theme(preferences.theme));
            host_settings::register(app.handle(), &preferences.launcher_shortcut)?;
            let handle = app.handle().clone();
            let instance_bridge = std::thread::Builder::new()
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
            app.state::<DesktopState>()
                .set_instance_bridge(instance_bridge);
            Ok(())
        })
        .build(context)
        .map_err(|error| error.to_string())?
        .run({
            let mut stopping = false;
            let stopped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            move |app, event| {
                if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                    if stopped.load(std::sync::atomic::Ordering::Acquire) {
                        return;
                    }
                    api.prevent_exit();
                    if stopping {
                        return;
                    }
                    stopping = true;
                    let app = app.clone();
                    let stopped = std::sync::Arc::clone(&stopped);
                    tauri::async_runtime::spawn_blocking(move || {
                        app.state::<std::sync::Arc<icon_protocol::IconProtocol>>()
                            .shutdown();
                        app.state::<DesktopState>().shutdown();
                        app.state::<host_settings::HostSettings>().shutdown();
                        stopped.store(true, std::sync::atomic::Ordering::Release);
                        app.exit(code.unwrap_or(0));
                    });
                }
            }
        });
    Ok(())
}

#[path = "SettingsWriteResult.rs"]
mod settings_write_result;
pub(crate) use settings_write_result::*;
#[path = "HostSettingsChange.rs"]
mod host_settings_change;
pub(crate) use host_settings_change::*;

#[path = "SettingsApplications.rs"]
mod settings_applications;
pub(crate) use settings_applications::*;
#[cfg(test)]
#[path = "../tests/SettingsApplications.rs"]
mod settings_applications_tests;

#[cfg(test)]
#[path = "../tests/SettingsSnapshot.rs"]
mod settings_snapshot_tests;
