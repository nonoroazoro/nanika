fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .codegen(tauri_build::CodegenContext::new())
            .app_manifest(tauri_build::AppManifest::new().commands(&[
                "dismiss_launcher",
                "invoke_candidate",
                "open_session",
                "publish_query",
            ])),
    )
    .expect("failed to build the Nanika desktop shell");
}
