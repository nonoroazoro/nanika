fn main() {
    println!("cargo:rerun-if-changed=isolation");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .codegen(tauri_build::CodegenContext::new())
            .app_manifest(tauri_build::AppManifest::new().commands(&[
                "verification_report",
                "verification_progress",
                "verification_resources"
            ]))
    )
    .expect("failed to build native verification");
}
