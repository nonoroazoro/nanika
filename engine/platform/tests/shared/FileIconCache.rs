use crate::{FileIconCache, file_icon_pixels, shell_file_icon_pixels};

#[cfg(target_os = "windows")]
#[test]
fn application_and_clipboard_rows_use_the_same_native_artwork() {
    let source = std::env::current_exe().expect("fixture executable");
    for size in [32, 64, 128] {
        let application = file_icon_pixels(&source, 0, size).expect("application icon");
        let clipboard =
            crate::adapter::file_icon::list_pixels(&source, size).expect("clipboard list icon");
        assert_eq!(application, clipboard);
        assert!(
            application
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[3] != 0)
        );
    }
}

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
        // The native icon service may return a low-alpha template to a headless CLI test process.
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
    assert!(shell_file_icon_pixels(&root, 513).is_err());
    assert!(shell_file_icon_pixels(std::path::Path::new("relative.txt"), 128).is_err());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[test]
fn clipboard_images_use_upright_content_thumbnails() {
    let root = std::env::temp_dir().join(format!("nanika-file-thumbnail-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("fixture root");
    let source = root.join("thumbnail.png");
    let file = std::fs::File::create(&source).expect("thumbnail fixture");
    let mut encoder = png::Encoder::new(file, 8, 8);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header");
    let mut source_pixels = [220, 20, 30, 255].repeat(8 * 4);
    source_pixels.extend([20, 30, 220, 255].repeat(8 * 4));
    writer.write_image_data(&source_pixels).expect("PNG pixels");
    writer.finish().expect("PNG finish");

    let pixels = shell_file_icon_pixels(&source, 128).expect("native image thumbnail");
    let cache_root = root.join("icons");
    let mut cache = FileIconCache::new(cache_root.clone());
    let reference = cache.get(&source).expect("cached content thumbnail");
    let bytes = std::fs::read(cache_root.join(reference.key()).join("512.png"))
        .expect("cached thumbnail PNG");
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .expect("cached PNG header");
    let mut cached_pixels = vec![0; reader.output_buffer_size().expect("output size")];
    reader
        .next_frame(&mut cached_pixels)
        .expect("cached PNG pixels");
    // Sample away from edges and the color boundary to allow native resampling.
    for (pixels, size) in [(&pixels, 128), (&cached_pixels, 512)] {
        for (y, expected) in [
            (size / 4, [220, 20, 30, 255]),
            (size * 3 / 4, [20, 30, 220, 255])
        ] {
            let offset = (y * size + size / 2) * 4;
            for (actual, expected) in pixels[offset..offset + 4].iter().zip(expected) {
                assert!(
                    actual.abs_diff(expected) < 10,
                    "thumbnail row {y}: {:?}",
                    &pixels[offset..offset + 4]
                );
            }
        }
    }

    let list_pixels = crate::file_icon::cached_list_pixels(&source, &[]).expect("list icon");
    assert_eq!(list_pixels.len(), 128 * 128 * 4);
    // A red/blue image must not become a content thumbnail in the list. Its row represents
    // the native file association, while the collection preview represents content.
    let content_color_count = list_pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| {
            pixel[3] > 200
                && pixel[1] < 60
                && ((pixel[0] > 180 && pixel[2] < 70) || (pixel[2] > 180 && pixel[0] < 70))
        })
        .count();
    assert!(content_color_count < 128 * 128 / 2);

    std::fs::remove_dir_all(root).expect("cleanup");
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
#[test]
fn native_4k_file_thumbnail_has_bounded_output() {
    use std::io::Write;

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root")
        .join("target")
        .join(format!("thumbnail-4k-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("fixture root");
    let source = root.join("4k.png");
    let file = std::fs::File::create(&source).expect("fixture file");
    let mut encoder = png::Encoder::new(file, 3840, 2160);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header");
    // Stream fixture rows so the test does not allocate a full-resolution bitmap.
    let row = [220, 20, 30, 255].repeat(3840);
    {
        let mut stream = writer.stream_writer().expect("PNG stream");
        for _ in 0..2160 {
            stream.write_all(&row).expect("PNG row");
        }
        stream.finish().expect("PNG stream finish");
    }
    writer.finish().expect("PNG finish");

    let started = std::time::Instant::now();
    let pixels = shell_file_icon_pixels(&source, 512).expect("4K thumbnail");
    eprintln!(
        "4K thumbnail: {:?}, {} RGBA bytes",
        started.elapsed(),
        pixels.len()
    );
    assert_eq!(pixels.len(), 512 * 512 * 4);
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| { pixel[0] > 180 && pixel[1] < 60 && pixel[2] < 70 && pixel[3] > 200 })
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
