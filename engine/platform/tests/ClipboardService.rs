use std::time::{Duration, Instant};

use crate::clipboard_service::{ensure_before_deadline, read_validated_png};

#[test]
fn clipboard_images_enforce_encoded_and_decoded_resource_limits() {
    let root = std::env::temp_dir().join(format!(
        "nanika-clipboard-service-limits-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("test root");

    let oversized_file = root.join("oversized.png");
    let file = std::fs::File::create(&oversized_file).expect("oversized image");
    file.set_len(16 * 1024 * 1024 + 1)
        .expect("oversized image length");
    assert!(
        read_validated_png(&oversized_file.to_string_lossy(), &root)
            .expect_err("encoded limit should apply")
            .contains("encoded size")
    );

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
    assert!(
        read_validated_png(&oversized_dimensions.to_string_lossy(), &root)
            .expect_err("dimension limit should apply")
            .contains("dimension")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn expired_clipboard_requests_are_rejected_before_writing() {
    assert!(
        ensure_before_deadline(Instant::now() - Duration::from_millis(1))
            .expect_err("expired request should fail")
            .contains("expired")
    );
}
