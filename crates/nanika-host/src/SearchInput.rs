use egui::{FontId, Id, Margin, Response, TextEdit, Ui};

use crate::{DesignSystem, SearchInputStyle};

/// Shared single-line search input with stable typography and dimensions.
pub(crate) struct SearchInput;

impl SearchInput {
    pub(crate) fn root(ui: &mut Ui, id: Id, text: &mut String, placeholder: &str) -> Response {
        let style = SearchInputStyle {
            font_size: DesignSystem::ROOT_INPUT_FONT_SIZE,
            height: DesignSystem::ROOT_INPUT_HEIGHT,
            horizontal_margin: DesignSystem::ROOT_INPUT_HORIZONTAL_MARGIN,
            framed: false,
        };
        Self::show(ui, id, text, placeholder, &style)
    }

    pub(crate) fn compact(ui: &mut Ui, id: Id, text: &mut String, placeholder: &str) -> Response {
        let style = SearchInputStyle {
            font_size: DesignSystem::COMPACT_INPUT_FONT_SIZE,
            height: DesignSystem::COMPACT_INPUT_HEIGHT,
            horizontal_margin: DesignSystem::COMPACT_INPUT_HORIZONTAL_MARGIN,
            framed: true,
        };
        Self::show(ui, id, text, placeholder, &style)
    }

    fn show(
        ui: &mut Ui,
        id: Id,
        text: &mut String,
        placeholder: &str,
        style: &SearchInputStyle,
    ) -> Response {
        let font = FontId::proportional(style.font_size);
        let row_height = ui.fonts_mut(|fonts| fonts.row_height(&font));
        let vertical_margin = ((style.height - row_height) * 0.5)
            .round()
            .clamp(0.0, f32::from(i8::MAX)) as i8;
        let mut input = TextEdit::singleline(text)
            .id(id)
            .hint_text(placeholder)
            .font(font)
            .desired_width(f32::INFINITY)
            .margin(Margin::symmetric(style.horizontal_margin, vertical_margin))
            .vertical_align(egui::Align::Center);
        if !style.framed {
            input = input
                .frame(
                    egui::Frame::new()
                        .inner_margin(Margin::symmetric(style.horizontal_margin, vertical_margin)),
                )
                .background_color(egui::Color32::TRANSPARENT);
        }
        ui.add(input)
    }
}
