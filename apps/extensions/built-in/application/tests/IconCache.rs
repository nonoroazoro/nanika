use crate::ApplicationEntryData;
use std::path::PathBuf;

use crate::{ApplicationArguments, DiscoveryState, IconCache, platform};

#[cfg(target_os = "macos")]
#[test]
fn system_bundle_icons_generate_all_sizes_and_reuse_complete_caches() {
    let source = PathBuf::from("/System/Applications/Books.app");
    if !source.is_dir() {
        return;
    }
    let root = test_root("books");
    let mut entry = platform::read_entry(&mut DiscoveryState::new(), &source, 0)
        .expect("Books bundle should parse")
        .expect("Books bundle should contribute an entry");
    assert_eq!(entry.icon_source.as_deref(), Some(source.as_path()));
    let cache = IconCache::new(&root);

    entry.icon_key = IconCache::key(&entry, &mut DiscoveryState::new()).unwrap();
    cache.prepare(&entry).expect("Books icon should extract");

    for size in [32, 64, 128] {
        let file = std::fs::File::open(root.join(&entry.icon_key).join(format!("{size}.png")))
            .expect("cached Books icon should exist");
        let mut reader = png::Decoder::new(std::io::BufReader::new(file))
            .read_info()
            .expect("cached Books icon should decode");
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let output = reader
            .next_frame(&mut pixels)
            .expect("cached Books icon should read");
        assert_eq!((output.width, output.height), (size, size));
        assert_eq!(output.color_type, png::ColorType::Rgba);
        // The native icon service may return low-alpha template artwork to a headless CLI process.
        // This test covers cache integrity; opacity is verified in the actual desktop UI.
        assert!(
            pixels[..output.buffer_size()]
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[3] != 0),
            "cached system icon must contain visible pixels"
        );
    }
    let image = root.join(&entry.icon_key).join("128.png");
    let modified = image.metadata().unwrap().modified().unwrap();
    entry.icon_source = Some(root.join("Missing.app"));
    cache
        .prepare(&entry)
        .expect("a complete cache must not reacquire the system icon");
    assert_eq!(image.metadata().unwrap().modified().unwrap(), modified);

    std::fs::remove_file(root.join(&entry.icon_key).join("64.png")).unwrap();
    assert!(
        cache.prepare(&entry).is_err(),
        "a missing source must retain its failure"
    );
    assert!(cache.cached(&entry.icon_key).is_none());
    assert_eq!(cache.fallback().unwrap().key(), IconCache::fallback_key());

    entry.icon_source = Some(source);
    cache
        .prepare(&entry)
        .expect("an explicit attempt should repair every cache size");
    assert_eq!(cache.cached(&entry.icon_key).unwrap().key(), entry.icon_key);
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
    let mut entry = platform::read_entry(&mut DiscoveryState::new(), &executable, 0)
        .expect("executable should parse")
        .expect("executable should contribute an entry");
    let cache = IconCache::new(&root);
    entry.icon_key = IconCache::key(&entry, &mut DiscoveryState::new()).unwrap();
    let original = entry.clone();
    let icon = cache.prepare(&entry).expect("icon should extract");
    assert_eq!(
        entry, original,
        "cache preparation cannot change application state"
    );
    assert_eq!(icon.key(), entry.icon_key);
    assert_ne!(entry.icon_key, IconCache::fallback_key());
    for size in [32, 64] {
        let bytes = std::fs::read(root.join(&entry.icon_key).join(format!("{size}.png")))
            .expect("cached icon should exist");
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(target_os = "macos")]
#[test]
fn bundle_icon_keys_follow_resources_and_custom_icons_without_requiring_an_icon_file() {
    use std::os::unix::fs::PermissionsExt;

    let root = test_root("bundle-key");
    let bundle = root.join("Sample.app");
    let contents = bundle.join("Contents");
    let resources = contents.join("Resources");
    std::fs::create_dir_all(&resources).unwrap();
    std::fs::create_dir_all(contents.join("MacOS")).unwrap();
    let executable = contents.join("MacOS/Sample");
    std::fs::write(&executable, b"sample executable").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    let info = contents.join("Info.plist");
    std::fs::write(
        &info,
        br#"<?xml version="1.0"?><plist version="1.0"><dict>
        <key>CFBundleExecutable</key><string>Sample</string>
        <key>CFBundleName</key><string>Sample</string>
        <key>CFBundleIconFile</key><string>Artwork</string>
        </dict></plist>"#,
    )
    .unwrap();
    let entry = platform::read_entry(&mut DiscoveryState::new(), &bundle, 0)
        .unwrap()
        .unwrap();
    assert_eq!(entry.icon_source.as_deref(), Some(bundle.as_path()));
    let mut previous = IconCache::key(&entry, &mut DiscoveryState::new()).unwrap();
    assert_ne!(previous, IconCache::fallback_key());
    assert_eq!(
        previous,
        IconCache::key(&entry, &mut DiscoveryState::new()).unwrap()
    );

    for (path, content) in [
        (resources.join("Artwork.icns"), b"original".as_slice()),
        (
            resources.join("Artwork.icns"),
            b"replacement artwork".as_slice(),
        ),
        (
            resources.join("Assets.car"),
            b"compiled asset catalog".as_slice(),
        ),
        (bundle.join("Icon\r"), b"custom icon".as_slice()),
        (executable, b"replacement executable".as_slice()),
    ] {
        std::fs::write(path, content).unwrap();
        let next = IconCache::key(&entry, &mut DiscoveryState::new()).unwrap();
        assert_ne!(
            next, previous,
            "icon inputs must invalidate their cached image"
        );
        previous = next;
    }
    std::fs::remove_file(resources.join("Artwork.icns")).unwrap();
    assert_ne!(
        IconCache::key(&entry, &mut DiscoveryState::new()).unwrap(),
        previous
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fallback_icons_are_valid_png_files() {
    let root = test_root("fallback");
    let cache = IconCache::new(&root);
    let mut entry = platform::read_entry(
        &mut DiscoveryState::new(),
        PathBuf::from("missing").as_path(),
        0,
    )
    .unwrap_or(None)
    .unwrap_or_else(|| {
        crate::ApplicationEntry::new(ApplicationEntryData {
            entry_id: "app.missing".to_owned(),
            source_key: "missing".to_owned(),
            display_name: "Missing".to_owned(),
            normalized_name: "missing".to_owned(),
            normalized_tokens: "missing".to_owned(),
            launch_kind: "executable".to_owned(),
            target_path: "missing".to_owned(),
            arguments_json: ApplicationArguments::empty()
                .to_json()
                .expect("arguments should encode"),
            icon_key: String::new(),
            icon_source: None,
            icon_index: 0,
            priority: 0,
        })
    });
    entry.icon_key = IconCache::key(&entry, &mut DiscoveryState::new()).unwrap();
    let original = entry.clone();
    let icon = cache.prepare(&entry).expect("fallback should prepare");
    assert_eq!(icon.key(), IconCache::fallback_key());
    assert_eq!(entry, original);
    let bytes = std::fs::read(root.join(IconCache::fallback_key()).join("32.png"))
        .expect("fallback icon should exist");
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let mut reader = png::Decoder::new(std::io::Cursor::new(&bytes))
        .read_info()
        .expect("fallback should decode");
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut pixels).unwrap();
    assert_eq!((frame.width, frame.height), (32, 32));
    assert_eq!(pixels[3], 0, "fallback has no background plate");
    assert_eq!(
        pixels[(8 * 32 + 16) * 4 + 3],
        0,
        "document interior is transparent"
    );
    assert_eq!(pixels[16 * 4 + 3], 255, "document reaches the top edge");
    assert_eq!(
        pixels[(31 * 32 + 16) * 4 + 3],
        255,
        "document reaches the bottom edge"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn unavailable_icons_use_the_shared_fallback_until_the_cache_is_complete() {
    let root = test_root("presentation-fallback");
    let cache = IconCache::new(&root);
    assert!(cache.cached("pending-icon").is_none());
    let fallback = cache.fallback().expect("fallback should prepare");
    assert_eq!(fallback.key(), IconCache::fallback_key());
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
    assert_eq!(
        cache
            .cached(key)
            .expect("complete cache should resolve")
            .key(),
        key
    );
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
    assert!(cache.cached(key).is_none());
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

#[cfg(windows)]
#[test]
fn failed_icon_extraction_is_retried_for_the_same_cache_key() {
    let root = test_root("retry");
    let executable = root.join("Invalid.exe");
    std::fs::write(&executable, []).expect("invalid executable should exist");
    let mut entry = crate::ApplicationEntry::new(ApplicationEntryData {
        entry_id: "app.invalid".to_owned(),
        source_key: executable.to_string_lossy().into_owned(),
        display_name: "Invalid".to_owned(),
        normalized_name: "invalid".to_owned(),
        normalized_tokens: "invalid".to_owned(),
        launch_kind: "executable".to_owned(),
        target_path: executable.to_string_lossy().into_owned(),
        arguments_json: ApplicationArguments::empty()
            .to_json()
            .expect("arguments should encode"),
        icon_key: String::new(),
        icon_source: Some(executable),
        icon_index: 0,
        priority: 0,
    });
    let cache = IconCache::new(root.join("icons"));

    entry.icon_key = IconCache::key(&entry, &mut DiscoveryState::new()).unwrap();
    let original = entry.clone();
    assert!(cache.prepare(&entry).is_err());
    assert!(cache.prepare(&entry).is_err());
    assert_eq!(entry, original);
    assert!(cache.cached(&entry.icon_key).is_none());
    assert!(
        root.join("icons")
            .join(&entry.icon_key)
            .join("fallback.marker")
            .is_file()
    );
    assert!(
        !root.join("icons").join(IconCache::fallback_key()).exists(),
        "extraction failures do not write fallback files; presentation belongs to the worker"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn preparation_rejects_missing_discovery_metadata_without_creating_cache_files() {
    let root = test_root("missing-metadata");
    let cache_root = root.join("cache");
    let cache = IconCache::new(&cache_root);
    for key in ["", "missing-source"] {
        let entry = test_entry(key);
        assert!(cache.prepare(&entry).is_err());
    }
    assert!(!cache_root.exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_persisted_keys_cannot_write_outside_the_icon_cache() {
    let root = test_root("invalid-persisted-keys");
    let cache_root = root.join("cache");
    std::fs::create_dir_all(&cache_root).unwrap();
    let outside = root.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let cache = IconCache::new(&cache_root);
    for key in [
        "../outside".to_owned(),
        outside.to_string_lossy().into_owned(),
    ] {
        for size in [32, 64, 128] {
            std::fs::write(outside.join(format!("{size}.png")), b"unrelated data").unwrap();
        }
        std::fs::write(outside.join("fallback.marker"), []).unwrap();
        let mut entry = test_entry(&key);
        entry.icon_source = Some(root.join("Missing.exe"));
        let result = cache.prepare(&entry);
        let preserved = [32, 64, 128].into_iter().all(|size| {
            std::fs::read(outside.join(format!("{size}.png"))).unwrap() == b"unrelated data"
        });
        assert!(result.is_err());
        assert!(
            preserved,
            "an invalid persisted key must not alter sibling files: {key}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
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
    crate::ApplicationEntry::new(ApplicationEntryData {
        entry_id: "app.test".to_owned(),
        source_key: "test".to_owned(),
        display_name: "Test".to_owned(),
        normalized_name: "test".to_owned(),
        normalized_tokens: "test".to_owned(),
        launch_kind: "executable".to_owned(),
        target_path: "test".to_owned(),
        arguments_json: ApplicationArguments::empty()
            .to_json()
            .expect("arguments should encode"),
        icon_key: icon_key.to_owned(),
        icon_source: None,
        icon_index: 0,
        priority: 0,
    })
}
