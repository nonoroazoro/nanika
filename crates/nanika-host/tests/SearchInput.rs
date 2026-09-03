use std::cell::Cell;

use crate::{SearchInput, load_system_ui_font};

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn root_input_has_compact_native_font_metrics_and_stable_height() {
    let font = load_system_ui_font().expect("a supported OS must provide system UI fonts");
    let context = egui::Context::default();
    context.set_fonts(font.into_font_definitions());

    let metrics = Cell::new((0.0, 0.0));
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        ui.set_width(640.0);
        let row_height = ui.fonts_mut(|fonts| fonts.row_height(&egui::FontId::proportional(24.0)));
        let mut text = "1+112".to_owned();
        let response =
            SearchInput::root(ui, egui::Id::new("search-input-test"), &mut text, "Search");
        metrics.set((row_height, response.rect.height()));
    });
    output.drop_without_applying_deltas();
    let (row_height, input_height) = metrics.get();

    assert!(row_height <= 34.0, "unexpected row height: {row_height}");
    assert!(
        (62.0..=66.0).contains(&input_height),
        "unexpected input height: {input_height}"
    );
}
