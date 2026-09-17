use crate::{FileIconCache, file_icon_pixels};

#[test]
fn native_file_icons_are_cached_at_both_sizes_without_rewriting_complete_entries() {
    let root = std::env::temp_dir().join(format!("nanika-file-icons-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("fixture root");
    // The running executable has a concrete native icon on both supported platforms.
    let source = std::env::current_exe().expect("fixture executable");
    let cache_root = root.join("icons");
    let mut cache = FileIconCache::new(cache_root.clone());
    assert_eq!(cache.cached(&source).expect("empty cache lookup"), None);
    let started = std::time::Instant::now();
    let reference = cache.get(&source).expect("system file icon");
    eprintln!("native file icon cold cache: {:?}", started.elapsed());
    for size in [128, 512] {
        let bytes = std::fs::read(cache_root.join(reference.key()).join(format!("{size}.png")))
            .expect("cached PNG");
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .expect("PNG header");
        assert_eq!((reader.info().width, reader.info().height), (size, size));
        let mut pixels = vec![0; reader.output_buffer_size().expect("output size")];
        reader.next_frame(&mut pixels).expect("PNG frame");
        // AppKit may return a low-alpha template icon to a headless CLI test process.
        // This cache test verifies valid non-empty native output; rendered opacity is UI-tested.
        assert!(pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] != 0));
    }
    let large = cache_root.join(reference.key()).join("512.png");
    let modified = large
        .metadata()
        .expect("cached metadata")
        .modified()
        .expect("mtime");
    let started = std::time::Instant::now();
    assert_eq!(cache.get(&source).expect("warm icon"), reference);
    assert_eq!(
        cache.cached(&source).expect("cached icon lookup"),
        Some(reference.clone())
    );
    eprintln!("native file icon warm cache: {:?}", started.elapsed());
    assert_eq!(
        large
            .metadata()
            .expect("cached metadata")
            .modified()
            .expect("mtime"),
        modified
    );
    let mut restarted = FileIconCache::new(cache_root.clone());
    assert_eq!(
        restarted.cached(&source).expect("persistent cache lookup"),
        Some(reference.clone())
    );
    assert_eq!(restarted.get(&source).expect("persistent icon"), reference);
    std::fs::remove_file(&large).expect("incomplete cache fixture");
    assert_eq!(
        cache.cached(&source).expect("incomplete cache lookup"),
        None
    );
    assert_eq!(cache.get(&source).expect("repair missing image"), reference);
    assert!(large.is_file());
    let missing = root.join("missing-file");
    assert_eq!(
        cache
            .get(&missing)
            .expect_err("missing files retain concrete failure")
            .kind(),
        std::io::ErrorKind::NotFound
    );
    assert!(file_icon_pixels(&root, 0, 513).is_err());
    assert!(file_icon_pixels(std::path::Path::new("relative.txt"), 0, 128).is_err());
    std::fs::remove_dir_all(root).expect("cleanup");
}
