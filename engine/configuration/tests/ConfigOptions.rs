use nanika_config::ConfigStore;

#[test]
fn options_resolve_independently_without_writing() {
    let root = std::env::temp_dir().join(format!("nanika-options-{}", uuid::Uuid::new_v4()));
    let store = ConfigStore::open(root.join("machine"), root.join("config")).unwrap();
    let path = store.config_file();
    let text = r#"{"formatVersion":1,"valid":false,"wrongType":"false","outOfRange":-1}"#;
    std::fs::write(&path, text).unwrap();
    let options = store.options(&path).unwrap();
    assert!(!options.get("valid", true));
    assert!(options.get("wrongType", true));
    assert_eq!(options.get("missing", 17), 17);
    assert_eq!(
        options.get_validated("outOfRange", 50, |value: &i64| {
            if *value > 0 {
                Ok(())
            } else {
                Err("must be positive".into())
            }
        }),
        50
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    for invalid in ["{ malformed", "[]", r#"{"formatVersion":2,"valid":false}"#] {
        std::fs::write(&path, invalid).unwrap();
        assert!(store.options(&path).unwrap().get("valid", true));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
    }
    std::fs::remove_file(&path).unwrap();
    assert!(store.options(&path).unwrap().get("valid", true));
    assert!(!path.exists());
    std::fs::remove_dir_all(root).unwrap();
}
