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
