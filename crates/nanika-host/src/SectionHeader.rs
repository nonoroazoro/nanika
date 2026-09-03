use egui::{Align, Layout, RichText, Ui};

use crate::DesignSystem;

/// Shared compact section label for result collections.
pub(crate) struct SectionHeader;

impl SectionHeader {
    pub(crate) fn show(ui: &mut Ui, title: &str, horizontal_inset: f32) {
        let palette = DesignSystem::palette(ui);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), DesignSystem::ROOT_SECTION_HEIGHT),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.add_space(horizontal_inset);
                ui.label(
                    RichText::new(title)
                        .size(DesignSystem::FONT_SECTION)
                        .strong()
                        .color(palette.text_secondary),
                );
            },
        );
    }
}
