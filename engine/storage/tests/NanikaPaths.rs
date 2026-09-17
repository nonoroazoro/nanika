use crate::NanikaPaths;

#[test]
fn paths_keep_config_and_generated_data_separate() {
    let paths = NanikaPaths::from_roots("local-data", "cache", "synced-config");
    assert_eq!(paths.app_data_root(), std::path::Path::new("local-data"));
    assert_eq!(paths.cache_root(), std::path::Path::new("cache"));
    assert_eq!(paths.config_root(), std::path::Path::new("synced-config"));
    assert!(paths.host_database().starts_with(paths.app_data_root()));
    assert!(paths.bootstrap_file().starts_with(paths.app_data_root()));
}

#[test]
fn default_paths_use_the_product_name_and_separate_configuration() {
    let paths = NanikaPaths::discover().expect("platform paths should resolve");

    assert_eq!(
        paths
            .app_data_root()
            .file_name()
            .and_then(|name| name.to_str()),
        Some("Nanika")
    );
    assert_eq!(paths.config_root(), paths.app_data_root().join("config"));
    assert_eq!(
        paths
            .cache_root()
            .file_name()
            .and_then(|name| name.to_str()),
        Some(if cfg!(target_os = "windows") {
            "cache"
        } else {
            "Nanika"
        })
    );
}
