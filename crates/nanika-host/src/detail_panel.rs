use nanika_protocol::DetailView;

use crate::DesignSystem;

pub(crate) fn detail_panel(
    ui: &mut egui::Ui,
    detail: &DetailView,
    scroll_id: egui::Id,
    max_height: f32,
) {
    let palette = DesignSystem::palette(ui);
    egui::Frame::new()
        .fill(palette.surface_detail)
        .stroke(egui::Stroke::new(1.0, palette.border_detail))
        .corner_radius(egui::CornerRadius::same(DesignSystem::DETAIL_RADIUS))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt(scroll_id)
                .auto_shrink([false, false])
                .max_height((max_height - 28.0).max(80.0))
                .show(ui, |ui| {
                    if let Some(title) = &detail.title {
                        ui.label(
                            egui::RichText::new(title)
                                .size(DesignSystem::FONT_DETAIL_TITLE)
                                .strong()
                                .color(palette.text_heading),
                        );
                        ui.add_space(10.0);
                    }
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&detail.body).color(palette.text_detail),
                        )
                        .selectable(true)
                        .wrap(),
                    );
                    if !detail.metadata.is_empty() {
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);
                        for metadata in &detail.metadata {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(&metadata.title)
                                        .color(palette.text_secondary),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(&metadata.value)
                                                .color(palette.text_primary),
                                        );
                                    },
                                );
                            });
                        }
                    }
                });
        });
}

pub(crate) fn empty_detail_panel(ui: &mut egui::Ui, height: f32) {
    let palette = DesignSystem::palette(ui);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(DesignSystem::DETAIL_RADIUS),
        palette.surface_detail,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Select an item to preview",
        egui::TextStyle::Body.resolve(ui.style()),
        palette.text_secondary,
    );
}
