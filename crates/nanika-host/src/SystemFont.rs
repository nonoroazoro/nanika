use fontdb::{Database, Family, Query};

use crate::system_font_paths;

/// Cross-platform system UI font data with Latin and CJK glyph coverage.
pub(crate) struct SystemFont {
    pub(crate) data: Vec<u8>,
    pub(crate) family: String,
    pub(crate) index: u32,
}

impl SystemFont {
    pub(crate) fn into_font_definitions(self) -> egui::FontDefinitions {
        let mut definitions = egui::FontDefinitions::default();
        let mut data = egui::FontData::from_owned(self.data);
        data.index = self.index;
        let name = "nanika-system-ui".to_owned();
        definitions.font_data.insert(name.clone(), data.into());
        definitions
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, name.clone());
        definitions
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push(name);
        definitions
    }
}

pub(crate) fn load_system_ui_font() -> Option<SystemFont> {
    for (path, family) in system_font_paths::candidates() {
        let mut database = Database::new();
        if database.load_font_file(path).is_err() {
            continue;
        }
        let Some(id) = database.query(&Query {
            families: &[Family::Name(family)],
            ..Query::default()
        }) else {
            continue;
        };
        if let Some(font) = database.with_face_data(id, |data, index| SystemFont {
            data: data.to_vec(),
            family: family.to_owned(),
            index,
        }) {
            return Some(font);
        }
    }
    None
}
