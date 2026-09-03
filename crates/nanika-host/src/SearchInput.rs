use egui::{FontId, Id, Margin, Response, TextEdit, Ui};

/// Shared single-line search input with stable typography and dimensions.
pub(crate) struct SearchInput;

impl SearchInput {
    const ROOT_FONT_SIZE: f32 = 24.0;
    const ROOT_HEIGHT: f32 = 64.0;
    const COMPACT_FONT_SIZE: f32 = 17.0;
    const COMPACT_HEIGHT: f32 = 44.0;

    pub(crate) fn root(ui: &mut Ui, id: Id, text: &mut String, placeholder: &str) -> Response {
        Self::show(
            ui,
            id,
            text,
            placeholder,
            Self::ROOT_FONT_SIZE,
            Self::ROOT_HEIGHT,
            16,
        )
    }

    pub(crate) fn compact(ui: &mut Ui, id: Id, text: &mut String, placeholder: &str) -> Response {
        Self::show(
            ui,
            id,
            text,
            placeholder,
            Self::COMPACT_FONT_SIZE,
            Self::COMPACT_HEIGHT,
            12,
        )
    }

    fn show(
        ui: &mut Ui,
        id: Id,
        text: &mut String,
        placeholder: &str,
        font_size: f32,
        height: f32,
        horizontal_margin: i8,
    ) -> Response {
        let font = FontId::proportional(font_size);
        let row_height = ui.fonts_mut(|fonts| fonts.row_height(&font));
        let vertical_margin = ((height - row_height) * 0.5)
            .round()
            .clamp(0.0, f32::from(i8::MAX)) as i8;
        ui.add(
            TextEdit::singleline(text)
                .id(id)
                .hint_text(placeholder)
                .font(font)
                .desired_width(f32::INFINITY)
                .margin(Margin::symmetric(horizontal_margin, vertical_margin))
                .vertical_align(egui::Align::Center),
        )
    }
}
