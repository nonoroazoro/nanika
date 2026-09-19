fn main() {
    let commands: &'static [&'static str] = &[
        "dismiss_launcher",
        "acknowledge_search",
        "close_session",
        "invoke_candidate",
        "open_session",
        "publish_query",
        "refresh_search",
        "view_event",
        "open_settings",
        "read_settings",
        "settings_ready",
        "save_settings",
        "pick_settings_directory",
        "save_host_settings",
        "set_shortcut_recording",
        "read_startup",
        "set_startup",
    ];
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .codegen(tauri_build::CodegenContext::new())
            .app_manifest(tauri_build::AppManifest::new().commands(commands)),
    )
    .expect("failed to build the Nanika desktop shell");
}
