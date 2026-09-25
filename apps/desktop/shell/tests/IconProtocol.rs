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
        &Default::default(),
        "launcher",
        &request(&format!("/com.nanika.clipboard/payload/{RESOURCE_NAME}")),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["Cache-Control"],
        "private, max-age=31536000, immutable"
    );

    let mutable_name = resolve_request(
        &cache_root,
        &payload_root,
        &Default::default(),
        "launcher",
        &request("/com.nanika.clipboard/payload/preview.png"),
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
        &Default::default(),
        "launcher",
        &request(&format!(
            "/com.nanika.clipboard/payload/{}",
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

#[test]
fn cached_file_icons_support_large_previews_and_reject_arbitrary_sizes() {
    let root =
        std::env::temp_dir().join(format!("nanika-file-icon-protocol-{}", std::process::id()));
    let directory = root.join("icons/com.nanika.clipboard/file-icon");
    std::fs::create_dir_all(&directory).expect("icon directory");
    write_png(&directory.join("512.png"));
    let response = resolve_request(
        &root,
        &root.join("payloads"),
        &Default::default(),
        "launcher",
        &request("/com.nanika.clipboard/cache/file-icon/512.png"),
    );
    assert_eq!(response.status(), StatusCode::OK);
    let response = resolve_request(
        &root,
        &root.join("payloads"),
        &Default::default(),
        "launcher",
        &request("/com.nanika.clipboard/cache/file-icon/1024.png"),
    );
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn package_icons_are_scoped_shared_with_settings_and_fail_independently() {
    let root = std::env::temp_dir().join(format!("nanika-package-icons-{}", std::process::id()));
    let package = root.join("package");
    std::fs::create_dir_all(package.join("assets")).unwrap();
    write_png(&package.join("assets/icon.png"));
    std::fs::write(package.join("assets/broken.png"), "not a PNG").unwrap();
    let packages = std::collections::HashMap::from([("example.tools".to_owned(), package)]);
    for surface in ["launcher", "settings"] {
        for (path, status) in [
            (
                "/example.tools/package/assets/missing.png",
                StatusCode::NOT_FOUND,
            ),
            (
                "/example.tools/package/assets/broken.png",
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                "/example.other/package/assets/icon.png",
                StatusCode::NOT_FOUND,
            ),
            (
                "/example.tools/package/../outside.png",
                StatusCode::BAD_REQUEST,
            ),
            (
                "/example.tools/package/%2e%2e/outside.png",
                StatusCode::BAD_REQUEST,
            ),
            ("/example.tools/package/assets/icon.png", StatusCode::OK),
        ] {
            let response = resolve_request(&root, &root, &packages, surface, &request(path));
            assert_eq!(response.status(), status, "{surface}: {path}");
            assert_eq!(response.headers()["Cache-Control"], "no-store");
            if status == StatusCode::OK {
                assert_eq!(response.headers()["Content-Type"], "image/png");
            }
        }
    }
    for path in [
        "/example.tools/cache/file/128.png",
        &format!("/example.tools/payload/{RESOURCE_NAME}"),
    ] {
        assert_eq!(
            resolve_request(&root, &root, &packages, "settings", &request(path)).status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        resolve_request(
            &root,
            &root,
            &packages,
            "unknown",
            &request("/example.tools/package/assets/icon.png")
        )
        .status(),
        StatusCode::FORBIDDEN
    );
    std::fs::remove_dir_all(root).unwrap();
}
