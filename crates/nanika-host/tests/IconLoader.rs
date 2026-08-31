use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use crate::{IconIdentity, IconLoader};

#[test]
fn loader_decodes_an_extension_scoped_rgba_icon() {
    let root = std::env::temp_dir().join(format!("nanika-icon-loader-{}", std::process::id()));
    let icon_path = root
        .join("icons")
        .join("com.nanika.application")
        .join("test-icon")
        .join("32.png");
    std::fs::create_dir_all(icon_path.parent().expect("icon parent"))
        .expect("icon directory should be created");
    write_icon(&icon_path);
    let (wake_sender, wake_receiver) = mpsc::sync_channel(1);
    let loader = IconLoader::spawn(
        &root,
        Arc::new(move || {
            let _ = wake_sender.try_send(());
        }),
    )
    .expect("icon loader should start");

    loader
        .request(IconIdentity::new("com.nanika.application", "test-icon"))
        .expect("icon request should enqueue");
    wake_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("icon result should wake the host");
    let result = loader
        .take_results()
        .pop()
        .expect("icon result should be available");
    let image = result.image.expect("icon should decode");

    assert_eq!(image.size, [2, 2]);
    assert_eq!(image.pixels.len(), 4);
    drop(loader);
    std::fs::remove_dir_all(root).expect("test root should be removable");
}

fn write_icon(path: &std::path::Path) {
    let file = std::fs::File::create(path).expect("icon should be created");
    let mut encoder = png::Encoder::new(file, 2, 2);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header should write");
    writer
        .write_image_data(&[
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ])
        .expect("PNG pixels should write");
    writer.finish().expect("PNG should finish");
}
