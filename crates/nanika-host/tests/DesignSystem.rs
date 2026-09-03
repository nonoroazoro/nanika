use crate::DesignSystem;

#[test]
fn install_applies_semantic_system_theme_tokens() {
    let context = egui::Context::default();

    DesignSystem::install(&context);

    let palette = DesignSystem::palette_for_theme(egui::Theme::Dark);
    let style = context.style_of(egui::Theme::Dark);
    assert_eq!(style.spacing.item_spacing, egui::vec2(8.0, 6.0));
    assert_eq!(
        style.visuals.override_text_color,
        Some(palette.text_primary)
    );
    assert_eq!(style.visuals.panel_fill, palette.surface_root);
    assert_eq!(
        style.visuals.widgets.hovered.bg_fill,
        palette.surface_row_selected
    );
    assert_eq!(style.visuals.selection.stroke.color, palette.accent_strong);

    let light = DesignSystem::palette_for_theme(egui::Theme::Light);
    assert_eq!(
        context.style_of(egui::Theme::Light).visuals.panel_fill,
        light.surface_root
    );
}

#[test]
fn interactive_row_states_have_distinct_semantic_fills() {
    let palette = DesignSystem::palette_for_theme(egui::Theme::Dark);
    let idle = DesignSystem::interactive_row_fill(palette, false, false, false);
    let hovered = DesignSystem::interactive_row_fill(palette, false, true, false);
    let selected = DesignSystem::interactive_row_fill(palette, true, false, false);
    let selected_hovered = DesignSystem::interactive_row_fill(palette, true, true, false);
    let active = DesignSystem::interactive_row_fill(palette, true, true, true);

    assert_eq!(idle, egui::Color32::TRANSPARENT);
    assert_ne!(hovered, selected);
    assert_eq!(selected, selected_hovered);
    assert_ne!(selected, active);
}
