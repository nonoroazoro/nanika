/// Host-rendered content for one interactive row.
pub(crate) struct InteractiveRowContent<'a> {
    pub(crate) texture_id: Option<egui::TextureId>,
    pub(crate) title: &'a str,
    pub(crate) subtitle: Option<&'a str>,
    pub(crate) accessory: Option<&'a str>,
}
