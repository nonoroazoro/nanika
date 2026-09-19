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

    assert_eq!(config.max_entries, Some(50));
    assert_eq!(config.max_age_days, Some(7));
}

#[test]
fn null_disables_each_retention_limit_independently() {
    for (count, age) in [(None, None), (Some(50), None), (None, Some(7))] {
        let configuration = ExtensionConfiguration::new(BTreeMap::from([
            ("clipboard.maxEntries".to_owned(), serde_json::json!(count)),
            ("clipboard.maxAgeDays".to_owned(), serde_json::json!(age)),
        ]));
        let config = ClipboardConfig::from_configuration(&configuration).unwrap();
        assert_eq!(config.max_entries, count);
        assert_eq!(config.max_age_days, age);
        assert_eq!(config.cutoff_millis(u64::MAX).is_none(), age.is_none());
    }
}

#[test]
fn rejects_missing_values() {
    let error = ClipboardConfig::from_configuration(&ExtensionConfiguration::default())
        .expect_err("missing retention values must fail");

    assert!(error.contains("clipboard.maxEntries"));
}
