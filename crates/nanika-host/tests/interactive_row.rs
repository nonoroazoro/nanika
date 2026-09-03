use std::cell::Cell;

use crate::{DesignSystem, list_row, root_result_row};

#[test]
fn row_variants_keep_their_design_system_heights() {
    let context = egui::Context::default();
    DesignSystem::install(&context);
    let heights = Cell::new((0.0, 0.0));

    let output = context.run_ui(egui::RawInput::default(), |ui| {
        ui.set_width(640.0);
        let root = root_result_row(ui, None, "Application", Some("Command"), true);
        let list = list_row(ui, "Clipboard item", Some("Text"), false, true);
        heights.set((root.rect.height(), list.rect.height()));
    });
    output.drop_without_applying_deltas();

    assert_eq!(heights.get().0, DesignSystem::ROOT_RESULT_ROW_HEIGHT);
    assert_eq!(heights.get().1, DesignSystem::VIEW_ROW_HEIGHT);
}
