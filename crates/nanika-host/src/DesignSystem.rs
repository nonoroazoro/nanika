use egui::{Color32, Context, Stroke, Theme, ThemePreference, Ui};

use crate::DesignPalette;

/// Host-owned visual tokens and global egui style configuration.
pub(crate) struct DesignSystem;

impl DesignSystem {
    pub(crate) const ROOT_INPUT_FONT_SIZE: f32 = 24.0;
    pub(crate) const ROOT_INPUT_HEIGHT: f32 = 60.0;
    pub(crate) const ROOT_INPUT_HORIZONTAL_MARGIN: i8 = 20;
    pub(crate) const COMPACT_INPUT_FONT_SIZE: f32 = 16.0;
    pub(crate) const COMPACT_INPUT_HEIGHT: f32 = 40.0;
    pub(crate) const COMPACT_INPUT_HORIZONTAL_MARGIN: i8 = 12;
    pub(crate) const BACK_BUTTON_WIDTH: f32 = 68.0;
    pub(crate) const BACK_BUTTON_HEIGHT: f32 = 32.0;
    pub(crate) const ROOT_SECTION_HEIGHT: f32 = 32.0;
    pub(crate) const ROOT_CHROME_HEIGHT: f32 =
        Self::ROOT_INPUT_HEIGHT + Self::ROOT_SECTION_HEIGHT + 1.0;
    pub(crate) const ROOT_VISIBLE_ROWS: usize = 8;
    pub(crate) const ACTION_BAR_HEIGHT: f32 = 40.0;
    pub(crate) const VIEW_INSET: i8 = 16;

    pub(crate) const FONT_BACK: f32 = 14.0;
    pub(crate) const FONT_ROUTE_TITLE: f32 = 16.0;
    pub(crate) const FONT_DETAIL_TITLE: f32 = 15.0;
    pub(crate) const FONT_SECTION: f32 = 12.0;
    pub(crate) const FONT_ROW_TITLE: f32 = 16.0;
    pub(crate) const FONT_ROW_SUBTITLE: f32 = 12.0;

    pub(crate) const ROOT_RESULT_ROW_HEIGHT: f32 = 48.0;
    pub(crate) const VIEW_ROW_HEIGHT: f32 = 48.0;
    pub(crate) const ROOT_RESULT_ICON_SIZE: f32 = 28.0;
    pub(crate) const ROW_RADIUS: u8 = 8;
    pub(crate) const CONTROL_RADIUS: u8 = 7;
    pub(crate) const DETAIL_RADIUS: u8 = 10;

    const DARK: DesignPalette = DesignPalette {
        text_primary: Color32::from_rgb(242, 242, 247),
        text_row: Color32::from_rgb(235, 235, 240),
        text_selected: Color32::WHITE,
        text_heading: Color32::from_rgb(245, 245, 247),
        text_detail: Color32::from_rgb(210, 210, 216),
        text_muted: Color32::from_rgb(151, 151, 160),
        text_secondary: Color32::from_rgb(142, 142, 151),
        text_disabled: Color32::from_rgb(112, 112, 120),
        text_error: Color32::from_rgb(255, 105, 97),
        text_destructive: Color32::from_rgb(255, 190, 188),
        surface_root: Color32::from_rgb(28, 28, 32),
        surface_input: Color32::from_rgb(36, 36, 41),
        surface_detail: Color32::from_rgb(32, 32, 37),
        surface_control: Color32::from_rgb(44, 44, 50),
        surface_row_hovered: Color32::from_rgb(40, 40, 45),
        surface_row_selected: Color32::from_rgb(51, 51, 57),
        surface_row_active: Color32::from_rgb(58, 58, 65),
        surface_action_primary: Color32::from_rgb(58, 58, 65),
        surface_action_destructive: Color32::from_rgb(91, 42, 43),
        border_control: Color32::from_rgb(62, 62, 69),
        border_hovered: Color32::from_rgb(78, 78, 86),
        border_active: Color32::from_rgb(92, 92, 101),
        border_detail: Color32::from_rgb(52, 52, 58),
        accent_strong: Color32::from_rgb(64, 156, 255),
        selection_fill: Color32::from_rgb(30, 70, 113),
    };

