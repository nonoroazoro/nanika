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
    ];
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .codegen(tauri_build::CodegenContext::new())
            .app_manifest(tauri_build::AppManifest::new().commands(commands)),
    )
    .expect("failed to build the Nanika desktop shell");
}
