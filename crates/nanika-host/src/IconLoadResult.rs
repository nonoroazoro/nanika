use crate::IconIdentity;

pub(crate) struct IconLoadResult {
    pub(crate) identity: IconIdentity,
    pub(crate) image: Result<egui::ColorImage, String>,
}
