use nanika_extension_package::{ExtensionActivation, parse_extension_manifest};

#[test]
fn activation_is_explicit_and_rejects_dynamic_or_acp_on_demand() {
    let mut manifest = serde_json::json!({
        "format": "nanika-extension", "name": "Test Extension", "icon": "extension",
        "manifestVersion": 1,
        "id": "test.extension", "version": "0.1.0", "hostApi": "^0.1",
        "targets": { "x86_64-pc-windows-msvc": { "entrypoint": "bin/x86_64-pc-windows-msvc/test.exe" } },
        "runtime": { "protocol": "nanika", "protocolVersion": 1 },
        "contributes": { "commands": [{ "command": "test", "title": "Test", "description": "Test command" }] }
    });
    assert_eq!(
        parse_extension_manifest(&manifest.to_string())
            .unwrap()
            .activation,
        ExtensionActivation::Startup
    );
    manifest["activation"] = "onDemand".into();
    assert_eq!(
        parse_extension_manifest(&manifest.to_string())
            .unwrap()
            .activation,
        ExtensionActivation::OnDemand
    );
    manifest["contributes"]["rootSearch"] = serde_json::json!({});
    assert!(parse_extension_manifest(&manifest.to_string()).is_err());
    manifest["contributes"]
        .as_object_mut()
        .unwrap()
        .remove("rootSearch");
    manifest["runtime"]["protocol"] = "acp".into();
    assert!(parse_extension_manifest(&manifest.to_string()).is_err());
    manifest["activation"] = "unknown".into();
    assert!(parse_extension_manifest(&manifest.to_string()).is_err());
}
