use egui::Color32;

/// Semantic colors for one host appearance.
#[derive(Clone, Copy)]
pub(crate) struct DesignPalette {
    pub(crate) text_primary: Color32,
    pub(crate) text_row: Color32,
    pub(crate) text_selected: Color32,
    pub(crate) text_heading: Color32,
    pub(crate) text_detail: Color32,
    pub(crate) text_muted: Color32,
    pub(crate) text_secondary: Color32,
    pub(crate) text_disabled: Color32,
    pub(crate) text_error: Color32,
    pub(crate) text_destructive: Color32,
    pub(crate) surface_root: Color32,
    pub(crate) surface_input: Color32,
    pub(crate) surface_detail: Color32,
    pub(crate) surface_control: Color32,
    pub(crate) surface_row_hovered: Color32,
    pub(crate) surface_row_selected: Color32,
    pub(crate) surface_row_active: Color32,
    pub(crate) surface_action_primary: Color32,
    pub(crate) surface_action_destructive: Color32,
    pub(crate) border_control: Color32,
    pub(crate) border_hovered: Color32,
    pub(crate) border_active: Color32,
    pub(crate) border_detail: Color32,
    pub(crate) accent_strong: Color32,
    pub(crate) selection_fill: Color32,
}
