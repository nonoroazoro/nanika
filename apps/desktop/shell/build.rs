fn main() {
    // Isolation is embedded by codegen, including development builds.
    println!("cargo:rerun-if-changed=isolation");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .codegen(tauri_build::CodegenContext::new())
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS))
    )
    .expect("failed to build the Nanika desktop shell");
}

const COMMANDS: &[&str] = &[
    "dismiss_launcher",
    "read_context_menu",
    "read_configuration_menu",
    "invoke_configuration_menu",
    "invoke_context_menu",
    "acknowledge_search",
    "close_session",
    "invoke_candidate",
    "open_session",
    "publish_query",
    "read_results",
    "read_text",
    "read_localization_resources",
    "read_startup_diagnostic",
    "view_event",
    "open_settings",
    "read_settings",
    "search_settings",
    "acknowledge_settings_delivery",
    "settings_ready",
    "settings_window_action",
    "save_settings",
    "invoke_settings_action",
    "set_extension_enabled",
    "pick_settings_directory",
    "save_host_settings",
    "set_shortcut_recording",
    "read_startup",
    "set_startup"
];
