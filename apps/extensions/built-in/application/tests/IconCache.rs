use std::path::PathBuf;

use crate::{ApplicationArguments, DiscoveryState, IconCache, platform};

#[cfg(target_os = "macos")]
#[test]
fn books_icon_skips_empty_icns_slots() {
    let source = PathBuf::from("/System/Applications/Books.app/Contents/Resources/AppIcon.icns");
    if !source.is_file() {
        return;
    }
    let root = test_root("books");
    let mut entry = crate::ApplicationEntry {
        entry_id: "com.apple.iBooksX".to_owned(),
        source_key: source.to_string_lossy().into_owned(),
        display_name: "Books".to_owned(),
        normalized_name: "books".to_owned(),
        normalized_tokens: "books".to_owned(),
        launch_kind: "bundle".to_owned(),
        target_path: "/System/Applications/Books.app".to_owned(),
        working_directory: None,
        arguments_json: ApplicationArguments::empty()
            .to_json()
            .expect("arguments should encode"),
        bundle_id: Some("com.apple.iBooksX".to_owned()),
        icon_key: String::new(),
        file_identity: "books".to_owned(),
        last_seen_at: 1,
        stale: false,
        icon_source: Some(source),
        icon_index: 0,
        priority: 0,
    };
    let cache = IconCache::new(&root);

    cache
        .prepare(&mut entry)
        .expect("Books icon should extract");

    let file = std::fs::File::open(root.join(&entry.icon_key).join("32.png"))
        .expect("cached Books icon should exist");
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("cached Books icon should decode");
    let mut pixels = vec![
        0_u8;
        reader
            .output_buffer_size()
            .expect("decoded size should be available")
    ];
    let output = reader
        .next_frame(&mut pixels)
        .expect("cached Books icon should read");
    assert!(
        pixels[..output.buffer_size()]
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] >= 16),
        "cached Books icon must contain visible pixels"
    );
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

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
fn unavailable_icons_use_the_shared_fallback_until_the_cache_is_complete() {
    let root = test_root("presentation-fallback");
    let cache = IconCache::new(&root);
    let mut entries = vec![test_entry("pending-icon")];

    cache
        .use_available_icons(&mut entries)
        .expect("presentation icons should resolve");

    assert_eq!(entries[0].icon_key, IconCache::fallback_key());
    assert!(
        root.join(IconCache::fallback_key())
            .join("128.png")
            .is_file()
    );
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn complete_cached_icons_are_exposed_to_the_frontend() {
    let root = test_root("presentation-ready");
    let cache = IconCache::new(&root);
    let key = "ready-icon";
    let directory = root.join(key);
    std::fs::create_dir_all(&directory).expect("icon directory should exist");
    for size in [32, 64, 128] {
        std::fs::write(directory.join(format!("{size}.png")), []).expect("icon should exist");
    }
    let mut entries = vec![test_entry(key)];

    cache
        .use_available_icons(&mut entries)
        .expect("presentation icons should resolve");

    assert_eq!(entries[0].icon_key, key);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[test]
fn fallback_markers_keep_failed_extractions_out_of_the_frontend() {
    let root = test_root("presentation-marker");
    let cache = IconCache::new(&root);
    let key = "failed-icon";
    let directory = root.join(key);
    std::fs::create_dir_all(&directory).expect("icon directory should exist");
    for size in [32, 64, 128] {
        std::fs::write(directory.join(format!("{size}.png")), []).expect("icon should exist");
    }
    std::fs::write(directory.join("fallback.marker"), []).expect("marker should exist");
    let mut entries = vec![test_entry(key)];

    cache
        .use_available_icons(&mut entries)
        .expect("presentation icons should resolve");

    assert_eq!(entries[0].icon_key, IconCache::fallback_key());
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

fn test_entry(icon_key: &str) -> crate::ApplicationEntry {
    crate::ApplicationEntry {
        entry_id: "app.test".to_owned(),
        source_key: "test".to_owned(),
        display_name: "Test".to_owned(),
        normalized_name: "test".to_owned(),
        normalized_tokens: "test".to_owned(),
        launch_kind: "executable".to_owned(),
        target_path: "test".to_owned(),
        working_directory: None,
        arguments_json: ApplicationArguments::empty()
            .to_json()
            .expect("arguments should encode"),
        bundle_id: None,
        icon_key: icon_key.to_owned(),
        file_identity: "test".to_owned(),
        last_seen_at: 1,
        stale: false,
        icon_source: None,
        icon_index: 0,
        priority: 0,
    }
}
