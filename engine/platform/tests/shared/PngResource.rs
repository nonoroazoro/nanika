use crate::{PngResourceError, read_png_resource};

#[test]
fn png_resources_enforce_encoded_and_decoded_limits() {
    let root =
        std::env::temp_dir().join(format!("nanika-png-resource-limits-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("test root");

    let oversized_file = root.join("oversized.png");
    let file = std::fs::File::create(&oversized_file).expect("oversized image");
    file.set_len(16 * 1024 * 1024 + 1)
        .expect("oversized image length");
    assert!(matches!(
        read_png_resource(&oversized_file, &root),
        Err(PngResourceError::EncodedSize)
    ));

    let oversized_dimensions = root.join("dimensions.png");
    let file = std::fs::File::create(&oversized_dimensions).expect("dimension image");
    let mut encoder = png::Encoder::new(file, 8_193, 1);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .expect("PNG header")
        .write_image_data(&vec![0; 8_193])
        .expect("PNG data");
    assert!(matches!(
        read_png_resource(&oversized_dimensions, &root),
        Err(PngResourceError::Dimensions { .. })
    ));

    assert!(matches!(
        read_png_resource(&root.join("missing.png"), &root),
        Err(PngResourceError::NotFound)
    ));

    let outside_root = root.with_extension("outside.png");
    write_png(&outside_root, 1, 1);
    assert!(matches!(
        read_png_resource(&outside_root, &root),
        Err(PngResourceError::OutsideRoot)
    ));

    let invalid = root.join("invalid.png");
    std::fs::write(&invalid, b"not a PNG").expect("invalid image");
    assert!(matches!(
        read_png_resource(&invalid, &root),
        Err(PngResourceError::Decode(_))
    ));

    let _ = std::fs::remove_file(outside_root);
    let _ = std::fs::remove_dir_all(root);
}

fn write_png(path: &std::path::Path, width: u32, height: u32) {
    let file = std::fs::File::create(path).expect("PNG resource");
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .expect("PNG header")
        .write_image_data(&vec![0; width as usize * height as usize * 4])
        .expect("PNG data");
}

#[test]
fn prepared_clipboard_input_survives_producer_removal_and_releases_its_budget() {
    let root =
        std::env::temp_dir().join(format!("nanika-prepared-clipboard-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("image.png");
    write_png(&path, 2, 2);
    let expected = std::fs::read(&path).unwrap();
    let service = crate::ClipboardService::spawn().unwrap();
    let prepared = service
        .prepare(
            nanika_protocol::ClipboardContent::PngFile {
                path: path.to_string_lossy().into()
            },
            Some(&root),
            &mut || false
        )
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    let crate::PreparedClipboardContent::Png(bytes) = &prepared.content else {
        panic!("image input")
    };
    assert_eq!(bytes, &expected);
    // A second image waits for bounded memory, but termination before admission
    // must remain observable even while the first prepared input is retained.
    assert!(
        service
            .prepare(
                nanika_protocol::ClipboardContent::PngFile {
                    path: path.to_string_lossy().into()
                },
                Some(&root),
                &mut || true
            )
            .is_err()
    );
    drop(prepared);
    write_png(&path, 1, 1);
    drop(
        service
            .prepare(
                nanika_protocol::ClipboardContent::PngFile {
                    path: path.to_string_lossy().into()
                },
                Some(&root),
                &mut || false
            )
            .unwrap()
    );
    drop(service);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn clipboard_rejects_truncated_pixels_and_terminal_chunks_before_native_admission() {
    let root = std::env::temp_dir().join(format!("nanika-png-truncated-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("image.png");
    write_png(&path, 2, 2);
    let bytes = std::fs::read(&path).unwrap();
    let service = crate::ClipboardService::spawn().unwrap();
    for end in 0..bytes.len() {
        std::fs::write(&path, &bytes[..end]).unwrap();
        assert!(
            service
                .prepare(
                    nanika_protocol::ClipboardContent::PngFile {
                        path: path.to_string_lossy().into()
                    },
                    Some(&root),
                    &mut || false,
                )
                .is_err(),
            "accepted a truncated PNG at {end}/{} bytes",
            bytes.len()
        );
    }
    // Failed preparation releases its image credit; a complete image is still accepted.
    std::fs::write(&path, &bytes).unwrap();
    let prepared = service
        .prepare(
            nanika_protocol::ClipboardContent::PngFile {
                path: path.to_string_lossy().into()
            },
            Some(&root),
            &mut || false
        )
        .unwrap();
    assert!(
        matches!(&prepared.content, crate::PreparedClipboardContent::Png(actual) if actual == &bytes)
    );
    drop(prepared);
    drop(service);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn png_resources_reject_animation_before_pixel_decoding() {
    let root = std::env::temp_dir().join(format!("nanika-static-png-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("image.png");
    for separate_default in [false, true] {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_animated(2, 0).unwrap();
            encoder.set_sep_def_img(separate_default).unwrap();
            let mut writer = encoder.write_header().unwrap();
            for _ in 0..(2 + usize::from(separate_default)) {
                writer.write_image_data(&[128; 16]).unwrap();
            }
            writer.finish().unwrap();
        }
        std::fs::write(&path, &bytes).unwrap();
        assert!(matches!(
            read_png_resource(&path, &root),
            Err(PngResourceError::Animation)
        ));
        assert!(matches!(
            crate::validate_png_pixels(&bytes, &mut || false),
            Err(PngResourceError::Animation)
        ));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cancellation_interrupts_png_decoding_and_releases_image_admission() {
    let root = std::env::temp_dir().join(format!("nanika-cancel-png-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("image.png");
    write_png(&path, 128, 128);
    let bytes = std::fs::read(&path).unwrap();
    let mut checks = 0;
    assert!(matches!(
        crate::validate_png_pixels(&bytes, &mut || {
            checks += 1;
            checks == 4
        }),
        Err(PngResourceError::Cancelled)
    ));
    assert_eq!(checks, 4);
    let service = crate::ClipboardService::spawn().unwrap();
    let content = nanika_protocol::ClipboardContent::PngFile {
        path: path.to_string_lossy().into()
    };
    let mut checks = 0;
    let error = service
        .prepare(content.clone(), Some(&root), &mut || {
            checks += 1;
            checks == 4
        })
        .err()
        .expect("cancelled PNG preparation");
    assert!(error.contains("cancelled"));
    drop(
        service
            .prepare(content, Some(&root), &mut || false)
            .unwrap()
    );
    drop(service);
    std::fs::remove_dir_all(root).unwrap();
}
