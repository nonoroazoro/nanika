//! Nanika host entry point.

fn main() {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Nanika")
            .with_inner_size([720.0, 480.0])
            .with_min_inner_size([480.0, 240.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_visible(false),
        ..Default::default()
    };
    let result = eframe::run_native(
        "Nanika",
        options,
        Box::new(|_creation_context| Ok(Box::new(nanika_host::HostApp::new()))),
    );
    if let Err(error) = result {
        eprintln!("Nanika failed to start: {error}");
    }
}
