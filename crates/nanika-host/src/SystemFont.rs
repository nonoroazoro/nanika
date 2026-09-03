use std::path::PathBuf;

use fontdb::{Database, Style, Weight};

use crate::{SystemFontFace, system_font_paths};

/// Cross-platform native UI font stack with a CJK fallback.
pub(crate) struct SystemFont {
    pub(crate) primary: SystemFontFace,
    pub(crate) fallback: SystemFontFace,
}

impl SystemFont {
    pub(crate) fn into_font_definitions(self) -> egui::FontDefinitions {
        let mut definitions = egui::FontDefinitions::default();
        let primary_name = insert_font(&mut definitions, "nanika-system-ui", self.primary);
        let fallback_name = insert_font(&mut definitions, "nanika-system-cjk", self.fallback);
        let proportional = definitions
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default();
        proportional.insert(0, fallback_name.clone());
        proportional.insert(0, primary_name);
        definitions
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push(fallback_name);
        definitions
    }
}

pub(crate) fn load_system_ui_font() -> Option<SystemFont> {
    let primary = load_face(system_font_paths::primary_candidates())?;
    let fallback = load_face(system_font_paths::fallback_candidates())?;
    Some(SystemFont { primary, fallback })
}

fn load_face(candidates: Vec<PathBuf>) -> Option<SystemFontFace> {
    for path in candidates {
        let mut database = Database::new();
        if database.load_font_file(path).is_err() {
            continue;
        }
        let Some(face) = database
            .faces()
            .min_by_key(|face| {
                (
                    face.monospaced,
                    face.style != Style::Normal,
                    face.weight.0.abs_diff(Weight::NORMAL.0),
                )
            })
            .cloned()
        else {
            continue;
        };
        let family = face
            .families
            .first()
            .map(|(family, _)| family.clone())
            .unwrap_or(face.post_script_name);
        if let Some(font) = database.with_face_data(face.id, |data, index| SystemFontFace {
            data: data.to_vec(),
            family,
            index,
        }) {
            return Some(font);
        }
    }
    None
}

fn insert_font(
    definitions: &mut egui::FontDefinitions,
    name: &str,
    face: SystemFontFace,
) -> String {
    let mut data = egui::FontData::from_owned(face.data);
    data.index = face.index;
    let name = name.to_owned();
    definitions.font_data.insert(name.clone(), data.into());
    name
}
