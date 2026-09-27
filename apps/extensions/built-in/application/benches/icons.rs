//! Opt-in native extraction comparison. Inputs and outputs stay outside application state.
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() {
    let Ok(sources) = std::env::var("NANIKA_ICON_BENCH_SOURCES") else {
        println!(
            "Set NANIKA_ICON_BENCH_SOURCES (JSON paths), NANIKA_ICON_BENCH_OUTPUT, and NANIKA_ICON_BENCH_MODE (separate, single128, single256)."
        );
        return;
    };
    let sources: Vec<PathBuf> = serde_json::from_str(&sources).expect("source paths");
    let output =
        PathBuf::from(std::env::var_os("NANIKA_ICON_BENCH_OUTPUT").expect("output directory"));
    let mode = std::env::var("NANIKA_ICON_BENCH_MODE").expect("comparison mode");
    assert!(matches!(
        mode.as_str(),
        "separate" | "single128" | "single256"
    ));
    std::fs::create_dir_all(&output).unwrap();
    let handshake = std::env::var_os("NANIKA_ICON_BENCH_HANDSHAKE").is_some();
    if handshake {
        _signal("ready");
    }
    let mut durations = Vec::new();
    for round in 0..10 {
        let start = Instant::now();
        for (index, source) in sources.iter().enumerate() {
            let directory = output.join(format!("{round}-{index}"));
            std::fs::create_dir_all(&directory).unwrap();
            if mode == "separate" {
                for size in [32, 64, 128] {
                    assert!(source.metadata().unwrap().len() > 0);
                    let pixels = nanika_platform::file_icon_pixels(source, 0, size).unwrap();
                    _write(&directory, size, size, &pixels);
                }
            } else {
                let size = if mode == "single128" { 128 } else { 256 };
                assert!(source.metadata().unwrap().len() > 0);
                let pixels = nanika_platform::file_icon_pixels(source, 0, size).unwrap();
                for target in [32, 64, 128] {
                    _write(&directory, size, target, &pixels);
                }
            }
        }
        durations.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    println!(
        "{}",
        serde_json::json!({"mode": mode, "sources": sources.len(), "round_ms": durations})
    );
    if handshake {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
    }
}

fn _write(directory: &Path, source_size: u32, size: u32, pixels: &[u8]) {
    let pixels =
        nanika_platform::normalize_icon_rgba(pixels, source_size, source_size, size).unwrap();
    let target = directory.join(format!("{size}.png"));
    let temporary = target.with_extension("png.tmp");
    let file = std::fs::File::create(&temporary).unwrap();
    let mut encoder = png::Encoder::new(file, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
    std::fs::rename(temporary, target).unwrap();
}

fn _signal(message: &str) {
    println!("{message}");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
}
