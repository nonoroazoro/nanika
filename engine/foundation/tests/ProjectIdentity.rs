use nanika_foundation::{PRODUCT_NAME, PROJECT_IDENTITY};

const TAURI_CONFIG: &str = include_str!("../../../apps/desktop/shell/tauri.conf.json");

#[test]
fn foundation_identity_matches_the_desktop_bundle() {
    let config: serde_json::Value =
        serde_json::from_str(TAURI_CONFIG).expect("Tauri configuration should be valid JSON");

    assert_eq!(config["productName"].as_str(), Some(PRODUCT_NAME));
    assert_eq!(
        config["identifier"].as_str(),
        Some(PROJECT_IDENTITY.bundle_id)
    );
}
