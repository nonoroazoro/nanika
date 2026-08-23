use std::path::PathBuf;

use crate::{ApplicationArguments, DiscoveryState, IconCache, platform};

#[cfg(windows)]
#[test]
fn windows_executable_icons_are_cached_at_both_densities() {
    let Some(system_root) = std::env::var_os("SystemRoot") else {
        return;
    };
    let executable = PathBuf::from(system_root).join("System32/notepad.exe");
    if !executable.is_file() {
        return;
    }
    let root = test_root("native");
    let mut entry = platform::read_entry(&mut DiscoveryState::new(), &executable, 1, 0)
        .expect("executable should parse")
        .expect("executable should contribute an entry");
    let cache = IconCache::new(&root);
    cache.prepare(&mut entry).expect("icon should extract");
    assert_ne!(entry.icon_key, IconCache::fallback_key());
    for size in [32, 64] {
        let bytes = std::fs::read(root.join(&entry.icon_key).join(format!("{size}.png")))
            .expect("cached icon should exist");
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn fallback_icons_are_valid_png_files() {
    let root = test_root("fallback");
    let cache = IconCache::new(&root);
    let mut entry = platform::read_entry(
        &mut DiscoveryState::new(),
        PathBuf::from("missing").as_path(),
        1,
        0,
    )
    .unwrap_or(None)
    .unwrap_or_else(|| crate::ApplicationEntry {
        entry_id: "app.missing".to_owned(),
        source_key: "missing".to_owned(),
        display_name: "Missing".to_owned(),
        normalized_name: "missing".to_owned(),
        normalized_tokens: "missing".to_owned(),
        launch_kind: "executable".to_owned(),
        target_path: "missing".to_owned(),
        working_directory: None,
        arguments_json: ApplicationArguments::empty()
            .to_json()
            .expect("arguments should encode"),
        bundle_id: None,
        icon_key: String::new(),
        file_identity: "missing".to_owned(),
        last_seen_at: 1,
        stale: false,
        icon_source: None,
        icon_index: 0,
        priority: 0,
    });
    cache.prepare(&mut entry).expect("fallback should prepare");
    let bytes = std::fs::read(root.join(IconCache::fallback_key()).join("32.png"))
        .expect("fallback icon should exist");
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn prune_removes_only_unreferenced_cache_entries() {
    let root = test_root("prune");
    let cache = IconCache::new(&root);
    let retained = root.join("retained");
    let stale = root.join("stale");
    std::fs::create_dir_all(&retained).expect("retained cache should exist");
    std::fs::create_dir_all(&stale).expect("stale cache should exist");
    std::fs::write(root.join("orphan.tmp"), []).expect("orphan should exist");
    let entry = crate::ApplicationEntry {
        entry_id: "app.retained".to_owned(),
        source_key: "retained".to_owned(),
        display_name: "Retained".to_owned(),
        normalized_name: "retained".to_owned(),
        normalized_tokens: "retained".to_owned(),
        launch_kind: "executable".to_owned(),
        target_path: "retained".to_owned(),
        working_directory: None,
        arguments_json: ApplicationArguments::empty()
            .to_json()
            .expect("arguments should encode"),
        bundle_id: None,
        icon_key: "retained".to_owned(),
        file_identity: "retained".to_owned(),
        last_seen_at: 1,
        stale: false,
        icon_source: None,
        icon_index: 0,
        priority: 0,
    };

    cache.prune(&[entry]).expect("cache should prune");

    assert!(retained.is_dir());
    assert!(!stale.exists());
    assert!(!root.join("orphan.tmp").exists());
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn prune_reports_an_invalid_cache_root() {
    let root = test_root("invalid-prune-root");
    let cache_path = root.join("icons");
    std::fs::write(&cache_path, []).expect("invalid cache root should exist");
    let cache = IconCache::new(&cache_path);

    let error = cache
        .prune(&[])
        .expect_err("invalid cache root should be reported");

    assert!(matches!(error, crate::ApplicationError::Io(_)));
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[cfg(windows)]
#[test]
fn failed_icon_extraction_is_retried_for_the_same_cache_key() {
    let root = test_root("retry");
    let executable = root.join("Invalid.exe");
    std::fs::write(&executable, []).expect("invalid executable should exist");
    let mut entry = crate::ApplicationEntry {
        entry_id: "app.invalid".to_owned(),
        source_key: executable.to_string_lossy().into_owned(),
        display_name: "Invalid".to_owned(),
        normalized_name: "invalid".to_owned(),
        normalized_tokens: "invalid".to_owned(),
        launch_kind: "executable".to_owned(),
        target_path: executable.to_string_lossy().into_owned(),
        working_directory: None,
        arguments_json: ApplicationArguments::empty()
            .to_json()
            .expect("arguments should encode"),
        bundle_id: None,
        icon_key: String::new(),
        file_identity: executable.to_string_lossy().into_owned(),
        last_seen_at: 1,
        stale: false,
        icon_source: Some(executable),
        icon_index: 0,
        priority: 0,
    };
    let cache = IconCache::new(root.join("icons"));

    assert!(cache.prepare(&mut entry).is_err());
    assert!(cache.prepare(&mut entry).is_err());

    let _ = std::fs::remove_dir_all(root);
}

fn test_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "nanika-application-icons-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("test root should exist");
    root
}
