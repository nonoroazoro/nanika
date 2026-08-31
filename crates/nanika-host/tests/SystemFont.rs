use crate::load_system_ui_font;

#[cfg(any(windows, target_os = "macos"))]
fn mixed_script_glyph_bounds() -> Vec<(char, egui::Rect)> {
    let font = load_system_ui_font().expect("a supported OS must provide a CJK-capable UI font");
    let context = egui::Context::default();
    context.set_fonts(font.into_font_definitions());

    let mut bounds = Vec::new();
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        let galley = ui.painter().layout_no_wrap(
            "Had中文".to_owned(),
            egui::FontId::proportional(32.0),
            egui::Color32::WHITE,
        );
        let row = &galley.rows[0];
        for glyph in &row.glyphs {
            let start = glyph.first_vertex as usize;
            let vertices = &row.visuals.mesh.vertices[start..start + 4];
            let mut rect = egui::Rect::NOTHING;
            for vertex in vertices {
                rect.extend_with(vertex.pos);
            }
            bounds.push((glyph.chr, rect));
        }
    });
    output.drop_without_applying_deltas();
    bounds
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn supported_platform_has_a_cjk_system_font() {
    let font = load_system_ui_font().expect("a supported OS must provide a CJK-capable UI font");

    assert!(!font.data.is_empty());
    assert!(!font.family.is_empty());
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn mixed_script_visual_bounds_are_aligned() {
    let glyphs = mixed_script_glyph_bounds();
    let mut latin = egui::Rect::NOTHING;
    let mut cjk = egui::Rect::NOTHING;

    for (character, bounds) in glyphs {
        if character.is_ascii_alphabetic() {
            latin = latin.union(bounds);
        } else {
            cjk = cjk.union(bounds);
        }
    }

    assert!(latin.is_finite());
    assert!(cjk.is_finite());
    let height_ratio = cjk.height() / latin.height();
    assert!((latin.center().y - cjk.center().y).abs() <= 1.0);
    assert!((1.0..=1.3).contains(&height_ratio));
}