    const LIGHT: DesignPalette = DesignPalette {
        text_primary: Color32::from_rgb(28, 28, 30),
        text_row: Color32::from_rgb(32, 32, 35),
        text_selected: Color32::from_rgb(18, 18, 20),
        text_heading: Color32::from_rgb(34, 34, 37),
        text_detail: Color32::from_rgb(62, 62, 67),
        text_muted: Color32::from_rgb(99, 99, 105),
        text_secondary: Color32::from_rgb(112, 112, 118),
        text_disabled: Color32::from_rgb(150, 150, 156),
        text_error: Color32::from_rgb(196, 43, 28),
        text_destructive: Color32::from_rgb(168, 34, 26),
        surface_root: Color32::from_rgb(242, 242, 244),
        surface_input: Color32::from_rgb(232, 232, 235),
        surface_detail: Color32::from_rgb(248, 248, 249),
        surface_control: Color32::from_rgb(226, 226, 230),
        surface_row_hovered: Color32::from_rgb(232, 232, 235),
        surface_row_selected: Color32::from_rgb(218, 218, 222),
        surface_row_active: Color32::from_rgb(208, 208, 213),
        surface_action_primary: Color32::from_rgb(218, 218, 223),
        surface_action_destructive: Color32::from_rgb(250, 221, 218),
        border_control: Color32::from_rgb(207, 207, 212),
        border_hovered: Color32::from_rgb(188, 188, 194),
        border_active: Color32::from_rgb(170, 170, 177),
        border_detail: Color32::from_rgb(218, 218, 222),
        accent_strong: Color32::from_rgb(0, 122, 255),
        selection_fill: Color32::from_rgb(181, 215, 252),
    };

    pub(crate) fn install(context: &Context) {
        context.set_theme(ThemePreference::System);
        for theme in [Theme::Dark, Theme::Light] {
            let palette = Self::palette_for_theme(theme);
            let mut style = (*context.style_of(theme)).clone();
            style.spacing.item_spacing = egui::vec2(8.0, 6.0);
            style.spacing.button_padding = egui::vec2(10.0, 6.0);
            style.visuals.override_text_color = Some(palette.text_primary);
            style.visuals.weak_text_color = Some(palette.text_muted);
            style.visuals.extreme_bg_color = palette.surface_input;
            style.visuals.text_edit_bg_color = Some(palette.surface_input);
            style.visuals.panel_fill = palette.surface_root;
            style.visuals.window_fill = palette.surface_root;
            style.visuals.widgets.inactive.bg_fill = palette.surface_control;
            style.visuals.widgets.inactive.weak_bg_fill = palette.surface_control;
            style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border_control);
            style.visuals.widgets.hovered.bg_fill = palette.surface_row_selected;
            style.visuals.widgets.hovered.weak_bg_fill = palette.surface_row_selected;
            style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.border_hovered);
            style.visuals.widgets.active.bg_fill = palette.surface_row_active;
            style.visuals.widgets.active.weak_bg_fill = palette.surface_row_active;
            style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.border_active);
            style.visuals.widgets.open = style.visuals.widgets.active;
            style.visuals.selection.bg_fill = palette.selection_fill;
            style.visuals.selection.stroke = Stroke::new(1.0, palette.accent_strong);
            context.set_style_of(theme, style);
        }
    }

    pub(crate) fn palette(ui: &Ui) -> DesignPalette {
        if ui.visuals().dark_mode {
            Self::DARK
        } else {
            Self::LIGHT
        }
    }

    pub(crate) fn palette_for_theme(theme: Theme) -> DesignPalette {
        match theme {
            Theme::Dark => Self::DARK,
            Theme::Light => Self::LIGHT,
        }
    }

    pub(crate) fn root_surface(alpha: u8, theme: Theme) -> Color32 {
        let surface = Self::palette_for_theme(theme).surface_root;
        Color32::from_rgba_unmultiplied(surface.r(), surface.g(), surface.b(), alpha)
    }

    pub(crate) fn clear_color(theme: Theme) -> [f32; 4] {
        Self::palette_for_theme(theme)
            .surface_root
            .to_normalized_gamma_f32()
    }

    pub(crate) fn interactive_row_fill(
        palette: DesignPalette,
        selected: bool,
        hovered: bool,
        pressed: bool,
    ) -> Color32 {
        if pressed {
            palette.surface_row_active
        } else if selected {
            palette.surface_row_selected
        } else if hovered {
            palette.surface_row_hovered
        } else {
            Color32::TRANSPARENT
        }
    }
}
