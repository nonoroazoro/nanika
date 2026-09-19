use nanika_extension_package::{ContributionIcon, parse_extension_manifest};

#[test]
fn presentation_metadata_is_required_bounded_and_host_rendered() {
    let baseline = serde_json::json!({
        "format": "nanika-extension", "manifestVersion": 1,
        "id": "example.tools", "name": "Example Tools", "icon": "script",
        "version": "0.1.0", "hostApi": "^0.1",
        "targets": { "x86_64-pc-windows-msvc": { "entrypoint": "bin/x86_64-pc-windows-msvc/tools.exe" } },
        "runtime": { "protocol": "nanika", "protocolVersion": 1 }
    });
    let parsed = parse_extension_manifest(&baseline.to_string()).unwrap();
    assert_eq!(parsed.name, "Example Tools");
    assert_eq!(parsed.icon, ContributionIcon::Script);
    for name in [
        String::new(),
        "  ".into(),
        "a".repeat(129),
        "Tools\n".into(),
    ] {
        let mut invalid = baseline.clone();
        invalid["name"] = name.into();
        assert!(parse_extension_manifest(&invalid.to_string()).is_err());
    }
    for icon in ["https://example.com/icon.svg", "<svg/>", "unknown"] {
        let mut invalid = baseline.clone();
        invalid["icon"] = icon.into();
        assert!(parse_extension_manifest(&invalid.to_string()).is_err());
    }
    for key in ["name", "icon"] {
        let mut invalid = baseline.clone();
        invalid.as_object_mut().unwrap().remove(key);
        assert!(parse_extension_manifest(&invalid.to_string()).is_err());
    }
}
