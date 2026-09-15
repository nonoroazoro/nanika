use std::path::Path;

use tauri::http::{Method, Request, StatusCode};

use crate::icon_protocol::resolve_request;

const RESOURCE_NAME: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.png";

#[test]
fn image_resources_are_bounded_and_content_addressed() {
    let root = std::env::temp_dir().join(format!(
        "nanika-icon-protocol-resource-{}",
        std::process::id()
    ));
    let cache_root = root.join("cache");
    let payload_root = root.join("payloads");
    let extension_root = payload_root.join("com.nanika.clipboard");
    std::fs::create_dir_all(&extension_root).expect("resource root");
    write_png(&extension_root.join(RESOURCE_NAME));

    let response = resolve_request(
        &cache_root,
        &payload_root,
        "launcher",
        &request(&format!("/com.nanika.clipboard/{RESOURCE_NAME}")),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["Cache-Control"],
        "private, max-age=31536000, immutable"
    );

    let mutable_name = resolve_request(
        &cache_root,
        &payload_root,
        "launcher",
        &request("/com.nanika.clipboard/preview.png"),
    );
    assert_eq!(mutable_name.status(), StatusCode::BAD_REQUEST);

    let oversized_path =
        extension_root.join("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789.png");
    let oversized = std::fs::File::create(&oversized_path).expect("oversized resource");
    oversized
        .set_len((nanika_protocol::MAX_PNG_ENCODED_BYTES + 1) as u64)
        .expect("oversized resource length");
    let response = resolve_request(
        &cache_root,
        &payload_root,
        "launcher",
        &request(&format!(
            "/com.nanika.clipboard/{}",
            oversized_path.file_name().unwrap().to_string_lossy()
        )),
    );
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);

    std::fs::remove_dir_all(root).expect("test root cleanup");
}

fn request(path: &str) -> Request<Vec<u8>> {
    Request::builder()
        .method(Method::GET)
        .uri(path)
        .body(Vec::new())
        .expect("resource request")
}

fn write_png(path: &Path) {
    let file = std::fs::File::create(path).expect("PNG resource");
    let mut encoder = png::Encoder::new(file, 1, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .expect("PNG header")
        .write_image_data(&[0, 0, 0, 0])
        .expect("PNG data");
}
