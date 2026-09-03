use crate::{DesignSystem, InteractiveRowContent, InteractiveRowStyle};

pub(crate) fn root_result_row(
    ui: &mut egui::Ui,
    texture_id: Option<egui::TextureId>,
    title: &str,
    accessory: Option<&str>,
    selected: bool,
) -> egui::Response {
    let style = InteractiveRowStyle {
        height: DesignSystem::ROOT_RESULT_ROW_HEIGHT,
        content_inset: 12.0,
        outer_inset: 8.0,
    };
    let content = InteractiveRowContent {
        texture_id,
        title,
        subtitle: None,
        accessory,
    };
    interactive_row(ui, &style, &content, selected, true)
}

pub(crate) fn list_row(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: Option<&str>,
    selected: bool,
    enabled: bool,
) -> egui::Response {
    let style = InteractiveRowStyle {
        height: DesignSystem::VIEW_ROW_HEIGHT,
        content_inset: 12.0,
        outer_inset: 0.0,
    };
    let content = InteractiveRowContent {
        texture_id: None,
        title,
        subtitle,
        accessory: None,
    };
    interactive_row(ui, &style, &content, selected, enabled)
}

fn interactive_row(
    ui: &mut egui::Ui,
    style: &InteractiveRowStyle,
    content: &InteractiveRowContent<'_>,
    selected: bool,
    enabled: bool,
) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), style.height), sense);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, content.title)
    });
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let palette = DesignSystem::palette(ui);
    let painter = ui.painter_at(rect);
    let visual_rect = rect.shrink2(egui::vec2(style.outer_inset, 2.0));
    painter.rect_filled(
        visual_rect,
        egui::CornerRadius::same(DesignSystem::ROW_RADIUS),
        DesignSystem::interactive_row_fill(
            palette,
            selected,
            response.hovered() && enabled,
            response.is_pointer_button_down_on() && enabled,
        ),
    );

    let content_left = visual_rect.left() + style.content_inset;
    let text_left = if let Some(texture_id) = content.texture_id {
        let icon_size = DesignSystem::ROOT_RESULT_ICON_SIZE;
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(content_left, visual_rect.center().y - icon_size * 0.5),
            egui::vec2(icon_size, icon_size),
        );
        painter.image(
            texture_id,
            icon_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        icon_rect.right() + 8.0
    } else {
        content_left
    };

    let title_color = if !enabled {
        palette.text_disabled
    } else if selected && content.subtitle.is_none() {
        palette.text_selected
    } else {
        palette.text_row
    };
    let title_position = if content.subtitle.is_some() {
        egui::pos2(text_left, visual_rect.top() + 15.0)
    } else {
        egui::pos2(text_left, visual_rect.center().y)
    };
    let title_clip_right = if let Some(accessory) = content.accessory {
        let accessory_font = egui::FontId::proportional(13.0);
        let accessory_galley =
            painter.layout_no_wrap(accessory.to_owned(), accessory_font, palette.text_secondary);
        let accessory_position = egui::pos2(
            visual_rect.right() - style.content_inset - accessory_galley.size().x,
            visual_rect.center().y - accessory_galley.size().y * 0.5,
        );
        painter.galley(accessory_position, accessory_galley, palette.text_secondary);
        accessory_position.x - 12.0
    } else {
        visual_rect.right() - style.content_inset
    };
    let title_painter = painter.with_clip_rect(egui::Rect::from_min_max(
        egui::pos2(text_left, visual_rect.top()),
        egui::pos2(title_clip_right.max(text_left), visual_rect.bottom()),
    ));
    title_painter.text(
        title_position,
        egui::Align2::LEFT_CENTER,
        content.title,
        egui::FontId::proportional(DesignSystem::FONT_ROW_TITLE),
        title_color,
    );
    if let Some(subtitle) = content.subtitle {
        painter.text(
            egui::pos2(text_left, visual_rect.bottom() - 9.0),
            egui::Align2::LEFT_BOTTOM,
            subtitle,
            egui::FontId::proportional(DesignSystem::FONT_ROW_SUBTITLE),
            palette.text_secondary,
        );
    }
    response
}
