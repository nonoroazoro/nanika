use nanika_protocol::{IconReference, IconSource, is_valid_package_icon_path};

#[test]
fn sources_have_one_unambiguous_wire_identity() {
    for (source, json) in [
        (IconSource::Empty, serde_json::json!({"kind":"empty"})),
        (
            IconSource::Package {
                path: "assets/icon.png".into(),
            },
            serde_json::json!({"kind":"package", "path":"assets/icon.png"}),
        ),
        (
            IconSource::Cache(IconReference::new("file-icon").unwrap()),
            serde_json::json!({"kind":"cache", "key":"file-icon"}),
        ),
    ] {
        assert_eq!(serde_json::to_value(&source).unwrap(), json);
        assert_eq!(serde_json::from_value::<IconSource>(json).unwrap(), source);
        assert!(source.is_valid());
    }
    for value in [
        serde_json::json!("calculator"),
        serde_json::json!({"key":"file-icon"}),
        serde_json::json!({"kind":"package","path":"icon.png","key":"file-icon"}),
    ] {
        assert!(serde_json::from_value::<IconSource>(value).is_err());
    }
}

#[test]
fn package_paths_cannot_address_host_files_or_urls() {
    for valid in ["icon.png", "assets/nested/icon-2.png"] {
        assert!(is_valid_package_icon_path(valid));
    }
    for invalid in [
        "calculator",
        "",
        "/icon.png",
        "../icon.png",
        "assets/../icon.png",
        "assets//icon.png",
        "./icon.png",
        "C:/icon.png",
        "assets\\icon.png",
        "%2e%2e/icon.png",
        "https://example.com/icon.png",
        "icon.svg",
        "icon.png?x",
        "icon.png#x",
    ] {
        assert!(!is_valid_package_icon_path(invalid), "{invalid}");
    }
}
