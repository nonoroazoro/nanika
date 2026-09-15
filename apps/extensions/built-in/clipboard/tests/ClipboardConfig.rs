use std::collections::BTreeMap;

use nanika_protocol::ExtensionConfiguration;

use crate::ClipboardConfig;

#[test]
fn parses_host_configuration() {
    let configuration = ExtensionConfiguration::new(BTreeMap::from([
        ("clipboard.maxAgeDays".to_owned(), serde_json::json!(7)),
        ("clipboard.maxEntries".to_owned(), serde_json::json!(50)),
    ]));

    let config = ClipboardConfig::from_configuration(&configuration)
        .expect("host configuration should be valid");

    assert_eq!(config.max_entries, 50);
    assert_eq!(config.max_age_days, 7);
}

#[test]
fn rejects_missing_values() {
    let error = ClipboardConfig::from_configuration(&ExtensionConfiguration::default())
        .expect_err("missing retention values must fail");

    assert!(error.contains("clipboard.maxEntries"));
}
